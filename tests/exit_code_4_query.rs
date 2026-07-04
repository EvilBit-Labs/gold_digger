//! End-to-end exit-code coverage for query failures (HIGH #14, F005).
//!
//! `src/exit.rs::EXIT_QUERY_ERROR` (4) is the documented public exit code
//! for any query-side failure: malformed SQL, missing tables, missing
//! columns, or any server-side error during execution. Unit tests in
//! `src/exit.rs` exercise the classifier in process, but exit-code 4 had
//! no end-to-end test asserting that a real MySQL server returning a
//! query error produces the documented public exit code.
//!
//! This file fills that gap by spinning up a MySQL container, running
//! queries that the server will actively reject, and asserting code 4 —
//! never 0/1/3/5/255 (which would each indicate a misclassification of
//! the failure surface).
//!
//! Skipped automatically when Docker is unavailable so the suite stays
//! green on hosts without a container runtime.

mod fixtures;
mod integration;
mod test_support;

use anyhow::Result;

use crate::integration::{
    TestDatabase,
    containers::{database_container::DatabaseContainer, utils::skip_if_no_docker},
};
use crate::test_support::cli::clean_cmd;

/// Querying a table that does not exist must surface as
/// `EXIT_QUERY_ERROR` (4). Pre-fix this branch could be misclassified as
/// a connection error (3) when the typed-error mapper failed to peel the
/// server-side `ER_NO_SUCH_TABLE` out of the wrapped `mysql::Error`.
#[test]
fn missing_table_exits_query_error() -> Result<()> {
    skip_if_no_docker()?;

    let container = DatabaseContainer::new(TestDatabase::mysql())?;
    let temp_dir = tempfile::tempdir()?;
    let output_path = temp_dir.path().join("missing_table.json");

    clean_cmd()
        .args([
            "--db-url",
            container.connection_url(),
            "--query",
            "SELECT * FROM nonexistent_table_xyz_unique_name",
            "--output",
            output_path.to_str().expect("utf-8 path"),
            // testcontainers MySQL ships a self-signed cert; skip TLS
            // pinning so this test focuses on the query-error branch.
            "--allow-invalid-certificate",
        ])
        .assert()
        .code(4);

    Ok(())
}

/// Syntactically malformed SQL must also map to `EXIT_QUERY_ERROR` (4).
/// `SELECT FROM WHERE` is valid lexically (all tokens are keywords) but
/// the parser will reject it; the server returns `ER_PARSE_ERROR`.
#[test]
fn bad_sql_syntax_exits_query_error() -> Result<()> {
    skip_if_no_docker()?;

    let container = DatabaseContainer::new(TestDatabase::mysql())?;
    let temp_dir = tempfile::tempdir()?;
    let output_path = temp_dir.path().join("bad_syntax.json");

    clean_cmd()
        .args([
            "--db-url",
            container.connection_url(),
            "--query",
            "SELECT FROM WHERE",
            "--output",
            output_path.to_str().expect("utf-8 path"),
            "--allow-invalid-certificate",
        ])
        .assert()
        .code(4);

    Ok(())
}
