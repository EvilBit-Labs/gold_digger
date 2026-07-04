//! Shared credential-redaction utilities for the `gold_digger` binary.
//!
//! All credential-scrubbing logic in the codebase routes through this
//! module. Three public entry points cover the surfaces where secrets can
//! leak:
//!
//!   - [`redact_sql_error`]: scrubs MySQL error strings and arbitrary log
//!     lines (matches `password=`, `identified by`, `token`, `api_key`,
//!     `secret`, and `scheme://user:pw@host` URLs).
//!   - [`redact_url`]: structurally redacts the userinfo portion of a
//!     parseable URL; falls back to a hard-coded placeholder if parsing
//!     fails so a malformed URL never leaks intact.
//!   - [`redact_dump_query`]: redacts a SQL query intended for the
//!     `--dump-config` JSON output. Delegates to [`redact_sql_error`] so
//!     the same regex set covers both error paths and dump output (no
//!     drift between weakest and strongest redactor).
//!
//! Patterns compile once via [`OnceLock`]. Callers must never log an
//! un-redacted MySQL error, connection URL, or SQL query.

use std::sync::OnceLock;

use regex::Regex;

/// Shared placeholder used in regex-based redaction. The URL-aware
/// redactor in [`redact_url`] uses the same token so a `grep` for it in
/// stderr returns hits regardless of which path produced the redaction.
pub const REDACTION_PLACEHOLDER: &str = "***REDACTED***";

/// Placeholder returned by [`redact_url`] when the input cannot be parsed
/// as a URL. Distinct from [`REDACTION_PLACEHOLDER`] so test failures can
/// distinguish "URL with credentials redacted" from "URL was unparseable
/// and replaced wholesale".
pub const REDACTED_URL_PLACEHOLDER: &str = "***REDACTED_URL***";

/// Pre-compiled redaction patterns for sensitive information in error messages.
/// Each entry is a (pattern, replacement) tuple compiled once on first use.
static REDACTION_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();

/// Source-of-truth list of redaction `(pattern, replacement)` tuples.
///
/// Kept as a plain slice (not a `OnceLock`) so [`EXPECTED_REDACTION_PATTERN_COUNT`]
/// and the fail-closed initialiser can both reference the same definition.
///
/// # Word-boundary anchoring (HIGH #8)
///
/// Every label pattern starts with `\b` so the matcher fires only on
/// real secret labels, not on substrings inside larger words. Without
/// the boundary, `boarding_pass=ABC` matched the bare `pass` pattern
/// and got mangled, and `the secret ingredient is salt` matched the
/// bare `secret\s+\S+` pattern. The bare-space variants (`token \S+`
/// and `secret \S+`) are deleted entirely — real credential leaks use
/// the `[=:]` separator form, while the bare-space variants guarantee
/// false positives on prose.
///
/// # URL userinfo placeholder (MEDIUM idempotence)
///
/// The URL-userinfo replacement uses [`REDACTION_PLACEHOLDER`] for both
/// user and password components. Previously the replacement was the
/// literal `://***:***@`, which produced a different placeholder than
/// [`redact_url`] (which inserts [`REDACTION_PLACEHOLDER`]). Operators
/// grepping for "REDACTED" missed lines that flowed through the URL
/// regex, and chaining `redact_sql_error` over `redact_url` output
/// rewrote `***REDACTED***` back into `***`. The shared placeholder
/// makes both redactors converge to the same fixed point.
const REDACTION_PATTERN_DEFS: &[(&str, &str)] = &[
    // Common key=value / key:value secret pairs (English).
    (r"(?i)\bpassword\s*[=:]\s*\S+", REDACTION_PLACEHOLDER),
    (r"(?i)\bpasswd\s*[=:]\s*\S+", REDACTION_PLACEHOLDER),
    (r"(?i)\bpwd\s*[=:]\s*\S+", REDACTION_PLACEHOLDER),
    (r"(?i)\bpass\s*[=:]\s*\S+", REDACTION_PLACEHOLDER),
    // The secret token may be a quoted SQL string literal containing spaces
    // (e.g. IDENTIFIED BY 'pass word', or IDENTIFIED BY "pass word" under
    // ANSI_QUOTES); `\S+` alone would stop at the first space and leak the
    // remainder, so match a single- OR double-quoted literal OR a bare token.
    (
        r#"(?i)\bidentified\s+by\s+(?:'[^']*'|"[^"]*"|\S+)"#,
        REDACTION_PLACEHOLDER,
    ),
    (
        r#"(?i)\bidentified\s+with\s+\S+\s+by\s+(?:'[^']*'|"[^"]*"|\S+)"#,
        REDACTION_PLACEHOLDER,
    ),
    (r"(?i)\btoken\s*[=:]\s*\S+", REDACTION_PLACEHOLDER),
    (r"(?i)\bapi[_-]?key\s*[=:]\s*\S+", REDACTION_PLACEHOLDER),
    (r"(?i)\bsecret\s*[=:]\s*\S+", REDACTION_PLACEHOLDER),
    // GRANT ... IDENTIFIED BY '<pw>' (already covered by identified_by, kept for clarity).
    // SET PASSWORD = 'x' / SET PASSWORD FOR user = 'x'.
    (
        r#"(?i)\bset\s+password\s+(?:for\s+\S+\s+)?=\s*(?:'[^']*'|"[^"]*"|\S+)"#,
        REDACTION_PLACEHOLDER,
    ),
    // Non-English secret labels we have observed in production logs.
    (r"(?i)\bkennwort\s*[=:]\s*\S+", REDACTION_PLACEHOLDER),
    (
        r"(?i)\bmot[_-]?de[_-]?passe\s*[=:]\s*\S+",
        REDACTION_PLACEHOLDER,
    ),
    (r"(?i)\bcontrase[nñ]a\s*[=:]\s*\S+", REDACTION_PLACEHOLDER),
    // URL userinfo (any scheme). Replacement uses REDACTION_PLACEHOLDER
    // for both user and password components so a chained pass through
    // redact_sql_error after redact_url is a no-op (MEDIUM idempotence).
    (r"(?i)://[^:/\s]+:[^@\s]+@", URL_USERINFO_REPLACEMENT),
];

/// Replacement for matched URL userinfo (`scheme://user:pass@`). Kept as a
/// literal because [`REDACTION_PATTERN_DEFS`] is `const` and `format!` is not
/// available there; the `url_userinfo_replacement_uses_shared_placeholder`
/// test pins it to [`REDACTION_PLACEHOLDER`] so the two cannot silently drift.
const URL_USERINFO_REPLACEMENT: &str = "://***REDACTED***:***REDACTED***@";

/// Number of redaction patterns the `redact_sql_error` pipeline expects to
/// have available. A test pins this to the slice length so a contributor
/// who silently drops a pattern (or a `Regex::new` that fails on a future
/// regex-crate breaking change) is caught at test time.
#[cfg(test)]
pub(crate) const EXPECTED_REDACTION_PATTERN_COUNT: usize = REDACTION_PATTERN_DEFS.len();

/// Returns the pre-compiled redaction regex patterns, initialising them on
/// first call.
///
/// # Fail-closed semantics (todo #066)
///
/// A pattern in [`REDACTION_PATTERN_DEFS`] that fails to compile is a
/// programmer bug, not a runtime condition we can recover from: the
/// silently-dropped pattern leaves a credential class un-scrubbed. We
/// therefore:
///
///   - **Debug builds**: panic with the offending pattern + regex
///     compile error. CI / `cargo test` always uses debug profile, so
///     any future edit that breaks a pattern fails the build.
///   - **Release builds**: emit a `tracing::error!` and continue with
///     the surviving patterns. Refusing to start the binary on a
///     redaction-pattern bug would be worse than running with reduced
///     coverage; the error log is durable enough to flag the regression
///     to operators.
///
/// The `EXPECTED_REDACTION_PATTERN_COUNT` test in the same module pins
/// the post-compile count so the silent-drop case is also caught at
/// test time.
fn get_redaction_patterns() -> &'static Vec<(Regex, &'static str)> {
    REDACTION_PATTERNS.get_or_init(|| {
        // `filter_map` is intentional even though the debug-build arm
        // panics: `cfg(not(debug_assertions))` returns `None` so a
        // failed `Regex::new` is dropped from the live pattern set
        // (release builds prefer partial coverage to a crash). Clippy
        // can't see across the `cfg` boundary so we suppress its
        // "could be `map`" hint locally.
        #[allow(clippy::unnecessary_filter_map)]
        REDACTION_PATTERN_DEFS
            .iter()
            .filter_map(|(pattern, replacement)| match Regex::new(pattern) {
                Ok(re) => Some((re, *replacement)),
                Err(e) => {
                    // Fail-closed in debug / test builds so a regression
                    // surfaces immediately. Release builds log loudly
                    // and skip the broken pattern (better partial
                    // coverage than aborting the binary).
                    #[cfg(debug_assertions)]
                    panic!(
                        "Redaction regex failed to compile (todo #066): pattern={:?} error={}",
                        pattern, e
                    );
                    #[cfg(not(debug_assertions))]
                    {
                        tracing::error!(
                            pattern = %pattern,
                            error = %e,
                            "Redaction regex failed to compile; this leaves a credential class un-scrubbed"
                        );
                        None
                    }
                }
            })
            .collect()
    })
}

/// Redacts sensitive information from SQL error messages and log lines.
///
/// Uses pre-compiled regex patterns to identify and replace passwords,
/// tokens, API keys, secrets, and URL userinfo with redaction markers.
/// Patterns are compiled once on first call using `OnceLock` for
/// thread-safe lazy initialization.
///
/// This is the canonical entry point for redacting any string that may
/// have been built from a `mysql::Error`, a `Pool::new` failure, or any
/// other path where the source string is not under our control.
///
/// # Arguments
/// * `message` - The error message or log line to redact
///
/// # Returns
/// * `String` - The redacted message
///
/// # Example
/// ```
/// use gold_digger::utils::redact_sql_error;
///
/// // `password: YES` is replaced with the exact placeholder used across
/// // the codebase; callers can grep for it to audit redaction coverage.
/// let error = "Error: Access denied for user 'test' (using password: YES)";
/// let redacted = redact_sql_error(error);
/// assert!(redacted.contains("***REDACTED***"), "placeholder must appear");
/// assert!(!redacted.contains("password: YES"), "original text must be gone");
///
/// // URL userinfo is replaced with the shared REDACTION_PLACEHOLDER so
/// // chaining redactors converges to a single fixed point (operators
/// // grepping stderr for "REDACTED" hit every scrubbed surface).
/// let url_error = "Failed to connect to mysql://alice:secret@db:3306/prod";
/// let redacted = redact_sql_error(url_error);
/// assert!(redacted.contains("://***REDACTED***:***REDACTED***@"), "URL userinfo replaced");
/// assert!(!redacted.contains("alice:secret"), "credentials must be gone");
///
/// // Redaction is idempotent — running on an already-redacted string is a no-op.
/// let twice = redact_sql_error(&redacted);
/// assert_eq!(twice, redacted, "redaction must be idempotent");
/// ```
pub fn redact_sql_error(message: &str) -> String {
    let mut redacted = message.to_string();

    for (re, replacement) in get_redaction_patterns() {
        redacted = re.replace_all(&redacted, *replacement).to_string();
    }

    redacted
}

/// Redacts sensitive information from URLs for safe error logging.
///
/// Uses structural URL parsing (rather than regex) to redact userinfo. If
/// the input cannot be parsed as a URL OR a userinfo-mutation call
/// fails (cannot-be-a-base URL, no-authority URL, etc.), returns
/// [`REDACTED_URL_PLACEHOLDER`] so a URL whose credentials cannot be
/// scrubbed never leaks intact.
///
/// # CRITICAL #6 fix
///
/// The previous implementation discarded the `Result<(), ()>` from
/// `Url::set_password` / `Url::set_username` via `let _ =`. URLs that
/// parse but reject userinfo modification (e.g. `data:text/plain,user:pass@host`)
/// flowed through the redactor unchanged, leaking the original
/// credentials. We now fail closed: any mutation failure replaces the
/// whole string with [`REDACTED_URL_PLACEHOLDER`].
///
/// # Arguments
/// * `url` - The URL string to redact
///
/// # Returns
/// * `String` - The redacted URL, or [`REDACTED_URL_PLACEHOLDER`] on
///   parse failure or userinfo-mutation failure
pub fn redact_url(url: &str) -> String {
    let Ok(parsed) = url::Url::parse(url) else {
        return REDACTED_URL_PLACEHOLDER.to_string();
    };
    let mut redacted = parsed.clone();

    if parsed.password().is_some() && redacted.set_password(Some(REDACTION_PLACEHOLDER)).is_err() {
        return REDACTED_URL_PLACEHOLDER.to_string();
    }
    let username = parsed.username();
    if !username.is_empty() && redacted.set_username(REDACTION_PLACEHOLDER).is_err() {
        return REDACTED_URL_PLACEHOLDER.to_string();
    }
    redacted.to_string()
}

/// Redacts a SQL query intended for the `--dump-config` JSON output.
///
/// Delegates to [`redact_sql_error`] so the same pattern set covers
/// queries, error strings, and log lines (no drift between the weakest
/// and strongest redactor). Returned strings are safe to print to stdout
/// or include in a bug report.
///
/// # Arguments
/// * `query` - The SQL query text to redact
///
/// # Returns
/// * `String` - The redacted query
pub fn redact_dump_query(query: &str) -> String {
    redact_sql_error(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the number of compiled redaction patterns to
    /// [`EXPECTED_REDACTION_PATTERN_COUNT`]. Catches the silent-drop
    /// case where a pattern fails `Regex::new` in release builds — the
    /// debug fail-closed path would already panic, but this guards
    /// against the release-mode `filter_map(...)` skip path going
    /// unnoticed in test runs that happen to use a release profile
    /// (todo #066).
    #[test]
    fn redaction_pattern_count_matches_expected() {
        assert_eq!(
            get_redaction_patterns().len(),
            EXPECTED_REDACTION_PATTERN_COUNT,
            "fail-closed regex validation: a pattern silently failed to compile"
        );
    }

    #[test]
    fn test_redact_sql_error_password_keyvalue() {
        let error = "Error: Access denied for user 'test' (using password: YES)";
        let redacted = redact_sql_error(error);
        assert!(redacted.contains(REDACTION_PLACEHOLDER));
        assert!(!redacted.contains("password: YES"));
    }

    #[test]
    fn url_userinfo_replacement_uses_shared_placeholder() {
        // The URL-userinfo replacement is a literal (const array can't call
        // format!). Pin it to REDACTION_PLACEHOLDER so a rename or value change
        // of the constant fails CI instead of silently leaving the URL path
        // un-synced.
        let expected = format!("://{p}:{p}@", p = REDACTION_PLACEHOLDER);
        assert_eq!(URL_USERINFO_REPLACEMENT, expected);
    }

    #[test]
    fn test_redact_sql_error_identified_by() {
        let error = "Error: CREATE USER failed with identified by 'secret123'";
        let redacted = redact_sql_error(error);
        assert!(redacted.contains(REDACTION_PLACEHOLDER));
        assert!(!redacted.contains("'secret123'"));
    }

    #[test]
    fn test_redact_sql_error_quoted_password_with_spaces() {
        // Regression: `\S+` stopped at the first space and leaked the tail of a
        // quoted, space-containing password. The quoted-literal alternation now
        // consumes the whole single- or double-quoted value across all three
        // SQL DDL/DCL password patterns. Each pattern is an independent regex
        // literal, so exercise them individually (a copy-paste regression in
        // one would otherwise hide behind the others).
        let cases = [
            "CREATE USER 'u'@'%' IDENTIFIED BY 'pass word with spaces'",
            r#"CREATE USER 'u'@'%' IDENTIFIED BY "pass word with spaces""#,
            "CREATE USER 'u'@'%' IDENTIFIED WITH caching_sha2_password BY 'pass word with spaces'",
            r#"CREATE USER 'u'@'%' IDENTIFIED WITH caching_sha2_password BY "pass word with spaces""#,
            "SET PASSWORD FOR 'u'@'%' = 'pass word with spaces'",
            r#"SET PASSWORD FOR 'u'@'%' = "pass word with spaces""#,
        ];
        for error in cases {
            let redacted = redact_sql_error(error);
            assert!(
                redacted.contains(REDACTION_PLACEHOLDER),
                "should redact: {error}"
            );
            assert!(
                !redacted.contains("pass word with spaces"),
                "leaked full password: {error} -> {redacted}"
            );
            assert!(
                !redacted.contains("word with spaces"),
                "leaked password tail: {error} -> {redacted}"
            );
        }
    }

    #[test]
    fn test_redact_sql_error_token_and_apikey() {
        // HIGH #8: bare-space `token \S+` matched arbitrary prose
        // (e.g. `JSON_TOKEN parser`) and was deleted. Real credential
        // leaks use the `[=:]` separator form, which still matches.
        let redacted = redact_sql_error("Error: Invalid token=abc123");
        assert!(redacted.contains(REDACTION_PLACEHOLDER));
        assert!(!redacted.contains("abc123"));

        let redacted = redact_sql_error("Error: api_key=sensitive_value");
        assert!(redacted.contains(REDACTION_PLACEHOLDER));
        assert!(!redacted.contains("sensitive_value"));
    }

    #[test]
    fn test_redact_sql_error_unchanged_when_clean() {
        let normal_error = "Error: Table 'test.users' doesn't exist";
        assert_eq!(redact_sql_error(normal_error), normal_error);
    }

    #[test]
    fn test_redact_sql_error_url_userinfo() {
        let msg = "connect failed mysql://alice:hunter2@host:3306/db";
        let redacted = redact_sql_error(msg);
        assert!(!redacted.contains("hunter2"));
        assert!(!redacted.contains("alice"));
        // MEDIUM idempotence fix: URL userinfo is replaced with the
        // shared REDACTION_PLACEHOLDER so a follow-up pass through
        // redact_sql_error (or grep for "REDACTED") sees consistent output.
        assert!(redacted.contains("://***REDACTED***:***REDACTED***@"));
    }

    #[test]
    fn test_redact_sql_error_passwd_pwd_aliases() {
        for raw in [
            "passwd=hunter2",
            "PWD=hunter2",
            "pwd:hunter2",
            "pass=hunter2",
            "PASS:hunter2",
        ] {
            let redacted = redact_sql_error(raw);
            assert!(
                !redacted.contains("hunter2"),
                "leaked sentinel from {:?} -> {:?}",
                raw,
                redacted
            );
        }
    }

    #[test]
    fn test_redact_sql_error_set_password() {
        let q = "SET PASSWORD FOR alice = 'hunter2'";
        let redacted = redact_sql_error(q);
        assert!(!redacted.contains("'hunter2'"), "leaked: {:?}", redacted);
    }

    #[test]
    fn test_redact_sql_error_non_english_labels() {
        for raw in [
            "Kennwort=hunter2",
            "mot_de_passe=hunter2",
            "mot-de-passe=hunter2",
            "contraseña=hunter2",
            "contrasena=hunter2",
        ] {
            let redacted = redact_sql_error(raw);
            assert!(
                !redacted.contains("hunter2"),
                "leaked sentinel from {:?} -> {:?}",
                raw,
                redacted
            );
        }
    }

    #[test]
    fn test_redact_sql_error_idempotent() {
        let raw = "password=hunter2 token=abc api_key=xyz secret=q";
        let once = redact_sql_error(raw);
        let twice = redact_sql_error(&once);
        assert_eq!(once, twice, "redaction should be idempotent");
    }

    #[test]
    fn test_redact_url_with_password() {
        let url = "mysql://user:password@localhost:3306/db";
        let redacted = redact_url(url);
        assert!(redacted.contains(REDACTION_PLACEHOLDER));
        assert!(!redacted.contains("password"));
    }

    #[test]
    fn test_redact_url_username_only() {
        let url = "mysql://user@localhost:3306/db";
        let redacted = redact_url(url);
        assert!(redacted.contains(REDACTION_PLACEHOLDER));
        assert!(!redacted.contains("user@"));
    }

    #[test]
    fn test_redact_url_no_credentials_unchanged() {
        // Non-sensitive URL preserved for debugging traceability.
        let url = "mysql://localhost:3306/db";
        assert_eq!(redact_url(url), url);
    }

    #[test]
    fn test_redact_url_unparseable_falls_back() {
        let redacted = redact_url("not-a-valid-url");
        assert_eq!(redacted, REDACTED_URL_PLACEHOLDER);
    }

    /// CRITICAL #6 regression: URLs where the url crate exposes
    /// userinfo (`user`/`password`) MUST have those fields redacted.
    /// This battery iterates schemes where userinfo is well-defined and
    /// asserts the redactor never lets the raw `user:secret@` substring
    /// through. Together with the fail-closed `set_password`/`set_username`
    /// error handling in `redact_url`, this pins the invariant: even if a
    /// future url-crate release introduces a userinfo-mutation rejection
    /// for a scheme we already handle, the redactor falls back to
    /// [`REDACTED_URL_PLACEHOLDER`] rather than leaking credentials.
    #[test]
    fn test_redact_url_with_special_schemes_never_contains_raw_userinfo() {
        let inputs = [
            "ftp://user:secret123@host.example/path",
            "ssh://user:secret123@host.example",
            "http://user:secret123@host.example/x",
            "https://user:secret123@host.example",
            "mysql://user:secret123@host.example:3306/db",
        ];
        for url in inputs {
            let redacted = redact_url(url);
            assert!(
                !redacted.contains("secret123"),
                "leaked secret from {url:?} -> {redacted:?}"
            );
            assert!(
                !redacted.contains("user:secret123@"),
                "leaked userinfo substring from {url:?} -> {redacted:?}"
            );
        }
    }

    /// CRITICAL #6 fail-closed assertion: synthetic URL inputs that the
    /// `url::Url` parser rejects must always return the placeholder
    /// rather than the original string. Pre-CRITICAL #6 the function
    /// already did this on parse failure; the new contract additionally
    /// covers `set_password` / `set_username` mutation failure on
    /// successfully-parsed URLs. Either failure mode produces the same
    /// observable output (the placeholder), so we assert that downstream.
    #[test]
    fn test_redact_url_returns_placeholder_for_unparseable() {
        for raw in &["", "not-a-url", "://no-scheme", " "] {
            let redacted = redact_url(raw);
            assert_eq!(redacted, REDACTED_URL_PLACEHOLDER, "input was {raw:?}");
        }
    }

    #[test]
    fn test_redact_dump_query_delegates_to_sql_redactor() {
        // Same patterns, no drift.
        let q = "CREATE USER 'x' IDENTIFIED BY 'hunter2'";
        let via_dump = redact_dump_query(q);
        let via_sql = redact_sql_error(q);
        assert_eq!(via_dump, via_sql);
        assert!(!via_dump.contains("'hunter2'"));
    }

    /// HIGH #8 regression: label patterns must not fire on substrings
    /// that happen to contain a label as part of a larger identifier.
    /// Pre-fix the bare `pass` pattern matched `boarding_pass=ABC` and
    /// turned it into `***REDACTED***`. The `\b` left-anchor restricts
    /// the matcher to real label boundaries.
    #[test]
    fn test_redact_sql_error_does_not_match_substring_labels() {
        // `boarding_pass` contains `pass` but is not a credential label;
        // \b ensures the matcher does not fire on `_pass`.
        let raw = "boarding_pass=ABC";
        let redacted = redact_sql_error(raw);
        assert_eq!(redacted, raw, "boarding_pass=ABC must survive unchanged");

        // `the secret ingredient is salt` previously matched the bare
        // `secret \S+` pattern and got mangled. With the bare-space
        // variant deleted (and `\b` on the [=:] form), prose survives.
        let raw = "the secret ingredient is salt";
        let redacted = redact_sql_error(raw);
        assert_eq!(redacted, raw, "prose containing 'secret' must survive");

        // `JSON_TOKEN parser` previously matched the bare `token \S+`
        // pattern. Same deletion + boundary fix as above.
        let raw = "JSON_TOKEN parser";
        let redacted = redact_sql_error(raw);
        assert_eq!(redacted, raw, "JSON_TOKEN parser must survive unchanged");
    }

    /// HIGH #8 positive control: real `password=` lines still get
    /// scrubbed after the boundary fix. Establishes that we did not
    /// over-restrict the matcher while removing false positives.
    #[test]
    fn test_redact_sql_error_password_keyvalue_still_redacts() {
        let raw = "password=hunter2";
        let redacted = redact_sql_error(raw);
        assert!(
            redacted.contains(REDACTION_PLACEHOLDER),
            "real password label must still be redacted; got {redacted:?}"
        );
        assert!(!redacted.contains("hunter2"), "secret leaked: {redacted:?}");
    }

    /// Corpus-based assertion that EVERY pattern in
    /// [`REDACTION_PATTERN_DEFS`] actually fires on a representative
    /// secret-bearing input. The pre-existing
    /// [`redaction_pattern_count_matches_expected`] test only catches
    /// the silent-drop case (a regex that fails to compile); this test
    /// catches the bypass case (a regex that compiles but doesn't
    /// match its target — e.g. a future maintainer mistypes the regex
    /// in a way that still parses).
    ///
    /// Each case pairs a representative input with a label naming the
    /// pattern it exercises. Adding a new pattern to
    /// [`REDACTION_PATTERN_DEFS`] should be paired with adding a new
    /// case here; otherwise the redaction surface grows without test
    /// coverage and a regression in the new pattern goes undetected.
    #[test]
    fn each_pattern_actually_redacts_its_target() {
        let cases: &[(&str, &str)] = &[
            ("password=hunter2", "password"),
            ("PASSWORD=hunter2", "password (case)"),
            ("passwd=hunter2", "passwd"),
            ("pwd=hunter2", "pwd"),
            ("pass=hunter2", "pass"),
            ("token=abc123", "token"),
            ("api_key=k1", "api_key"),
            ("api-key=k2", "api-key"),
            ("secret=s1", "secret"),
            ("identified by 'pw'", "identified by"),
            (
                "identified with mysql_native_password by 'pw'",
                "identified with ... by",
            ),
            ("kennwort=pw", "kennwort"),
            ("mot_de_passe=pw", "mot_de_passe"),
            ("contrasena=pw", "contrasena"),
            ("set password = 'pw'", "set password"),
            ("set password for u@h = 'pw'", "set password for"),
        ];
        for (input, label) in cases {
            let redacted = redact_sql_error(input);
            assert!(
                redacted.contains(REDACTION_PLACEHOLDER),
                "pattern `{}` should have redacted: `{}` -> `{}`",
                label,
                input,
                redacted
            );
        }

        // URL userinfo case: the userinfo regex emits its own
        // `://***REDACTED***:***REDACTED***@` substring rather than the
        // bare placeholder, so check for the URL-shaped marker
        // explicitly.
        let url_input = "mysql://u:p@h/d";
        let url_redacted = redact_sql_error(url_input);
        assert!(
            url_redacted.contains("://***REDACTED***:***REDACTED***@"),
            "URL userinfo pattern should have redacted: `{}` -> `{}`",
            url_input,
            url_redacted,
        );
    }

    /// MEDIUM idempotence regression: `redact_url` rewrites a URL's
    /// userinfo to use [`REDACTION_PLACEHOLDER`]; `redact_sql_error`
    /// must then leave that output untouched (or, equivalently, produce
    /// a stable fixed point). Pre-fix the URL regex emitted `://***:***@`
    /// (a different placeholder), so chaining the two redactors changed
    /// the output and operators grepping for "REDACTED" missed lines
    /// that flowed through `redact_sql_error` after `redact_url`.
    #[test]
    fn redact_sql_error_is_idempotent_over_redact_url_output() {
        let url = "mysql://alice:secret@host:3306/db";
        let r1 = redact_url(url);
        let r2 = redact_sql_error(&r1);
        let r3 = redact_sql_error(&r2);
        assert_eq!(
            r2, r3,
            "redact_sql_error must be idempotent over redact_url output"
        );
        assert!(!r2.contains("alice"), "username must be redacted: {r2:?}");
        assert!(!r2.contains("secret"), "password must be redacted: {r2:?}");
        assert!(
            r2.contains(REDACTION_PLACEHOLDER),
            "shared placeholder must appear: {r2:?}"
        );
    }
}
