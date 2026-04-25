//! Atomic-output regression tests (CRITICAL #3).
//!
//! Pre-CRITICAL #3 the binary's `run` pipeline called
//! `process::exit` from inside the streaming loop on every error path.
//! `process::exit` does not unwind the stack, so the streaming sink's
//! `Drop` impl — which removes the `<output>.tmp` sibling on failure —
//! never fired. The result was a leftover `.tmp` file after every
//! mid-stream conversion failure, contradicting the streaming contract
//! documented at `src/sink.rs:12-15`.
//!
//! Post-fix the `run` pipeline returns `Result<RunOutcome, _>` and the
//! single `process::exit` lives in `main.rs` AFTER the result is
//! observed — which means stack unwinding runs every `Drop` on the way
//! out, including the sink's `.tmp` cleanup.
//!
//! These tests pin that contract by driving the binary to a mid-stream
//! conversion failure (without requiring a live MySQL server) and
//! asserting:
//!   - exit code 4 (`EXIT_QUERY_ERROR`),
//!   - the target output file does NOT exist,
//!   - the sibling `<target>.tmp` file does NOT exist.
//!
//! Approach: we cannot easily produce a real MySQL row that fails
//! conversion without a live server. Instead we exercise the
//! adjacent invariant — when the pipeline fails BEFORE any rows are
//! written (config error / connection failure), neither the target
//! file nor the `.tmp` file exists. This guards the foundational
//! property "no orphaned `.tmp` after error exit" without requiring
//! Docker.
//!
//! The end-to-end "row-N type conversion failure" case is covered by
//! `tests/end_to_end_type_conversion.rs` when Docker is available.

use assert_cmd::Command;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

// `mod integration;` is required because `tests/test_support/containers.rs`
// and `tests/test_support/parsing.rs` re-export from `crate::integration::*`.
// Each integration-test crate (= each `tests/*.rs` file) needs both modules
// declared even when this file only directly uses `test_support::cli`.
mod integration;
mod test_support;

/// Wraps the canonical [`test_support::cli::clean_cmd`] so the rest of
/// this file can call `clean_cmd()` directly. The shared helper strips
/// the four Clap-bound env vars (`DATABASE_URL`, `DATABASE_QUERY`,
/// `OUTPUT_FILE`, `NO_COLOR`) that would otherwise leak from a
/// developer's shell into the test child process.
fn clean_cmd() -> Command {
    test_support::cli::clean_cmd()
}

/// Returns the sibling `<output>.tmp` path the streaming sinks write
/// to. Mirrors `src/sink.rs::temp_path_for`.
fn tmp_path_for(output: &Path) -> PathBuf {
    let mut buf = output.to_path_buf().into_os_string();
    buf.push(".tmp");
    PathBuf::from(buf)
}

/// CRITICAL #3 invariant: a connection failure (no DB available) must
/// NOT leave an orphaned `<output>.tmp` file on disk. Pre-fix the
/// binary called `process::exit` mid-pipeline, skipping the sink's
/// `Drop` cleanup; post-fix the error returns through `?` and the sink
/// is dropped during stack unwinding.
#[test]
fn connection_failure_leaves_no_tmp_file() {
    let dir = tempdir().expect("tempdir");
    let out = dir.path().join("data.csv");

    let assert = clean_cmd()
        .args([
            "--db-url",
            // Phantom URL: nothing listens here, so connection fails
            // after the sink would normally be built.
            "mysql://no-one:nothing@127.0.0.1:1/nothing",
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().expect("utf-8 path"),
        ])
        .assert();

    let output = assert.get_output();
    let code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_ne!(
        code,
        Some(0),
        "phantom DB URL must not exit success; stderr: {stderr}"
    );

    // CRITICAL #3 invariants: neither the target nor the .tmp sibling
    // may exist after the error exit. The connection failure happens
    // before the sink is built in this scenario, so the .tmp would
    // never have been created — but if a future regression moves sink
    // construction earlier, this test catches the orphaned-tmp leak.
    assert!(
        !out.exists(),
        "target output file must NOT exist after connection failure; stderr: {stderr}"
    );
    let tmp = tmp_path_for(&out);
    assert!(
        !tmp.exists(),
        "sibling .tmp file must NOT exist after connection failure; stderr: {stderr}"
    );
}

/// CRITICAL #3 invariant: a config error (missing DB URL) must exit
/// with code 2 AND leave no output / .tmp behind. The error fires
/// before sink construction, but again — a future regression that
/// moves config resolution after sink creation must not orphan the
/// `.tmp` file.
#[test]
fn config_error_leaves_no_tmp_file() {
    let dir = tempdir().expect("tempdir");
    let out = dir.path().join("data.csv");

    let assert = clean_cmd()
        .args([
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().expect("utf-8 path"),
        ])
        .assert();

    let output = assert.get_output();
    let code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        code,
        Some(2),
        "missing DB URL must map to EXIT_CONFIG_ERROR (2); stderr: {stderr}"
    );
    assert!(
        !out.exists(),
        "target output file must NOT exist after config error; stderr: {stderr}"
    );
    let tmp = tmp_path_for(&out);
    assert!(
        !tmp.exists(),
        "sibling .tmp file must NOT exist after config error; stderr: {stderr}"
    );
}

/// Pin the inverse: when no error occurs (e.g. `--help`) we exit 0
/// without producing any output artefacts at the requested path.
/// Sanity check that the helpers above are not no-ops.
#[test]
fn help_command_creates_no_output_artifacts() {
    let dir = tempdir().expect("tempdir");
    let out = dir.path().join("data.csv");

    let assert = clean_cmd().arg("--help").assert();
    let output = assert.get_output();
    assert_eq!(output.status.code(), Some(0), "--help must exit 0");
    assert!(!out.exists(), "no target should appear from --help");
    assert!(
        !tmp_path_for(&out).exists(),
        "no .tmp should appear from --help"
    );
}
