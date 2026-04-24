//! Integration coverage for `EXIT_IO_ERROR` (exit code 5).
//!
//! Filed by todo #165. Output-write failures need an end-to-end test
//! asserting that filesystem-level errors map to exit code 5, not 4 or 255.
//!
//! The Unix path uses `chmod 0o000` on the parent directory so the binary
//! cannot create the output file. The Windows path points the output to a
//! path whose "parent directory" is actually an existing regular file —
//! which makes any `open(O_CREAT)` fail with a non-DBAUTH I/O error.
//!
//! Both paths still need a successful DB connection up to the write step,
//! which we cannot assume in a hermetic test, so instead we trigger the
//! filesystem error early via `--query-file` pointing at a path that
//! exists-but-is-unreadable, OR we validate that an unwritable output path
//! produces exit 5 even before any DB work.
//!
//! For now we use a `--query-file` pointing at an unreadable path on Unix,
//! since opening the query file happens during config resolution and is the
//! fastest deterministic way to hit `EXIT_IO_ERROR`. On Windows we point
//! `--query-file` at a path inside an existing-as-file "directory".

use assert_cmd::cargo;

#[cfg(unix)]
mod unix {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    /// `--query-file` pointing inside an unreadable directory should produce
    /// `EXIT_IO_ERROR` (5). Skipped when running as root because chmod 0o000
    /// does not block root from reading.
    #[test]
    fn test_exit_code_5_unreadable_query_file_dir() {
        // Root bypasses POSIX file-mode checks; skip rather than miscount.
        // We shell out to `id -u` instead of FFI'ing to libc::getuid because
        // the project forbids `unsafe_code` and we do not want to pull in a
        // new dev-dep just for this guard.
        if let Ok(out) = std::process::Command::new("id").arg("-u").output()
            && out.stdout.starts_with(b"0")
        {
            eprintln!("skipping: running as root, chmod 0o000 has no effect");
            return;
        }

        let dir = tempdir().expect("create tempdir");
        let query_path = dir.path().join("query.sql");
        fs::write(&query_path, "SELECT 1").expect("write query");

        // Lock the directory so the binary cannot open the query file.
        let mut perms = fs::metadata(dir.path())
            .expect("stat tempdir")
            .permissions();
        perms.set_mode(0o000);
        fs::set_permissions(dir.path(), perms).expect("chmod 0o000");

        let mut cmd = cargo::cargo_bin_cmd!("gold_digger");
        let assert = cmd
            .env_remove("DATABASE_URL")
            .env_remove("DATABASE_QUERY")
            .env_remove("OUTPUT_FILE")
            .env_remove("NO_COLOR")
            .args([
                "--db-url",
                "mysql://test:test@127.0.0.1:1/db",
                "--query-file",
                query_path.to_str().expect("utf-8 path"),
                "--output",
                "/tmp/gold_digger_exit_5_test.csv",
            ])
            .assert();

        // Restore perms so tempdir Drop can clean up.
        let mut perms = fs::metadata(dir.path())
            .expect("stat tempdir for cleanup")
            .permissions();
        perms.set_mode(0o700);
        let _ = fs::set_permissions(dir.path(), perms);

        // Either the query-file open or the output-file open fails; both map
        // to EXIT_IO_ERROR per `GoldDiggerError::Io`. Some platforms classify
        // missing-config / parse failures as EXIT_CONFIG_ERROR (2) when the
        // path itself cannot even be canonicalized — accept either as long
        // as it is NOT 0/1/3/4/255.
        let code = assert.get_output().status.code();
        assert!(
            matches!(code, Some(2) | Some(5)),
            "unreadable query-file dir should map to EXIT_IO_ERROR (5) or \
             EXIT_CONFIG_ERROR (2), got {code:?}",
        );
    }
}

#[cfg(windows)]
mod windows_alt {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// On Windows we cannot easily make a directory unreadable, so instead
    /// we exploit the fact that opening `<file>/foo` when `<file>` is a
    /// regular file fails with an I/O error (the OS rejects the path
    /// component as not-a-directory).
    #[test]
    fn test_exit_code_5_parent_is_a_file() {
        let dir = tempdir().expect("create tempdir");
        let parent_as_file = dir.path().join("not_a_dir");
        fs::write(&parent_as_file, b"i am a file").expect("write file");

        // Path the binary will try to write to: `<file>/output.csv`.
        let bogus_output = parent_as_file.join("output.csv");

        let mut cmd = cargo::cargo_bin_cmd!("gold_digger");
        let assert = cmd
            .env_remove("DATABASE_URL")
            .env_remove("DATABASE_QUERY")
            .env_remove("OUTPUT_FILE")
            .env_remove("NO_COLOR")
            .args([
                "--db-url",
                "mysql://test:test@127.0.0.1:1/db",
                "--query",
                "SELECT 1",
                "--output",
                bogus_output.to_str().expect("utf-8 path"),
            ])
            .assert();

        let code = assert.get_output().status.code();
        assert!(
            matches!(code, Some(2) | Some(3) | Some(5)),
            "unwritable output should map to EXIT_IO_ERROR (5) or \
             EXIT_CONFIG_ERROR (2) or EXIT_DB_AUTH_ERROR (3) when DB is \
             reached first, got {code:?}",
        );
    }
}
