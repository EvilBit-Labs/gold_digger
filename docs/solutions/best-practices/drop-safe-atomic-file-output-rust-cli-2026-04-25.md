---
title: Drop-safe atomic file output in Rust binaries
date: 2026-04-25
last_updated: 2026-06-23
category: best-practices
module: src/run, src/sink, src/main
problem_type: best_practice
component: tooling
severity: high
applies_when:
  - Building a Rust CLI that writes a single output artifact (file, archive, report)
  - main() currently calls process::exit() based on a Result returned from a helper
  - Output uses a .tmp + rename "atomic write" pattern, or std::fs::rename directly
  - A buffered writer / temp file is owned by a Drop type that the binary depends on for cleanup
related_components:
  - service_object
tags:
  - rust
  - atomic-file-write
  - raii
  - process-exit
  - cross-platform-io
---

# Drop-safe atomic file output in Rust binaries

## Context

Rust binaries that produce a file as their primary output (CLI exporters, code generators, build tools) want an atomic-write contract: on success the target path holds a complete file; on failure either the target is untouched or no partial file is visible. The natural Rust idiom is "write to `<output>.tmp`, rename on success, let `Drop` clean up the `.tmp` on error."

There are three traps in that idiom that bite under load and are not obvious from small examples:

1. **`std::process::exit` skips stack unwinding entirely.** Any `Drop` impl that owns the `.tmp` cleanup never runs, and a crash mid-stream leaves a permanent `.tmp` orphan beside the target. Tests that assert on the target path pass; the user's filesystem accumulates `.tmp` files.
2. **`std::fs::rename` is not TOCTOU-safe and silently clobbers the destination on Unix.** The pre-flight `if !force && output.exists()` check has a race window; an attacker (or a concurrent run) can plant a file at the target between the check and the rename, and the rename overwrites it.
3. **An open file handle blocks rename/unlink on Windows.** `std::fs::rename` and `std::fs::remove_file` fail with `AccessDenied` while the `.tmp` is still open (Windows does not open handles with `FILE_SHARE_DELETE` by default). A buffered writer held by value in the sink — rather than dropped before commit/cleanup — flushes its *partial* contents on the final `Drop` and leaves a corrupt `.tmp` that trips the `create_new` stale-tmp guard on the next run. On Unix the same code is harmless (open handles can be renamed/unlinked), so this only surfaces on a first-class Windows target.

Gold Digger's `RowSink` (`src/sink.rs`) hit both. The streaming refactor (F007) introduced the `.tmp → rename` pattern; the original `src/main.rs` called `exit_with_error(...)` (which calls `process::exit`) on every error path; and `commit_tmp` used `std::fs::rename`. Tests passed. Production runs left `.tmp` files behind on every connection-pool error and silently overwrote pre-existing target files.

The half of the fix that lives in `main` was identified by the original comprehensive code review as a code-quality concern (`[H1]` — "`run::main` is mostly `match … exit_with_error(e, …)` boilerplate"). The review framed it purely as DRY, not Drop-safety. The Drop-safety implication only surfaced during the follow-up PR review, which connected `exit_with_error → process::exit → no unwind → no Drop → orphaned .tmp` into one diagnosis. *(session history)*

## Guidance

The pattern has three pieces. All three must be in place — fixing one without the others leaves a bug that hides until production.

### 1. Make the main pipeline return `Result`. Never `process::exit` from inside it.

The pipeline (`run`, `stream_query`, anything that owns a buffered writer or temp file) returns `Result<T, anyhow::Error>` (or your typed error of choice). Every `?` short-circuit unwinds normally. `main` is a thin shim that calls the pipeline, lets the stack unwind on error, and *then* maps the typed error to an exit code at the outermost frame:

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

`exit_with_error` is now reached *after* the stack unwinds, so every Drop between the failure site and `main` runs.

### 2. Open `<output>.tmp` with `create_new(true)`.

A stale tmp from a prior crash should collide instead of being silently truncated. Using `force=true` for the `.tmp` path while `force=false` for the target is asymmetric and lets a co-tenant in the output directory plant a file at the predictable `.tmp` path. Use `create_new` for both.

### 3. Atomic commit uses platform-specific fail-on-clobber primitives.

`std::fs::rename` overwrites silently on Unix. Replace with:

| Platform               | Primitive                                                                                           | Behaviour on existing target                                             |
| ---------------------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Linux                  | `nix::fcntl::renameat2(.., RenameFlags::RENAME_NOREPLACE)`                                          | Single syscall, atomic, returns `EEXIST`                                 |
| macOS / non-Linux Unix | `std::fs::hard_link(tmp, target)` then `fs::remove_file(tmp)`                                       | `hard_link` fails atomically with `EEXIST`; the link is the commit point |
| Windows                | `OpenOptions::new().create_new(true).write(true).open(target)`, then `fs::copy` + `fs::remove_file` | `create_new` is the atomic claim                                         |

When the user explicitly opts into overwrite (`--force` or equivalent), fall back to plain `fs::rename` — it's the documented semantic.

### 4. Two-flag `Drop` guard preserves `.tmp` on commit failure.

The `Drop` impl on the sink needs two distinct flags so a *failed* `finalize` preserves the `.tmp` for user recovery, while an error *before* finalize cleans up:

```rust
struct DelimitedSink {
    // ...
    committed: bool,             // rename succeeded; .tmp is gone
    committed_or_aborted: bool,  // commit_tmp was entered; .tmp may exist
}

impl Drop for DelimitedSink {
    fn drop(&mut self) {
        if self.committed_or_aborted {
            // Either rename succeeded (committed = true; nothing to do)
            // or commit_tmp entered, rename failed, and we deliberately
            // preserved the .tmp for user recovery. Do NOT delete.
            return;
        }
        // Stream errored before commit_tmp ran. Clean up the orphan.
        let _ = std::fs::remove_file(&self.tmp_path);
    }
}
```

On rename failure, return an error that includes the `.tmp` path verbatim:

> "Failed to commit output to `<target>`: \<io_error>. Your data is preserved at `<target>.tmp`; move it manually if the target path becomes writable."

This converts data loss into an inconvenience.

### 5. Close the writer before commit or cleanup (mandatory on Windows).

Wrap the buffered writer in `Option` so both `finalize` and `Drop` can `take()` it — closing the OS file handle — *before* the `.tmp` is renamed or unlinked. On Windows `fs::rename`/`fs::remove_file` fail on an open handle; without the `take()` the failed removal is swallowed and the final `BufWriter` drop flushes partial output, leaving a corrupt `.tmp`.

```rust
struct JsonSink {
    // Option so finalize()/drop() can take() and close the handle first.
    writer: Option<BufWriter<File>>,
    // ...
}

impl Drop for JsonSink {
    fn drop(&mut self) {
        if self.committed_or_aborted {
            return;
        }
        self.writer.take();              // close handle BEFORE unlink (Windows)
        let _ = std::fs::remove_file(&self.tmp_path);
    }
}

fn finalize(mut self: Box<Self>) -> Result<WriteOutcome> {
    let mut w = self.writer.take().context("writer already finalized")?;
    write!(w, "]}}")?;
    w.flush()?;
    drop(w);                             // close handle BEFORE rename (Windows)
    self.committed_or_aborted = true;
    commit_tmp(&self.tmp_path, &self.output, self.force)?;
    // ...
}
```

The symmetric `DelimitedSink` already used `Option<csv::Writer<BufWriter<File>>>` and `writer.take()`; the JSON sink originally held its `BufWriter<File>` by value and skipped the close, which is what leaked the partial `.tmp` on Windows. Keep both sinks on the same `Option`-and-`take()` discipline.

### 6. (Bonus) Type-state at the sink boundary.

A one-bit `SinkState` field (`NeedsHeaders | InRows`) prevents `on_row` before `on_headers` — making invalid call orders unrepresentable at runtime. For the JSON sink, the alternative is a file containing a row before the `{"data":[` preamble. Pair with `#[must_use]` on the trait return values:

```rust
pub trait RowSink {
    #[must_use = "header-write errors must be propagated; ignoring them leaves a partial .tmp file"]
    fn on_headers(&mut self, columns: &[String]) -> Result<()>;
    #[must_use = "row-write errors must be propagated; ignoring them leaves a partial .tmp file"]
    fn on_row(&mut self, row: &mysql::Row) -> Result<()>;
    #[must_use = "finalize errors signal incomplete output; the .tmp will linger if ignored"]
    fn finalize(self: Box<Self>) -> Result<WriteOutcome>;
}
```

## Why This Matters

The bug class is **silent data loss / silent overwrite**. The symptoms only appear in production:

- A `process::exit` from inside the pipeline orphans the `.tmp`. The user re-runs without `--force` and trips the no-clobber check on the new invocation's `.tmp` — confusing, but at least visible.
- `std::fs::rename` silently overwrites whatever happened to be at the target path. If a co-tenant or attacker pre-planted a file there (e.g., `/tmp/results.json`), the gold_digger output replaces it with no warning. This is information disclosure to whoever planted the file (they can later read the query results from the location they chose).
- Without the two-flag Drop guard, a cross-device rename failure (`EXDEV`) triggers a delete of the `.tmp` containing the only complete copy of the user's data. The user gets exit 5 *and* loses minutes-to-hours of query work.

None of these surface in unit tests of the sink in isolation. They surface when the sink is composed with the binary's exit pathway — which is exactly where most test suites stop looking.

## When to Apply

- Any Rust binary that owns a single output file: code generators, transpilers, CLI exporters, build tools.
- Any binary whose `main` is more than ~50 lines and dispatches on multiple error classes (so a single-`process::exit`-at-the-end refactor is worth the ceremony).
- Any output path that is predictable from the user-supplied destination (the `.tmp` collision attack relies on the attacker knowing the path ahead of time — `<output>.tmp` qualifies; `tempfile::NamedTempFile` with a random component does not).

The pattern scales down — even a single-file `Write` is worth the ceremony if a partial write would be observable.

## Examples

The full implementation in this repo:

- **`src/sink.rs`** — `commit_tmp` (the dispatch), the per-platform helpers (`commit_tmp_linux` using `renameat2`, `commit_tmp_unix_via_hardlink`, `commit_tmp_windows`), the `committed` / `committed_or_aborted` flags on `DelimitedSink` and `JsonSink`, the `Option`-wrapped writers (`take()`-before-commit/cleanup so the handle closes first — required on Windows), the `Drop` impls, and the `SinkState` guard.
- **`src/run.rs`** — `run` and `stream_query` returning `Result`, `ProgressGuard` RAII wrapper for the indicatif spinner so `finish_and_clear` fires on every return path automatically.
- **`src/main.rs`** — the single top-level `match` that converts the `Result` to an exit code via `exit_with_error`.
- **Commits**: `1ea0c3d` (Result-returning run + foundation), `e3b9e7e` (per-platform atomic rename + state guard + .tmp preservation), `d42c7cb` (`ResolvedConfig` + `ProgressGuard`), `d8aab09` (`JsonSink` writer wrapped in `Option` + `take()` before commit/cleanup so the handle closes first — fixes a partial-`.tmp` leak on Windows).

### Trade-offs noted during implementation

**Deterministic `.tmp` name vs `tempfile::NamedTempFile`.** The original code review (`[M4]`) suggested `tempfile::NamedTempFile::new_in(parent).persist()`. The actual implementation uses a manual `<output>.tmp` path. The deterministic name was intentional: the user always knows exactly where the partial file is, which is the entire point of the "preserve `.tmp` on rename failure" recovery story. The trade-off is that a sufficiently privileged attacker could pre-create the `.tmp` path as a symlink — which is why `open_tmp_for` uses `create_new(true)` and `O_NOFOLLOW`-equivalent semantics. *(session history)*

**Open follow-ups still relevant to anyone adopting this pattern:**

- **SIGINT.** A SIGINT (Ctrl-C) is not stack unwinding. The Drop guard does not run on SIGINT, so a `.tmp` is orphaned. Wiring a signal handler that walks the active sinks and runs cleanup is a separate problem (tracked in this repo as todo #164). Document SIGINT behaviour explicitly so users don't expect `.tmp` cleanup on Ctrl-C. *(session history)*
- **`--force` CLI flag** as the explicit opt-in to overwrite an existing target is a documented part of the contract. The Linux path takes `force` and switches between `RENAME_NOREPLACE` and plain `rename`; other platforms can't easily express "fail if the target exists, but overwrite if force is set" without conditional logic at every call site. Document the platform asymmetry. *(session history)*

## Related

- `src/sink.rs:11-22` — module-level `//!` doc enumerates the atomic-write contract.
- `src/sink.rs::commit_tmp` and the per-platform helpers (`commit_tmp_linux`, `commit_tmp_unix_via_hardlink`, `commit_tmp_windows`).
- `src/run.rs::ProgressGuard` — companion RAII pattern for the indicatif progress bar (relies on the same Drop-on-unwind contract).
- `src/main.rs` — the top-level `match` that maps `Result → exit code`.
- `tests/atomic_output.rs` — the regression test pinning "no orphaned `.tmp` after a mid-stream failure."
- `tests/sink_atomicity.rs` — the unit-level state-guard tests (sink rejects `on_row` before `on_headers`).
- AGENTS.md "Closed gaps" section — `F007` reference for the streaming pipeline this builds on.
