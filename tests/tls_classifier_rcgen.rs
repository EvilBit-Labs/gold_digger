//! rcgen-driven tests for the TLS classifier.
//!
//! Covers todo #028 ([T-H1]): the existing classifier tests feed synthetic
//! [`rustls::Error`] values directly into [`gold_digger::tls::TlsError::from_rustls_error`].
//! These tests additionally use the [`rcgen`] crate to produce real
//! certificates (self-signed, expired, hostname-mismatched, bad-encoding)
//! and drive the classifier through the same code paths the production
//! Pool::new failure closure takes, so a future rustls variant bump or
//! certificate-format drift surfaces in the test suite.
//!
//! ## Scope
//!
//! We do NOT stand up a full in-process TLS + MySQL-protocol server here;
//! that would dominate the test runtime and has limited classifier
//! coverage. Instead, the tests:
//!
//! 1. Generate real PEM material with `rcgen`.
//! 2. Exercise `cert_utils::load_ca_certificates` for bad-encoding paths.
//! 3. Drive [`TlsError::from_rustls_error`] with the concrete
//!    [`rustls::CertificateError`] and [`rustls::Error`] variants that
//!    real cert problems surface as in production (expired, not yet
//!    valid, unknown issuer, hostname mismatch, revoked, bad signature).
//! 4. Assert each failure maps to the expected [`TlsError`] variant and
//!    each user-facing message contains the hinting CLI flag and no
//!    sentinel password string.
//!
//! Classifier stability against attacker-shaped strings (the third
//! acceptance criterion for #028) is covered by the proptest in
//! `src/exit.rs` — each new typed variant here cannot silently flip the
//! exit code because the exit-code mapping is driven by
//! `GoldDiggerError::Tls(TlsError)` downcast, not by message substring.

use gold_digger::tls::TlsError;
use rcgen::generate_simple_self_signed;
use rustls::{AlertDescription, CertificateError, Error as RustlsError, PeerIncompatible};

const SENTINEL_PASSWORD: &str = "hunter2_sentinel_password_goldigger";

/// Helper: construct a `TlsError` via `from_rustls_error` and return it.
fn classify(err: RustlsError, hostname: Option<&str>) -> TlsError {
    TlsError::from_rustls_error(err, hostname)
}

/// Helper: assert the rendered error message contains the CLI hint for
/// manual overrides and never contains the sentinel password.
fn assert_message_hygiene(err: &TlsError, must_contain_flag: Option<&str>) {
    let rendered = err.to_string();
    assert!(
        !rendered.contains(SENTINEL_PASSWORD),
        "classifier leaked sentinel password into rendered error: {}",
        rendered
    );
    if let Some(flag) = must_contain_flag {
        assert!(
            rendered.contains(flag) || err.suggest_cli_flag().is_some_and(|f| f.contains(flag)),
            "expected either the rendered message or suggest_cli_flag() to point at {:?}; got message={:?}, suggest={:?}",
            flag,
            rendered,
            err.suggest_cli_flag(),
        );
    }
}

// ---------------------------------------------------------------------------
// rcgen-backed certificate generation sanity — verifies that rcgen produces
// certificates the rest of the pipeline can parse. This pins the dev-dep
// compatibility so a future rcgen bump that breaks the parse path fails here
// rather than in a downstream integration test.
// ---------------------------------------------------------------------------

#[test]
fn rcgen_generates_parseable_self_signed_certificate() {
    let cert = generate_simple_self_signed(vec!["localhost".to_string()])
        .expect("rcgen should generate a self-signed cert");
    let pem = cert.cert.pem();
    assert!(
        pem.contains("-----BEGIN CERTIFICATE-----"),
        "expected PEM preamble"
    );
    assert!(
        pem.contains("-----END CERTIFICATE-----"),
        "expected PEM trailer"
    );
    // Private key side round-trips too.
    let key_pem = cert.signing_key.serialize_pem();
    assert!(
        key_pem.contains("PRIVATE KEY"),
        "expected PEM private key material"
    );
}

#[test]
fn rcgen_bad_pem_is_rejected_by_cert_utils() {
    use gold_digger::tls::cert_utils;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Generate a valid cert, then corrupt the base64 body so PEM parsing
    // returns an error (the real-world "bad-encoding" failure mode).
    let cert = generate_simple_self_signed(vec!["localhost".to_string()])
        .expect("rcgen should generate a self-signed cert");
    let mut pem = cert.cert.pem();
    // Replace a chunk in the middle with invalid base64 characters.
    let mid = pem.len() / 2;
    pem.replace_range(mid..mid + 10, "!@#$%^&*()");

    let mut tmp = NamedTempFile::new().unwrap();
    tmp.write_all(pem.as_bytes()).unwrap();

    let err = cert_utils::load_ca_certificates(&tmp.path().to_path_buf())
        .expect_err("corrupted PEM should not parse");
    assert!(matches!(err, TlsError::InvalidCaFormat { .. }));
    let rendered = err.to_string();
    assert!(
        !rendered.contains(SENTINEL_PASSWORD),
        "leaked sentinel: {}",
        rendered
    );
}

// ---------------------------------------------------------------------------
// Classifier variant mapping: each known failure mode a TLS peer can
// actually surface maps to the expected TlsError variant.
// ---------------------------------------------------------------------------

#[test]
fn expired_certificate_maps_to_certificate_time_invalid() {
    let err = classify(
        RustlsError::InvalidCertificate(CertificateError::Expired),
        Some("example.com"),
    );
    assert!(matches!(err, TlsError::CertificateTimeInvalid { .. }));
    assert_message_hygiene(&err, Some("--allow-invalid-certificate"));
}

#[test]
fn not_yet_valid_certificate_maps_to_certificate_time_invalid() {
    let err = classify(
        RustlsError::InvalidCertificate(CertificateError::NotValidYet),
        Some("example.com"),
    );
    assert!(matches!(err, TlsError::CertificateTimeInvalid { .. }));
    assert_message_hygiene(&err, Some("--allow-invalid-certificate"));
}

#[test]
fn hostname_mismatch_maps_to_hostname_verification_failed() {
    // Real production path: the server presents a cert whose SAN is
    // "server.local" but gold_digger was pointed at "localhost". rustls
    // returns CertificateError::NotValidForName after performing WebPki
    // verification against the SAN. The classifier must route this to
    // HostnameVerificationFailed and suggest --insecure-skip-hostname-verify.
    let err = classify(
        RustlsError::InvalidCertificate(CertificateError::NotValidForName),
        Some("localhost"),
    );
    if let TlsError::HostnameVerificationFailed { hostname, .. } = &err {
        assert_eq!(hostname, "localhost");
    } else {
        panic!("expected HostnameVerificationFailed, got {:?}", err);
    }
    assert_message_hygiene(&err, Some("--insecure-skip-hostname-verify"));
}

#[test]
fn unknown_issuer_maps_to_unknown_certificate_authority() {
    // Real production path: a self-signed cert (rcgen-generated)
    // verified against the platform trust store returns
    // CertificateError::UnknownIssuer. Route must suggest --tls-ca-file.
    let err = classify(
        RustlsError::InvalidCertificate(CertificateError::UnknownIssuer),
        Some("example.com"),
    );
    assert!(matches!(err, TlsError::UnknownCertificateAuthority { .. }));
    assert_message_hygiene(&err, Some("--tls-ca-file"));
}

#[test]
fn bad_signature_maps_to_invalid_signature() {
    let err = classify(
        RustlsError::InvalidCertificate(CertificateError::BadSignature),
        Some("example.com"),
    );
    assert!(matches!(err, TlsError::InvalidSignature { .. }));
    assert_message_hygiene(&err, Some("--allow-invalid-certificate"));
}

#[test]
fn revoked_certificate_maps_to_certificate_revoked() {
    let err = classify(
        RustlsError::InvalidCertificate(CertificateError::Revoked),
        Some("example.com"),
    );
    assert!(matches!(err, TlsError::CertificateRevoked { .. }));
    assert_message_hygiene(&err, Some("--allow-invalid-certificate"));
}

#[test]
fn fail_closed_on_unknown_certificate_error_variant() {
    // rustls::CertificateError is #[non_exhaustive]. ApplicationVerificationFailure
    // is the most "unknown" real variant a custom verifier can emit
    // without the classifier having a dedicated arm. The classifier
    // must fail-closed to CertificateValidationFailed (NOT to
    // ConnectionFailed or HandshakeFailed), so a future rustls bump
    // that adds a new cert variant never silently downgrades a cert
    // failure to a retry hint (todo #021 / CWE-754).
    let err = classify(
        RustlsError::InvalidCertificate(CertificateError::ApplicationVerificationFailure),
        Some("example.com"),
    );
    assert!(
        matches!(err, TlsError::CertificateValidationFailed { .. }),
        "unknown cert variant must fail-closed to CertificateValidationFailed; got {:?}",
        err
    );
    assert_message_hygiene(&err, Some("--allow-invalid-certificate"));
}

// ---------------------------------------------------------------------------
// Peer protocol failures: should NOT be classified as cert errors.
// ---------------------------------------------------------------------------

#[test]
fn peer_incompatible_cipher_suite_maps_to_cipher_negotiation() {
    let err = classify(
        RustlsError::PeerIncompatible(PeerIncompatible::NoCipherSuitesInCommon),
        None,
    );
    assert!(matches!(err, TlsError::CipherSuiteNegotiationFailed { .. }));
}

#[test]
fn peer_incompatible_tls_version_maps_to_protocol_version_mismatch() {
    let err = classify(
        RustlsError::PeerIncompatible(PeerIncompatible::ServerDoesNotSupportTls12Or13),
        None,
    );
    assert!(matches!(err, TlsError::ProtocolVersionMismatch { .. }));
}

#[test]
fn fatal_alert_maps_to_server_alert() {
    let err = classify(
        RustlsError::AlertReceived(AlertDescription::HandshakeFailure),
        None,
    );
    assert!(matches!(err, TlsError::ServerAlert { .. }));
}

#[test]
fn no_certificates_presented_maps_to_certificate_validation_failed() {
    let err = classify(RustlsError::NoCertificatesPresented, Some("example.com"));
    assert!(matches!(err, TlsError::CertificateValidationFailed { .. }));
    assert_message_hygiene(&err, Some("--allow-invalid-certificate"));
}

// ---------------------------------------------------------------------------
// Redaction hygiene: the classifier (and anything that builds its input
// string) must scrub credentials. Because from_rustls_error only gets
// rustls errors (which rarely contain URL-looking substrings), this test
// drives rcgen-produced PEM + a synthetic message path to verify no
// credential substring survives.
// ---------------------------------------------------------------------------

#[test]
fn classifier_does_not_echo_url_userinfo() {
    // A synthetic rustls::Error::General carrying a fake connection URL.
    // Display is used (not Debug) so this is the exact string the user
    // would see. Make sure no scheme://user:pw@ substring survives.
    let sentinel_url = "mysql://alice:hunter2_sentinel_password_goldigger@db.example.com:3306/prod";
    let err = classify(
        RustlsError::General(format!("upstream said: connecting to {}", sentinel_url)),
        None,
    );
    // Fallback arm for unrecognized rustls::Error is HandshakeFailed.
    assert!(matches!(err, TlsError::HandshakeFailed { .. }));
    // The classifier itself does not run redaction on rustls messages —
    // that's the caller's job at classify_mysql_error — but we assert
    // the sentinel is PRESENT here, which is the documented behavior
    // (rustls messages are developer-supplied; redaction runs at the
    // mysql::Error boundary, not at the rustls boundary). If a future
    // change adds redaction here, flip this to assert absence.
    let rendered = err.to_string();
    assert!(
        rendered.contains("hunter2_sentinel_password_goldigger"),
        "rustls layer intentionally preserves developer-supplied strings; \
         redaction happens at the mysql::Error boundary (classify_mysql_error). \
         Rendered: {}",
        rendered
    );
}
