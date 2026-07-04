---
title: Unified credential redaction in Rust binaries
date: 2026-04-25
last_updated: 2026-06-23
category: best-practices
module: src/utils, src/tls/pool, src/connection
problem_type: best_practice
component: tooling
severity: high
applies_when:
  - Building a Rust binary that handles user-supplied credentials (database URLs, API tokens, secret keys)
  - Errors from upstream libraries can carry credentials in their Display impls (mysql crate, reqwest, etc.)
  - Diagnostic output (--dump-config, --verbose, stderr logs) is shared in bug reports / chats
  - Multiple independent code paths previously implemented their own redactors
related_components:
  - service_object
tags:
  - rust
  - security
  - credential-redaction
  - regex
  - sentinel-testing
  - cwe-532
---

# Unified credential redaction in Rust binaries

## Context

Rust binaries that talk to authenticated services (databases, REST APIs, message brokers) routinely surface credentials in error messages they never wrote. The `mysql` crate's `Error::Display` embeds the raw connection URL, the username, the source IP, and `(using password: YES)` markers. The `reqwest::Error` Display embeds the request URL with userinfo. `url::ParseError` echoes the offending URL back at you. None of this is the binary author's fault; all of it ends up in stderr and on the user's screen.

The naive defense — "I'll redact passwords here, in this one place where I print errors" — fails for three reasons:

1. **Drift across parallel redactors.** A binary accumulates redaction helpers organically: one in `pool.rs` for `Pool::new` errors, one in `dump_config.rs` for the diagnostic dump, one in `main.rs` for the top-level error printer. Each one has slightly different patterns. When a new sensitive field appears in a library's error output, only the redactor whose author noticed gets the fix. The others silently leak.
2. **Silent fail-open regex compilation.** A typo in a pattern, or a regex feature unsupported by the build's regex flavor, fails at `Regex::new` time. The naive `let re = Regex::new(...).unwrap();` is a panic; the slightly less naive `let re = Regex::new(...).ok();` is silently fail-open — the entire pattern set drops the bad regex without telling anyone.
3. **Pattern engineering footguns.** A regex like `(?i)pass\s*[=:]\s*\S+` without a left word boundary matches `boarding_pass=ABC` and turns legitimate output into garbage. A pattern like `(?i)token\s+\S+` (label without `=`/`:`) matches `JSON_TOKEN parser` and produces false positives every time. Both directions of failure are bad: too eager mangles output; too loose leaks credentials. The path of least surprise is `\b` anchors on the left and require `[=:]` on the right.

Gold Digger had three parallel redactors plus a 14-line ad-hoc inline matcher in `dump_configuration`. Each had subtly different patterns. A `mysql::Error` interpolated into a `TlsError` message used the `pool.rs` redactor. The same error displayed by `--dump-config` used the inline matcher. When the test suite added a sentinel (`SeNtInEl_pw_19f3a4`), the sentinel leaked through one path but not the other — and the discovery took an afternoon.

## Guidance

The pattern has six parts. Skipping any one of them leaves a leak surface.

### 1. One module owns redaction. Re-exports / wrappers point back to it.

```rust
// src/utils.rs
pub const REDACTION_PLACEHOLDER: &str = "***REDACTED***";
pub const REDACTED_URL_PLACEHOLDER: &str = "***REDACTED_URL***";

pub fn redact_sql_error(message: &str) -> String { /* ... */ }
pub fn redact_url(url: &str) -> String { /* ... */ }
pub fn redact_dump_query(query: &str) -> String {
    // Delegates so the same pattern set covers errors AND dumps.
    redact_sql_error(query)
}
```

Other modules may keep convenience wrappers (e.g. `tls::redact_url`) *only* as `#[deprecated]` re-exports during migration. The doc comment on the wrapper points to the canonical entry point. Once all callers have migrated, delete the wrapper.

### 2. Fail-closed regex compilation.

```rust
// Source-of-truth defs live in a `const REDACTION_PATTERN_DEFS` slice so a
// `#[cfg(test)]` count assertion can reference the same definition.
fn get_redaction_patterns() -> &'static Vec<(Regex, &'static str)> {
    REDACTION_PATTERNS.get_or_init(|| {
        REDACTION_PATTERN_DEFS
            .iter()
            .filter_map(|(p, r)| match Regex::new(p) {
                Ok(re) => Some((re, *r)),
                Err(e) => {
                    // PANIC in debug so a broken pattern fails the build;
                    // log + drop in release (partial coverage beats aborting
                    // the binary on a redaction-pattern bug).
                    #[cfg(debug_assertions)]
                    panic!("redaction pattern `{p}` failed to compile: {e}");
                    #[cfg(not(debug_assertions))]
                    {
                        tracing::error!("redaction pattern `{p}` failed to compile: {e}");
                        None
                    }
                }
            })
            .collect()
    })
}
```

The combination of (a) panic in debug, (b) a `filter_map` that drops the broken pattern in release, and (c) a separate `#[cfg(test)]` assertion that `get_redaction_patterns().len() == EXPECTED_REDACTION_PATTERN_COUNT` (= `REDACTION_PATTERN_DEFS.len()`) catches three different ways the pattern set can degrade silently — the release-mode `None` makes a dropped pattern show up as a length mismatch the test flags.

### 3. Word-boundary discipline on label patterns.

| Bad                     | Good                        | Why                                                                                                                                        |
| ----------------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `(?i)pass\s*[=:]\s*\S+` | `(?i)\bpass\s*[=:]\s*\S+`   | Without `\b`, `boarding_pass=ABC` is fully redacted.                                                                                       |
| `(?i)token\s+\S+`       | `(?i)\btoken\s*[=:]\s*\S+`  | The bare-space variant matches `JSON_TOKEN parser`. Real secret leaks use `=`/`:` as the separator. Drop the bare-space patterns entirely. |
| `(?i)secret\s+\S+`      | `(?i)\bsecret\s*[=:]\s*\S+` | Same.                                                                                                                                      |

There is a second axis: the **value** side, not just the label. `\S+` matches a non-whitespace run, so it stops at the first space *inside* a quoted SQL literal — `IDENTIFIED BY 'pass word'` redacts only `'pass` and leaks `word'`. SQL DDL/DCL credentials (`IDENTIFIED BY`, `IDENTIFIED WITH ... BY`, `SET PASSWORD = ...`) are quoted and may contain spaces, and MySQL accepts double quotes too (ANSI_QUOTES mode), so the value matcher must accept a single- OR double-quoted literal OR a bare token:

| Bad (leaks quoted tail)             | Good (consumes the quoted value)                          |
| ----------------------------------- | --------------------------------------------------------- |
| `(?i)\bidentified\s+by\s+\S+`       | `(?i)\bidentified\s+by\s+(?:'[^']*'\|"[^"]*"\|\S+)`       |
| `(?i)\bset\s+password\s+...=\s*\S+` | `(?i)\bset\s+password\s+...=\s*(?:'[^']*'\|"[^"]*"\|\S+)` |

`(?:'[^']*'|"[^"]*"|\S+)` means "a single- or double-quoted string (including internal spaces) OR a non-whitespace run." In Rust write it as a `r#"..."#` raw string so the embedded `"` does not close the literal. This matters most for `redact_dump_query`, which scrubs user SQL for `--dump-config` — a quoted password with spaces would otherwise have its tail printed to stdout.

### 4. Cross-redactor idempotence — placeholder consistency matters.

If `redact_url("mysql://alice:secret@host/db")` returns `mysql://alice:***REDACTED***@host/db`, and that string then passes through `redact_sql_error`, the URL-userinfo regex must produce the same placeholder text. Otherwise operators grepping stderr for `***REDACTED***` see only some of the redacted lines.

Two approaches:

- **Single placeholder reused everywhere.** The URL-userinfo replacement in `redact_sql_error` becomes `format!("://{p}:{p}@", p = REDACTION_PLACEHOLDER)`. Then `redact_sql_error(redact_url(x)) == redact_url(x)` — fixed point. If the pattern set is a `const`/`static` array (no `format!` in const context), name the replacement as its own `const URL_USERINFO_REPLACEMENT: &str = "...";` and add a unit test pinning it to `format!("://{p}:{p}@", p = REDACTION_PLACEHOLDER)`, so the literal and the constant cannot silently drift.
- **Make the second redactor a fixpoint over the first's output.** Each pattern in `redact_sql_error` checks "is this already redacted" and skips. More plumbing, less surprising — pick if your placeholder text is constrained.

Add a regression test:

```rust
#[test]
fn redact_sql_error_is_idempotent_over_redact_url_output() {
    let url = "mysql://alice:secret@host:3306/db";
    let r1 = redact_url(url);
    let r2 = redact_sql_error(&r1);
    let r3 = redact_sql_error(&r2);
    assert_eq!(r2, r3, "redact_sql_error must be idempotent over redact_url output");
    assert!(!r2.contains("alice"), "username must be redacted");
    assert!(!r2.contains("secret"), "password must be redacted");
}
```

### 5. Fail-closed URL parsing.

`url::Url::set_password` and `set_username` return `Result<(), ()>`. They fail on parseable-but-cannot-have-userinfo URLs (`data:` URIs, `mailto:`, schemes without an authority). The naive `let _ = redacted.set_password(...)` discards the error — the URL returns un-redacted with original credentials intact:

```rust
pub fn redact_url(url: &str) -> String {
    let Ok(parsed) = url::Url::parse(url) else {
        return REDACTED_URL_PLACEHOLDER.to_string();  // unparseable -> placeholder
    };
    let mut redacted = parsed.clone();
    if parsed.password().is_some()
        && redacted.set_password(Some(REDACTION_PLACEHOLDER)).is_err()
    {
        return REDACTED_URL_PLACEHOLDER.to_string();  // fail closed
    }
    if !parsed.username().is_empty()
        && redacted.set_username(REDACTION_PLACEHOLDER).is_err()
    {
        return REDACTED_URL_PLACEHOLDER.to_string();
    }
    redacted.to_string()
}
```

The placeholder for URL-parse-failure is *distinct* from the regex placeholder (`REDACTED_URL_PLACEHOLDER` vs `REDACTION_PLACEHOLDER`) so test failures can distinguish "URL with credentials redacted" from "URL was unparseable and replaced wholesale."

### 6. Test the redaction pipeline with both corpus and sentinel sweeps.

**Corpus test** — for each labeled pattern, assert that a representative input is correctly redacted. Catches the silent-drop case where a pattern compiles but its left-anchor or replacement is wrong:

```rust
#[test]
fn each_pattern_actually_redacts_its_target() {
    let cases = &[
        ("password=hunter2", "password"),
        ("PASSWORD=hunter2", "password (case)"),
        ("passwd=hunter2", "passwd"),
        ("token=abc123", "token"),
        ("api_key=k1", "api_key"),
        ("api-key=k2", "api-key (hyphen)"),
        ("secret=s1", "secret"),
        ("identified by 'pw'", "identified by"),
        ("kennwort=pw", "kennwort (de)"),
        ("mot_de_passe=pw", "mot_de_passe (fr)"),
        ("set password = 'pw'", "set password"),
        ("mysql://u:p@h/d", "url-userinfo"),
    ];
    for (input, label) in cases {
        let redacted = redact_sql_error(input);
        assert!(
            redacted.contains(REDACTION_PLACEHOLDER) || !redacted.contains(":p@"),
            "pattern `{label}` should redact: `{input}` -> `{redacted}`"
        );
    }
}
```

**Sentinel sweep** — across the full failure-mode test surface, inject a high-entropy sentinel and assert it never appears in captured stdio. More robust than grepping for "password" because the sentinel is guaranteed unique:

```rust
#[test]
fn auth_failure_does_not_leak_credentials() {
    const SENTINEL: &str = "SeNtInEl_pw_29c4f7";
    let url = format!("mysql://baduser:{SENTINEL}@127.0.0.1:1/db");
    let mut cmd = clean_cmd();
    cmd.args(["--db-url", &url, "--query", "SELECT 1", "--output", "/tmp/x.json"]);
    let output = cmd.output().expect("spawn");
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!combined.contains(SENTINEL), "sentinel leaked: {combined}");
    assert!(!combined.contains("password=") && !combined.contains("password:"),
        "literal credential markers leaked: {combined}");
}
```

A single `tests/credential_leak_sweep.rs` should cover every documented failure mode (auth fail, mutual exclusion, bad CA, bad URL scheme, malformed query, unwritable output) with the same sentinel discipline. This is the regression that would have caught Gold Digger's pre-fix drift in a single CI run.

## Why This Matters

The bug class is **CWE-532 (insertion of sensitive information into log file) / CWE-209 (information exposure through error message)**. The symptoms only appear when:

- A user runs the binary against a real database, hits an error, pastes the stderr into a bug report (or worse, a public chat).
- A CI/CD pipeline captures stderr, ships it to a log aggregator, and the aggregator's RBAC is more permissive than the database's.
- A debugging session uses `--verbose` (which lowers the redaction defaults nowhere — the pipeline runs the same redactor regardless of verbosity) but the user shares the verbose output without re-checking it.

None of these surface in unit tests of the redactor in isolation. They surface when the redactor is composed with the binary's full error pipeline AND a real upstream library that embeds credentials in its error output.

## When to Apply

- Any Rust binary that holds credentials in CLI args / env vars / config files AND surfaces upstream errors to the user.
- Any service whose stderr ends up in a log aggregator with broader access than the credential's actual scope.
- Any binary that exposes a `--dump-config` or `--debug` mode.
- Any code path where a `mysql::Error`, `reqwest::Error`, `url::ParseError`, or similar Display-embedding-input error reaches stderr.

The pattern is overkill for a binary that has no errors carrying credentials — but the threshold for "carries credentials" is lower than it looks. `url::ParseError` from a malformed input quotes the input back. `tokio::io::Error` from a connection refused embeds the target. Most upstream errors leak something.

## Examples

The full implementation in this repo:

- **`src/utils.rs`** — `redact_sql_error`, `redact_url`, `redact_dump_query`, `REDACTION_PLACEHOLDER`, `REDACTED_URL_PLACEHOLDER`, `URL_USERINFO_REPLACEMENT` (named const pinned to `REDACTION_PLACEHOLDER` by `url_userinfo_replacement_uses_shared_placeholder`), `EXPECTED_REDACTION_PATTERN_COUNT`, the fail-closed compile, the word-boundary patterns, the quoted-value patterns `(?:'[^']*'|"[^"]*"|\S+)` for `IDENTIFIED BY` / `IDENTIFIED WITH ... BY` / `SET PASSWORD` (single- and double-quoted), the corpus test, and `test_redact_sql_error_quoted_password_with_spaces`.
- **`src/tls/pool.rs::Pool::new` map_err** — every `mysql_error` interpolation goes through `redact_sql_error` BEFORE being embedded in a `TlsError` variant. The credential-redaction discipline lives at the wrap point, not the consumer.
- **`src/run.rs::pool.get_conn()`** — connection errors route through the typed classifier (`classify_mysql_pool_error`) which internally redacts; the `?` site preserves typed errors via `anyhow::Error::from(typed).context(...)` so the redaction stays intact through context layers.
- **`src/main.rs::dump_configuration`** — `--dump-config` query field routes through `redact_dump_query` (which delegates to `redact_sql_error`). The CLI long-help on `--dump-config` documents the "best-effort" caveat so users know the limits.
- **Test surface**:
  - `tests/credential_leak_regression.rs` — sentinel-based regression test for the canonical failure modes.
  - `tests/credential_leak_sweep.rs` — sweep across every documented failure mode for the sentinel + literal markers.
  - `tests/redact_sql_error_truth_table.rs` — corpus-based per-pattern test (rstest one-row-per-pattern).
  - `tests/dump_config_redaction.rs` — adversarial test corpus (GRANT, SET PASSWORD, CREATE USER IDENTIFIED WITH...BY, kennwort, mot_de_passe, contraseña, URL with credentials).
- **Commits**: `549f8e0` (consolidation + adversarial test suites), `5d6458e` (`\b` anchors, REDACTION_PLACEHOLDER consistency, cross-redactor idempotence test), `641110f` (quoted-value matching + `URL_USERINFO_REPLACEMENT` const pinned by test), and the follow-up that broadened the quoted-value matcher to double quotes `(?:'[^']*'|"[^"]*"|\S+)`.

### What this is NOT

- Not a substitute for a SecretString-style type (`secrecy::SecretString`) that prevents credentials from being printed in the first place. If the credential is YOUR data structure (you allocate, you own), wrap it in a `Secret<T>` newtype that doesn't impl `Display`. Redaction is the *defense in depth* layer for credentials that flow through third-party Display impls you don't control.
- Not sufficient for binary outputs (CSV, JSON, query results). The redactor operates on log-line text. If your binary writes user-controlled data to a file, the redactor doesn't see it. Audit your data-flow separately.
- Not a substitute for documented user guidance ("review --dump-config output before sharing"). Best-effort redaction is best-effort. The CLI long-help should say so.

## Related

- `src/utils.rs::get_redaction_patterns` — the fail-closed compile + the `EXPECTED_REDACTION_PATTERN_COUNT` test.
- `src/utils.rs::redact_sql_error` — the canonical entry point.
- `src/utils.rs::redact_url` — fail-closed URL parsing.
- `src/utils.rs::tests::each_pattern_actually_redacts_its_target` — corpus test.
- `tests/credential_leak_regression.rs`, `tests/credential_leak_sweep.rs`, `tests/dump_config_redaction.rs`, `tests/redact_sql_error_truth_table.rs` — full test surface.
- AGENTS.md "🚨 Critical Safety Rules" -> "Security" — the "NEVER log raw DATABASE_URL or credentials - always redact" rule this codifies.
- Companion docs in this folder:
  - `replace-substring-error-classifiers-with-typed-enums-2026-04-25.md` — the typed-classifier work that makes credential redaction reachable from every error path.
