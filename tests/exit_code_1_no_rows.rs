//! End-to-end exit-code coverage for empty result sets (todo #077, F005).
//!
//! `src/run.rs::handle_empty_result` branches on `--allow-empty`:
//!
//! - default: drop the streaming sink (its `Drop` cleans the `.tmp`),
//!   exit with [`gold_digger::exit::EXIT_NO_ROWS`] (1).
//! - `--allow-empty`: finalize the sink (commit an empty
//!   `{"data":[]}` JSON envelope or header-only CSV/TSV), exit 0.
//!
//! `src/exit.rs` and `src/sink.rs` cover this in process; the gap was a
//! real-binary E2E run pinning both flag positions against an actually
//! empty result set. This file fills that gap by spinning up a MySQL
//! container, running a query that returns zero rows, and asserting the
//! documented exit codes plus output-file disposition.
//!
//! Skipped automatically when Docker is unavailable (CI runners without
//! a Docker daemon, sandboxed local builds) so the suite stays green
//! without a daemon.

mod fixtures;
mod integration;
mod test_support;

use anyhow::Result;
use assert_cmd::cargo;
use std::path::Path;

use crate::integration::{
    TestDatabase,
    containers::{database_container::DatabaseContainer, utils::skip_if_no_docker},
};
use crate::test_support::cli::ENV_VARS_TO_REMOVE;

/// SQL guaranteed to return zero rows on an empty seed schema. Uses
/// `WHERE 1=0` so it parses on MySQL and MariaDB without needing any
/// particular table to exist beyond `information_schema`.
const EMPTY_QUERY: &str = "SELECT table_name FROM information_schema.tables WHERE 1=0";

/// Build a `gold_digger` invocation with all Clap-bound env vars
/// stripped — same hygiene every other integration test in this crate
/// applies (project memory: env-leakage from developer shells is the
/// recurring failure mode for `assert_cmd` runs).
fn fresh_cmd() -> assert_cmd::Command {
    let mut cmd = cargo::cargo_bin_cmd!("gold_digger");
    for var in ENV_VARS_TO_REMOVE {
        cmd.env_remove(var);
    }
    cmd
}

/// Default empty result -> EXIT_NO_ROWS (1), no output file committed.
#[test]
fn empty_result_default_exits_no_rows() -> Result<()> {
    skip_if_no_docker()?;

    let container = DatabaseContainer::new(TestDatabase::mysql())?;
    let temp_dir = tempfile::tempdir()?;
    let output_path = temp_dir.path().join("empty.json");

    let assert = fresh_cmd()
        .args([
            "--db-url",
            container.connection_url(),
            "--query",
            EMPTY_QUERY,
            "--output",
            output_path.to_str().expect("utf-8 path"),
            // testcontainers MySQL exposes a self-signed cert; bypass
            // platform validation to keep this test focused on the
            // empty-result branch in handle_empty_result, not on TLS.
            "--allow-invalid-certificate",
        ])
        .assert()
        .code(1);

    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No records found"),
        "expected 'No records found' diagnostic on stderr, got: {stderr}"
    );

    // Default branch drops the sink without finalize: the `.tmp` is
    // cleaned up and the final output path is never created.
    assert!(
        !Path::new(&output_path).exists(),
        "default empty result must NOT commit an output file, found: {}",
        output_path.display()
    );

    Ok(())
}

/// `--allow-empty` empty result -> exit 0, empty envelope committed.
#[test]
fn empty_result_allow_empty_exits_success() -> Result<()> {
    skip_if_no_docker()?;

    let container = DatabaseContainer::new(TestDatabase::mysql())?;
    let temp_dir = tempfile::tempdir()?;
    let output_path = temp_dir.path().join("empty_allowed.json");

    fresh_cmd()
        .args([
            "--db-url",
            container.connection_url(),
            "--query",
            EMPTY_QUERY,
            "--output",
            output_path.to_str().expect("utf-8 path"),
            "--allow-empty",
            "--allow-invalid-certificate",
        ])
        .assert()
        .success();

    // `--allow-empty` finalizes the sink: the file is committed with an
    // empty `{"data":[]}` JSON envelope. The exact byte content is
    // covered by `src/sink.rs::tests::json_sink_empty_result_emits_envelope`;
    // here we just assert the file exists and parses as JSON with an
    // empty `data` array.
    let raw = std::fs::read_to_string(&output_path)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).expect("--allow-empty JSON output must be valid JSON");
    assert_eq!(
        parsed["data"],
        serde_json::json!([]),
        "--allow-empty must emit `{{\"data\":[]}}` for an empty result; got: {raw}"
    );

    Ok(())
}

/// CSV variant: `--allow-empty` commits a header-only file. Pins the
/// dispatch contract that the format selection still applies on the
/// empty path.
#[test]
fn empty_result_allow_empty_csv_writes_header_only() -> Result<()> {
    skip_if_no_docker()?;

    let container = DatabaseContainer::new(TestDatabase::mysql())?;
    let temp_dir = tempfile::tempdir()?;
    let output_path = temp_dir.path().join("empty_allowed.csv");

    fresh_cmd()
        .args([
            "--db-url",
            container.connection_url(),
            "--query",
            EMPTY_QUERY,
            "--output",
            output_path.to_str().expect("utf-8 path"),
            "--allow-empty",
            "--allow-invalid-certificate",
        ])
        .assert()
        .success();

    let raw = std::fs::read_to_string(&output_path)?;
    // Header-only CSV: one non-empty line, no trailing data rows.
    let non_empty_lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        non_empty_lines.len(),
        1,
        "expected header-only CSV (1 non-empty line); got {} lines:\n{}",
        non_empty_lines.len(),
        raw
    );

    Ok(())
}
