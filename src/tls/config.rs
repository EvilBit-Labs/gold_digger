//! [`TlsConfig`] DTO + [`TlsValidationMode`] enum.
//!
//! `TlsConfig` is the only object `src/main.rs` and `src/cli.rs` hand to
//! [`super::pool::create_tls_connection`]. It captures the user's chosen
//! validation posture and converts itself to [`mysql::SslOpts`] on demand.
//!
//! The CLI adapter (`TlsOptions::to_tls_config`) lives in [`crate::cli`] to
//! keep this module dependent only on primitive types; see commit `ce7685a`
//! for the rationale (presentation → infrastructure coupling).

use super::ca;
use super::error::TlsError;
use anyhow::Result;
use mysql::SslOpts;
use std::path::PathBuf;

/// TLS validation modes for different security requirements
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TlsValidationMode {
    /// Use platform certificate store with full validation (default)
    #[default]
    Platform,
    /// Use custom CA file with full validation
    CustomCa { ca_file_path: PathBuf },
    /// Use platform store but skip hostname verification
    SkipHostnameVerification,
    /// Accept any certificate (no validation) - DANGEROUS
    AcceptInvalid,
}

/// TLS configuration for MySQL connections
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TlsConfig {
    /// TLS validation mode
    pub validation_mode: TlsValidationMode,
}

impl TlsConfig {
    /// Creates a new TLS configuration with platform validation
    pub fn new() -> Self {
        Self {
            validation_mode: TlsValidationMode::Platform,
        }
    }

    /// Creates a TLS configuration from CLI arguments with validation
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

        let validation_mode = if let Some(ca_file_path) = ca_file {
            // Validate CA file exists and is readable
            if !ca_file_path.exists() {
                return Err(TlsError::ca_file_not_found(
                    ca_file_path.display().to_string(),
                ));
            }
            TlsValidationMode::CustomCa {
                ca_file_path: ca_file_path.clone(),
            }
        } else if skip_hostname {
            TlsValidationMode::SkipHostnameVerification
        } else if accept_invalid {
            TlsValidationMode::AcceptInvalid
        } else {
            TlsValidationMode::Platform
        };

        Ok(Self { validation_mode })
    }

    /// Displays security warnings for insecure TLS modes.
    ///
    /// For [`TlsValidationMode::AcceptInvalid`] the warning is followed by
    /// a 3-second deliberate delay (todo #022). The companion flag gate
    /// (`--i-understand-this-is-insecure`) already blocks accidental
    /// activation, but the delay gives a chance to `Ctrl+C` and makes the
    /// `[DANGER]` line unmissable in interactive terminals. The delay is
    /// skipped in test builds to keep unit tests fast.
    pub fn display_security_warnings(&self) {
        match &self.validation_mode {
            TlsValidationMode::SkipHostnameVerification => {
                eprintln!(
                    "[WARNING] Hostname verification disabled. Connection is vulnerable to man-in-the-middle attacks."
                );
                eprintln!("   Only use this option if you understand the security implications.");
            }
            TlsValidationMode::AcceptInvalid => {
                eprintln!("[DANGER] Certificate validation completely disabled!");
                eprintln!(
                    "   This connection provides NO security against man-in-the-middle attacks."
                );
                eprintln!(
                    "   Only use this for testing with self-signed certificates in secure environments."
                );
                // Deliberate pause so the banner cannot be missed and the
                // user has a chance to Ctrl+C (todo #022). Skipped under
                // `cfg(test)` so unit tests don't sleep.
                #[cfg(not(test))]
                {
                    eprintln!("   Proceeding in 3 seconds (press Ctrl+C to abort)...");
                    std::thread::sleep(std::time::Duration::from_secs(3));
                }
            }
            TlsValidationMode::Platform | TlsValidationMode::CustomCa { .. } => {
                // No warnings for secure modes
            }
        }
    }

    /// Creates a TLS configuration with custom CA file validation
    pub fn with_custom_ca<P: Into<PathBuf>>(ca_file_path: P) -> Self {
        Self {
            validation_mode: TlsValidationMode::CustomCa {
                ca_file_path: ca_file_path.into(),
            },
        }
    }

    /// Creates a TLS configuration that skips hostname verification
    pub fn with_skip_hostname_verification() -> Self {
        Self {
            validation_mode: TlsValidationMode::SkipHostnameVerification,
        }
    }

    /// Creates a TLS configuration that accepts invalid certificates
    pub fn with_accept_invalid() -> Self {
        Self {
            validation_mode: TlsValidationMode::AcceptInvalid,
        }
    }

    /// Returns the validation mode
    pub fn validation_mode(&self) -> &TlsValidationMode {
        &self.validation_mode
    }

    /// Converts the TLS configuration to mysql::SslOpts using rustls-only implementation
    pub fn to_ssl_opts(&self) -> Result<Option<SslOpts>, TlsError> {
        // For custom CA validation, validate the CA file exists and is readable
        if let TlsValidationMode::CustomCa { ca_file_path } = &self.validation_mode {
            ca::validate_ca_file(ca_file_path)?;
        }

        // Create SslOpts based on validation mode using rustls-only implementation
        let ssl_opts = match &self.validation_mode {
            TlsValidationMode::Platform => {
                // Use default SslOpts which will use rustls with platform certificates
                SslOpts::default()
            }
            TlsValidationMode::CustomCa { ca_file_path } => {
                // Set the CA file path for custom CA validation
                SslOpts::default().with_root_cert_path(Some(ca_file_path.clone()))
            }
            TlsValidationMode::SkipHostnameVerification => {
                // Use SslOpts that skips hostname verification
                SslOpts::default().with_danger_skip_domain_validation(true)
            }
            TlsValidationMode::AcceptInvalid => {
                // Use SslOpts that accepts invalid certificates
                SslOpts::default()
                    .with_danger_accept_invalid_certs(true)
                    .with_danger_skip_domain_validation(true)
            }
        };

        Ok(Some(ssl_opts))
    }
}

/// Helper function to create a TLS configuration from URL parameters
/// Note: This is a placeholder for future URL-based TLS configuration
pub fn tls_config_from_url(_url: &str) -> Result<Option<TlsConfig>> {
    // The mysql crate doesn't support URL-based SSL configuration
    // This function is provided for future extensibility
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_default() {
        let config = TlsConfig::default();
        assert!(matches!(
            config.validation_mode,
            TlsValidationMode::Platform
        ));
    }

    #[test]
    fn test_tls_config_new() {
        let config = TlsConfig::new();
        assert!(matches!(
            config.validation_mode,
            TlsValidationMode::Platform
        ));
    }

    #[test]
    fn test_tls_config_builder_patterns() {
        let config = TlsConfig::with_custom_ca("/path/to/ca.pem");
        assert!(matches!(
            config.validation_mode,
            TlsValidationMode::CustomCa { .. }
        ));

        let config = TlsConfig::with_skip_hostname_verification();
        assert!(matches!(
            config.validation_mode,
            TlsValidationMode::SkipHostnameVerification
        ));

        let config = TlsConfig::with_accept_invalid();
        assert!(matches!(
            config.validation_mode,
            TlsValidationMode::AcceptInvalid
        ));
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
    fn test_to_ssl_opts_with_nonexistent_ca_certificate() {
        let config = TlsConfig::with_custom_ca("/nonexistent/ca.pem");

        let ssl_opts = config.to_ssl_opts();
        assert!(ssl_opts.is_err());

        let error = ssl_opts.unwrap_err();
        assert!(error.to_string().contains("CA certificate file not found"));
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
        let config1 = TlsConfig::with_custom_ca("/path/to/ca.pem");
        let config2 = config1.clone();

        assert_eq!(config1, config2);
    }

    #[test]
    fn test_from_cli_args_platform_default() {
        let config = TlsConfig::from_cli_args(None, false, false).unwrap();
        assert!(matches!(
            config.validation_mode,
            TlsValidationMode::Platform
        ));
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

        // Test with valid file would require creating a temporary file
        // For now, we test the error case which is the expected behavior
    }

    #[test]
    fn test_from_cli_args_skip_hostname() {
        let config = TlsConfig::from_cli_args(None, true, false).unwrap();
        assert!(matches!(
            config.validation_mode,
            TlsValidationMode::SkipHostnameVerification
        ));
    }

    #[test]
    fn test_from_cli_args_accept_invalid() {
        let config = TlsConfig::from_cli_args(None, false, true).unwrap();
        assert!(matches!(
            config.validation_mode,
            TlsValidationMode::AcceptInvalid
        ));
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
    fn test_tls_config_from_url_placeholder() {
        // This tests the placeholder function
        let result = tls_config_from_url("mysql://user:pass@localhost:3306/db?ssl-mode=required");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
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

        // Test custom CA with nonexistent file
        let config = TlsConfig::with_custom_ca("/nonexistent/ca.pem");
        let ssl_opts = config.to_ssl_opts();
        assert!(ssl_opts.is_err());
        assert!(
            ssl_opts
                .unwrap_err()
                .to_string()
                .contains("CA certificate file not found")
        );
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
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temporary file with a valid self-signed certificate (for testing purposes)
        let mut temp_file = NamedTempFile::new().unwrap();
        // This is a valid self-signed certificate for testing
        writeln!(temp_file, "-----BEGIN CERTIFICATE-----").unwrap();
        writeln!(
            temp_file,
            "MIIDXTCCAkWgAwIBAgIJAKoK/heBjcOuMA0GCSqGSIb3DQEBBQUAMEUxCzAJBgNV"
        )
        .unwrap();
        writeln!(
            temp_file,
            "BAYTAkFVMRMwEQYDVQQIDApTb21lLVN0YXRlMSEwHwYDVQQKDBhJbnRlcm5ldCBX"
        )
        .unwrap();
        writeln!(
            temp_file,
            "aWRnaXRzIFB0eSBMdGQwHhcNMTcwODI4MTkzNDA5WhcNMTgwODI4MTkzNDA5WjBF"
        )
        .unwrap();
        writeln!(
            temp_file,
            "MQswCQYDVQQGEwJBVTETMBEGA1UECAwKU29tZS1TdGF0ZTEhMB8GA1UECgwYSW50"
        )
        .unwrap();
        writeln!(
            temp_file,
            "ZXJuZXQgV2lkZ2l0cyBQdHkgTHRkMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIB"
        )
        .unwrap();
        writeln!(
            temp_file,
            "CgKCAQEAuuExKvY1nOmAHO13nPiOxvTnoFrL23apFR9W+VdtPGrb+sQXebHjZ/UU"
        )
        .unwrap();
        writeln!(
            temp_file,
            "kKtjWQqLQlHgHOgFbt7jr8I2J2jFiaNBBBYuHBw6NMVBnhkdXRJDn9LxMa02cx1q"
        )
        .unwrap();
        writeln!(
            temp_file,
            "BxuFqV7zUg4EQVXveZd0HFDZrpVeUiA21IlQpFYxyFveOiGspMdYjI5u3Ngkqbz6"
        )
        .unwrap();
        writeln!(
            temp_file,
            "pXrbRqZzjXaFUcuJpPMFRNKGWv5wyAcb5B2fHX1sGtSaYvNilgxnE8+ykQs6rp+j"
        )
        .unwrap();
        writeln!(
            temp_file,
            "kVf3lbVvB4zUHg9S5RoQBQ1CuHnRkl9wjw03EBEQ4h2z4k5cyR2DpmdJ0b+2cxJl"
        )
        .unwrap();
        writeln!(
            temp_file,
            "Ww9cDcTgWwIDAQABo1AwTjAdBgNVHQ4EFgQUhG9lFWZWnPfLwB9gQQd8it/u+MQw"
        )
        .unwrap();
        writeln!(
            temp_file,
            "HwYDVR0jBBgwFoAUhG9lFWZWnPfLwB9gQQd8it/u+MQwDAYDVR0TBAUwAwEB/zAN"
        )
        .unwrap();
        writeln!(
            temp_file,
            "BgkqhkiG9w0BAQUFAAOCAQEAeM9ahJ6iAJfyFq4wzSmpOddgfGqJWjXiH+OqZlHO"
        )
        .unwrap();
        writeln!(
            temp_file,
            "2k8sVjCjmHylI+XleLu2dDxwjNuBllhid/Qs6TRcZxEqn+cAskHReXlZjQoHuSHx"
        )
        .unwrap();
        writeln!(
            temp_file,
            "VxHp2+PpVUFnuU19LFbmqZ3+/dvTVc0V0QNFS4HgBXkKwA9fPQ+k/roUe0is7d+8"
        )
        .unwrap();
        writeln!(
            temp_file,
            "O4ArHZka85ZMd1qY4z0xvFvbMmJuC0KJvEieakGFkCEc7trGwfIuXgFMLJLBB5uZ"
        )
        .unwrap();
        writeln!(
            temp_file,
            "F74imqDbImh5tbwQcQYBYVHhkCjDOw+XdXUSPiOBueno0soKjOxjVmooPdxyaAuW"
        )
        .unwrap();
        writeln!(
            temp_file,
            "fuFhiGI+bI90H4+17ceuJAOzOFvhPH1RTwf5k+7+BzXrqbHlt+2RfEECAwEAAQ=="
        )
        .unwrap();
        writeln!(temp_file, "-----END CERTIFICATE-----").unwrap();
        temp_file.flush().unwrap();

        // Test custom CA mode with the temporary file
        let config = TlsConfig::with_custom_ca(temp_file.path());

        // Verify the configuration is set up correctly
        assert_eq!(
            config.validation_mode(),
            &TlsValidationMode::CustomCa {
                ca_file_path: temp_file.path().to_path_buf()
            }
        );

        // The to_ssl_opts() call may fail due to invalid certificate, which is expected
        // We're testing that the error handling works correctly
        match config.to_ssl_opts() {
            Ok(Some(ssl_opts)) => {
                // If it succeeds, verify the configuration
                assert!(!ssl_opts.skip_domain_validation());
                assert!(!ssl_opts.accept_invalid_certs());
                assert!(ssl_opts.root_cert_path().is_some());
                assert_eq!(ssl_opts.root_cert_path().unwrap(), temp_file.path());
            }
            Err(TlsError::CertificateValidationFailed { .. })
            | Err(TlsError::InvalidCaFormat { .. }) => {
                // This is expected with an invalid test certificate
                // The important thing is that the error is properly classified
            }
            other => panic!("Unexpected result: {:?}", other),
        }
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
    fn test_ssl_opts_nonexistent_ca_file() {
        let nonexistent_path = PathBuf::from("/nonexistent/cert.pem");
        let config = TlsConfig {
            validation_mode: TlsValidationMode::CustomCa {
                ca_file_path: nonexistent_path,
            },
        };

        let ssl_opts_result = config.to_ssl_opts();
        assert!(ssl_opts_result.is_err());
        assert!(matches!(
            ssl_opts_result.unwrap_err(),
            TlsError::CaFileNotFound { .. }
        ));
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
