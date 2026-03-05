use assert_cmd::cargo;
use predicates::prelude::*;

#[test]
fn test_exit_code_config_error() {
    // Test missing database URL (should exit with code 2)
    // Clear env vars to prevent leakage from user's shell
    cargo::cargo_bin_cmd!("gold_digger")
        .env_remove("DATABASE_URL")
        .env_remove("DATABASE_QUERY")
        .env_remove("OUTPUT_FILE")
        .args(["--query", "SELECT 1", "--output", "/tmp/test.csv"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Database URL resolution failed"));
}

#[test]
fn test_exit_code_success() {
    // Test help command (should exit with code 0)
    cargo::cargo_bin_cmd!("gold_digger")
        .args(["--help"])
        .assert()
        .success();
}

#[test]
fn test_exit_code_dump_config() {
    // Test dump-config command (should exit with code 0)
    cargo::cargo_bin_cmd!("gold_digger")
        .args(["--dump-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("database_url"));
}
