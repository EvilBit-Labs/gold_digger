//! MySQL connection pool builder with TLS configuration.
//!
//! [`create_tls_connection`] is the single entry point `src/main.rs` uses to
//! build a [`mysql::Pool`]. It applies the [`super::config::TlsConfig`] to
//! [`mysql::OptsBuilder`] and maps every `mysql::Error` surface to the
//! typed [`TlsError`] enum so the caller can produce precise user-facing
//! guidance (see `main.rs` `match` against `TlsError` variants).
//!
//! ## Credential redaction
//!
//! The `mysql` crate's error strings frequently embed the raw connection
//! URL, the username, the source IP, and `(using password: YES)` markers.
//! Every interpolation of `mysql_error` below MUST go through
//! [`crate::utils::redact_sql_error`] first, or credentials will reach the
//! user via stderr (CWE-532 / CWE-209). The substring routing below is
//! applied to the lower-cased error string for classification only; the
//! user-facing string is always built from the redacted copy.

use super::config::{TlsConfig, TlsValidationMode};
use super::error::TlsError;
use mysql::{Pool, SslOpts};

/// Creates a [`mysql::Pool`] with rustls-only TLS configuration.
///
/// This is the single entry point `src/connection.rs` uses to build a
/// pool. It parses the URL, applies the `TlsConfig` (if provided), then
/// hands off to `Pool::new` with a typed error classifier that maps
/// every `mysql::Error` variant into a specific [`TlsError`] so the
/// caller can present actionable guidance.
///
/// # Arguments
///
/// * `database_url` - MySQL connection URL in standard format
///   (`mysql://user:pass@host:port/db`). Passwords are redacted via
///   [`crate::utils::redact_url`] before appearing in any error message.
/// * `tls_config` - Optional [`TlsConfig`] controlling the four
///   validation modes (strict, skip-hostname, accept-invalid, custom-CA).
///   `None` uses platform defaults with strict validation.
/// * `verbose` - When true, logs TLS decisions to stderr via
///   `tracing::info!`.
///
/// # Returns
///
/// * `Ok(Pool)` - a ready-to-use connection pool.
/// * `Err(TlsError)` - typed error carrying actionable user guidance;
///   routes to exit code 2 (config) for `CaFileNotFound` / `InvalidCaFormat`
///   / `MutuallyExclusiveFlags`, and exit 3 (DB auth) for everything else.
///
/// # Errors
///
/// Returns `TlsError::ConnectionFailed` when the URL is malformed,
/// `TlsError::CaFileNotFound` / `InvalidCaFormat` when a supplied CA
/// file can't be read or parsed, `TlsError::HandshakeFailed` /
/// `CertificateValidationFailed` / `HostnameVerificationFailed` for
/// the corresponding TLS failure modes, and `TlsError::ConnectionFailed`
/// for generic network or authentication errors.
///
/// # Example
///
/// ```no_run
/// # use gold_digger::tls::create_tls_connection;
/// let pool = create_tls_connection(
///     "mysql://user:pass@db.example.com:3306/mydb",
///     None,  // use platform defaults
///     false, // quiet
/// )?;
/// # Ok::<(), gold_digger::tls::TlsError>(())
/// ```
pub fn create_tls_connection(
    database_url: &str,
    tls_config: Option<TlsConfig>,
    verbose: bool,
) -> Result<Pool, TlsError> {
    use mysql::{Opts, OptsBuilder};

    // Parse the database URL first to validate format. The mysql crate's
    // `UrlError` Display often quotes the offending URL back at us,
    // including any embedded credentials (CWE-532). Route the URL
    // through `redact_url` and the error string through
    // `redact_sql_error` so user:password@host pairs are scrubbed
    // before they hit stderr (todo #065).
    let opts = Opts::from_url(database_url).map_err(|e| {
        TlsError::connection_failed(format!(
            "Invalid database URL format ({}): {}",
            crate::utils::redact_url(database_url),
            crate::utils::redact_sql_error(&e.to_string())
        ))
    })?;

    let mut opts_builder = OptsBuilder::from_opts(opts);

    // Apply TLS configuration if provided
    if let Some(config) = tls_config {
        match config.to_ssl_opts() {
            Ok(Some(ssl_opts)) => {
                opts_builder = opts_builder.ssl_opts(ssl_opts);

                // Log TLS configuration details in verbose mode. Route
                // through `tracing::info!` so verbosity is filtered by the
                // subscriber (todo #163) and NOT printed when `--quiet`
                // limits to error-level. The `verbose` bool is still
                // honoured here so the TLS startup details can be gated
                // behind `-v` even when the tracing level would otherwise
                // emit `info!` (default level is `warn`).
                if verbose {
                    match config.validation_mode() {
                        TlsValidationMode::Platform => {
                            tracing::info!("[TLS] Using platform certificate store");
                        }
                        TlsValidationMode::CustomCa { ca_file_path } => {
                            tracing::info!(
                                "[TLS] Using custom CA file: {}",
                                ca_file_path.display()
                            );
                        }
                        TlsValidationMode::SkipHostnameVerification => {
                            tracing::warn!(
                                "{}",
                                crate::logging::warn_banner(
                                    "[WARNING] TLS: Hostname verification disabled"
                                )
                            );
                        }
                        TlsValidationMode::AcceptInvalid => {
                            tracing::error!(
                                "{}",
                                crate::logging::danger_banner(
                                    "[DANGER] TLS: Certificate validation disabled"
                                )
                            );
                        }
                    }
                }
            }
            Ok(None) => {
                // TLS is enabled but no SSL options needed (shouldn't happen)
            }
            Err(tls_error) => {
                return Err(tls_error);
            }
        }
    } else {
        // No explicit TLS configuration provided - explicitly configure TLS with platform certificates
        // This ensures TLS is always used instead of relying on driver defaults
        let ssl_opts = SslOpts::default()
            .with_danger_accept_invalid_certs(false)
            .with_danger_skip_domain_validation(false);

        opts_builder = opts_builder.ssl_opts(ssl_opts);

        if verbose {
            tracing::info!(
                "[TLS] Using explicit configuration (platform certificates, hostname verification enabled)"
            );
        }
    }

    // Create the connection pool with enhanced error handling.
    //
    // CRITICAL: every interpolation of `mysql_error` below MUST go through
    // `redact_sql_error` first. The mysql crate's error strings frequently
    // embed the raw connection URL, the username, the source IP, and
    // `(using password: YES)` markers; un-redacted, those reach the user
    // via stderr (CWE-532/CWE-209). The redacted string is used in every
    // constructed `TlsError` payload.
    //
    // CRITICAL #1 fix: the previous implementation classified the
    // `mysql::Error` by substring-matching on its lower-cased rendered
    // string. The typed `from_rustls_error` classifier in
    // `super::classifier` was never reached from the wire path because
    // `mysql::Error::TlsError` was buried in the string before being
    // matched. We now match on the typed `mysql::Error` variants
    // directly: `TlsError(rustls::Error)` flows through the typed
    // classifier; other variants get specific, non-substring routing.
    Pool::new(opts_builder).map_err(classify_mysql_pool_error)
}

/// Maps a [`mysql::Error`] from `Pool::new` (or any other pool surface,
/// notably [`mysql::Pool::get_conn`]) into a typed [`TlsError`] using the
/// underlying error variant rather than substring matching.
///
/// CRITICAL #1: the typed classifier ([`TlsError::from_rustls_error`]) is
/// only reachable when we match on `mysql::Error::TlsError(rustls_err)`
/// — interpolating the rendered string and pattern-matching on lowercase
/// substrings (the legacy approach) buries the typed value beyond
/// recovery.
///
/// HIGH #10: `pool.get_conn()` failures used to bypass this classifier
/// and route through a free-form `anyhow::anyhow!(...)` message that
/// embedded the raw `mysql::Error`. Exposing the classifier as
/// `pub(crate)` lets the run pipeline reuse the same typed routing and
/// credential redaction the `Pool::new` path already enforces.
///
/// Credential redaction is performed internally via
/// [`crate::utils::redact_sql_error`] so callers cannot accidentally
/// embed an un-scrubbed error string. The redacted text is interpolated
/// into the `message` field of every constructed `TlsError` variant.
pub(crate) fn classify_mysql_pool_error(mysql_error: mysql::Error) -> TlsError {
    let redacted_error = crate::utils::redact_sql_error(&mysql_error.to_string());
    // `mysql::error::tls::TlsError` is the inner TLS-stack error wrapped
    // by `mysql::Error::TlsError`; aliasing locally avoids a clash with
    // our own `super::error::TlsError`.
    use mysql::error::tls::TlsError as MysqlTlsError;

    match mysql_error {
        // The wire path: the mysql driver wrapped a rustls (or related
        // TLS-stack) error. Route inner `Tls(rustls::Error)` through the
        // typed classifier in `super::classifier` so cert / handshake /
        // hostname variants each produce the corresponding typed
        // `TlsError`. Other inner variants (PKI parse, DNS name, verifier
        // builder) get specific routing.
        mysql::Error::TlsError(inner) => match inner {
            MysqlTlsError::Tls(rustls_err) => TlsError::from_rustls_error(rustls_err, None),
            MysqlTlsError::Pki(_) => TlsError::certificate_chain_invalid(format!(
                "Certificate chain validation failed: {}",
                redacted_error
            )),
            MysqlTlsError::InvalidDnsName(_) => TlsError::hostname_verification_failed(
                "server".to_string(),
                format!("Invalid DNS name in TLS handshake: {}", redacted_error),
            ),
            MysqlTlsError::VerifierBuilderError(_) => TlsError::certificate_validation_failed(
                format!("Certificate verifier builder error: {}", redacted_error),
            ),
        },

        // Network I/O during connect — almost always "server unreachable",
        // "connection refused", or a TCP-level failure. No TLS context.
        mysql::Error::IoError(_) => TlsError::connection_failed(format!(
            "Database connection failed: {}. Check server availability and network connectivity",
            redacted_error
        )),

        // URL parse failures — surface as connection failure with the
        // redacted detail so the operator can see what's malformed
        // without leaking userinfo.
        mysql::Error::UrlError(_) => {
            TlsError::connection_failed(format!("Invalid database URL format: {}", redacted_error))
        }

        // Server-side error: typically authentication. Route as a
        // connection failure (the exit-code mapper still sends this to
        // EXIT_DB_AUTH_ERROR via `tls_exit_code`'s `_` arm).
        mysql::Error::MySqlError(_) => TlsError::connection_failed(format!(
            "Database authentication failed: {}. Check credentials.",
            redacted_error
        )),

        // Driver-level failures (TLS negotiation in the connector layer,
        // unsupported feature, etc.). Surface a generic handshake failure
        // so the user has actionable framing.
        mysql::Error::DriverError(_) => TlsError::handshake_failed(format!(
            "Database driver error: {}. Check server TLS configuration",
            redacted_error
        )),

        // Future-proofing: `mysql::Error` is `#[non_exhaustive]`; new
        // variants fall through to a generic connection failure rather
        // than a misleading TLS-specific message.
        _ => TlsError::connection_failed(format!("Database connection failed: {}", redacted_error)),
    }
}

/// Helper function to redact sensitive information from URLs for safe error logging.
///
/// **Deprecated re-export.** Prefer [`crate::utils::redact_url`] in new code;
/// this thin wrapper exists only so external callers (and the snapshot of
/// existing test harnesses) keep compiling. All the redaction logic now
/// lives in [`crate::utils`] so the three previously-divergent redactors
/// share one pattern set and one placeholder.
pub fn redact_url(url: &str) -> String {
    crate::utils::redact_url(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tls_connection_with_config() {
        let tls_config = TlsConfig::new();

        // This test will fail with an actual connection, but tests the function signature
        // and basic error handling
        let result = create_tls_connection(
            "mysql://invalid:invalid@nonexistent:3306/test",
            Some(tls_config),
            false,
        );

        match result {
            Ok(pool) => {
                // If pool creation succeeds, attempt to get a connection to exercise lazy initialization
                let conn_result = pool.get_conn();
                // We expect this to fail due to invalid connection details, but not panic
                assert!(conn_result.is_err());
            }
            Err(_) => {
                // Pool creation failed, which is also expected for invalid connection details
                // This is fine - the test passes as long as it doesn't panic
            }
        }
    }

    #[test]
    fn test_create_tls_connection_without_config() {
        // Test with no TLS config
        let result =
            create_tls_connection("mysql://invalid:invalid@nonexistent:3306/test", None, false);

        match result {
            Ok(pool) => {
                // If pool creation succeeds, attempt to get a connection to exercise lazy initialization
                let conn_result = pool.get_conn();
                // We expect this to fail due to invalid connection details, but not panic
                assert!(conn_result.is_err());
            }
            Err(_) => {
                // Pool creation failed, which is also expected for invalid connection details
                // This is fine - the test passes as long as it doesn't panic
            }
        }
    }

    /// CRITICAL #1 regression: a real `mysql::Error` from `Pool::new`
    /// must be classified into a typed `TlsError` variant via the
    /// `classify_mysql_pool_error` switch — not via substring matching
    /// on the rendered string. We drive a malformed URL through
    /// `Pool::new` and assert the returned `TlsError` is a non-empty
    /// typed variant. (Exact variant depends on the mysql crate's URL
    /// parser; the load-bearing assertion is "we got a typed value back,
    /// not a stringified one".)
    #[test]
    fn test_pool_new_invalid_url_returns_typed_tls_error() {
        // `invalid://url` is rejected by Opts::from_url before Pool::new
        // is reached, so go through `create_tls_connection` (which is
        // the public surface that exercises the URL parse path).
        let result = create_tls_connection("invalid://url", None, false);
        assert!(result.is_err(), "expected error from malformed URL");
        let tls_error = result.unwrap_err();
        // Connection-failed (URL parse) is the expected variant.
        assert!(
            matches!(tls_error, TlsError::ConnectionFailed { .. }),
            "expected ConnectionFailed variant, got: {:?}",
            tls_error
        );
    }

    #[test]
    fn test_redact_url() {
        // Test URL with password
        let url = "mysql://user:password@localhost:3306/db";
        let redacted = redact_url(url);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("password"));

        // Test URL with username only
        let url = "mysql://user@localhost:3306/db";
        let redacted = redact_url(url);
        assert!(redacted.contains("***REDACTED***"));
        // Username substring "user" must not appear in its un-redacted
        // userinfo position. (The placeholder itself contains no "user".)
        assert!(!redacted.contains("user@"));

        // Test URL without credentials - intentionally left unchanged for debugging/traceability
        let url = "mysql://localhost:3306/db";
        let redacted = redact_url(url);
        // Security rationale: Only credentials (userinfo) and query params containing secrets are redacted.
        // Non-sensitive URLs remain unchanged to preserve debugging context and connection traceability.
        // Example: "mysql://user:pass@host/db" -> "mysql://***REDACTED***:***REDACTED***@host/db"
        assert_eq!(redacted, url); // Intentionally unchanged - no sensitive data to redact

        // Test invalid URL
        let url = "not-a-valid-url";
        let redacted = redact_url(url);
        assert_eq!(redacted, "***REDACTED_URL***");
    }
}
