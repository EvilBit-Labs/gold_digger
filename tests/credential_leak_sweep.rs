//! Crate-wide credential-leak sentinel sweep (HIGH #16).
//!
//! For every documented failure mode the binary can hit, this file runs
//! the binary, captures stdout + stderr, and asserts that:
//!
//!   1. The literal sentinel password (`SeNtInEl_pw_29c4f7`) never
//!      appears in either stream. The sentinel is intentionally
//!      distinctive so a regression triages by `rg` alone.
//!   2. None of the common credential-leak markers
//!      (`password=`, `password:`, `pwd=`, `secret=`, `token=`,
//!      `api_key=`) appear, which would each indicate a different
//!      class of leak (URL parser, dump-config, error wrap, etc.).
//!
//! Each case targets a distinct error surface so a regression on one
//! code path can't be hidden by a fix on another. The sweep is the
//! "dragnet" complement to `tests/credential_leak_regression.rs`,
//! which targets specific historical regressions.
//!
//! Failure modes covered (Docker NOT required for any of them — every
//! mode uses either a refused-port URL, a bad scheme, a missing file,
//! or a config policy violation that fires before any DB I/O):
//!
//!   - Auth fail (refused port 1, bogus credentials in URL)
//!   - Mutual exclusion (`--query` + `--query-file` together)
//!   - Bad CA file (nonexistent path)
//!   - Bad URL scheme (`postgres://`)
//!   - Malformed URL (`not-a-url`)
//!   - Refused query-file extension (`.exe` per #023)
//!   - Unwritable output (existing file without `--force`)

mod fixtures;
mod integration;
mod test_support;

use crate::test_support::cli::clean_cmd;
use std::fs;
use tempfile::tempdir;

/// Distinctive password sentinel. Must not appear in any captured stdio
/// regardless of the failure mode under test.
const PW_SENTINEL: &str = "SeNtInEl_pw_29c4f7";

/// Generic credential-leak markers. Any one of these in stdout or stderr
/// is a regression — they signal a parser, dump-config, or error-wrap
/// path that did not run through the redactor.
///
/// `bearer ` / `Bearer ` cover HTTP `Authorization: Bearer <token>` headers
/// and the lowercase variant that some tooling logs. `auth=` covers the
/// URL query parameter / form field convention.
const LEAK_MARKERS: &[&str] = &[
    "password=",
    "password:",
    "pwd=",
    "secret=",
    "token=",
    "api_key=",
    "auth=",
    "bearer ",
    "Bearer ",
];

/// Build a `mysql://` URL embedding the password sentinel. Pointing
/// `host:port` at `127.0.0.1:1` makes the connection refused before any
/// authentication round-trip, so this works without a live MySQL.
fn sentinel_url() -> String {
    format!("mysql://baduser:{}@127.0.0.1:1/db", PW_SENTINEL)
}

/// Asserts the sentinel and every leak marker is absent from the
/// combined stdout+stderr capture for `label`. The error message
/// includes the offending stream verbatim so a regression triages
/// with no additional logging.
fn assert_no_credential_leak(stdout: &str, stderr: &str, label: &str) {
    for (stream, body) in [("stdout", stdout), ("stderr", stderr)] {
        assert!(
            !body.contains(PW_SENTINEL),
            "[{label}] password sentinel leaked in {stream}: {body:?}",
        );
        for marker in LEAK_MARKERS {
            assert!(
                !body.contains(marker),
                "[{label}] credential marker {marker:?} leaked in {stream}: {body:?}",
            );
        }
    }
}

/// Auth fail (connection refused). Bogus password embedded in URL must
/// not echo to stderr regardless of how the connection error surfaces.
#[test]
fn auth_fail_refused_port_does_not_leak() {
    let temp = tempdir().expect("tempdir");
    let out = temp.path().join("out.json");

    let url = sentinel_url();
    let output = clean_cmd()
        .args([
            "--db-url",
            &url,
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run");

    assert!(
        !output.status.success(),
        "auth fail must not exit success; got code {:?}",
        output.status.code()
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_no_credential_leak(&stdout, &stderr, "auth_fail_refused_port");
}

/// Mutual exclusion: `--query` and `--query-file` together must trigger
/// a Clap-level error. The URL still embeds the sentinel; even though
/// Clap rejects before the URL is parsed, dump-config code paths could
/// theoretically log it. Pin that they don't.
#[test]
fn query_and_query_file_mutual_exclusion_does_not_leak() {
    let temp = tempdir().expect("tempdir");
    let qfile = temp.path().join("q.sql");
    fs::write(&qfile, b"SELECT 1").expect("write query file");
    let out = temp.path().join("out.json");

    let url = sentinel_url();
    let output = clean_cmd()
        .args([
            "--db-url",
            &url,
            "--query",
            "SELECT 2",
            "--query-file",
            qfile.to_str().expect("utf-8 path"),
            "--output",
            out.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run");

    assert!(
        !output.status.success(),
        "mutual exclusion must not exit success"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_no_credential_leak(&stdout, &stderr, "mutual_exclusion");
}

/// Bad CA file: a nonexistent `--tls-ca-file` path is a config error
/// (`TlsError::CaFileNotFound`). The error message must not include the
/// embedded URL credentials.
#[test]
fn bad_ca_file_does_not_leak() {
    let temp = tempdir().expect("tempdir");
    let out = temp.path().join("out.json");

    let url = sentinel_url();
    let output = clean_cmd()
        .args([
            "--tls-ca-file",
            "/nonexistent/path/to/ca.pem",
            "--db-url",
            &url,
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run");

    assert!(
        !output.status.success(),
        "bad CA file must not exit success"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_no_credential_leak(&stdout, &stderr, "bad_ca_file");
}

/// Bad URL scheme (`postgres://...`). Whatever the URL parser says, the
/// password segment must be redacted before any echo to stderr.
#[test]
fn bad_url_scheme_does_not_leak() {
    let temp = tempdir().expect("tempdir");
    let out = temp.path().join("out.json");

    // `postgres://` URL — gold_digger only speaks MySQL, so this should
    // be rejected at parse / connection time. Sentinel is embedded just
    // like a real `mysql://` URL would be.
    let url = format!("postgres://baduser:{}@127.0.0.1:1/db", PW_SENTINEL);
    let output = clean_cmd()
        .args([
            "--db-url",
            &url,
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run");

    assert!(
        !output.status.success(),
        "bad URL scheme must not exit success"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_no_credential_leak(&stdout, &stderr, "bad_url_scheme");
}

/// Completely malformed URL string (no scheme, no host). Whatever
/// surface the URL parser surfaces this on, no sentinel reaches stdio.
#[test]
fn malformed_url_does_not_leak() {
    let temp = tempdir().expect("tempdir");
    let out = temp.path().join("out.json");

    // Embed sentinel into the malformed string so a parse-error log
    // that quotes the input is detectable.
    let url = format!("not-a-url-{}", PW_SENTINEL);
    let output = clean_cmd()
        .args([
            "--db-url",
            &url,
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run");

    assert!(
        !output.status.success(),
        "malformed URL must not exit success"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_no_credential_leak(&stdout, &stderr, "malformed_url");
}

/// Malformed query file: `.exe` extension is rejected by the path-safety
/// guard (#023). Sentinel is embedded in the URL so a config-error log
/// that echoes the URL is detectable.
#[test]
fn refused_query_file_extension_does_not_leak() {
    let temp = tempdir().expect("tempdir");
    let qfile = temp.path().join("query.exe");
    fs::write(&qfile, b"SELECT 1").expect("write query file");
    let out = temp.path().join("out.json");

    let url = sentinel_url();
    let output = clean_cmd()
        .args([
            "--db-url",
            &url,
            "--query-file",
            qfile.to_str().expect("utf-8 path"),
            "--output",
            out.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run");

    assert!(
        !output.status.success(),
        "refused extension must not exit success"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_no_credential_leak(&stdout, &stderr, "refused_extension");
}

/// Unwritable output: pre-existing file without `--force`. Sentinel-
/// bearing URL must not surface in the "use --force to overwrite" hint
/// or any other stderr diagnostic.
#[test]
fn unwritable_output_does_not_leak() {
    let temp = tempdir().expect("tempdir");
    let out = temp.path().join("preexisting.csv");
    fs::write(&out, b"do not clobber").expect("seed output file");

    let url = sentinel_url();
    let output = clean_cmd()
        .args([
            "--db-url",
            &url,
            "--query",
            "SELECT 1",
            "--output",
            out.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run");

    assert!(
        !output.status.success(),
        "preexisting output must not exit success"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_no_credential_leak(&stdout, &stderr, "unwritable_output");
}
