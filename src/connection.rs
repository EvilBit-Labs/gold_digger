//! Database connection pool construction with rustls-only TLS.
//!
//! The `TlsOptions` → `TlsConfig` adapter lives in [`crate::cli`] so the
//! `tls` module has zero dependency on CLI types.

use anyhow::Result;
use mysql::Pool;

use crate::cli::Cli;
use crate::tls::create_tls_connection;

/// Creates a database connection pool with rustls-only TLS configuration from CLI.
///
/// Installs the rustls default crypto provider on first call (todo #169).
/// Previously this happened unconditionally at the top of `main()`, which
/// charged the ~5-10 ms install cost to every invocation — including
/// `--help`, `--version`, `completion`, and `--dump-config`. Moving it
/// here means only paths that actually open a database connection pay
/// that cost.
pub fn create_database_connection(database_url: &str, cli: &Cli) -> Result<Pool> {
    // Lazy crypto-provider install. `init_crypto_provider` is idempotent
    // (guarded by an `OnceLock`) so repeated calls within a process are
    // safe — only the first does any work.
    crate::init_crypto_provider();

    // Create TLS configuration from CLI options
    let tls_config = if cli.tls_options.tls_ca_file.is_some()
        || cli.tls_options.insecure_skip_hostname_verify
        || cli.tls_options.allow_invalid_certificate
    {
        let config = cli
            .tls_options
            .to_tls_config()
            .map_err(|e| anyhow::anyhow!("TLS configuration error: {}", e))?;

        // Display security warnings for insecure modes (includes the
        // mandatory DANGER delay for AcceptInvalid — see #022).
        config.display_security_warnings();

        Some(config)
    } else {
        // Use default TLS behavior when no explicit TLS flags are provided
        // This will use platform certificate store with rustls
        None
    };

    // Use rustls-only TLS connection creation with enhanced error handling
    create_tls_connection(database_url, tls_config, cli.verbose > 0).map_err(|tls_error| {
        // Convert TLS errors to anyhow errors with appropriate context
        match &tls_error {
            crate::tls::TlsError::CertificateValidationFailed { .. }
            | crate::tls::TlsError::CertificateTimeInvalid { .. }
            | crate::tls::TlsError::InvalidSignature { .. }
            | crate::tls::TlsError::UnknownCertificateAuthority { .. }
            | crate::tls::TlsError::InvalidCertificatePurpose { .. }
            | crate::tls::TlsError::CertificateChainInvalid { .. }
            | crate::tls::TlsError::CertificateRevoked { .. } => {
                // Certificate validation errors - suggest appropriate CLI flag
                if let Some(suggestion) = tls_error.suggest_cli_flag() {
                    anyhow::anyhow!("{}. Suggestion: {}", tls_error, suggestion)
                } else {
                    anyhow::anyhow!("{}", tls_error)
                }
            }
            crate::tls::TlsError::HostnameVerificationFailed { .. } => {
                // Hostname verification errors - suggest skip hostname flag
                anyhow::anyhow!(
                    "{}. Suggestion: {}",
                    tls_error,
                    tls_error
                        .suggest_cli_flag()
                        .unwrap_or("--insecure-skip-hostname-verify")
                )
            }
            crate::tls::TlsError::CaFileNotFound { .. }
            | crate::tls::TlsError::InvalidCaFormat { .. }
            | crate::tls::TlsError::MutuallyExclusiveFlags { .. } => {
                // Client configuration errors - no additional context needed
                anyhow::anyhow!("{}", tls_error)
            }
            _ => {
                // Other TLS errors (handshake, connection, server issues)
                anyhow::anyhow!("Database connection failed: {}", tls_error)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// Creates a CLI instance with common test arguments
    fn build_test_cli() -> Cli {
        Cli::parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
        ])
    }

    /// Creates a CLI instance with TLS options for testing
    fn build_test_cli_with_tls() -> Cli {
        Cli::parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--insecure-skip-hostname-verify",
        ])
    }

    #[test]
    fn test_create_database_connection_invalid_url() {
        // Test with invalid URL to ensure error handling works
        let cli = build_test_cli();
        let result = create_database_connection("invalid://url", &cli);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_database_connection() {
        // Test that the function exists and handles errors properly
        let cli = build_test_cli();
        let result =
            create_database_connection("mysql://invalid:invalid@nonexistent:3306/test", &cli);
        // Should fail due to invalid connection details, but not panic
        assert!(result.is_err());
    }

    #[test]
    fn test_create_database_connection_with_tls_options() {
        // Test TLS configuration path
        let cli = build_test_cli_with_tls();
        let result =
            create_database_connection("mysql://invalid:invalid@nonexistent:3306/test", &cli);
        // Should fail due to invalid connection details, but TLS config should be processed
        assert!(result.is_err());
    }
}
