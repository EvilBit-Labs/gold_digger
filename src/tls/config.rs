//! [`TlsValidationMode`] enum (the canonical TLS configuration type).
//!
//! `TlsValidationMode` is the only object `src/main.rs` and `src/cli.rs`
//! hand to [`super::pool::create_tls_connection`]. It captures the
//! user's chosen validation posture and converts itself to
//! [`mysql::SslOpts`] on demand.
//!
//! `TlsConfig` is now a backward-compatible type alias for
//! `TlsValidationMode` (see #3 type-design fix). Existing imports
//! continue to compile; the methods previously living on the old
//! `TlsConfig` wrapper are now inherent methods on the enum itself.
//!
//! The CLI adapter (`TlsOptions::to_tls_config`) lives in [`crate::cli`] to
//! keep this module dependent only on primitive types; see commit `ce7685a`
//! for the rationale (presentation → infrastructure coupling).

use super::ca::CaFile;
use super::error::TlsError;
use mysql::SslOpts;
use std::path::PathBuf;

/// TLS validation modes for different security requirements.
///
/// This is the canonical TLS configuration type. The historical
/// `TlsConfig` wrapper was a single-field newtype that added no real
/// abstraction — every method just delegated to the inner mode — so
/// it has been collapsed into this enum (#3 type-design fix).
/// `TlsConfig` remains as a type alias for backward compatibility with
/// existing imports.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TlsValidationMode {
    /// Use platform certificate store with full validation (default)
    #[default]
    Platform,
    /// Use a custom CA bundle that has been validated at construction
    /// time (canonicalised, exists, ≥ 1 valid PEM cert).
    CustomCa { ca_file: CaFile },
    /// Use platform store but skip hostname verification
    SkipHostnameVerification,
    /// Accept any certificate (no validation) - DANGEROUS
    AcceptInvalid,
}

/// Backward-compatibility alias for the historical `TlsConfig` wrapper.
///
/// `TlsConfig` was a single-field struct around [`TlsValidationMode`];
/// the wrapper added no abstraction so it has been collapsed into the
/// enum itself (#3 type-design fix). This alias keeps existing imports
/// (`use gold_digger::tls::TlsConfig`) compiling.
pub type TlsConfig = TlsValidationMode;

impl TlsValidationMode {
    /// Creates a new TLS configuration with platform validation.
    ///
    /// Equivalent to `TlsValidationMode::Platform`; retained for the
    /// fluent `TlsConfig::new()` style used by the existing call sites.
    pub fn new() -> Self {
        Self::Platform
    }

    /// Creates a TLS configuration from CLI arguments with validation.
    ///
    /// When `ca_file` is supplied, the path is loaded and validated
    /// eagerly via [`CaFile::load`] — bogus paths produce
    /// [`TlsError::CaFileNotFound`] / [`TlsError::InvalidCaFormat`] at
    /// construction time rather than later inside the mysql driver.
    pub fn from_cli_args(
        ca_file: Option<&PathBuf>,
        skip_hostname: bool,
        accept_invalid: bool,
    ) -> Result<Self, TlsError> {
        // Check for mutually exclusive flags
        let flag_count = [ca_file.is_some(), skip_hostname, accept_invalid]
            .iter()
            .filter(|&&x| x)
            .count();

        if flag_count > 1 {
            let mut flags = Vec::new();
            if ca_file.is_some() {
                flags.push("--tls-ca-file");
            }
            if skip_hostname {
                flags.push("--insecure-skip-hostname-verify");
            }
            if accept_invalid {
                flags.push("--allow-invalid-certificate");
            }
            return Err(TlsError::mutually_exclusive_flags(flags.join(", ")));
        }

        let mode = if let Some(ca_file_path) = ca_file {
            // Eager validation at construction time (#2 type-design fix):
            // CaFile::load canonicalises, checks existence, opens the
            // file, and parses PEM. Bogus paths cannot survive past
            // this point.
            let ca_file = CaFile::load(ca_file_path)?;
            Self::CustomCa { ca_file }
        } else if skip_hostname {
            Self::SkipHostnameVerification
        } else if accept_invalid {
            Self::AcceptInvalid
        } else {
            Self::Platform
        };

        Ok(mode)
    }

    /// Displays security warnings for insecure TLS modes.
    ///
    /// Routed through `tracing` so credential redaction / level filtering
    /// applies and `--quiet` is honoured. Colouring respects `NO_COLOR`
    /// and TTY detection via `logging::warn_banner` / `danger_banner`.
    pub fn display_security_warnings(&self) {
        use crate::logging::{danger_banner, warn_banner};

        match self {
            Self::SkipHostnameVerification => {
                tracing::warn!(
                    "{}",
                    warn_banner(
                        "[WARNING] Hostname verification disabled. Connection is vulnerable to man-in-the-middle attacks."
                    )
                );
                tracing::warn!(
                    "{}",
                    warn_banner(
                        "   Only use this option if you understand the security implications."
                    )
                );
            }
            Self::AcceptInvalid => {
                tracing::error!(
                    "{}",
                    danger_banner("[DANGER] Certificate validation completely disabled!")
                );
                tracing::error!(
                    "{}",
                    danger_banner(
                        "   This connection provides NO security against man-in-the-middle attacks."
                    )
                );
                tracing::error!(
                    "{}",
                    danger_banner(
                        "   Only use this for testing with self-signed certificates in secure environments."
                    )
                );
            }
            Self::Platform | Self::CustomCa { .. } => {
                // No warnings for secure modes
            }
        }
    }

    /// Creates a TLS configuration with custom CA file validation.
    ///
    /// The CA file is loaded and validated eagerly via [`CaFile::load`].
    /// Returns [`TlsError`] when the path is missing, unreadable, or
    /// not valid PEM (#2 type-design fix).
    pub fn with_custom_ca<P: AsRef<std::path::Path>>(ca_file_path: P) -> Result<Self, TlsError> {
        let ca_file = CaFile::load(ca_file_path)?;
        Ok(Self::CustomCa { ca_file })
    }

    /// Creates a TLS configuration that skips hostname verification.
    pub fn with_skip_hostname_verification() -> Self {
        Self::SkipHostnameVerification
    }

    /// Creates a TLS configuration that accepts invalid certificates.
    pub fn with_accept_invalid() -> Self {
        Self::AcceptInvalid
    }

    /// Returns the validation mode.
    ///
    /// Historically this was `TlsConfig::validation_mode(&self) ->
    /// &TlsValidationMode`. After collapsing the wrapper, the value
    /// IS the mode — so this just returns `self`. Retained as an
    /// inherent method so existing call sites
    /// (`config.validation_mode()`) keep compiling.
    pub fn validation_mode(&self) -> &Self {
        self
    }

    /// Converts the TLS configuration to mysql::SslOpts using rustls-only implementation.
    pub fn to_ssl_opts(&self) -> Result<Option<SslOpts>, TlsError> {
        // Create SslOpts based on validation mode using rustls-only implementation.
        // CustomCa skips re-validation: CaFile::load already canonicalised
        // the path and confirmed at least one valid PEM certificate at
        // construction time, so we can hand the path straight to mysql
        // without re-reading the file (#2 type-design fix).
        let ssl_opts = match self {
            Self::Platform => {
                // Use default SslOpts which will use rustls with platform certificates
                SslOpts::default()
            }
            Self::CustomCa { ca_file } => {
                // Set the CA file path for custom CA validation
                SslOpts::default().with_root_cert_path(Some(ca_file.path().to_path_buf()))
            }
            Self::SkipHostnameVerification => {
                // Use SslOpts that skips hostname verification
                SslOpts::default().with_danger_skip_domain_validation(true)
            }
            Self::AcceptInvalid => {
                // Use SslOpts that accepts invalid certificates
                SslOpts::default()
                    .with_danger_accept_invalid_certs(true)
                    .with_danger_skip_domain_validation(true)
            }
        };

        Ok(Some(ssl_opts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Writes a freshly-generated self-signed PEM certificate to a
    /// temp file and returns it. Used by every CustomCa test in this
    /// module so the eager-validation path in `CaFile::load` succeeds.
    fn temp_valid_ca_file() -> NamedTempFile {
        use rcgen::generate_simple_self_signed;
        let cert =
            generate_simple_self_signed(vec!["localhost".to_string()]).expect("generate cert");
        let pem = cert.cert.pem();
        let mut tf = NamedTempFile::new().unwrap();
        tf.write_all(pem.as_bytes()).unwrap();
        tf.flush().unwrap();
        tf
    }

    #[test]
    fn test_tls_config_default() {
        let config = TlsConfig::default();
        assert!(matches!(config, TlsValidationMode::Platform));
    }

    #[test]
    fn test_tls_config_new() {
        let config = TlsConfig::new();
        assert!(matches!(config, TlsValidationMode::Platform));
    }

    #[test]
    fn test_tls_config_builder_patterns() {
        let ca = temp_valid_ca_file();
        let config = TlsConfig::with_custom_ca(ca.path()).expect("valid ca");
        assert!(matches!(config, TlsValidationMode::CustomCa { .. }));

        let config = TlsConfig::with_skip_hostname_verification();
        assert!(matches!(
            config,
            TlsValidationMode::SkipHostnameVerification
        ));

        let config = TlsConfig::with_accept_invalid();
        assert!(matches!(config, TlsValidationMode::AcceptInvalid));
    }

    #[test]
    fn test_to_ssl_opts_default() {
        let config = TlsConfig::default();
        let ssl_opts = config.to_ssl_opts();
        assert!(ssl_opts.is_ok());
        let opt = ssl_opts.unwrap();
        assert!(opt.is_some());
    }

    #[test]
    fn test_to_ssl_opts_platform_mode() {
        let config = TlsConfig::new(); // platform validation by default
        let ssl_opts = config.to_ssl_opts();
        assert!(ssl_opts.is_ok());
        assert!(ssl_opts.unwrap().is_some());
    }

    #[test]
    fn test_with_custom_ca_nonexistent_returns_not_found() {
        // The eager-validation path now rejects bogus paths at
        // construction time (#2 fix), not at to_ssl_opts() time.
        let result = TlsConfig::with_custom_ca("/nonexistent/ca.pem");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TlsError::CaFileNotFound { .. }
        ));
    }

    #[test]
    fn test_to_ssl_opts_with_validation_modes() {
        // Test skip hostname verification
        let config = TlsConfig::with_skip_hostname_verification();
        let ssl_opts = config.to_ssl_opts();
        assert!(ssl_opts.is_ok());
        assert!(ssl_opts.unwrap().is_some());

        // Test accept invalid certificates
        let config = TlsConfig::with_accept_invalid();
        let ssl_opts = config.to_ssl_opts();
        assert!(ssl_opts.is_ok());
        assert!(ssl_opts.unwrap().is_some());
    }

    #[test]
    fn test_tls_config_clone() {
        let ca = temp_valid_ca_file();
        let config1 = TlsConfig::with_custom_ca(ca.path()).expect("valid ca");
        let config2 = config1.clone();

        assert_eq!(config1, config2);
    }

    #[test]
    fn test_from_cli_args_platform_default() {
        let config = TlsConfig::from_cli_args(None, false, false).unwrap();
        assert!(matches!(config, TlsValidationMode::Platform));
    }

    #[test]
    fn test_from_cli_args_custom_ca() {
        // Test with non-existent file should fail
        let ca_path = PathBuf::from("/path/to/ca.pem");
        let result = TlsConfig::from_cli_args(Some(&ca_path), false, false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("CA certificate file not found")
        );
    }

    #[test]
    fn test_from_cli_args_skip_hostname() {
        let config = TlsConfig::from_cli_args(None, true, false).unwrap();
        assert!(matches!(
            config,
            TlsValidationMode::SkipHostnameVerification
        ));
    }

    #[test]
    fn test_from_cli_args_accept_invalid() {
        let config = TlsConfig::from_cli_args(None, false, true).unwrap();
        assert!(matches!(config, TlsValidationMode::AcceptInvalid));
    }

    #[test]
    fn test_from_cli_args_mutually_exclusive() {
        // Test ca_file + skip_hostname
        let path = PathBuf::from("/path");
        let result = TlsConfig::from_cli_args(Some(&path), true, false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Mutually exclusive TLS flags")
        );

        // Test ca_file + accept_invalid
        let path = PathBuf::from("/path");
        let result = TlsConfig::from_cli_args(Some(&path), false, true);
        assert!(result.is_err());

        // Test skip_hostname + accept_invalid
        let result = TlsConfig::from_cli_args(None, true, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_security_warnings_display() {
        // Test that security warnings are properly formatted
        let config = TlsConfig::with_skip_hostname_verification();
        // This should not panic and should display warning to stderr
        config.display_security_warnings();

        let config = TlsConfig::with_accept_invalid();
        // This should not panic and should display danger warning to stderr
        config.display_security_warnings();

        let config = TlsConfig::new();
        // This should not display any warnings
        config.display_security_warnings();
    }

    #[test]
    fn test_mutually_exclusive_flags_comprehensive() {
        // Test skip_hostname + accept_invalid
        let result = TlsConfig::from_cli_args(None, true, true);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Mutually exclusive TLS flags")
        );

        // Test ca_file + skip_hostname + accept_invalid (all three)
        let path = PathBuf::from("/path");
        let result = TlsConfig::from_cli_args(Some(&path), true, true);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Mutually exclusive TLS flags")
        );

        // Test that error message contains all conflicting flags
        let path = PathBuf::from("/path");
        let result = TlsConfig::from_cli_args(Some(&path), true, false);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("--tls-ca-file"));
        assert!(error_msg.contains("--insecure-skip-hostname-verify"));
    }

    #[test]
    fn test_tls_config_to_ssl_opts_with_rustls() {
        // Test platform mode
        let config = TlsConfig::new();
        let ssl_opts = config.to_ssl_opts();
        assert!(ssl_opts.is_ok());
        assert!(ssl_opts.unwrap().is_some());

        // Test skip hostname verification mode
        let config = TlsConfig::with_skip_hostname_verification();
        let ssl_opts = config.to_ssl_opts();
        assert!(ssl_opts.is_ok());
        assert!(ssl_opts.unwrap().is_some());

        // Test accept invalid mode
        let config = TlsConfig::with_accept_invalid();
        let ssl_opts = config.to_ssl_opts();
        assert!(ssl_opts.is_ok());
        assert!(ssl_opts.unwrap().is_some());

        // Custom CA with bogus path now fails at construction (not
        // at to_ssl_opts) — see test_with_custom_ca_nonexistent_returns_not_found.
    }

    #[test]
    fn test_to_ssl_opts_validation_mode_configuration() {
        // Test that each validation mode produces the correct SslOpts configuration

        // Platform mode - should use default settings
        let config = TlsConfig::new();
        let ssl_opts = config.to_ssl_opts().unwrap().unwrap();
        assert!(!ssl_opts.skip_domain_validation());
        assert!(!ssl_opts.accept_invalid_certs());
        assert!(ssl_opts.root_cert_path().is_none());

        // Skip hostname verification mode
        let config = TlsConfig::with_skip_hostname_verification();
        let ssl_opts = config.to_ssl_opts().unwrap().unwrap();
        assert!(ssl_opts.skip_domain_validation());
        assert!(!ssl_opts.accept_invalid_certs());
        assert!(ssl_opts.root_cert_path().is_none());

        // Accept invalid certificates mode
        let config = TlsConfig::with_accept_invalid();
        let ssl_opts = config.to_ssl_opts().unwrap().unwrap();
        assert!(ssl_opts.skip_domain_validation());
        assert!(ssl_opts.accept_invalid_certs());
        assert!(ssl_opts.root_cert_path().is_none());
    }

    #[test]
    fn test_to_ssl_opts_custom_ca_with_temp_file() {
        // Build a freshly-generated valid PEM via rcgen so CaFile::load
        // accepts it (the historical hardcoded cert was malformed and
        // relied on the lazy-validation path failing at to_ssl_opts).
        let ca_file = temp_valid_ca_file();

        // Construct via the public builder (eager validation).
        let config = TlsConfig::with_custom_ca(ca_file.path()).expect("valid ca");

        // Verify the configuration carries a CaFile pointing at the
        // canonicalised temp path.
        let canonical = std::fs::canonicalize(ca_file.path()).unwrap();
        match &config {
            TlsValidationMode::CustomCa { ca_file: cf } => {
                assert_eq!(cf.path(), canonical);
                assert!(cf.cert_count() >= 1);
            }
            other => panic!("expected CustomCa, got {:?}", other),
        }

        // to_ssl_opts() should succeed without re-reading the file.
        let ssl_opts = config.to_ssl_opts().expect("to_ssl_opts succeeds").unwrap();
        assert!(!ssl_opts.skip_domain_validation());
        assert!(!ssl_opts.accept_invalid_certs());
        assert_eq!(ssl_opts.root_cert_path().unwrap(), canonical);
    }

    #[test]
    fn test_to_ssl_opts_integration() {
        // Test that to_ssl_opts() works correctly with from_cli_args()

        // Test platform mode
        let config = TlsConfig::from_cli_args(None, false, false).unwrap();
        let ssl_opts = config.to_ssl_opts().unwrap();
        assert!(ssl_opts.is_some());
        let ssl_opts = ssl_opts.unwrap();
        assert!(!ssl_opts.skip_domain_validation());
        assert!(!ssl_opts.accept_invalid_certs());

        // Test skip hostname mode
        let config = TlsConfig::from_cli_args(None, true, false).unwrap();
        let ssl_opts = config.to_ssl_opts().unwrap();
        assert!(ssl_opts.is_some());
        let ssl_opts = ssl_opts.unwrap();
        assert!(ssl_opts.skip_domain_validation());
        assert!(!ssl_opts.accept_invalid_certs());

        // Test accept invalid mode
        let config = TlsConfig::from_cli_args(None, false, true).unwrap();
        let ssl_opts = config.to_ssl_opts().unwrap();
        assert!(ssl_opts.is_some());
        let ssl_opts = ssl_opts.unwrap();
        assert!(ssl_opts.skip_domain_validation());
        assert!(ssl_opts.accept_invalid_certs());
    }

    #[test]
    fn test_tls_config_mutual_exclusion_validation() {
        // Create a fake path for testing (file doesn't need to exist for this test)
        let fake_cert_path = PathBuf::from("/fake/cert.pem");

        // Test ca_file + skip_hostname (should fail)
        let result = TlsConfig::from_cli_args(Some(&fake_cert_path), true, false);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TlsError::MutuallyExclusiveFlags { .. }
        ));

        // Test ca_file + accept_invalid (should fail)
        let result = TlsConfig::from_cli_args(Some(&fake_cert_path), false, true);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TlsError::MutuallyExclusiveFlags { .. }
        ));

        // Test skip_hostname + accept_invalid (should fail)
        let result = TlsConfig::from_cli_args(None, true, true);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TlsError::MutuallyExclusiveFlags { .. }
        ));

        // Test all three flags (should fail)
        let result = TlsConfig::from_cli_args(Some(&fake_cert_path), true, true);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TlsError::MutuallyExclusiveFlags { .. }
        ));
    }

    #[test]
    fn test_certificate_file_validation() {
        // Test nonexistent file
        let nonexistent_path = PathBuf::from("/nonexistent/path/to/cert.pem");
        let result = TlsConfig::from_cli_args(Some(&nonexistent_path), false, false);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TlsError::CaFileNotFound { .. }
        ));

        // Test empty path
        let empty_path = PathBuf::from("");
        let result = TlsConfig::from_cli_args(Some(&empty_path), false, false);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TlsError::CaFileNotFound { .. }
        ));
    }

    #[test]
    fn test_ssl_opts_generation_for_all_modes() {
        // Test platform mode
        let config = TlsConfig::new();
        let ssl_opts = config.to_ssl_opts().unwrap();
        assert!(ssl_opts.is_some());

        // Test skip hostname mode
        let config = TlsConfig::with_skip_hostname_verification();
        let ssl_opts = config.to_ssl_opts().unwrap();
        assert!(ssl_opts.is_some());

        // Test accept invalid mode
        let config = TlsConfig::with_accept_invalid();
        let ssl_opts = config.to_ssl_opts().unwrap();
        assert!(ssl_opts.is_some());

        // Test default TLS (platform mode)
        let config = TlsConfig::default();
        let ssl_opts = config.to_ssl_opts().unwrap();
        assert!(ssl_opts.is_some());
    }

    #[test]
    fn test_tls_config_equality_and_cloning() {
        let config1 = TlsConfig::new();
        let config2 = config1.clone();

        assert_eq!(config1, config2);
        assert_eq!(config1.validation_mode(), config2.validation_mode());

        // Test inequality
        let config3 = TlsConfig::with_accept_invalid();
        assert_ne!(config1, config3);
    }

    #[test]
    fn test_security_warnings_display_comprehensive() {
        // These tests verify that display_security_warnings() doesn't panic
        // The actual warning output is tested by not panicking

        // Platform mode - no warnings
        let config = TlsConfig::new();
        config.display_security_warnings();

        // Skip hostname mode - should display warning
        let config = TlsConfig::with_skip_hostname_verification();
        config.display_security_warnings();

        // Accept invalid mode - should display warning
        let config = TlsConfig::with_accept_invalid();
        config.display_security_warnings();
    }

    #[test]
    fn test_tls_validation_mode_default() {
        let mode = TlsValidationMode::default();
        assert!(matches!(mode, TlsValidationMode::Platform));
    }

    #[test]
    fn test_tls_config_accessors() {
        let config = TlsConfig::new();
        assert!(matches!(
            config.validation_mode(),
            TlsValidationMode::Platform
        ));

        let config = TlsConfig::default();
        assert!(matches!(
            config.validation_mode(),
            TlsValidationMode::Platform
        ));

        let config = TlsConfig::with_skip_hostname_verification();
        assert!(matches!(
            config.validation_mode(),
            TlsValidationMode::SkipHostnameVerification
        ));

        let config = TlsConfig::with_accept_invalid();
        assert!(matches!(
            config.validation_mode(),
            TlsValidationMode::AcceptInvalid
        ));
    }

    #[test]
    fn test_display_security_warnings() {
        // Test that security warnings don't panic (we can't easily test stderr output in unit tests)
        let config = TlsConfig::with_skip_hostname_verification();
        config.display_security_warnings(); // Should not panic

        let config = TlsConfig::with_accept_invalid();
        config.display_security_warnings(); // Should not panic

        let config = TlsConfig::new();
        config.display_security_warnings(); // Should not panic (no warnings for secure mode)
    }
}
