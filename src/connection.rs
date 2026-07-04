//! Database connection pool construction with rustls-only TLS.
//!
//! The `TlsOptions` → `TlsConfig` adapter lives in [`crate::cli`] so the
//! `tls` module has zero dependency on CLI types. This module accepts an
//! already-resolved `database_url` and `TlsConfig` so it depends only on
//! infrastructure primitives — no `Cli`-typed input survives past the
//! `ResolvedConfig` boundary.

use anyhow::Result;
use mysql::Pool;

use crate::tls::{TlsConfig, create_tls_connection};

/// Creates a database connection pool with rustls-only TLS configuration.
///
/// Installs the rustls default crypto provider on first call (todo #169).
/// Previously this happened unconditionally at the top of `main()`, which
/// charged the ~5-10 ms install cost to every invocation — including
/// `--help`, `--version`, `completion`, and `--dump-config`. Moving it
/// here means only paths that actually open a database connection pay
/// that cost.
///
/// # Arguments
///
/// * `database_url` — already-resolved MySQL connection URL (typically
///   sourced from [`crate::config::ResolvedConfig::database_url`]).
/// * `tls_config` — already-resolved TLS configuration (typically
///   sourced from [`crate::config::ResolvedConfig::tls`]). The default
///   value ([`TlsConfig::Platform`]) is treated as "no special TLS
///   handling": platform certificate validation with rustls.
/// * `verbose` — verbose-flag count, used to gate the TLS info log.
pub fn create_database_connection(
    database_url: &str,
    tls_config: &TlsConfig,
    verbose: u8,
) -> Result<Pool> {
    // Lazy crypto-provider install. `init_crypto_provider` is idempotent
    // (guarded by an `OnceLock`) so repeated calls within a process are
    // safe — only the first does any work.
    crate::init_crypto_provider();

    // Decide whether the TLS configuration needs to be passed through to
    // `create_tls_connection`. The default `Platform` value matches the
    // historical "no flags supplied" behaviour where platform
    // certificate validation kicks in via rustls; we still hand `None`
    // to the underlying pool builder for that case so the existing
    // OptsBuilder defaults apply.
    let tls_config_for_pool: Option<TlsConfig> = if matches!(tls_config, TlsConfig::Platform) {
        None
    } else {
        // Display security warnings for insecure modes (includes the
        // mandatory DANGER delay for AcceptInvalid — see #022). Cloning
        // is cheap (the validated `TlsConfig` is a small enum) and lets
        // the pool builder consume it by value.
        tls_config.display_security_warnings();
        Some(tls_config.clone())
    };

    // Use rustls-only TLS connection creation with enhanced error handling.
    //
    // CRITICAL #5 fix: the previous implementation interpolated the typed
    // `TlsError` into `anyhow::anyhow!("{}", tls_error)` which flattened
    // it into a plain string — defeating the downcast in
    // `crate::exit::map_error_to_exit_code` and forcing every TLS error
    // through the substring fallback. We now build the anyhow value from
    // the typed error (`anyhow::Error::from(tls_error)`) so the variant
    // survives chain-walking. Suggestion text (where applicable) is
    // emitted as a separate `tracing::error!` BEFORE returning so the
    // user-actionable hint still surfaces without burying the typed value.
    create_tls_connection(database_url, tls_config_for_pool, verbose > 0).map_err(|tls_error| {
        // Emit suggestion-augmented branches as a separate log line; the
        // returned anyhow value carries the typed `TlsError` for the
        // downcast classifier.
        let context_prefix: &str = match &tls_error {
            crate::tls::TlsError::CertificateValidationFailed { .. }
            | crate::tls::TlsError::CertificateTimeInvalid { .. }
            | crate::tls::TlsError::InvalidSignature { .. }
            | crate::tls::TlsError::UnknownCertificateAuthority { .. }
            | crate::tls::TlsError::InvalidCertificatePurpose { .. }
            | crate::tls::TlsError::CertificateChainInvalid { .. }
            | crate::tls::TlsError::CertificateRevoked { .. } => {
                if let Some(suggestion) = tls_error.suggest_cli_flag() {
                    tracing::error!(
                        "TLS certificate validation failed: {}. Suggestion: {}",
                        tls_error,
                        suggestion
                    );
                }
                "TLS certificate validation failed"
            }
            crate::tls::TlsError::HostnameVerificationFailed { .. } => {
                let suggestion = tls_error
                    .suggest_cli_flag()
                    .unwrap_or("--insecure-skip-hostname-verify");
                tracing::error!(
                    "TLS hostname verification failed: {}. Suggestion: {}",
                    tls_error,
                    suggestion
                );
                "TLS hostname verification failed"
            }
            crate::tls::TlsError::CaFileNotFound { .. }
            | crate::tls::TlsError::InvalidCaFormat { .. }
            | crate::tls::TlsError::MutuallyExclusiveFlags { .. } => {
                "TLS client configuration error"
            }
            _ => "Database connection failed",
        };
        anyhow::Error::from(tls_error).context(context_prefix)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tls::TlsConfig;

    #[test]
    fn test_create_database_connection_invalid_url() {
        // Test with invalid URL to ensure error handling works
        let tls = TlsConfig::Platform;
        let result = create_database_connection("invalid://url", &tls, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_database_connection() {
        // Test that the function exists and handles errors properly.
        // Use 127.0.0.1:1 (tcpmux) -- a guaranteed-unbound localhost port that
        // fails fast without live DNS resolution, keeping the test offline-friendly.
        let tls = TlsConfig::Platform;
        let result =
            create_database_connection("mysql://invalid:invalid@127.0.0.1:1/test", &tls, 0);
        // Should fail due to invalid connection details, but not panic
        assert!(result.is_err());
    }

    #[test]
    fn test_create_database_connection_with_tls_options() {
        // Test TLS configuration path.
        // Use 127.0.0.1:1 (tcpmux) -- a guaranteed-unbound localhost port that
        // fails fast without live DNS resolution, keeping the test offline-friendly.
        let tls = TlsConfig::SkipHostnameVerification;
        let result =
            create_database_connection("mysql://invalid:invalid@127.0.0.1:1/test", &tls, 0);
        // Should fail due to invalid connection details, but TLS config should be processed
        assert!(result.is_err());
    }
}
