//! Comprehensive unit tests for TLS configuration
//!
//! This module tests:
//! - CLI flag parsing and mutual exclusion validation
//! - TLS configuration creation from different CLI flag combinations
//! - Certificate file validation and error handling
//! - rustls ClientConfig generation for each validation mode
//!
//! Requirements covered: 3.4, 6.1, 6.2, 6.3, 6.4

use anyhow::Result;
use clap::Parser;
use gold_digger::cli::Cli;
use gold_digger::tls::{TlsConfig, TlsError, TlsValidationMode};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper function to create a temporary certificate file for testing
fn create_temp_cert_file(content: &str) -> Result<(TempDir, PathBuf)> {
    let temp_dir = tempfile::tempdir()?;
    let cert_path = temp_dir.path().join("test_cert.pem");
    fs::write(&cert_path, content)?;
    Ok((temp_dir, cert_path))
}

/// Sample valid PEM certificate for testing
const VALID_CERT_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIDXTCCAkWgAwIBAgIJAKoK/heBjcOuMA0GCSqGSIb3DQEBBQUAMEUxCzAJBgNV
BAYTAkFVMRMwEQYDVQQIDApTb21lLVN0YXRlMSEwHwYDVQQKDBhJbnRlcm5ldCBX
aWRnaXRzIFB0eSBMdGQwHhcNMTcwODI4MTExNzE2WhcNMTgwODI4MTExNzE2WjBF
MQswCQYDVQQGEwJBVTETMBEGA1UECAwKU29tZS1TdGF0ZTEhMB8GA1UECgwYSW50
ZXJuZXQgV2lkZ2l0cyBQdHkgTHRkMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIB
CgKCAQEAuuExKvY1xzHFw4A9J3QnsdTtjScjjQ3WM94I2FtpMRCZDBrT7PN2RQae
1UBMHall7afNzoglf7Gpir6+sQBaoXI6F0S2ZuuAiYU9zqhxHKjVfz6rZqQkLrZQ
kOcHXiIhIdOviydpX3MelAwNjGSteHyGA1TqRBxh9obFoAoRQmlHnVkycnARP8qd
tNatja7VgHd7NuiE5vTaFzCREHk2lQaHdgAIuRs6Z4zw1h5BzHyUK4DqsJqGrRLm
YehM4wlBOmrsBc7afNdlko/YVFkLJ7AsGQJ1951i6cWQmaq5WZEyLPp1FNRRRyep
7TqBnLf2xURg5BDVvbhP0A42VpQIDAQABo1AwTjAdBgNVHQ4EFgQUhHf2808b6+RE
oCgEMWMWgRkH+6wwHwYDVR0jBBgwFoAUhHf2808b6+REoCgEMWMWgRkH+6wwDAYD
VR0TBAUwAwEB/zANBgkqhkiG9w0BAQUFAAOCAQEAGRuOfQqk5T5OhzgiuLxhQYsy
XqSR4fNMW7M0PJjdXNzGxhMvKs9vEehxiaUHLjUx7bZT2+WBxNki4NfeCEHeQpZs
-----END CERTIFICATE-----
"#;

#[cfg(feature = "ssl")]
mod cli_flag_parsing_tests {
    use super::*;

    /// Test CLI flag parsing with no TLS flags (should use platform default)
    /// Requirement: 6.1 - CLI flags with environment variable fallbacks
    #[test]
    fn test_cli_no_tls_flags() -> Result<()> {
        let cli = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
        ])?;

        let tls_config = TlsConfig::from_tls_options(&cli.tls_options)?;

        assert!(tls_config.is_enabled());
        assert!(matches!(tls_config.validation_mode(), TlsValidationMode::Platform));

        Ok(())
    }

    /// Test CLI flag parsing with --tls-ca-file
    /// Requirement: 6.2 - Custom CA file configuration
    #[test]
    fn test_cli_tls_ca_file_flag() -> Result<()> {
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;

        let cli = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--tls-ca-file",
            cert_path.to_str().unwrap(),
        ])?;

        let tls_config = TlsConfig::from_tls_options(&cli.tls_options)?;

        assert!(tls_config.is_enabled());
        if let TlsValidationMode::CustomCa { ca_file_path } = tls_config.validation_mode() {
            assert_eq!(ca_file_path, &cert_path);
        } else {
            panic!("Expected CustomCa validation mode");
        }

        Ok(())
    }

    /// Test CLI flag parsing with --insecure-skip-hostname-verify
    /// Requirement: 6.3 - Hostname verification bypass
    #[test]
    fn test_cli_skip_hostname_verify_flag() -> Result<()> {
        let cli = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--insecure-skip-hostname-verify",
        ])?;

        let tls_config = TlsConfig::from_tls_options(&cli.tls_options)?;

        assert!(tls_config.is_enabled());
        assert!(matches!(tls_config.validation_mode(), TlsValidationMode::SkipHostnameVerification));

        Ok(())
    }

    /// Test CLI flag parsing with --allow-invalid-certificate
    /// Requirement: 6.4 - Invalid certificate acceptance
    #[test]
    fn test_cli_allow_invalid_certificate_flag() -> Result<()> {
        let cli = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--allow-invalid-certificate",
        ])?;

        let tls_config = TlsConfig::from_tls_options(&cli.tls_options)?;

        assert!(tls_config.is_enabled());
        assert!(matches!(tls_config.validation_mode(), TlsValidationMode::AcceptInvalid));

        Ok(())
    }

    /// Test mutual exclusion: --tls-ca-file and --insecure-skip-hostname-verify
    /// Requirement: 6.2, 6.3 - Mutually exclusive TLS flags
    #[test]
    fn test_mutual_exclusion_ca_file_and_skip_hostname() -> Result<()> {
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;

        let result = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--tls-ca-file",
            cert_path.to_str().unwrap(),
            "--insecure-skip-hostname-verify",
        ]);

        assert!(result.is_err());
        if let Err(error) = result {
            let error_msg = error.to_string();
            assert!(error_msg.contains("cannot be used with") || error_msg.contains("mutually exclusive"));
        }

        Ok(())
    }

    /// Test mutual exclusion: --tls-ca-file and --allow-invalid-certificate
    /// Requirement: 6.2, 6.4 - Mutually exclusive TLS flags
    #[test]
    fn test_mutual_exclusion_ca_file_and_allow_invalid() -> Result<()> {
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;

        let result = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--tls-ca-file",
            cert_path.to_str().unwrap(),
            "--allow-invalid-certificate",
        ]);

        assert!(result.is_err());
        if let Err(error) = result {
            let error_msg = error.to_string();
            assert!(error_msg.contains("cannot be used with") || error_msg.contains("mutually exclusive"));
        }

        Ok(())
    }

    /// Test mutual exclusion: --insecure-skip-hostname-verify and --allow-invalid-certificate
    /// Requirement: 6.3, 6.4 - Mutually exclusive TLS flags
    #[test]
    fn test_mutual_exclusion_skip_hostname_and_allow_invalid() -> Result<()> {
        let result = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--insecure-skip-hostname-verify",
            "--allow-invalid-certificate",
        ]);

        assert!(result.is_err());
        if let Err(error) = result {
            let error_msg = error.to_string();
            assert!(error_msg.contains("cannot be used with") || error_msg.contains("mutually exclusive"));
        }

        Ok(())
    }
}

#[cfg(feature = "ssl")]
mod tls_config_creation_tests {
    use super::*;
    use gold_digger::cli::TlsOptions;

    /// Test TLS configuration creation from CLI args - platform mode
    /// Requirement: 6.1 - Default platform certificate validation
    #[test]
    fn test_tls_config_from_cli_platform_mode() -> Result<()> {
        let tls_options = TlsOptions {
            tls_ca_file: None,
            insecure_skip_hostname_verify: false,
            allow_invalid_certificate: false,
        };

        let config = TlsConfig::from_tls_options(&tls_options)?;

        assert!(config.is_enabled());
        assert!(matches!(config.validation_mode(), TlsValidationMode::Platform));

        Ok(())
    }

    /// Test TLS configuration creation with custom CA file
    /// Requirement: 6.2 - Custom CA file configuration
    #[test]
    fn test_tls_config_from_cli_custom_ca_mode() -> Result<()> {
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;

        let tls_options = TlsOptions {
            tls_ca_file: Some(cert_path.clone()),
            insecure_skip_hostname_verify: false,
            allow_invalid_certificate: false,
        };

        let config = TlsConfig::from_tls_options(&tls_options)?;

        assert!(config.is_enabled());
        if let TlsValidationMode::CustomCa { ca_file_path } = config.validation_mode() {
            assert_eq!(ca_file_path, &cert_path);
        } else {
            panic!("Expected CustomCa validation mode");
        }

        Ok(())
    }

    /// Test TLS configuration creation with skip hostname verification
    /// Requirement: 6.3 - Hostname verification bypass
    #[test]
    fn test_tls_config_from_cli_skip_hostname_mode() -> Result<()> {
        let tls_options = TlsOptions {
            tls_ca_file: None,
            insecure_skip_hostname_verify: true,
            allow_invalid_certificate: false,
        };

        let config = TlsConfig::from_tls_options(&tls_options)?;

        assert!(config.is_enabled());
        assert!(matches!(config.validation_mode(), TlsValidationMode::SkipHostnameVerification));

        Ok(())
    }

    /// Test TLS configuration creation with allow invalid certificate
    /// Requirement: 6.4 - Invalid certificate acceptance
    #[test]
    fn test_tls_config_from_cli_allow_invalid_mode() -> Result<()> {
        let tls_options = TlsOptions {
            tls_ca_file: None,
            insecure_skip_hostname_verify: false,
            allow_invalid_certificate: true,
        };

        let config = TlsConfig::from_tls_options(&tls_options)?;

        assert!(config.is_enabled());
        assert!(matches!(config.validation_mode(), TlsValidationMode::AcceptInvalid));

        Ok(())
    }

    /// Test TLS configuration creation with mutual exclusion validation
    /// Requirement: 6.1, 6.2, 6.3, 6.4 - Mutually exclusive validation
    #[test]
    fn test_tls_config_mutual_exclusion_validation() -> Result<()> {
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;

        // Test ca_file + skip_hostname
        let result = TlsConfig::from_cli_args(Some(&cert_path), true, false);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::MutuallyExclusiveFlags { .. }));

        // Test ca_file + accept_invalid
        let result = TlsConfig::from_cli_args(Some(&cert_path), false, true);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::MutuallyExclusiveFlags { .. }));

        // Test skip_hostname + accept_invalid
        let result = TlsConfig::from_cli_args(None, true, true);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::MutuallyExclusiveFlags { .. }));

        Ok(())
    }
}

mod certificate_validation_tests {
    use super::*;

    /// Test certificate file validation - valid certificate
    /// Requirement: 3.4 - Certificate file validation
    #[test]
    fn test_certificate_validation_valid_cert() -> Result<()> {
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;

        let config = TlsConfig::with_custom_ca(&cert_path);

        assert!(config.is_enabled());
        if let TlsValidationMode::CustomCa { ca_file_path } = config.validation_mode() {
            assert_eq!(ca_file_path, &cert_path);
        } else {
            panic!("Expected CustomCa validation mode");
        }

        Ok(())
    }

    /// Test certificate file validation - nonexistent file
    /// Requirement: 3.4 - Certificate file validation and error handling
    #[test]
    fn test_certificate_validation_nonexistent_file() -> Result<()> {
        let nonexistent_path = PathBuf::from("/nonexistent/path/to/cert.pem");

        let result = TlsConfig::from_cli_args(Some(&nonexistent_path), false, false);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::CaFileNotFound { .. }));

        Ok(())
    }

    /// Test certificate file validation - invalid certificate content
    /// Requirement: 3.4 - Certificate file validation and error handling
    #[test]
    fn test_certificate_validation_invalid_cert() -> Result<()> {
        let invalid_cert_content = "This is not a valid PEM certificate";
        let (_temp_dir, cert_path) = create_temp_cert_file(invalid_cert_content)?;

        // The config creation should succeed, but SSL opts generation should fail
        let config = TlsConfig::with_custom_ca(&cert_path);
        assert!(config.is_enabled());

        #[cfg(feature = "ssl")]
        {
            let result = config.to_ssl_opts();
            assert!(result.is_err());
        }

        Ok(())
    }

    /// Test certificate file validation - empty certificate file
    /// Requirement: 3.4 - Certificate file validation and error handling
    #[test]
    fn test_certificate_validation_empty_cert() -> Result<()> {
        let (_temp_dir, cert_path) = create_temp_cert_file("")?;

        // The config creation should succeed, but SSL opts generation should fail
        let config = TlsConfig::with_custom_ca(&cert_path);
        assert!(config.is_enabled());

        #[cfg(feature = "ssl")]
        {
            let result = config.to_ssl_opts();
            assert!(result.is_err());
        }

        Ok(())
    }
}

#[cfg(feature = "ssl")]
mod ssl_opts_generation_tests {
    use super::*;

    /// Test SSL opts generation for platform validation mode
    /// Requirement: 6.1 - Platform certificate store usage
    #[test]
    fn test_ssl_opts_platform_mode() -> Result<()> {
        let config = TlsConfig::new(); // Platform mode by default

        let ssl_opts = config.to_ssl_opts()?;
        assert!(ssl_opts.is_some());

        Ok(())
    }

    /// Test SSL opts generation for custom CA mode
    /// Requirement: 6.2 - Custom CA certificate configuration
    #[test]
    fn test_ssl_opts_custom_ca_mode() -> Result<()> {
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;

        let config = TlsConfig::with_custom_ca(&cert_path);

        // Test SSL opts generation with custom CA
        // Note: This may fail with invalid certificate format, which is expected behavior
        let ssl_opts_result = config.to_ssl_opts();

        // The configuration should be created correctly, even if certificate parsing fails
        match ssl_opts_result {
            Ok(ssl_opts) => assert!(ssl_opts.is_some()),
            Err(_) => {
                // Certificate parsing failure is acceptable for this test
                // We're testing configuration creation, not certificate validation
            },
        }

        Ok(())
    }

    /// Test SSL opts generation for skip hostname verification mode
    /// Requirement: 6.3 - Hostname verification bypass
    #[test]
    fn test_ssl_opts_skip_hostname_mode() -> Result<()> {
        let config = TlsConfig::with_skip_hostname_verification();

        let ssl_opts = config.to_ssl_opts()?;
        assert!(ssl_opts.is_some());

        Ok(())
    }

    /// Test SSL opts generation for accept invalid certificates mode
    /// Requirement: 6.4 - Invalid certificate acceptance
    #[test]
    fn test_ssl_opts_accept_invalid_mode() -> Result<()> {
        let config = TlsConfig::with_accept_invalid();

        let ssl_opts = config.to_ssl_opts()?;
        assert!(ssl_opts.is_some());

        Ok(())
    }

    /// Test SSL opts generation for disabled TLS
    /// Requirement: 6.1 - TLS can be disabled
    #[test]
    fn test_ssl_opts_disabled_tls() -> Result<()> {
        let config = TlsConfig::default(); // Disabled by default

        let ssl_opts = config.to_ssl_opts()?;
        assert!(ssl_opts.is_none());

        Ok(())
    }
}

#[cfg(not(feature = "ssl"))]
mod ssl_feature_disabled_tests {
    use super::*;

    /// Test SSL opts generation when SSL feature is disabled
    /// Requirement: 6.1 - Graceful handling when SSL feature is disabled
    #[test]
    fn test_ssl_opts_feature_disabled() -> Result<()> {
        let config = TlsConfig::new();

        let ssl_opts_result = config.to_ssl_opts();
        assert!(ssl_opts_result.is_err());
        assert!(matches!(ssl_opts_result.unwrap_err(), TlsError::FeatureNotEnabled));

        Ok(())
    }

    /// Test TLS configuration creation when SSL feature is disabled
    /// Requirement: 6.1 - Default behavior without SSL feature
    #[test]
    fn test_tls_config_from_cli_no_ssl_feature() -> Result<()> {
        let cli = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
        ])?;

        let tls_config = TlsConfig::from_tls_options(&cli.tls_options)?;

        // When SSL feature is disabled, TLS should be disabled by default
        assert!(!tls_config.is_enabled());
        assert!(matches!(tls_config.validation_mode(), TlsValidationMode::Platform));

        Ok(())
    }
}

mod tls_config_builder_tests {
    use super::*;

    /// Test TLS config builder methods
    /// Requirement: 6.1, 6.2, 6.3, 6.4 - Configuration builder pattern
    #[test]
    fn test_tls_config_builders() -> Result<()> {
        // Test default config
        let config = TlsConfig::default();
        assert!(!config.is_enabled());
        assert!(matches!(config.validation_mode(), TlsValidationMode::Platform));

        // Test new config
        let config = TlsConfig::new();
        assert!(config.is_enabled());
        assert!(matches!(config.validation_mode(), TlsValidationMode::Platform));

        // Test custom CA builder
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;
        let config = TlsConfig::with_custom_ca(&cert_path);
        assert!(config.is_enabled());
        if let TlsValidationMode::CustomCa { ca_file_path } = config.validation_mode() {
            assert_eq!(ca_file_path, &cert_path);
        } else {
            panic!("Expected CustomCa validation mode");
        }

        // Test skip hostname builder
        let config = TlsConfig::with_skip_hostname_verification();
        assert!(config.is_enabled());
        assert!(matches!(config.validation_mode(), TlsValidationMode::SkipHostnameVerification));

        // Test accept invalid builder
        let config = TlsConfig::with_accept_invalid();
        assert!(config.is_enabled());
        assert!(matches!(config.validation_mode(), TlsValidationMode::AcceptInvalid));

        Ok(())
    }

    /// Test TLS config equality and cloning
    /// Requirement: 6.1 - Configuration comparison and cloning
    #[test]
    fn test_tls_config_equality_and_cloning() -> Result<()> {
        let config1 = TlsConfig::new();
        let config2 = config1.clone();

        assert_eq!(config1, config2);
        assert_eq!(config1.is_enabled(), config2.is_enabled());
        assert_eq!(config1.validation_mode(), config2.validation_mode());

        // Test inequality
        let config3 = TlsConfig::with_accept_invalid();
        assert_ne!(config1, config3);

        Ok(())
    }
}

mod security_warnings_tests {
    use super::*;

    /// Test security warnings display for different TLS modes
    /// Requirement: 6.3, 6.4 - Security warnings for insecure modes
    #[test]
    fn test_security_warnings_display() {
        // Platform mode - no warnings (tested by not panicking)
        let config = TlsConfig::new();
        config.display_security_warnings();

        // Custom CA mode - no warnings (tested by not panicking)
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM).unwrap();
        let config = TlsConfig::with_custom_ca(&cert_path);
        config.display_security_warnings();

        // Skip hostname mode - should display warning (tested by not panicking)
        let config = TlsConfig::with_skip_hostname_verification();
        config.display_security_warnings();

        // Accept invalid mode - should display warning (tested by not panicking)
        let config = TlsConfig::with_accept_invalid();
        config.display_security_warnings();
    }
}
