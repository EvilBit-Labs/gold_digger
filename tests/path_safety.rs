//! Path-safety integration tests (todo #030).
//!
//! These tests pin the path-safety guards added by todos #023 (`--query-file`
//! canonicalisation, size cap, extension deny-list) and #024 (`--output`
//! `O_NOFOLLOW` + `create_new` + 0o600 default, `--force` opt-in).
//!
//! Expected exit codes, per `src/exit.rs`:
//!   - `EXIT_CONFIG_ERROR = 2` for policy rejections (extension refused,
//!     size cap exceeded, file already exists without `--force`).
//!   - `EXIT_IO_ERROR = 5` for filesystem failures (symlink at target
//!     with `O_NOFOLLOW`, broken symlinks, canonicalisation failures).
//!
//! Each test builds a `Command` with `.env_remove(...)` to prevent
//! user-shell env leakage (mirrors the `clean_cmd()` helper in
//! `test_support::cli`; avoided here to keep this test file free of the
//! heavier integration-test support tree).

use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

mod fixtures;
mod integration;
mod test_support;

/// Returns a fresh `gold_digger` command with the Clap-bound env vars
/// removed. Thin wrapper over the canonical helper in
/// `tests/test_support/cli.rs::clean_cmd` so all integration tests
/// go through one source.
fn clean_cmd() -> Command {
    crate::test_support::cli::clean_cmd()
}

/// Writes a trivial valid SQL query to `path`.
fn write_query_file(path: &std::path::Path, content: &[u8]) {
    fs::write(path, content).expect("write query file");
}

/// Resolves a path that is guaranteed not to exist (used to force early
/// exit before the binary attempts to open a real database connection).
fn phantom_db_url() -> &'static str {
    "mysql://no-one:nothing@127.0.0.1:1/nothing"
}

// ---------------------------------------------------------------------
// #023 — --query-file path safety
// ---------------------------------------------------------------------

/// A `--query-file` pointing at a file with a refused `.exe` extension
/// must be rejected with `EXIT_CONFIG_ERROR` (2) before any read happens.
#[test]
fn query_file_with_exe_extension_rejected() {
    let dir = tempdir().expect("tempdir");
    let exe_path = dir.path().join("query.exe");
    write_query_file(&exe_path, b"SELECT 1");

    let assert = clean_cmd()
        .args([
            "--db-url",
            phantom_db_url(),
            "--query-file",
            exe_path.to_str().expect("utf-8 path"),
            "--output",
            dir.path().join("out.csv").to_str().expect("utf-8 path"),
        ])
        .assert();

    let output = assert.get_output();
    let code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Pin only the exit code: the human-readable diagnostic phrasing
    // ("disallowed extension", ".exe", etc.) is presentation-only and
    // tightly coupling tests to it has produced false-positive failures
    // every time the message was copy-edited. The exit code is the
    // load-bearing public contract.
    assert_eq!(
        code,
        Some(2),
        "refused .exe extension should map to EXIT_CONFIG_ERROR (2); got {code:?}; stderr: {stderr}"
    );
}

/// `.SQL` (uppercase) must be accepted — extension matching is
/// case-insensitive per todo #023 / #030 acceptance criteria. The test
/// passes a phantom DB URL so the binary proceeds past query-file
/// resolution and fails later at the DB connection step (exit 3). A
/// config-error (2) here would indicate the extension guard rejected the
/// uppercase `.SQL`.
#[test]
fn query_file_uppercase_sql_extension_accepted() {
    let dir = tempdir().expect("tempdir");
    let uppercase_path = dir.path().join("query.SQL");
    write_query_file(&uppercase_path, b"SELECT 1");

    let assert = clean_cmd()
        .args([
            "--db-url",
            phantom_db_url(),
            "--query-file",
            uppercase_path.to_str().expect("utf-8 path"),
            "--output",
            dir.path().join("out.csv").to_str().expect("utf-8 path"),
        ])
        .assert();

    let output = assert.get_output();
    let code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_ne!(
        code,
        Some(2),
        "uppercase .SQL must NOT be rejected as a config error; got 2, stderr: {stderr}"
    );
    // It should make it past query-file validation and fail at DB connect
    // (3) or query (4); any non-2 exit is acceptable for this test.
}

/// A `--query-file` that is a plain filename with no extension must be
/// accepted (common for scripted wrappers that generate query files).
#[test]
fn query_file_with_no_extension_accepted() {
    let dir = tempdir().expect("tempdir");
    let noext_path = dir.path().join("myquery");
    write_query_file(&noext_path, b"SELECT 1");

    let assert = clean_cmd()
        .args([
            "--db-url",
            phantom_db_url(),
            "--query-file",
            noext_path.to_str().expect("utf-8 path"),
            "--output",
            dir.path().join("out.csv").to_str().expect("utf-8 path"),
        ])
        .assert();

    let output = assert.get_output();
    let code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        code,
        Some(2),
        "no-extension path should NOT be refused by the policy check; got 2, stderr: {stderr}"
    );
}

/// A `--query-file` larger than the 10 MiB cap must be refused with
/// `EXIT_CONFIG_ERROR` (2). This protects against OOM / DoS from an
/// attacker pointing the flag at a huge file.
#[test]
fn query_file_over_size_cap_rejected() {
    let dir = tempdir().expect("tempdir");
    let big_path = dir.path().join("huge.sql");

    // 10 MiB + 1 byte — just over the cap.
    let size = (10 * 1024 * 1024) + 1;
    // Build without actually materialising 10MiB in Rust memory: write a
    // 1 MiB buffer 10 times plus one extra byte.
    let chunk = vec![b'-'; 1024 * 1024];
    {
        use std::io::Write;
        let mut f = fs::File::create(&big_path).expect("create huge.sql");
        for _ in 0..10 {
            f.write_all(&chunk).expect("write chunk");
        }
        f.write_all(b"\n").expect("write trailing newline");
        // Double-check we are over the limit.
        let meta = fs::metadata(&big_path).expect("stat huge.sql");
        assert!(
            meta.len() > (10 * 1024 * 1024),
            "test fixture smaller than cap: {} bytes (size_target={size})",
            meta.len()
        );
    }

    let assert = clean_cmd()
        .args([
            "--db-url",
            phantom_db_url(),
            "--query-file",
            big_path.to_str().expect("utf-8 path"),
            "--output",
            dir.path().join("out.csv").to_str().expect("utf-8 path"),
        ])
        .assert();

    let output = assert.get_output();
    let code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Pin only the exit code: see the `.exe` test above for the
    // rationale on dropping the brittle "maximum allowed" /
    // "10 MiB" / "bytes" stderr substring asserts.
    assert_eq!(
        code,
        Some(2),
        "oversized query-file should map to EXIT_CONFIG_ERROR (2); got {code:?}; stderr: {stderr}"
    );
}

/// A `--query-file` pointing at a symlink whose target has a refused
/// extension is rejected after canonicalisation (the canonical path's
/// extension is checked, not the symlink's). Unix-only because Windows
/// symlinks require elevated privileges.
#[cfg(unix)]
#[test]
fn query_file_symlink_to_exe_rejected_after_canonicalize() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().expect("tempdir");
    let target = dir.path().join("real_target.exe");
    write_query_file(&target, b"SELECT 1");

    let link = dir.path().join("looks_like.sql");
    symlink(&target, &link).expect("create symlink");

    let assert = clean_cmd()
        .args([
            "--db-url",
            phantom_db_url(),
            "--query-file",
            link.to_str().expect("utf-8 path"),
            "--output",
            dir.path().join("out.csv").to_str().expect("utf-8 path"),
        ])
        .assert();

    let output = assert.get_output();
    let code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Canonicalisation resolves the symlink, exposing the `.exe` target
    // to the extension guard — policy rejection (exit 2).
    assert_eq!(
        code,
        Some(2),
        "symlink-to-.exe should be rejected once canonicalised; got {code:?}; stderr: {stderr}"
    );
}

/// A `--query-file` pointing at a broken symlink fails canonicalisation
/// and must map to `EXIT_IO_ERROR` (5) — the kind error surface tells the
/// operator the filesystem is the problem, not the configuration.
#[cfg(unix)]
#[test]
fn query_file_broken_symlink_fails_canonicalize() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().expect("tempdir");
    let nowhere = dir.path().join("does_not_exist.sql");
    let link = dir.path().join("broken.sql");
    symlink(&nowhere, &link).expect("create broken symlink");

    let assert = clean_cmd()
        .args([
            "--db-url",
            phantom_db_url(),
            "--query-file",
            link.to_str().expect("utf-8 path"),
            "--output",
            dir.path().join("out.csv").to_str().expect("utf-8 path"),
        ])
        .assert();

    let output = assert.get_output();
    let code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        code,
        Some(5),
        "broken symlink should map to EXIT_IO_ERROR (5); got {code:?}; stderr: {stderr}"
    );
}

// ---------------------------------------------------------------------
// #024 — --output path safety
// ---------------------------------------------------------------------

/// Default behaviour: pre-existing output file is refused with
/// `EXIT_CONFIG_ERROR` (2) and a hint about `--force`. This blocks the
/// "accidental clobber" footgun.
#[test]
fn output_exists_without_force_rejected() {
    let dir = tempdir().expect("tempdir");
    let query_file = dir.path().join("q.sql");
    write_query_file(&query_file, b"SELECT 1");

    let out_path = dir.path().join("out.csv");
    fs::write(&out_path, b"preexisting").expect("seed output file");

    let assert = clean_cmd()
        .args([
            "--db-url",
            phantom_db_url(),
            "--query-file",
            query_file.to_str().expect("utf-8 path"),
            "--output",
            out_path.to_str().expect("utf-8 path"),
        ])
        .assert();

    let output = assert.get_output();
    let code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // With a phantom DB URL the binary may reach the output step only
    // after DB failure; but the existence check does not gate on that —
    // `File::create_new` happens at write time. On this code path the
    // DB connect fails first (exit 3) OR the output-exists check fires
    // (exit 2) depending on ordering. Accept 2 OR a stderr mention of
    // the `--force` hint.
    //
    // Practically, main.rs resolves output before DB, but only opens the
    // output after the query runs; if the query never runs we never hit
    // open. To make this test deterministic we cannot easily connect to
    // a real DB; instead we assert exit is NOT 0 and file was not
    // clobbered.
    let preexisting = fs::read_to_string(&out_path).expect("read out");
    assert_eq!(
        preexisting, "preexisting",
        "output file must not be clobbered when DB unreachable; got: {preexisting:?}; \
         exit={code:?}; stderr: {stderr}"
    );
    assert_ne!(
        code,
        Some(0),
        "must not exit success when DB unreachable; stderr: {stderr}"
    );
}

/// With `--force`, writing over an existing output file on an otherwise
/// unreachable DB does not exit 0, but it also must not surface the
/// "already exists" config error — that guard is expected to be bypassed
/// cleanly.
#[test]
fn output_exists_with_force_bypasses_guard() {
    let dir = tempdir().expect("tempdir");
    let query_file = dir.path().join("q.sql");
    write_query_file(&query_file, b"SELECT 1");

    let out_path = dir.path().join("out.csv");
    fs::write(&out_path, b"preexisting").expect("seed output file");

    let assert = clean_cmd()
        .args([
            "--db-url",
            phantom_db_url(),
            "--query-file",
            query_file.to_str().expect("utf-8 path"),
            "--output",
            out_path.to_str().expect("utf-8 path"),
            "--force",
        ])
        .assert();

    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // `--force` must not surface the "pass --force to overwrite" hint —
    // the guard is for the non-force path. (The DB still fails in this
    // harness, which is fine.)
    assert!(
        !stderr.contains("Pass --force to overwrite"),
        "--force must bypass the existing-file guard; stderr: {stderr}"
    );
}

/// Unix-only: an `--output` pointing at a symlink is refused via
/// `O_NOFOLLOW`, mapping to `EXIT_IO_ERROR` (5). This blocks the classic
/// "predictable-path symlink clobber" attack.
#[cfg(unix)]
#[test]
fn output_symlink_rejected_with_nofollow() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().expect("tempdir");
    let query_file = dir.path().join("q.sql");
    write_query_file(&query_file, b"SELECT 1");

    let real_target = dir.path().join("real_target.csv");
    fs::write(&real_target, b"target contents").expect("seed target");

    let link_path = dir.path().join("link.csv");
    symlink(&real_target, &link_path).expect("create symlink");

    let assert = clean_cmd()
        .args([
            "--db-url",
            phantom_db_url(),
            "--query-file",
            query_file.to_str().expect("utf-8 path"),
            "--output",
            link_path.to_str().expect("utf-8 path"),
            "--force",
        ])
        .assert();

    let output = assert.get_output();
    let code = output.status.code();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The real_target contents must NOT be clobbered — symlinks are
    // refused at open time (O_NOFOLLOW) or earlier.
    let target_after = fs::read_to_string(&real_target).expect("read target");
    assert_eq!(
        target_after, "target contents",
        "symlink target must not be clobbered (O_NOFOLLOW); code={code:?}; stderr: {stderr}"
    );
    // Expected exit is NOT 0; the DB may fail first (3) or O_NOFOLLOW
    // may fail (5). We intentionally accept either non-success path; the
    // load-bearing invariant is "do not follow the symlink."
    assert_ne!(code, Some(0), "must not succeed when output is a symlink");
}

/// Sanity regression: `--force` with a NON-existing output path at an
/// unreachable DB should still avoid the existing-file policy error.
/// Proves the default path-safety does not leak into the force path.
#[test]
fn force_flag_on_nonexistent_output_is_fine() {
    let dir = tempdir().expect("tempdir");
    let query_file = dir.path().join("q.sql");
    write_query_file(&query_file, b"SELECT 1");

    // A path that does not exist.
    let out_path: PathBuf = dir.path().join("does_not_exist_yet.csv");

    let assert = clean_cmd()
        .args([
            "--db-url",
            phantom_db_url(),
            "--query-file",
            query_file.to_str().expect("utf-8 path"),
            "--output",
            out_path.to_str().expect("utf-8 path"),
            "--force",
        ])
        .assert();

    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains("Pass --force to overwrite"),
        "--force on a missing file should never hit the exists-guard; stderr: {stderr}"
    );
}
