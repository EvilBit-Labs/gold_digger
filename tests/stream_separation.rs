//! stdout/stderr separation contract (todo #176).
//!
//! Gold Digger's UX contract:
//! * Structured output (CSV/JSON/TSV) goes to the file path supplied via
//!   `--output`. Nothing data-shaped lands on stdout for a normal run.
//! * `--dump-config`, `--help`, `--version`, and `--completion` write
//!   their primary payload to stdout (no surrounding diagnostics).
//! * All diagnostics, warnings, errors, and progress feedback go to
//!   stderr via `tracing` so downstream `jq`/`awk` pipelines see clean
//!   stdout.
//!
//! Pre-this-todo, no test asserted that boundary. Color (#161) and
//! tracing-format (#163) work makes regressions cheap to introduce, so
//! this file is the canary. Each test pins one specific contract; new
//! commands should add a row here when they land.

use assert_cmd::cargo;
use predicates::prelude::*;
use tempfile::tempdir;

const ENV_VARS_TO_REMOVE: &[&str] = &["DATABASE_URL", "DATABASE_QUERY", "OUTPUT_FILE", "NO_COLOR"];

/// Build a `gold_digger` invocation with all Clap-bound env vars
/// stripped. Mirrors helpers in the rest of the test suite so a future
/// consolidation can pull from a single source.
fn fresh_cmd() -> assert_cmd::Command {
    let mut cmd = cargo::cargo_bin_cmd!("gold_digger");
    for var in ENV_VARS_TO_REMOVE {
        cmd.env_remove(var);
    }
    cmd.env("NO_COLOR", "1");
    cmd
}

/// `--help` emits to stdout. Stderr must stay empty (no banner, no
/// version chatter, no tracing init noise).
#[test]
fn help_routes_to_stdout_only() {
    fresh_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stderr(predicate::str::is_empty());
}

/// `--version` emits to stdout. Same contract as `--help`.
#[test]
fn version_routes_to_stdout_only() {
    fresh_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("gold_digger"))
        .stderr(predicate::str::is_empty());
}

/// `--dump-config` is the only command that intentionally writes JSON
/// to stdout (the user is expected to pipe it into `jq`). Stderr must
/// be empty so the JSON parses cleanly without grep-style filtering.
#[test]
fn dump_config_routes_json_to_stdout_only() {
    let assert = fresh_cmd().args(["--dump-config"]).assert().success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();

    // Must parse as valid JSON — any stderr leakage into stdout would
    // break this. Validates the routing contract more strictly than a
    // substring match.
    serde_json::from_str::<serde_json::Value>(&stdout)
        .expect("dump-config stdout must be valid JSON");

    assert!(
        stderr.is_empty(),
        "dump-config must emit no stderr, got: {stderr:?}"
    );
}

/// Completion script generation writes the script to stdout. Stderr
/// must stay empty so users can pipe directly into a shell init file
/// without extra filtering.
#[test]
fn completion_routes_to_stdout_only() {
    fresh_cmd()
        .args(["completion", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("gold_digger"))
        .stderr(predicate::str::is_empty());
}

/// Missing config (no DB URL, no env) is a config error: stderr gets
/// the diagnostic, stdout stays empty. Pins the negative case for the
/// "diagnostics on stderr" contract.
#[test]
fn missing_config_error_routes_to_stderr_only() {
    let temp = tempdir().expect("tempdir");
    let out = temp.path().join("data.csv");

    fresh_cmd()
        .args([
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().expect("utf-8 path"),
        ])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Database URL resolution failed"));
}

/// Connection failure (unreachable host) routes the error message to
/// stderr; stdout is empty because the run never produced output.
/// Validates the same contract for the runtime-error path.
#[test]
fn connection_error_routes_to_stderr_only() {
    let temp = tempdir().expect("tempdir");
    let out = temp.path().join("data.csv");

    fresh_cmd()
        .args([
            "--db-url",
            "mysql://baduser:badpass@127.0.0.1:1/db",
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().expect("utf-8 path"),
        ])
        .assert()
        .code(3)
        .stdout(predicate::str::is_empty());
}

/// `--allow-invalid-certificate` emits a `[DANGER]` banner to stderr.
/// Even with verbose disabled, stdout must remain untouched — the
/// security warning is not data-shaped and must not pollute pipes.
/// Snapshot of the exact wording lives in
/// `tests/tls_danger_banner_snapshot.rs`; here we only pin routing.
#[test]
fn allow_invalid_certificate_keeps_stdout_clean() {
    let temp = tempdir().expect("tempdir");
    let out = temp.path().join("data.csv");

    let assert = fresh_cmd()
        .args([
            "--db-url",
            "mysql://baduser:badpass@127.0.0.1:1/db",
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().expect("utf-8 path"),
            "--allow-invalid-certificate",
        ])
        .assert()
        .code(3);

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);

    assert!(
        stdout.is_empty(),
        "[DANGER] banner must not leak to stdout, got: {stdout:?}"
    );
    assert!(
        stderr.contains("[DANGER]"),
        "[DANGER] banner must appear on stderr, got: {stderr:?}"
    );
}
