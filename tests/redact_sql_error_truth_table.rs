//! Truth-table coverage for [`gold_digger::utils::redact_sql_error`] (todo #074).
//!
//! The `src/utils.rs` unit suite already covers the major redactor
//! behaviours (URL userinfo, `passwd`/`pwd`/`pass` aliases, set-password,
//! non-English labels, idempotence) but each test asserts against a
//! single shape. That shape coverage is hard to audit at a glance and
//! easy to under-grow when a new pattern lands in `pattern_defs`.
//!
//! This file is the explicit one-row-per-pattern table the todo asks
//! for: every primary pattern in
//! [`gold_digger::utils::redact_sql_error`]'s pattern set is paired
//! with a high-entropy sentinel and run through the redactor twice
//! (idempotence). Adding a new regex without a row here is a documented
//! review smell.
//!
//! See `src/utils.rs::tests::test_redact_sql_error_idempotent` and
//! `tests/credential_leak_regression.rs` for the in-process and
//! end-to-end leak suites that complement this file.

use gold_digger::utils::redact_sql_error;
use rstest::rstest;

/// One row per pattern in [`gold_digger::utils::get_redaction_patterns`].
///
/// `label` is purely diagnostic; `raw` is fed to the redactor; `sentinel`
/// is the high-entropy substring that must NOT appear in the redacted
/// output. The non-ASCII case (`contrasena_unicode`) doubles as the
/// non-ASCII boundary test required by todo #074.
#[rstest]
#[case::password_eq("password=", "password=Hunter2_pw01", "Hunter2_pw01")]
#[case::passwd_eq("passwd=", "passwd=Hunter2_pw02", "Hunter2_pw02")]
#[case::pwd_eq("pwd=", "pwd=Hunter2_pw03", "Hunter2_pw03")]
#[case::pass_eq("pass=", "pass=Hunter2_pw04", "Hunter2_pw04")]
#[case::identified_by(
    "identified by",
    "GRANT ALL ON *.* IDENTIFIED BY 'Hunter2_pw05'",
    "Hunter2_pw05"
)]
#[case::identified_with_by(
    "identified with ... by",
    "CREATE USER x IDENTIFIED WITH plug BY 'Hunter2_pw06'",
    "Hunter2_pw06"
)]
#[case::token_eq("token=", "token=Hunter2_pw07", "Hunter2_pw07")]
#[case::token_space("token <space>", "token Hunter2_pw08", "Hunter2_pw08")]
#[case::api_key_underscore("api_key=", "api_key=Hunter2_pw09", "Hunter2_pw09")]
#[case::api_key_dash("api-key=", "api-key=Hunter2_pw10", "Hunter2_pw10")]
#[case::secret_eq("secret=", "secret=Hunter2_pw11", "Hunter2_pw11")]
#[case::secret_space("secret <space>", "secret Hunter2_pw12", "Hunter2_pw12")]
#[case::set_password(
    "set password",
    "SET PASSWORD FOR alice = 'Hunter2_pw13'",
    "Hunter2_pw13"
)]
#[case::kennwort("kennwort=", "Kennwort=Hunter2_pw14", "Hunter2_pw14")]
#[case::mot_de_passe("mot_de_passe=", "mot_de_passe=Hunter2_pw15", "Hunter2_pw15")]
#[case::contrasena_ascii("contrasena=", "contrasena=Hunter2_pw16", "Hunter2_pw16")]
#[case::contrasena_unicode("contraseña=", "contraseña=Hunter2_pw17", "Hunter2_pw17")]
#[case::url_userinfo(
    "url userinfo",
    "mysql://alice:Hunter2_pw18@db.example.com:3306/x",
    "Hunter2_pw18"
)]
fn redact_sql_error_pattern_table(#[case] label: &str, #[case] raw: &str, #[case] sentinel: &str) {
    let once = redact_sql_error(raw);
    assert!(
        !once.contains(sentinel),
        "[{}] sentinel {:?} leaked through first redaction pass; output={:?}",
        label,
        sentinel,
        once
    );

    // Idempotence: a second pass must produce identical output. The
    // primary idempotence test in src/utils.rs::tests covers a multi-
    // pattern blob; this loop pins it per pattern so a regression in
    // one regex's replacement string surfaces here, not in aggregate.
    let twice = redact_sql_error(&once);
    assert_eq!(
        twice, once,
        "[{}] redaction is not idempotent: once={:?} twice={:?}",
        label, once, twice
    );
}

/// Sanity: a benign string with none of the redactor's trigger tokens
/// must pass through untouched. Pairs with the truth table to pin
/// "we don't over-redact" in the same place "we don't under-redact"
/// is asserted.
#[test]
fn redact_sql_error_benign_string_pass_through() {
    let benign = "Error: Table 'analytics.users' doesn't exist";
    assert_eq!(redact_sql_error(benign), benign);
}
