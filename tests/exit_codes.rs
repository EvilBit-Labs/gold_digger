use assert_cmd::cargo;
use predicates::prelude::*;

#[test]
fn test_exit_code_config_error() {
    // Test missing database URL (should exit with code 2). After
    // CRITICAL #3 the run pipeline returns errors via `?` rather than
    // exiting with a context-prefixed `exit_with_error(e, Some(...))`,
    // so the user-facing message is now the typed `ConfigError`'s
    // Display string (which still names the missing flag /env var).
    cargo::cargo_bin_cmd!("gold_digger")
        .env_remove("DATABASE_URL")
        .env_remove("DATABASE_QUERY")
        .env_remove("OUTPUT_FILE")
        .args(["--query", "SELECT 1", "--output", "/tmp/test.csv"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Missing database URL"));
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
