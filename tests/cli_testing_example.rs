//! Example test demonstrating the enhanced CLI testing with assert_cmd and predicates
//!
//! This test shows how to use the new GoldDiggerCli implementation with:
//! - assert_cmd::Command for robust CLI testing
//! - predicates for stdout/stderr validation
//! - timeout handling using assert_cmd's built-in timeout
//! - insta snapshots for CLI output verification and regression testing
//! - helper functions for common test scenarios (TLS, non-TLS, different formats)
//! - rstest for parameterized testing

#![allow(dead_code)]

use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use rstest::{fixture, rstest};
use std::time::Duration;
use tempfile::NamedTempFile;

/// Clap-bound env vars that must be removed from spawned binaries to prevent
/// developer-shell exports from leaking into integration tests.
const ENV_VARS_TO_REMOVE: &[&str] = &["DATABASE_URL", "DATABASE_QUERY", "OUTPUT_FILE", "NO_COLOR"];

/// Fixture for creating a basic CLI command with all Clap-bound env vars
/// stripped. Tests that need specific env vars set should use `.env(...)`
/// after the fixture is constructed; absent that, the binary sees a clean
/// environment regardless of the developer's shell.
#[fixture]
fn cli_command() -> Command {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("gold_digger");
    for var in ENV_VARS_TO_REMOVE {
        cmd.env_remove(var);
    }
    cmd
}

/// Fixture for creating a temporary output file
#[fixture]
fn temp_output_file() -> NamedTempFile {
    NamedTempFile::new().unwrap()
}

/// Fixture for setting up standard environment variables
#[fixture]
fn standard_env_vars() -> Vec<(&'static str, &'static str)> {
    vec![
        ("DATABASE_URL", "mysql://test:test@localhost/test"),
        ("DATABASE_QUERY", "SELECT 1"),
    ]
}

/// Example test showing basic assert_cmd usage
#[rstest]
fn test_assert_cmd_basic_usage(cli_command: Command) -> Result<()> {
    let mut cmd = cli_command;
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stderr(predicate::str::is_empty());

    Ok(())
}

/// Example test showing timeout handling
#[rstest]
fn test_timeout_handling(cli_command: Command) -> Result<()> {
    let mut cmd = cli_command;
    cmd.timeout(Duration::from_secs(5));
    cmd.arg("--help");

    let output = cmd.output()?;
    assert!(output.status.success());

    Ok(())
}

/// Example test showing predicate usage for output validation
#[rstest]
fn test_predicate_validation(cli_command: Command) -> Result<()> {
    let mut cmd = cli_command;
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(
            predicate::str::contains("Usage:")
                .and(predicate::str::contains("Options:"))
                .and(predicate::str::contains("--db-url")),
        )
        .stderr(predicate::str::is_empty());

    Ok(())
}

/// Example test showing error scenario testing
#[rstest]
fn test_error_scenario(cli_command: Command) -> Result<()> {
    let mut cmd = cli_command;

    // The fixture already strips DATABASE_URL/DATABASE_QUERY/OUTPUT_FILE, so
    // running with no flags should hit the "missing database URL" config path.
    // Per src/exit.rs::EXIT_CONFIG_ERROR this maps to exit 2.
    cmd.assert().code(2).stderr(
        predicate::str::contains("Missing database URL").or(predicate::str::contains("required")),
    );

    Ok(())
}

/// Test environment variable handling with parameterized scenarios.
///
/// Expected exit codes per src/exit.rs:
/// - `both_env_vars_set`: full config but unreachable host -> EXIT_DB_AUTH_ERROR (3)
/// - `only_db_url` / `only_query`: missing config field -> EXIT_CONFIG_ERROR (2)
#[rstest]
#[case("both_env_vars_set", 3)]
#[case("only_db_url", 2)]
#[case("only_query", 2)]
fn test_environment_variables(
    cli_command: Command,
    temp_output_file: NamedTempFile,
    #[case] scenario: &str,
    #[case] expected_exit_code: i32,
) -> Result<()> {
    let mut cmd = cli_command;

    // Set environment variables based on scenario
    match scenario {
        "both_env_vars_set" => {
            cmd.env("DATABASE_URL", "mysql://test:test@127.0.0.1:1/test");
            cmd.env("DATABASE_QUERY", "SELECT 1");
        }
        "only_db_url" => {
            cmd.env("DATABASE_URL", "mysql://test:test@127.0.0.1:1/test");
        }
        "only_query" => {
            cmd.env("DATABASE_QUERY", "SELECT 1");
        }
        _ => panic!("Unknown scenario: {}", scenario),
    }

    cmd.arg("--output").arg(temp_output_file.path());

    cmd.assert().code(expected_exit_code).stderr(
        predicate::str::contains("connection")
            .or(predicate::str::contains("error"))
            .or(predicate::str::contains("Missing"))
            .or(predicate::str::contains("resolution failed")),
    );

    Ok(())
}

/// Example test showing snapshot testing with insta
#[rstest]
fn test_snapshot_testing(cli_command: Command) -> Result<()> {
    let mut cmd = cli_command;

    // Clear env vars so clap does not embed their values in help output,
    // which would make snapshots environment-dependent
    cmd.env_remove("DATABASE_URL")
        .env_remove("OUTPUT_FILE")
        .arg("--help");

    let output = cmd.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Normalize binary name across platforms (gold_digger.exe -> gold_digger)
    let stdout = stdout.replace("gold_digger.exe", "gold_digger");

    // Create snapshot of help output for regression testing
    insta::assert_snapshot!("help_output", stdout);

    Ok(())
}

/// Test different output formats with parameterization.
///
/// Each invocation supplies a complete config but points at an unroutable
/// host, so the connection-establishment failure should map to
/// EXIT_DB_AUTH_ERROR (3) per src/exit.rs.
#[rstest]
#[case("csv")]
#[case("json")]
#[case("tsv")]
fn test_format_specification(
    cli_command: Command,
    temp_output_file: NamedTempFile,
    #[case] format: &str,
) -> Result<()> {
    let mut cmd = cli_command;
    cmd.env("DATABASE_URL", "mysql://test:test@127.0.0.1:1/test")
        .env("DATABASE_QUERY", "SELECT 1 as test_column")
        .arg("--output")
        .arg(temp_output_file.path())
        .arg("--format")
        .arg(format);

    cmd.assert()
        .code(3)
        .stderr(predicate::str::contains("connection").or(predicate::str::contains("error")));

    Ok(())
}

/// Example showing credential redaction testing
#[rstest]
fn test_credential_redaction(cli_command: Command, temp_output_file: NamedTempFile) -> Result<()> {
    let mut cmd = cli_command;

    // Use a database URL with credentials
    cmd.arg("--db-url")
        .arg("mysql://user:password@localhost/db")
        .arg("--query")
        .arg("SELECT 1")
        .arg("--verbose")
        .arg("--output")
        .arg(temp_output_file.path());

    let output = cmd.output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify that credentials are not exposed in output
    assert!(!stderr.contains("password"));
    assert!(!stderr.contains("mysql://user:password"));

    Ok(())
}

/// Test CLI flag precedence with parameterized scenarios.
///
/// All scenarios supply complete configuration pointed at an unroutable
/// host, so the resulting connection failure maps to EXIT_DB_AUTH_ERROR (3).
#[rstest]
#[case(
    "cli_overrides_env",
    "mysql://cli:cli@127.0.0.1:1/cli",
    "mysql://env:env@127.0.0.1:1/env"
)]
#[case("cli_only", "mysql://cli:cli@127.0.0.1:1/cli", "")]
#[case("env_only", "", "mysql://env:env@127.0.0.1:1/env")]
fn test_cli_flag_precedence(
    cli_command: Command,
    temp_output_file: NamedTempFile,
    #[case] _scenario_name: &str,
    #[case] cli_url: &str,
    #[case] env_url: &str,
) -> Result<()> {
    let mut cmd = cli_command;

    if !env_url.is_empty() {
        cmd.env("DATABASE_URL", env_url);
    }

    if !cli_url.is_empty() {
        cmd.arg("--db-url").arg(cli_url);
    }

    cmd.arg("--query")
        .arg("SELECT 1")
        .arg("--output")
        .arg(temp_output_file.path());

    cmd.assert()
        .code(3)
        .stderr(predicate::str::contains("connection").or(predicate::str::contains("error")));

    Ok(())
}

/// Test mutually exclusive options with parameterized conflicting flag combinations
#[rstest]
#[case("verbose_and_quiet")]
#[case("query_and_query_file")]
fn test_mutually_exclusive_options(
    cli_command: Command,
    temp_output_file: NamedTempFile,
    #[case] scenario: &str,
) -> Result<()> {
    let mut cmd = cli_command;

    cmd.arg("--db-url")
        .arg("mysql://test:test@localhost/test")
        .arg("--output")
        .arg(temp_output_file.path());

    // Add conflicting flags based on scenario
    match scenario {
        "verbose_and_quiet" => {
            cmd.arg("--verbose").arg("--quiet");
        }
        "query_and_query_file" => {
            cmd.arg("--query")
                .arg("SELECT 1")
                .arg("--query-file")
                .arg("/tmp/query.sql");
        }
        _ => panic!("Unknown scenario: {}", scenario),
    }

    // Clap rejects mutually-exclusive flags before main runs and exits with
    // its default error code (2), which lines up with EXIT_CONFIG_ERROR.
    cmd.assert().code(2).stderr(
        predicate::str::contains("cannot be used with").or(predicate::str::contains("conflict")),
    );

    Ok(())
}

/// Snapshot-pin the exact clap error wording for `--verbose` + `--quiet`
/// (todo #078). The substring assertion in
/// [`test_mutually_exclusive_options`] is too loose to catch a regression
/// where clap silently drops the conflict marker — the error would still
/// mention "cannot be used with" for some unrelated flag pair. This test
/// captures the verbatim stderr (with `NO_COLOR=1` to keep the snapshot
/// plain ASCII) and pins the exit code separately so a future clap upgrade
/// that changes the wording surfaces as a snapshot diff for review.
#[test]
fn verbose_quiet_mutex_error_snapshot() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("gold_digger");
    for var in ENV_VARS_TO_REMOVE {
        cmd.env_remove(var);
    }
    let output = cmd
        .env("NO_COLOR", "1")
        .args([
            "--db-url",
            "mysql://test:test@localhost/test",
            "--query",
            "SELECT 1",
            "--output",
            "/tmp/gd_verbose_quiet_mutex.csv",
            "--verbose",
            "--quiet",
        ])
        .output()
        .expect("spawn gold_digger");

    assert_eq!(
        output.status.code(),
        Some(2),
        "clap mutex must exit 2 (EXIT_CONFIG_ERROR); stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    insta::assert_snapshot!("verbose_quiet_mutex_error", stderr);
}
