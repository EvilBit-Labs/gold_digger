---
title: Replace substring error classifiers with typed enums + downcast
date: 2026-04-25
last_updated: 2026-06-23
category: best-practices
module: src/exit, src/connection, src/tls/pool
problem_type: best_practice
component: tooling
severity: high
applies_when:
  - Building a Rust binary with a documented exit-code contract (CI/automation switches on exit codes)
  - Currently classifying errors via `error.to_string().to_lowercase().contains(...)` keyword matching
  - Error messages can carry user-controlled input (server hostnames, query text, file paths)
  - Using `anyhow` for error propagation and want to retain typed-error guarantees through context layers
related_components:
  - service_object
tags:
  - rust
  - error-handling
  - thiserror
  - anyhow
  - exit-codes
  - typed-errors
---

# Replace substring error classifiers with typed enums + downcast

## Context

A common anti-pattern in Rust binaries is classifying errors by lowercased substring match:

```rust
fn map_error_to_exit_code(error: &anyhow::Error) -> i32 {
    let s = error.to_string().to_lowercase();
    if s.contains("missing") || s.contains("invalid") { return 2; }
    if s.contains("authentication") || s.contains("denied") { return 3; }
    if s.contains("syntax") { return 4; }
    if s.contains("io") || s.contains("permission") { return 5; }
    1
}
```

Three problems compound here:

1. **Every error message in the codebase becomes a versioned API.** Rewording a single `bail!("Missing database URL")` to "Database URL not provided" silently shifts the exit code from 2 to 4. Automation that switches on exit codes breaks. There's no compile-time signal.
2. **The classifier is attacker-steerable** when error messages embed user-controlled input. A server hostname containing the substring "denied" misroutes a network failure to the auth-failure exit code; a query containing "syntax" inside a string literal misroutes a missing table to the syntax-error code. CI/CD pipelines that gate on exit codes are then influenced by data the attacker controls.
3. **Refactoring is unsafe by default.** A reasonable cleanup ("the word 'invalid' is too noisy in the keyword set; let's drop it") changes the exit code for any caller whose typed error happened to contain "invalid" in its Display impl.

Gold Digger had exactly this shape in `src/exit.rs` and a matching 80-line substring closure in `src/tls/pool.rs::Pool::new`. The fix was the typed- enum + downcast pattern below — but the *full* fix had to land at every call site that constructed errors, not just at the classifier.

## Guidance

The pattern has four parts. Skipping any one of them leaves the substring classifier as the de-facto routing mechanism for the missed sites.

### 1. Define a typed enum with explicit `exit_code()` per variant.

Use `thiserror` for the Display derivation; nest sub-enums for finer- grained typing without flattening:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GoldDiggerError {
    #[error("query returned no rows")]
    NoRows,
    #[error("{0}")]
    Config(#[from] ConfigError),
    #[error("database authentication failed: {0}")]
    DbAuth(String),
    #[error("query error: {0}")]
    Query(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TLS error: {0}")]
    Tls(#[from] TlsError),
}

impl GoldDiggerError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::NoRows => 1,
            Self::Config(_) => 2,
            Self::DbAuth(_) => 3,
            Self::Query(_) => 4,
            Self::Io(_) => 5,
            Self::Tls(t) => t.exit_code(),
        }
    }
}
```

Sub-enums (`ConfigError`, `TlsError`) carry their own variants so the inner `String` of `Config(ConfigError::MissingDbUrl)` is replaced by a typed `ConfigError::MissingDbUrl` variant — eliminates assertions that read message text and gives `match` exhaustiveness.

### 2. The classifier walks the anyhow chain looking for downcasts. Substring matching is the legacy fallback only.

```rust
pub fn map_error_to_exit_code(error: &anyhow::Error) -> i32 {
    // Typed path: walk the chain, prefer the closest typed match.
    for cause in error.chain() {
        if let Some(typed) = cause.downcast_ref::<GoldDiggerError>() {
            return typed.exit_code();
        }
        if let Some(tls) = cause.downcast_ref::<TlsError>() {
            return tls.exit_code();
        }
        if cause.downcast_ref::<std::io::Error>().is_some() {
            return EXIT_IO_ERROR;
        }
    }
    // Substring fallback: only reached if no typed variant in the chain.
    // Mark this branch as deprecated and migrate sites to typed errors;
    // the goal is to delete this entire fallback.
    legacy_substring_classify(&error.to_string().to_lowercase())
}
```

`anyhow::Error::chain()` walks every layer the error has accumulated via `.context()`. As long as each call site preserves the typed value (next section), the chain walker finds it.

### 3. At every `?` boundary, preserve the typed value through `anyhow::Error::from(typed).context(...)`.

This is the failure mode that bites silently. Both forms compile, both produce reasonable user-facing output, but only one keeps the typed value visible to the downstream classifier:

```rust
// BROKEN: flattens TlsError to a string. The downcast walker sees only
// String — your typed enum is invisible past this point.
.map_err(|tls_error| anyhow::anyhow!("{}", tls_error))

// FIXED: preserves the typed TlsError through the anyhow chain. The
// classifier still finds it via downcast even after .context() layers.
.map_err(|tls_error| anyhow::Error::from(tls_error).context(prefix))
```

Same pattern for `?` propagation:

```rust
// BROKEN
let conn = pool.get_conn()
    .map_err(|e| anyhow::anyhow!("Database connection failed: {}", e))?;

// FIXED — typed classification preserved end-to-end
let conn = pool.get_conn()
    .map_err(|e| anyhow::Error::from(classify_mysql_pool_error(e))
        .context("Database connection failed"))?;
```

The classifier `classify_mysql_pool_error` does typed matching on `mysql::Error` variants AND routes the error string through `redact_sql_error` before interpolating — so credentials in MySQL error messages don't leak past this boundary either. A single point of discipline.

### 4. Property-test the typed path so accidental message dependence is caught at CI time.

```rust
proptest! {
    #[test]
    fn proptest_typed_config_is_stable(text in any::<String>()) {
        let err: anyhow::Error = GoldDiggerError::Config(
            ConfigError::Other(text)
        ).into();
        assert_eq!(map_error_to_exit_code(&err), EXIT_CONFIG_ERROR);
    }
}
```

Random-string payloads inside `Config(..)` / `DbAuth(..)` / `Query(..)` must always map to the same exit code. This guards against future refactors that accidentally make the typed path consult message text (e.g., a developer who "improves" the classifier by reading `error.to_string()` first and falling through to downcast only on keyword-miss has broken the contract — proptest catches it on the next run).

## Why This Matters

Real misclassifications observed in this codebase before the fix:

- An error message containing the substring `"invalid"` (intended to match config errors like "Invalid configuration") matched any error whose Display happened to use the word — including TLS variants like `"Invalid TLS message received from server"`. Routed to exit 2 (config) instead of exit 3 (DB auth). CI pipelines that retried on exit 3 (auth failures, transient network) didn't retry on exit 2 (config errors, permanent) — so genuine transient failures became hard exits.
- The TLS classifier in `pool.rs` was an 80-line cascade of nested substring checks. The typed `mysql::Error` enum was already available but unused; once the classifier moved to a typed `match`, the cascade collapsed to ~25 lines AND every variant was statically required to have an explicit routing decision.
- `connection.rs` originally interpolated `TlsError` via `anyhow!("{}", tls_error)` at six call sites. Each site flattened the typed enum to a string before it reached the classifier, so the substring fallback fired even though the typed enum existed. The fix was mechanical — replace `anyhow!("{}", typed)` with `anyhow::Error::from(typed).context(...)` at every site — and exit-code stability for TLS errors became a compiler-enforced property afterward.
- `run.rs::map_query_error` (the query-execution hot path) was the last major un-typed site. It returned `anyhow!("{}: {}", context, redact_sql_error(e))`, embedding the *server-controlled* MySQL message, then let the substring fallback classify it. A query failure referencing an ordinary identifier — a table or column named `connection_log`, or a partition named `missing` — matched the `"connection"`/`"missing"` keywords (checked before `"query"`/`"sql"`) and misrouted to exit 3 (DB auth) or exit 2 (config) instead of exit 4 (query). Reachable with no adversarial input. Fixed by returning `GoldDiggerError::Query` for query-class errors and `GoldDiggerError::DbAuth` for connection-class errors (a transport drop mid-query — `IoError` or `DriverError(CouldNotConnect | ConnectTimeout)` — detected by `is_connection_class_error`; note the mysql crate surfaces these as `DriverError`/`IoError`, *not* as a `MySqlError` carrying a `CR_*` code, so the `CR_*` arm is defensive-only), so the typed downcast path classifies it and the message wording is irrelevant. Regression tests pin the connection-named-identifier case and the real `IoError`/`DriverError` connection paths.

The bug class is **silent classification drift**. Symptoms:

- A passing test suite that misroutes errors in production
- An exit-code contract that survives the unit-test layer but regresses on innocuous message-text refactors
- Operators who can't trust that exit code 4 means "your query was bad" versus "the server caught fire in a way that produces an error message containing 'syntax'"

## When to Apply

- Any Rust binary with a documented exit-code contract that other systems depend on (CI, monitoring, automation).
- Any service that dispatches on error class (retry vs fail-fast, alert-vs-log, fail-open vs fail-closed).
- Any error path that handles user-controlled input that ends up in `Display` impls — this includes hostname, file path, query text, and any `mysql::Error` whose message embeds server output.
- Any codebase using `anyhow` end-to-end where the temptation to `anyhow!("{}", typed)` exists. The pattern is the standard idiom but it silently destroys typed-error guarantees.

The pattern does not require dropping `anyhow` for typed errors at every boundary — only at the boundaries where the classifier downstream needs to see the type. Internal helpers can keep `anyhow::Result<T>` freely.

## Examples

The full implementation:

- **`src/exit.rs`** — `GoldDiggerError` enum, the downcast-then-fallback classifier (`map_error_to_exit_code`), 1024-case proptest invariants (`proptest_typed_*_is_stable`).
- **`src/tls/pool.rs::classify_mysql_pool_error`** — typed switch on `mysql::Error` variants. The `mysql::Error::TlsError` arm forwards to `TlsError::from_rustls_error` (`src/tls/classifier.rs`), which is itself an exhaustive match on `rustls::Error` variants.
- **`src/connection.rs`** — every call site preserves the typed `TlsError` through `anyhow::Error::from(tls_error).context(prefix)`. Suggestions (e.g., `"Use --tls-ca-file"`) are emitted as separate `tracing::error!` lines so the typed value isn't sacrificed for user-facing context.
- **`src/run.rs`** — `pool.get_conn()` failures route through `classify_mysql_pool_error` for the same reasons. This was originally a `anyhow!("Database connection failed: {}", e)` site that bypassed both the typed classifier AND credential redaction. `map_query_error` likewise returns typed `GoldDiggerError::Query` / `DbAuth` (connection-class `IoError`/`DriverError` routed to exit 3 via `is_connection_class_error`) instead of an untyped `anyhow!`, with unit tests asserting exit codes for a connection-named identifier, real `IoError`/`DriverError` connection failures, and a leak-free access-denied message.
- **Commits**: `1ea0c3d` (foundation: typed enum + downcast classifier + preservation in `connection.rs`), `5d6458e` (`pool.get_conn()` typed routing), `19629b3` (`map_query_error` typed `Query`/`DbAuth` on the query-execution path).

### Subtle gotcha: `From` vs `Into` chains

```rust
// Both compile and look equivalent. Only the first preserves typing.
return Err(anyhow::Error::from(typed_err).context("..."));     // ✅
return Err(anyhow::anyhow!("{}", typed_err).context("..."));   // ❌
return Err(anyhow::anyhow!("{:?}", typed_err).context("..."));  // ❌
return Err(typed_err.into());  // ✅ (uses From<TypedErr> for anyhow::Error)
```

`anyhow!("{}", typed)` invokes Display, allocating a String. The typed value is gone from the chain. `Error::from(typed)` (and `typed.into()`) preserves the original error in the chain — the classifier walker can still find it past every `.context()` layer.

A grep that catches the bad pattern in code review:

```bash
rg -n 'anyhow::anyhow!\("\{[:?]?\}", *\w+_(err|error)' src/
```

If a hit appears at an error boundary that downstream code is supposed to classify, it's a bug.

## Related

- `src/exit.rs:106-130` — `GoldDiggerError` enum.
- `src/exit.rs:194-282` — `map_error_to_exit_code` chain walker + legacy substring fallback (kept until all sites are typed).
- `src/exit.rs::proptest_*` — message-stability invariants.
- `src/tls/pool.rs::classify_mysql_pool_error` — typed `mysql::Error` classifier; the `pub(crate)` boundary so `run.rs` can reuse it.
- `src/tls/classifier.rs::from_rustls_error` — typed `rustls::Error` classifier; exhaustive over the rustls variant set with documented Debug-only fallthrough for variants that don't implement Display.
- `src/mysql_errors.rs` — named `ER_*` / `CR_*` constants for MySQL error codes (1064, 1146, 1054, etc.) so the classifier matches on symbolic names rather than magic numbers.
- `src/utils.rs::redact_sql_error` — credential redaction the typed classifiers route through. Substring classifiers that bypass typed routing also bypass redaction; another reason to typify.
- AGENTS.md "Closed gaps" section — `F005` reference for the exit-code contract this protects.
