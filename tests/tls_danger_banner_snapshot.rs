//! Snapshot pin for the `--allow-invalid-certificate` DANGER banner (todo #082).
//!
//! `src/tls/config.rs::display_security_warnings` emits three
//! `tracing::error!` lines (`[DANGER] …`) when the caller picks
//! `--allow-invalid-certificate`. The banner is the only user-facing
//! signal that TLS validation is off — wording drift would silently
//! erode the warning's effectiveness, and a future "tighten default
//! verbosity" change could accidentally suppress it under `--quiet`.
//!
//! This file pins the exact banner lines (insta snapshot) and asserts
//! the banner survives `--quiet` (which only filters `info`/`warn`,
//! never `error`-level lines). Stderr is captured with `NO_COLOR=1` so
//! the snapshot stays plain ASCII; the colour helpers in
//! `src/logging.rs` short-circuit on that env var.
//!
//! Failure paths feed back into todo #082 acceptance criteria:
//! - banner text changed -> insta diff (review explicitly).
//! - banner suppressed under `--quiet` -> assertion fires.

use assert_cmd::cargo;
use insta::assert_snapshot;

/// Clap-bound env vars stripped from spawned binaries to keep
/// developer-shell exports out of the captured stderr (project-memory
/// pattern).
const ENV_VARS_TO_REMOVE: &[&str] = &["DATABASE_URL", "DATABASE_QUERY", "OUTPUT_FILE", "NO_COLOR"];

/// Run `gold_digger` with the supplied extra args, an unreachable
/// connection target, and `NO_COLOR=1` so banner output is plain ASCII.
fn run_with_args(extra_args: &[&str]) -> std::process::Output {
    let mut cmd = cargo::cargo_bin_cmd!("gold_digger");
    for var in ENV_VARS_TO_REMOVE {
        cmd.env_remove(var);
    }
    cmd.env("NO_COLOR", "1")
        // Unreachable host -> connection fails fast after the banner
        // has been emitted on stderr. Avoids depending on Docker.
        .args([
            "--db-url",
            "mysql://baduser:badpass@127.0.0.1:1/db",
            "--query",
            "SELECT 1",
            "--output",
            "/tmp/gold_digger_danger_banner_snapshot.csv",
        ]);
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.output().expect("failed to spawn gold_digger")
}

/// Extract the contiguous block of TLS-banner lines from stderr.
///
/// Only the first emitted line carries the literal `[DANGER]` token;
/// the two follow-up lines (`This connection provides NO security…`
/// and `Only use this for testing…`) are paired error-level prints
/// that complete the banner. We capture from the `[DANGER]` line
/// through the next non-`ERROR` line so the snapshot pins the full
/// three-line block plus its tracing-level prefix — a regression that
/// demoted any of them from `error!` to `warn!` shows up as a missing
/// `ERROR` prefix in the snapshot diff.
fn extract_danger_lines(stderr: &str) -> String {
    let mut lines = Vec::new();
    let mut in_block = false;
    for line in stderr.lines() {
        if line.contains("[DANGER]") {
            in_block = true;
            lines.push(line);
            continue;
        }
        if in_block {
            // The follow-up banner lines are emitted via `tracing::error!`
            // so they must start with `ERROR`. The first non-ERROR line
            // (e.g. the connection-failure message) ends the block.
            if line.starts_with("ERROR") && !line.contains("Database connection") {
                lines.push(line);
            } else {
                break;
            }
        }
    }
    lines.join("\n")
}

/// Default verbosity: `--allow-invalid-certificate` must emit all three
/// `[DANGER]` banner lines on stderr. Snapshot pins the wording so any
/// future copy edit goes through review.
#[test]
fn allow_invalid_certificate_emits_danger_banner() {
    let output = run_with_args(&["--allow-invalid-certificate"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let danger = extract_danger_lines(&stderr);
    assert!(
        !danger.is_empty(),
        "expected [DANGER] banner lines on stderr, got:\n{stderr}"
    );

    assert_snapshot!("allow_invalid_certificate_danger_banner", danger);
}

/// `--quiet` filters info/warn lines but must NOT suppress the
/// `[DANGER]` banner — the banner is emitted via `tracing::error!`
/// which sits below the quiet threshold (`error`). If a future change
/// reroutes the banner through `warn!` or `println!`, this test fires.
#[test]
fn allow_invalid_certificate_danger_banner_survives_quiet() {
    let output = run_with_args(&["--allow-invalid-certificate", "--quiet"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains("[DANGER]"),
        "[DANGER] banner must NOT be suppressed by --quiet; stderr was:\n{stderr}"
    );
}
