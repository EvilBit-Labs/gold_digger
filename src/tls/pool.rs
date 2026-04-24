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

/// Creates a MySQL connection pool with rustls-only TLS configuration
pub fn create_tls_connection(
    database_url: &str,
    tls_config: Option<TlsConfig>,
    verbose: bool,
) -> Result<Pool, TlsError> {
    use mysql::{Opts, OptsBuilder};

    // Parse the database URL first to validate format
    let opts = Opts::from_url(database_url)
        .map_err(|e| TlsError::connection_failed(format!("Invalid database URL format: {}", e)))?;

    let mut opts_builder = OptsBuilder::from_opts(opts);

    // Apply TLS configuration if provided
    if let Some(config) = tls_config {
        match config.to_ssl_opts() {
            Ok(Some(ssl_opts)) => {
                opts_builder = opts_builder.ssl_opts(ssl_opts);

                // Log TLS configuration details in verbose mode
                if verbose {
                    match config.validation_mode() {
                        TlsValidationMode::Platform => {
                            eprintln!("[TLS] Using platform certificate store");
                        }
                        TlsValidationMode::CustomCa { ca_file_path } => {
                            eprintln!("[TLS] Using custom CA file: {}", ca_file_path.display());
                        }
                        TlsValidationMode::SkipHostnameVerification => {
                            eprintln!("[WARNING] TLS: Hostname verification disabled");
                        }
                        TlsValidationMode::AcceptInvalid => {
                            eprintln!("[DANGER] TLS: Certificate validation disabled");
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
            eprintln!(
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
    // via stderr (CWE-532/CWE-209). The substring-classification below
    // looks at `error_lower` only for routing — the user-facing string is
    // built from `redacted_error`.
    Pool::new(opts_builder).map_err(|mysql_error| {
        let error_string = mysql_error.to_string();
        let error_lower = error_string.to_lowercase();
        let redacted_error = crate::utils::redact_sql_error(&error_string);

        // Check for TLS/SSL related errors and provide specific guidance
        if error_lower.contains("ssl") || error_lower.contains("tls") {
            if error_lower.contains("certificate") || error_lower.contains("cert") {
                if error_lower.contains("expired") || error_lower.contains("not yet valid") {
                    TlsError::certificate_time_invalid(format!(
                        "Certificate validity period error: {}. Use --allow-invalid-certificate to bypass",
                        redacted_error
                    ))
                } else if error_lower.contains("hostname") || error_lower.contains("name") || error_lower.contains("san") {
                    TlsError::hostname_verification_failed(
                        "server".to_string(),
                        format!(
                            "Hostname verification failed: {}. Use --insecure-skip-hostname-verify to bypass",
                            redacted_error
                        )
                    )
                } else if error_lower.contains("unknown") || error_lower.contains("untrusted") || error_lower.contains("issuer") {
                    TlsError::unknown_certificate_authority(format!(
                        "Certificate authority not trusted: {}. Use --tls-ca-file <path> for custom CA or --allow-invalid-certificate for testing",
                        redacted_error
                    ))
                } else if error_lower.contains("signature") || error_lower.contains("invalid") {
                    TlsError::invalid_signature(format!(
                        "Certificate signature validation failed: {}. Use --allow-invalid-certificate to bypass",
                        redacted_error
                    ))
                } else {
                    TlsError::certificate_validation_failed(format!(
                        "Certificate validation failed: {}. Try --allow-invalid-certificate for testing",
                        redacted_error
                    ))
                }
            } else if error_lower.contains("handshake") {
                TlsError::handshake_failed(format!(
                    "TLS handshake failed: {}. Check server TLS configuration and supported protocols",
                    redacted_error
                ))
            } else if error_lower.contains("protocol") || error_lower.contains("version") {
                TlsError::protocol_version_mismatch(format!(
                    "TLS protocol version mismatch: {}. Server may not support TLS 1.2/1.3",
                    redacted_error
                ))
            } else if error_lower.contains("cipher") {
                TlsError::cipher_suite_negotiation_failed(format!(
                    "TLS cipher suite negotiation failed: {}. Server and client have no compatible cipher suites",
                    redacted_error
                ))
            } else {
                TlsError::connection_failed(format!(
                    "TLS connection failed: {}. Check server TLS configuration",
                    redacted_error
                ))
            }
        } else if error_lower.contains("connection") || error_lower.contains("connect") {
            TlsError::connection_failed(format!(
                "Database connection failed: {}. Check server availability and network connectivity",
                redacted_error
            ))
        } else if error_lower.contains("auth") || error_lower.contains("access denied") || error_lower.contains("password") {
            TlsError::connection_failed(format!(
                "Database authentication failed: {}. Check username and password",
                redacted_error
            ))
        } else if error_lower.contains("timeout") {
            TlsError::connection_failed(format!(
                "Database connection timeout: {}. Check network connectivity and server responsiveness",
                redacted_error
            ))
        } else {
            // Generic connection error
            TlsError::connection_failed(format!(
                "Database connection failed: {}",
                redacted_error
            ))
        }
    })
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
