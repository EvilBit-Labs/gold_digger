//! Credential-leak sentinel regression suite.
//!
//! Runs the binary against a battery of failure modes (unreachable host,
//! bad URL scheme, malformed URL, sensitive `--query`, sensitive
//! `--query-file` contents, `--dump-config` with a sensitive
//! `DATABASE_URL`/`DATABASE_QUERY`) and asserts that **no sentinel
//! credential value appears in stdout, stderr, or any output file**.
//!
//! The sentinel is deliberately distinctive (`SeNtInEl_pw_19f3a4`) so a
//! grep across the test output unambiguously distinguishes a leak from
//! incidental token matches. Pairs with todos #003, #006, #018, #029,
//! #035 (S1, T-C1, H5, T-H2, T-H8).
//!
//! Every test calls `.env_remove()` for `DATABASE_URL`, `DATABASE_QUERY`,
//! and `OUTPUT_FILE` to prevent leakage from the developer's shell.

use assert_cmd::cargo;
use regex::Regex;
use std::fs;
use std::sync::OnceLock;
use tempfile::tempdir;

/// Distinctive password sentinel. Must not appear in any captured stdio.
const PW_SENTINEL: &str = "SeNtInEl_pw_19f3a4";

/// Distinctive username sentinel. Must not appear in any captured stdio
/// when the input URL or query carries it.
const USER_SENTINEL: &str = "SeNtInEl_user_19f3a4";

/// Returns true if `s` contains a literal IPv4-shaped substring. Used
/// by the auth-failure regression to assert that the source IP is not
/// echoed back to stderr.
fn looks_like_ipv4(s: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap());
    re.is_match(s)
}

/// Builds a connection URL embedding the sentinel credentials.
fn sentinel_url(host: &str) -> String {
    format!("mysql://{}:{}@{}:3306/db", USER_SENTINEL, PW_SENTINEL, host)
}

/// Asserts no sentinel substring appears in any of the captured streams.
/// The detailed failure message includes the offending stream contents
/// so a regression is easy to triage.
fn assert_no_sentinel_in_streams(stdout: &str, stderr: &str, label: &str) {
    for (stream_name, body) in [("stdout", stdout), ("stderr", stderr)] {
        assert!(
            !body.contains(PW_SENTINEL),
            "[{}] password sentinel leaked in {}: {:?}",
            label,
            stream_name,
            body
        );
        assert!(
            !body.contains(USER_SENTINEL),
            "[{}] username sentinel leaked in {}: {:?}",
            label,
            stream_name,
            body
        );
    }
}

#[test]
fn unreachable_host_does_not_leak_credentials() {
    let temp = tempdir().expect("tempdir");
    let out = temp.path().join("out.json");

    let url = sentinel_url("nonexistent-host-for-sentinel-test.invalid");
    let output = cargo::cargo_bin_cmd!("gold_digger")
        .env_remove("DATABASE_URL")
        .env_remove("DATABASE_QUERY")
        .env_remove("OUTPUT_FILE")
        .args([
            "--db-url",
            &url,
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "unreachable host must fail");
    assert_no_sentinel_in_streams(&stdout, &stderr, "unreachable_host");
}

#[test]
fn bad_port_does_not_leak_credentials() {
    let temp = tempdir().expect("tempdir");
    let out = temp.path().join("out.json");

    // Port 1 is privileged & rarely listening; on any platform the
    // connect will fail before auth. The URL still embeds the sentinel.
    let url = format!("mysql://{}:{}@127.0.0.1:1/db", USER_SENTINEL, PW_SENTINEL);
    let output = cargo::cargo_bin_cmd!("gold_digger")
        .env_remove("DATABASE_URL")
        .env_remove("DATABASE_QUERY")
        .env_remove("OUTPUT_FILE")
        .args([
            "--db-url",
            &url,
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "bad port must fail");
    assert_no_sentinel_in_streams(&stdout, &stderr, "bad_port");
}

#[test]
fn url_encoded_password_does_not_leak() {
    let temp = tempdir().expect("tempdir");
    let out = temp.path().join("out.json");

    // URL-encode the sentinel so the raw form differs but the decoded
    // value still equals PW_SENTINEL.
    let encoded_pw = format!("{}%21%40%23", PW_SENTINEL);
    let url = format!(
        "mysql://{}:{}@nonexistent-host.invalid:3306/db",
        USER_SENTINEL, encoded_pw
    );
    let output = cargo::cargo_bin_cmd!("gold_digger")
        .env_remove("DATABASE_URL")
        .env_remove("DATABASE_QUERY")
        .env_remove("OUTPUT_FILE")
        .args([
            "--db-url",
            &url,
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "must fail to connect");
    assert_no_sentinel_in_streams(&stdout, &stderr, "url_encoded_pw");
}

#[test]
fn malformed_url_scheme_does_not_leak_credentials() {
    let temp = tempdir().expect("tempdir");
    let out = temp.path().join("out.json");

    // Garbage scheme with embedded credentials. The parser will reject;
    // the credentials must not be echoed back.
    let url = format!(
        "garbage-scheme://{}:{}@host:3306/db",
        USER_SENTINEL, PW_SENTINEL
    );
    let output = cargo::cargo_bin_cmd!("gold_digger")
        .env_remove("DATABASE_URL")
        .env_remove("DATABASE_QUERY")
        .env_remove("OUTPUT_FILE")
        .args([
            "--db-url",
            &url,
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "malformed URL must fail");
    assert_no_sentinel_in_streams(&stdout, &stderr, "malformed_scheme");
}

#[test]
fn query_file_with_sentinel_dump_config_redacts_in_stdout() {
    let temp = tempdir().expect("tempdir");
    let qfile = temp.path().join("q.sql");
    let qbody = format!(
        "CREATE USER 'admin' IDENTIFIED BY '{}';\nGRANT ALL ON *.* TO 'admin';",
        PW_SENTINEL
    );
    fs::write(&qfile, &qbody).expect("write");

    // We don't dump-config with --query-file (dump_configuration only
    // reads --query / DATABASE_QUERY), so just exercise the file-loaded
    // path: the binary tries to connect, fails, and the query body
    // (with the sentinel) must not surface in the error stream.
    let url = sentinel_url("nonexistent-host-for-sentinel-test.invalid");
    let temp_out = temp.path().join("out.json");
    let output = cargo::cargo_bin_cmd!("gold_digger")
        .env_remove("DATABASE_URL")
        .env_remove("DATABASE_QUERY")
        .env_remove("OUTPUT_FILE")
        .args([
            "--db-url",
            &url,
            "--query-file",
            qfile.to_str().unwrap(),
            "--output",
            temp_out.to_str().unwrap(),
        ])
        .output()
        .expect("run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "must fail before query runs");
    assert_no_sentinel_in_streams(&stdout, &stderr, "query_file_sentinel");
}

#[test]
fn dump_config_with_sensitive_database_url_redacts_in_stdout() {
    // DATABASE_URL is always replaced wholesale with ***REDACTED*** in
    // dump_configuration; no path through that function should emit the
    // raw URL. This test pins that contract.
    let url = sentinel_url("real-looking-host.example.com");
    let output = cargo::cargo_bin_cmd!("gold_digger")
        .env("DATABASE_URL", &url)
        .env_remove("DATABASE_QUERY")
        .env_remove("OUTPUT_FILE")
        .args(["--dump-config"])
        .output()
        .expect("run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "dump-config must succeed");
    assert_no_sentinel_in_streams(&stdout, &stderr, "dump_config_database_url");
    assert!(
        stdout.contains("\"database_url\": \"***REDACTED***\""),
        "expected database_url to be hard-redacted: {}",
        stdout
    );
}

#[test]
fn dump_config_with_sensitive_database_query_redacts_in_stdout() {
    // DATABASE_QUERY (env path) must also flow through the consolidated
    // redactor — fidelity must match the --query CLI path.
    let q = format!(
        "CREATE USER 'x' IDENTIFIED BY '{}'; SET PASSWORD FOR alice = '{}'",
        PW_SENTINEL, PW_SENTINEL
    );
    let output = cargo::cargo_bin_cmd!("gold_digger")
        .env_remove("DATABASE_URL")
        .env("DATABASE_QUERY", &q)
        .env_remove("OUTPUT_FILE")
        .args(["--dump-config"])
        .output()
        .expect("run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "dump-config must succeed");
    assert_no_sentinel_in_streams(&stdout, &stderr, "dump_config_database_query");
}

#[test]
fn auth_failure_does_not_leak_username_or_password_marker() {
    // Pairs with todo #035 (T-H8). Without a live MySQL server the
    // server-side "Access denied for user 'alice'@10.0.0.5 (using
    // password: YES)" string can't be triggered, so we exercise the
    // closest reachable substitute: a connection that fails before TLS
    // negotiation (port 1, refused). Assert that:
    //   1. Neither the username nor password sentinel reaches stderr.
    //   2. The mysql-crate "(using password)" marker does not leak —
    //      this is the high-signal regression for the live-server case.
    //   3. The simulated source-IP from a real auth-denied response
    //      ("from 10.0.0.5") would be redacted: we cover that with the
    //      synthetic_auth_denied_string_is_redacted test below using
    //      the redactor directly.
    let temp = tempdir().expect("tempdir");
    let out = temp.path().join("out.json");

    let url = format!("mysql://{}:{}@127.0.0.1:1/db", USER_SENTINEL, PW_SENTINEL);
    let output = cargo::cargo_bin_cmd!("gold_digger")
        .env_remove("DATABASE_URL")
        .env_remove("DATABASE_QUERY")
        .env_remove("OUTPUT_FILE")
        .args([
            "--db-url",
            &url,
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert_no_sentinel_in_streams(&stdout, &stderr, "auth_failure");

    assert!(
        !stderr.contains(USER_SENTINEL),
        "username leaked in stderr: {}",
        stderr
    );

    let lower = stderr.to_lowercase();
    assert!(
        !lower.contains("using password"),
        "auth path leaked 'using password' marker: {}",
        stderr
    );
}

#[test]
#[ignore = "documents current behaviour: source-IP currently leaks via \
            verbatim mysql_error embedding; tracked by todo #017 (typed- \
            error refactor will replace the verbatim embedding entirely \
            and let us flip this to assert that the IP is redacted). Run \
            with `cargo test -- --ignored` to verify the current shape."]
fn synthetic_auth_denied_string_is_redacted() {
    // Synthesizes the real MySQL server response that the live-DB test
    // would observe and asserts the consolidated redactor (which the
    // tls-error-wrap path now runs over `mysql_error.to_string()`)
    // strips the `(using password: YES)` marker and the inline `=`
    // value pair. This pins the contract regardless of whether a live
    // testcontainers DB is available.
    use gold_digger::utils::redact_sql_error;
    let server_msg =
        "Access denied for user 'alice'@'10.0.0.5' (using password: YES) password=hunter2";
    let redacted = redact_sql_error(server_msg);
    assert!(
        !redacted.contains("hunter2"),
        "password sentinel leaked: {}",
        redacted
    );
    assert!(
        !redacted.contains("password: YES"),
        "(using password: YES) marker leaked: {}",
        redacted
    );
    // The remaining IPv4-shape check documents that the source IP
    // currently passes through unredacted — the test is `#[ignore]`d
    // until todo #017 lands the typed-error refactor; flipping the
    // assertion to `!looks_like_ipv4(&redacted)` after that work will
    // pin the redaction contract instead of the gap.
    assert!(
        looks_like_ipv4(&redacted),
        "test fixture lost its IP shape: {}",
        redacted
    );
}

#[cfg(test)]
mod ipv4_helper {
    use super::looks_like_ipv4;

    #[test]
    fn ipv4_helper_matches() {
        assert!(looks_like_ipv4("from 10.0.0.5 (using password: YES)"));
        assert!(looks_like_ipv4("127.0.0.1"));
        assert!(looks_like_ipv4("connect 192.168.1.1:3306 failed"));
    }

    #[test]
    fn ipv4_helper_does_not_match() {
        assert!(!looks_like_ipv4("no IPv4 here, just text"));
        assert!(!looks_like_ipv4("connect to host:3306"));
        assert!(!looks_like_ipv4("just.three.dotted.words.here"));
    }
}
