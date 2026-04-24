//! Integration coverage for `EXIT_DB_AUTH_ERROR` (exit code 3).
//!
//! Filed by todo #165. The unit tests in `src/exit.rs` exercise the
//! classifier in-process, but exit-code 3 had no end-to-end test asserting
//! that an actual unreachable database produces the documented public exit
//! code. This file fixes that.
//!
//! The strategy is to point the binary at a routable but unreachable host
//! (`127.0.0.1` on a port that no MySQL server listens on) so the connection
//! attempt fails fast without depending on testcontainers, network
//! conditions, or a populated DNS cache.

use assert_cmd::cargo;
use predicates::prelude::*;

/// Connecting to `127.0.0.1:1` (TCP port 1 is reserved and almost never
/// bound to a MySQL server) should produce `EXIT_DB_AUTH_ERROR` (3) per
/// `src/exit.rs::tls_exit_code` / typed `GoldDiggerError::DbAuth` mapping.
#[test]
fn test_exit_code_3_unreachable_host() {
    let mut cmd = cargo::cargo_bin_cmd!("gold_digger");
    cmd.env_remove("DATABASE_URL")
        .env_remove("DATABASE_QUERY")
        .env_remove("OUTPUT_FILE")
        .env_remove("NO_COLOR")
        .args([
            "--db-url",
            "mysql://baduser:badpass@127.0.0.1:1/db",
            "--query",
            "SELECT 1",
            "--output",
            // Point at a path that exists & is writable so the failure is
            // unambiguously a DB-connection issue, not an output-write one.
            "/tmp/gold_digger_exit_3_test.csv",
        ])
        .assert()
        .code(3)
        .stderr(
            predicate::str::contains("connection")
                .or(predicate::str::contains("connect"))
                .or(predicate::str::contains("refused"))
                .or(predicate::str::contains("MySQL"))
                .or(predicate::str::contains("mysql")),
        );
}

/// A syntactically-malformed `mysql://` URL still routes through the typed
/// `GoldDiggerError::DbAuth` path because connection-establishment fails;
/// either way the code must be 3, never 0/1/4/5/255.
#[test]
fn test_exit_code_3_bad_credentials_unreachable() {
    let mut cmd = cargo::cargo_bin_cmd!("gold_digger");
    cmd.env_remove("DATABASE_URL")
        .env_remove("DATABASE_QUERY")
        .env_remove("OUTPUT_FILE")
        .env_remove("NO_COLOR")
        .args([
            "--db-url",
            "mysql://wronguser:wrongpass@127.0.0.1:1/nonexistent",
            "--query",
            "SELECT 1",
            "--output",
            "/tmp/gold_digger_exit_3_creds_test.csv",
        ])
        .assert()
        .code(3);
}
