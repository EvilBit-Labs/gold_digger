---
title: Front-load validation with a ResolvedConfig type
date: 2026-04-25
category: best-practices
module: src/config, src/cli, src/main, src/run
problem_type: best_practice
component: tooling
severity: medium
applies_when:
  - Building a CLI binary with multiple input sources (CLI flags, env vars, config files)
  - The clap-derive Cli struct is currently passed past main() into core logic
  - Resolution helpers (resolve_database_url, resolve_database_query, etc.) are called separately at multiple points
  - Mutually-exclusive flag pairs are clap-validated at parse time but the struct can hold both filled in afterwards
related_components:
  - service_object
tags:
  - rust
  - clap
  - parse-dont-validate
  - type-state
  - config-resolution
---

# Front-load validation with a ResolvedConfig type

## Context

CLI parsers like `clap` produce a struct (`Cli`) of optional, unvalidated fields. The natural temptation is to pass `Cli` everywhere — every downstream function knows what flags exist, can read whichever ones it needs, and the parser already enforced clap's `conflicts_with` / `requires` constraints at parse time.

The problem isn't that approach in isolation. It's what happens when:

1. **Resolution logic gets duplicated.** `resolve_database_url`, `resolve_database_query`, `resolve_output_file` each re-implement the "if cli.X then ... else env::var ... else error" rule. Three places to forget the env fallback. Three places to update if precedence changes.
2. **Re-validation of clap-validated invariants.** clap's `conflicts_with` only fires at parse time. After parse, the struct can hold both `cli.query: Some(_)` and `cli.query_file: Some(_)` — a callsite that mutates the struct, or constructs it from a non-clap source (e.g. a test fixture), can produce an "impossible" state. Downstream code defensively re-checks: "if let Some(q) = cli.query else if let Some(qf) = cli.query_file…"
3. **Silent precedence drift.** A forgotten `Some(&_)` pattern bug in any one of the three resolvers silently broke CLI > env precedence. The tests that asserted "CLI wins" passed for `db_url` but quietly stopped asserting it for `output_file` because the bug was scoped to that resolver. The bug only surfaced in production.
4. **Config dumps drift from the live config.** `--dump-config` walks `Cli` and `env`. The resolvers walk `Cli` and `env`. They take slightly different paths through the precedence rules. When the dump says "DATABASE_URL = ..." and the actual run uses a different value, debugging is hours.

The fix is *parse, don't validate* (Alexis King): project the unvalidated parser output into a fully-resolved, fully-typed `ResolvedConfig` exactly once at the binary boundary. Every downstream stage takes `&ResolvedConfig` and never touches `Cli`.

## Guidance

### 1. Define a `ResolvedConfig` whose fields are non-optional and pre-validated.

```rust
pub struct ResolvedConfig {
    pub database_url: String,
    pub query: ResolvedQuery,
    pub output: OutputTarget,
    pub tls: TlsConfig,
    pub allow_empty: bool,
    pub pretty: bool,
    pub force: bool,
    pub verbose: u8,
    pub quiet: bool,
}

pub enum ResolvedQuery {
    Inline(String),
    File { path: PathBuf, contents: String },
}

pub struct OutputTarget {
    pub path: PathBuf,
    pub format: OutputFormat,
}
```

All resolution, IO, and cross-field validation that depends only on `(Cli, EnvSnapshot)` happens at construction time:

- Query files are read from disk (so IO failures surface up-front, not three layers into `run`).
- Output formats are determined (so an unknown extension fails before the connection attempt).
- TLS configurations are materialised (so a bad CA path fails fast).

### 2. Mutually-exclusive flags become enum variants.

`--query` vs `--query-file` collapses into `ResolvedQuery::Inline | File`. The type system makes "both set" unrepresentable. Downstream code is exhaustive over the enum, not defensive.

### 3. Sub-types carry their own invariants.

`OutputTarget { path, format }` bundles the resolved path with the resolved format so no downstream code re-runs the extension dispatch. `TlsConfig` (the existing newtype, here aliased to `TlsValidationMode`) carries a `CaFile` that's been validated at construction. Composability beats flat structs once you have more than two or three fields with cross-cutting invariants.

### 4. Snapshot the environment once.

```rust
pub struct EnvSnapshot {
    pub database_url: Option<String>,
    pub database_query: Option<String>,
    pub output_file: Option<String>,
}

impl EnvSnapshot {
    pub fn from_process_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL").ok(),
            database_query: std::env::var("DATABASE_QUERY").ok(),
            output_file: std::env::var("OUTPUT_FILE").ok(),
        }
    }
}
```

Reading env vars exactly once at startup means concurrent env mutations from a parent process can't shift resolved values mid-run (CWE-284). `from_cli_with_env(cli, env)` becomes deterministic — same inputs, same output — so unit tests can construct `EnvSnapshot` from a literal value without any environment-mutation dance.

### 5. The single construction site is in `main`, immediately after parse.

```rust
fn main() {
    let cli = Cli::parse();
    init_subscriber(&cli);
    let resolved = match ResolvedConfig::from_cli(&cli) {
        Ok(r) => r,
        Err(e) => exit_with_error(e.into(), Some("Configuration error")),
    };
    match gold_digger::run::run(&resolved) {
        Ok(()) => exit_success(None),
        Err(e) => exit_with_error(e, None),
    }
}
```

`run::run` and every downstream function takes `&ResolvedConfig`. They never touch `Cli`. There is no "is this validated yet?" footgun because the type only exists in a validated state.

### 6. Document the documented exception.

`--dump-config` deliberately bypasses `ResolvedConfig::from_cli` because operators reach for the dump *precisely when their config is incomplete and they need to see what the binary sees*. Requiring full resolution would defeat the diagnostic. Document this exception inline so future readers don't "fix" it.

## Why This Matters

Before this refactor, `gold_digger`'s `main.rs` re-walked the parsed `Cli` at every stage and called `resolve_database_url`, `resolve_database_query`, `resolve_output_file` separately. Each resolver re-read the env, re-validated, and re-failed independently — and a forgotten `Some(&_)` pattern bug in one of them silently broke CLI > env precedence. The single construction site at `src/main.rs::main` makes the precedence rule one-line obvious; tests once-trusted apply everywhere thereafter.

Secondary wins:

- `--dump-config` and the live run are now driven by the same resolution path. They cannot disagree.
- Errors from config resolution route through `GoldDiggerError::Config(ConfigError::*)` — typed, downcast-able, and tied to the exit-code contract. (See the "Replace substring error classifiers with typed enums" doc in this folder for that side of the story.)
- The mutually-exclusive flag check (`--query` vs `--query-file`) is one runtime check at construction time, not five places of defensive `if let Some(_) … else if let Some(_) …`.

## When to Apply

- Any binary whose `main` is more than ~50 lines.
- Any tool with multiple input sources (CLI, env, config file) where precedence rules matter.
- Any service that needs deterministic config behaviour during a run (env-snapshotting prevents external mutation racing the run).
- Any codebase that has accumulated parallel `resolve_*` helpers — that's the signature smell.

The pattern does not apply to *every* CLI binary. A 30-line tool that takes one flag and exits has no resolution layer to consolidate. The threshold is roughly: "more than one fallback rule (CLI > env), or more than one mutually-exclusive flag pair, or more than one stage that needs to consult config."

## Examples

The full implementation in this repo:

- **`src/config.rs::ResolvedConfig`** — the resolved struct.
- **`src/config.rs::ResolvedQuery`** — `Inline(String) | File { path, contents }`.
- **`src/config.rs::OutputTarget`** — `{ path, format }` with format resolution baked in.
- **`src/config.rs::EnvSnapshot::from_process_env`** — single env read.
- **`src/config.rs::ResolvedConfig::from_cli` / `from_cli_with_env`** — construction site. Returns `Result<Self, GoldDiggerError>` with typed `ConfigError` variants for every failure mode.
- **`src/main.rs::main`** — the one place `ResolvedConfig::from_cli` is called. After this point, `cli` is dead code.
- **`src/run.rs::run(&ResolvedConfig)`** — the downstream consumer. Never touches `Cli`.
- **`src/connection.rs::create_database_connection(&str, &TlsConfig, u8)`** — takes pre-resolved primitives, not `&Cli`. The resolver shoulders the TLS-options-to-config translation.
- **Commit**: `d42c7cb` (introduces `ResolvedConfig` + `--max-query-file-size` flag + `ProgressGuard` RAII).

### The `--max-query-file-size` example

The size-limit knob illustrates the pattern's compounding payoff. Adding `--max-query-file-size <BYTES>` required:

1. Add a clap field on `Cli`.
2. Plumb the value through `ResolvedConfig::from_cli_with_env` (a `validate_query_file_path_with_limit(path, limit)` helper).
3. Update `ResolvedQuery::File` construction to use the limit.

Three changes, all in `src/config.rs` (plus the clap field). No downstream code in `src/run.rs` or `src/connection.rs` had to know the limit existed — they consume `ResolvedQuery`, which already has the post-validation contents. The pattern composed.

### Anti-pattern to avoid: hybrid `Cli` + `ResolvedConfig`

A common half-step is to keep `&Cli` available downstream "just for the booleans (`pretty`, `quiet`, `force`)" while the heavy fields go into `ResolvedConfig`. This defeats the type-state guarantee — every downstream function still has to remember which struct holds what. Either commit fully (every behavioral flag lives on `ResolvedConfig`) or don't bother with the refactor.

## Related

- `src/config.rs:34-66` — `EnvSnapshot` and `from_process_env`.
- `src/config.rs:307-314` — `ResolvedQuery` enum.
- `src/config.rs:339-360` — `OutputTarget` struct.
- `src/config.rs:378-453` — `ResolvedConfig` and the `from_cli_with_env` construction sequence.
- `src/main.rs::main` — the single call site.
- `src/run.rs::run` — first downstream consumer.
- AGENTS.md "Closed gaps" section — `F001-F003` reference for the CLI-first config pipeline this codifies.
- Companion docs in this folder:
  - `drop-safe-atomic-file-output-rust-cli-2026-04-25.md` — the Drop + rename pattern that the streaming output side uses.
  - `replace-substring-error-classifiers-with-typed-enums-2026-04-25.md` — typed `GoldDiggerError::Config(ConfigError::*)` is the failure surface this resolver returns.
