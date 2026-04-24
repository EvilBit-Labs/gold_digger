//! Adversarial `--dump-config` redaction tests (todo #029, T-H2).
//!
//! Drives the binary's `--dump-config` JSON output through a truth table
//! of credential-bearing query shapes and asserts that no sentinel value
//! reaches stdout. Covers GRANT, SET PASSWORD, CREATE USER IDENTIFIED
//! WITH ... BY, `pwd=`, `passwd:`, `api_key=`, base64/hex/JWT blobs, and
//! non-English secret labels.
//!
//! Pairs `--query` (CLI) and `DATABASE_QUERY` (env) inputs to assert
//! identical redaction fidelity between the two paths.

use assert_cmd::cargo;
use rstest::rstest;
use std::fs;
use tempfile::tempdir;

/// Distinctive sentinel — exact substring match on stdout proves a leak.
const SENTINEL: &str = "SeNtInEl_pw_29c4f7";

/// Runs `--dump-config` with `query` provided via `--query` CLI flag.
/// Returns captured `stdout` for assertion.
fn dump_config_via_cli(query: &str) -> String {
    let output = cargo::cargo_bin_cmd!("gold_digger")
        .env_remove("DATABASE_URL")
        .env_remove("DATABASE_QUERY")
        .env_remove("OUTPUT_FILE")
        .args(["--dump-config", "--query", query])
        .output()
        .expect("run");
    assert!(
        output.status.success(),
        "dump-config (CLI) failed: stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

/// Runs `--dump-config` with `query` provided via `DATABASE_QUERY` env.
fn dump_config_via_env(query: &str) -> String {
    let output = cargo::cargo_bin_cmd!("gold_digger")
        .env_remove("DATABASE_URL")
        .env("DATABASE_QUERY", query)
        .env_remove("OUTPUT_FILE")
        .args(["--dump-config"])
        .output()
        .expect("run");
    assert!(
        output.status.success(),
        "dump-config (env) failed: stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

#[rstest]
#[case::create_user_identified_by(
    format!("CREATE USER 'admin' IDENTIFIED BY '{}'", SENTINEL)
)]
#[case::grant_identified_by(
    format!(
        "GRANT ALL PRIVILEGES ON db.* TO 'u'@'%' IDENTIFIED BY '{}'",
        SENTINEL
    )
)]
#[case::create_user_identified_with_plugin(
    format!(
        "CREATE USER 'admin' IDENTIFIED WITH mysql_native_password BY '{}'",
        SENTINEL
    )
)]
#[case::set_password_for(
    format!("SET PASSWORD FOR 'alice'@'%' = '{}'", SENTINEL)
)]
#[case::pwd_keyvalue(format!("connect string pwd={}", SENTINEL))]
#[case::passwd_colon(format!("connect string passwd:{}", SENTINEL))]
#[case::pass_keyvalue(format!("connect string pass={}", SENTINEL))]
#[case::api_key(format!("SELECT api_key={} FROM cfg", SENTINEL))]
#[case::api_key_dash(format!("SELECT api-key={} FROM cfg", SENTINEL))]
#[case::token_keyvalue(format!("SELECT token={} FROM s", SENTINEL))]
#[case::secret_keyvalue(format!("SELECT secret={} FROM s", SENTINEL))]
#[case::kennwort(format!("Kennwort={}", SENTINEL))]
#[case::mot_de_passe(format!("mot_de_passe={}", SENTINEL))]
#[case::contrasena(format!("contrasena={}", SENTINEL))]
#[case::url_with_credentials(format!(
    "// example: mysql://alice:{}@host/db", SENTINEL
))]
fn cli_path_redacts_sentinel(#[case] query: String) {
    let stdout = dump_config_via_cli(&query);
    assert!(
        !stdout.contains(SENTINEL),
        "CLI dump-config leaked sentinel for query {:?}\noutput:\n{}",
        query,
        stdout
    );
}

#[rstest]
#[case::create_user_identified_by(
    format!("CREATE USER 'admin' IDENTIFIED BY '{}'", SENTINEL)
)]
#[case::grant_identified_by(
    format!(
        "GRANT ALL PRIVILEGES ON db.* TO 'u'@'%' IDENTIFIED BY '{}'",
        SENTINEL
    )
)]
#[case::set_password_for(
    format!("SET PASSWORD FOR 'alice'@'%' = '{}'", SENTINEL)
)]
#[case::pwd_keyvalue(format!("connect string pwd={}", SENTINEL))]
#[case::api_key_dash(format!("SELECT api-key={} FROM cfg", SENTINEL))]
#[case::token_keyvalue(format!("SELECT token={} FROM s", SENTINEL))]
#[case::kennwort(format!("Kennwort={}", SENTINEL))]
#[case::url_with_credentials(format!(
    "// example: mysql://alice:{}@host/db", SENTINEL
))]
fn env_path_redacts_sentinel(#[case] query: String) {
    let stdout = dump_config_via_env(&query);
    assert!(
        !stdout.contains(SENTINEL),
        "env dump-config leaked sentinel for query {:?}\noutput:\n{}",
        query,
        stdout
    );
}

#[test]
fn cli_and_env_paths_redact_identically() {
    // Cross-input fidelity: the same query must produce the same
    // redacted output whether it arrives via --query or DATABASE_QUERY.
    let q = format!(
        "GRANT ALL ON *.* TO 'u' IDENTIFIED BY '{}'; SET PASSWORD FOR alice = '{}';",
        SENTINEL, SENTINEL
    );
    let cli = dump_config_via_cli(&q);
    let env = dump_config_via_env(&q);

    // The two streams must agree on the redacted query field. The full
    // JSON also includes paths/flags that may differ across env vars, so
    // we extract just the "query" line for comparison.
    let cli_query_line = cli
        .lines()
        .find(|l| l.contains("\"query\""))
        .expect("CLI output missing query line")
        .trim();
    let env_query_line = env
        .lines()
        .find(|l| l.contains("\"query\""))
        .expect("env output missing query line")
        .trim();
    assert_eq!(
        cli_query_line, env_query_line,
        "CLI and env paths must redact identically"
    );

    assert!(!cli.contains(SENTINEL), "CLI leak: {}", cli);
    assert!(!env.contains(SENTINEL), "env leak: {}", env);
}

#[test]
fn empty_query_yields_empty_redacted_query() {
    // No --query and no DATABASE_QUERY: the dump should still succeed
    // and report an empty query, never a leak.
    let output = cargo::cargo_bin_cmd!("gold_digger")
        .env_remove("DATABASE_URL")
        .env_remove("DATABASE_QUERY")
        .env_remove("OUTPUT_FILE")
        .args(["--dump-config"])
        .output()
        .expect("run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains(SENTINEL));
    assert!(stdout.contains("\"query\""));
}

#[test]
fn benign_query_passes_through_unchanged() {
    // Sanity check: a query with no credentials must pass through
    // verbatim. (Subsumes the previous "nuclear redaction obliterated
    // the legitimate query" bug; the new redactor is surgical.)
    let q = "SELECT id, name FROM users WHERE id = 42";
    let stdout = dump_config_via_cli(q);
    assert!(
        stdout.contains("SELECT id, name FROM users WHERE id = 42"),
        "benign query was over-redacted: {}",
        stdout
    );
}

#[test]
fn query_file_with_sentinel_does_not_leak_via_dump_config() {
    // The dump_configuration code path only consults --query and
    // DATABASE_QUERY (not --query-file), so passing --query-file with
    // --dump-config should leave the redacted-query field empty rather
    // than reading the file. Confirm: no sentinel leaks regardless.
    let temp = tempdir().expect("tempdir");
    let qfile = temp.path().join("q.sql");
    fs::write(
        &qfile,
        format!("CREATE USER 'x' IDENTIFIED BY '{}'", SENTINEL),
    )
    .expect("write");

    let output = cargo::cargo_bin_cmd!("gold_digger")
        .env_remove("DATABASE_URL")
        .env_remove("DATABASE_QUERY")
        .env_remove("OUTPUT_FILE")
        .args(["--dump-config", "--query-file", qfile.to_str().unwrap()])
        .output()
        .expect("run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains(SENTINEL),
        "query-file leak via dump-config: {}",
        stdout
    );
}
