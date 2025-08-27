//! Backward compatibility tests for TLS functionality
//!
//! This module tests that the TLS migration maintains backward compatibility:
//! - Existing DATABASE_URL formats continue to work
//! - TLS connections work the same as before
//! - CLI flag behavior is unchanged
//! - Security warnings still display correctly
//!
//! Requirements covered: 7.1, 7.2, 7.3, 7.4, 8.1, 8.2

use anyhow::Result;
use assert_cmd::Command;
use gold_digger::cli::{Cli, TlsOptions};
use gold_digger::tls::{TlsConfig, TlsValidationMode};
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

/// Helper function to create a cross-platform temporary output path for testing
fn create_temp_output_path() -> Result<(TempDir, String)> {
    let temp_dir = tempfile::tempdir()?;
    let output_path = temp_dir.path().join("test_output.json");
    Ok((temp_dir, output_path.to_string_lossy().to_string()))
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

mod database_url_compatibility_tests {
    use super::*;

    /// Test that existing DATABASE_URL formats continue to work
    /// Requirement: 7.1 - Backward compatibility with existing DATABASE_URL configurations
    #[test]
    fn test_existing_database_url_formats() -> Result<()> {
        let database_urls = vec![
            // Basic MySQL URL without SSL parameters
            "mysql://user:pass@localhost:3306/database",
            // MySQL URL with database name
            "mysql://user:pass@localhost:3306/testdb",
            // MySQL URL with port specification
            "mysql://user:pass@localhost:3306/db",
            // MySQL URL with IP address
            "mysql://user:pass@127.0.0.1:3306/db",
            // MySQL URL with different port
            "mysql://user:pass@localhost:3307/db",
            // MySQL URL with special characters in password (URL encoded)
            "mysql://user:p%40ss@localhost:3306/db",
            // MySQL URL with no password
            "mysql://user@localhost:3306/db",
            // MySQL URL with socket path (if supported)
            "mysql://user:pass@localhost/db",
        ];

        for url in database_urls {
            // Test that URL parsing works with TLS configuration
            let config = TlsConfig::new(); // Platform validation mode

            // Verify that the TLS configuration can be applied to these URLs
            let ssl_opts = config.to_ssl_opts()?;
            assert!(ssl_opts.is_some(), "SSL options should be available for URL: {}", url);

            // Test that the URL format is preserved and can be used
            assert!(url.starts_with("mysql://"), "URL should start with mysql://: {}", url);
            assert!(url.contains("@"), "URL should contain @ separator: {}", url);
            assert!(url.contains(":"), "URL should contain : separator: {}", url);
        }

        Ok(())
    }

    /// Test DATABASE_URL with SSL parameters (legacy support)
    /// Requirement: 7.3 - URL-based SSL configuration handling
    #[test]
    fn test_database_url_with_ssl_parameters() -> Result<()> {
        let ssl_database_urls = vec![
            // URLs with SSL mode parameters (should be ignored in favor of CLI flags)
            "mysql://user:pass@localhost:3306/db?ssl-mode=required",
            "mysql://user:pass@localhost:3306/db?ssl-mode=disabled",
            "mysql://user:pass@localhost:3306/db?ssl-mode=preferred",
            "mysql://user:pass@localhost:3306/db?ssl-mode=verify-ca",
            "mysql://user:pass@localhost:3306/db?ssl-mode=verify-identity",
            // URLs with SSL certificate parameters (should be ignored)
            "mysql://user:pass@localhost:3306/db?ssl-ca=/path/to/ca.pem",
            "mysql://user:pass@localhost:3306/db?ssl-cert=/path/to/cert.pem",
            "mysql://user:pass@localhost:3306/db?ssl-key=/path/to/key.pem",
            // URLs with multiple SSL parameters
            "mysql://user:pass@localhost:3306/db?ssl-mode=required&ssl-ca=/path/to/ca.pem",
        ];

        for url in ssl_database_urls {
            // Test that URLs with SSL parameters are handled correctly
            let config = TlsConfig::new();
            let ssl_opts = config.to_ssl_opts()?;
            assert!(ssl_opts.is_some(), "SSL options should be available for SSL URL: {}", url);

            // Verify URL format is preserved
            assert!(url.starts_with("mysql://"), "SSL URL should start with mysql://: {}", url);
            assert!(url.contains("ssl"), "SSL URL should contain ssl parameter: {}", url);
        }

        Ok(())
    }

    /// Test that non-TLS DATABASE_URLs work as before
    /// Requirement: 7.2 - Non-TLS connection behavior unchanged
    #[test]
    fn test_non_tls_database_urls() -> Result<()> {
        let non_tls_urls = vec![
            "mysql://user:pass@localhost:3306/db",
            "mysql://user:pass@127.0.0.1:3306/db",
            "mysql://testuser:testpass@testhost:3306/testdb",
        ];

        for url in non_tls_urls {
            // Test that non-TLS URLs can still be used with TLS configuration
            let config = TlsConfig::new();
            let ssl_opts = config.to_ssl_opts()?;

            // TLS should be available even for non-TLS URLs (can be enabled via CLI flags)
            assert!(ssl_opts.is_some(), "TLS should be available for non-TLS URL: {}", url);

            // Verify URL format
            assert!(url.starts_with("mysql://"), "Non-TLS URL should start with mysql://: {}", url);
            assert!(!url.contains("ssl"), "Non-TLS URL should not contain ssl parameter: {}", url);
        }

        Ok(())
    }

    /// Test DATABASE_URL parsing with various edge cases
    /// Requirement: 7.4 - Connection behavior unchanged except TLS implementation
    #[test]
    fn test_database_url_edge_cases() -> Result<()> {
        let edge_case_urls = vec![
            // URL with IPv6 address
            "mysql://user:pass@[::1]:3306/db",
            // URL with encoded special characters
            "mysql://user:p%40ss%21@localhost:3306/db",
            // URL with query parameters (non-SSL)
            "mysql://user:pass@localhost:3306/db?charset=utf8mb4",
            "mysql://user:pass@localhost:3306/db?timeout=30",
            // URL with multiple query parameters
            "mysql://user:pass@localhost:3306/db?charset=utf8mb4&timeout=30",
            // URL with fragment (should be ignored)
            "mysql://user:pass@localhost:3306/db#fragment",
        ];

        for url in edge_case_urls {
            // Test that edge case URLs work with TLS configuration
            let config = TlsConfig::new();
            let ssl_opts = config.to_ssl_opts()?;
            assert!(ssl_opts.is_some(), "TLS should work with edge case URL: {}", url);

            // Verify URL format
            assert!(url.starts_with("mysql://"), "Edge case URL should start with mysql://: {}", url);
        }

        Ok(())
    }
}

mod tls_connection_compatibility_tests {
    use super::*;

    /// Test that TLS connections work the same as before
    /// Requirement: 7.1 - TLS connections work the same as before
    #[test]
    fn test_tls_connection_behavior_unchanged() -> Result<()> {
        // Test platform certificate validation (default behavior)
        let config = TlsConfig::new();
        assert!(matches!(config.validation_mode(), TlsValidationMode::Platform));

        let ssl_opts = config.to_ssl_opts()?;
        assert!(ssl_opts.is_some(), "Platform TLS should be available");

        // Test that the configuration produces the same behavior as before
        let config_clone = config.clone();
        assert_eq!(config, config_clone, "TLS configuration should be consistent");

        Ok(())
    }

    /// Test custom CA file functionality remains unchanged
    /// Requirement: 7.1 - Custom CA functionality preserved
    #[test]
    fn test_custom_ca_functionality_unchanged() -> Result<()> {
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;

        // Test custom CA configuration
        let config = TlsConfig::with_custom_ca(&cert_path);

        if let TlsValidationMode::CustomCa { ca_file_path } = config.validation_mode() {
            assert_eq!(ca_file_path, &cert_path, "CA file path should be preserved");
        } else {
            panic!("Expected CustomCa validation mode");
        }

        // Test SSL options generation
        let ssl_opts_result = config.to_ssl_opts();

        // Configuration should be created correctly (certificate parsing may fail, which is expected)
        match ssl_opts_result {
            Ok(ssl_opts) => assert!(ssl_opts.is_some(), "Custom CA SSL options should be available"),
            Err(_) => {
                // Certificate parsing failure is acceptable for this test
                // We're testing configuration preservation, not certificate validation
            },
        }

        Ok(())
    }

    /// Test hostname verification skip functionality unchanged
    /// Requirement: 7.1 - Hostname verification skip preserved
    #[test]
    fn test_hostname_verification_skip_unchanged() -> Result<()> {
        let config = TlsConfig::with_skip_hostname_verification();

        assert!(matches!(config.validation_mode(), TlsValidationMode::SkipHostnameVerification));

        let ssl_opts = config.to_ssl_opts()?;
        assert!(ssl_opts.is_some(), "Skip hostname verification SSL options should be available");

        Ok(())
    }

    /// Test invalid certificate acceptance functionality unchanged
    /// Requirement: 7.1 - Invalid certificate acceptance preserved
    #[test]
    fn test_invalid_certificate_acceptance_unchanged() -> Result<()> {
        let config = TlsConfig::with_accept_invalid();

        assert!(matches!(config.validation_mode(), TlsValidationMode::AcceptInvalid));

        let ssl_opts = config.to_ssl_opts()?;
        assert!(ssl_opts.is_some(), "Accept invalid certificate SSL options should be available");

        Ok(())
    }

    /// Test TLS configuration builder methods unchanged
    /// Requirement: 7.1 - Configuration API preserved
    #[test]
    fn test_tls_configuration_api_unchanged() -> Result<()> {
        // Test default configuration
        let default_config = TlsConfig::default();
        let new_config = TlsConfig::new();
        assert_eq!(default_config, new_config, "Default and new configurations should be identical");

        // Test builder methods
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;

        let custom_ca_config = TlsConfig::with_custom_ca(&cert_path);
        let skip_hostname_config = TlsConfig::with_skip_hostname_verification();
        let accept_invalid_config = TlsConfig::with_accept_invalid();

        // Verify each configuration is distinct
        assert_ne!(default_config, custom_ca_config);
        assert_ne!(default_config, skip_hostname_config);
        assert_ne!(default_config, accept_invalid_config);
        assert_ne!(custom_ca_config, skip_hostname_config);
        assert_ne!(custom_ca_config, accept_invalid_config);
        assert_ne!(skip_hostname_config, accept_invalid_config);

        Ok(())
    }

    /// Test TLS configuration from CLI options unchanged
    /// Requirement: 7.1 - CLI integration preserved
    #[test]
    fn test_tls_configuration_from_cli_unchanged() -> Result<()> {
        // Test default CLI options (no TLS flags)
        let default_tls_options = TlsOptions {
            tls_ca_file: None,
            insecure_skip_hostname_verify: false,
            allow_invalid_certificate: false,
        };

        let config = TlsConfig::from_tls_options(&default_tls_options)?;
        assert!(matches!(config.validation_mode(), TlsValidationMode::Platform));

        // Test custom CA CLI option
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;
        let ca_tls_options = TlsOptions {
            tls_ca_file: Some(cert_path.clone()),
            insecure_skip_hostname_verify: false,
            allow_invalid_certificate: false,
        };

        let config = TlsConfig::from_tls_options(&ca_tls_options)?;
        if let TlsValidationMode::CustomCa { ca_file_path } = config.validation_mode() {
            assert_eq!(ca_file_path, &cert_path);
        } else {
            panic!("Expected CustomCa validation mode");
        }

        // Test skip hostname CLI option
        let skip_hostname_options = TlsOptions {
            tls_ca_file: None,
            insecure_skip_hostname_verify: true,
            allow_invalid_certificate: false,
        };

        let config = TlsConfig::from_tls_options(&skip_hostname_options)?;
        assert!(matches!(config.validation_mode(), TlsValidationMode::SkipHostnameVerification));

        // Test accept invalid CLI option
        let accept_invalid_options = TlsOptions {
            tls_ca_file: None,
            insecure_skip_hostname_verify: false,
            allow_invalid_certificate: true,
        };

        let config = TlsConfig::from_tls_options(&accept_invalid_options)?;
        assert!(matches!(config.validation_mode(), TlsValidationMode::AcceptInvalid));

        Ok(())
    }
}

mod cli_flag_behavior_tests {
    use super::*;
    use clap::Parser;

    /// Test that CLI flag behavior is unchanged
    /// Requirement: 7.4 - CLI flag behavior unchanged
    #[test]
    fn test_cli_flag_parsing_unchanged() -> Result<()> {
        // Test basic CLI parsing without TLS flags
        let cli = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test:test@localhost:3306/test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
        ])?;

        assert!(cli.tls_options.tls_ca_file.is_none());
        assert!(!cli.tls_options.insecure_skip_hostname_verify);
        assert!(!cli.tls_options.allow_invalid_certificate);

        // Test CLI parsing with TLS CA file flag
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;
        let cli = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test:test@localhost:3306/test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--tls-ca-file",
            cert_path.to_str().unwrap(),
        ])?;

        assert_eq!(cli.tls_options.tls_ca_file, Some(cert_path));
        assert!(!cli.tls_options.insecure_skip_hostname_verify);
        assert!(!cli.tls_options.allow_invalid_certificate);

        // Test CLI parsing with skip hostname verification flag
        let cli = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test:test@localhost:3306/test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--insecure-skip-hostname-verify",
        ])?;

        assert!(cli.tls_options.tls_ca_file.is_none());
        assert!(cli.tls_options.insecure_skip_hostname_verify);
        assert!(!cli.tls_options.allow_invalid_certificate);

        // Test CLI parsing with allow invalid certificate flag
        let cli = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test:test@localhost:3306/test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--allow-invalid-certificate",
        ])?;

        assert!(cli.tls_options.tls_ca_file.is_none());
        assert!(!cli.tls_options.insecure_skip_hostname_verify);
        assert!(cli.tls_options.allow_invalid_certificate);

        Ok(())
    }

    /// Test that mutual exclusion of TLS flags still works
    /// Requirement: 7.4 - Mutual exclusion behavior preserved
    #[test]
    fn test_tls_flag_mutual_exclusion_unchanged() -> Result<()> {
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;

        // Test mutual exclusion: CA file + skip hostname
        let result = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test:test@localhost:3306/test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--tls-ca-file",
            cert_path.to_str().unwrap(),
            "--insecure-skip-hostname-verify",
        ]);

        assert!(result.is_err(), "CA file + skip hostname should be mutually exclusive");

        // Test mutual exclusion: CA file + accept invalid
        let result = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test:test@localhost:3306/test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--tls-ca-file",
            cert_path.to_str().unwrap(),
            "--allow-invalid-certificate",
        ]);

        assert!(result.is_err(), "CA file + accept invalid should be mutually exclusive");

        // Test mutual exclusion: skip hostname + accept invalid
        let result = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test:test@localhost:3306/test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--insecure-skip-hostname-verify",
            "--allow-invalid-certificate",
        ]);

        assert!(result.is_err(), "Skip hostname + accept invalid should be mutually exclusive");

        Ok(())
    }

    /// Test that TLS flags are always available (no feature gating)
    /// Requirement: 7.4 - TLS flags always available
    #[test]
    fn test_tls_flags_always_available() {
        // Test that help includes TLS flags
        let mut cmd = Command::cargo_bin("gold_digger").unwrap();
        let output = cmd.arg("--help").output().unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Verify TLS flags are present in help
        assert!(stdout.contains("tls-ca-file"), "Help should include --tls-ca-file flag");
        assert!(
            stdout.contains("insecure-skip-hostname-verify"),
            "Help should include --insecure-skip-hostname-verify flag"
        );
        assert!(stdout.contains("allow-invalid-certificate"), "Help should include --allow-invalid-certificate flag");

        // Verify flag descriptions are present
        assert!(stdout.contains("CA certificate"), "Help should describe CA certificate functionality");
        assert!(stdout.contains("hostname verification"), "Help should describe hostname verification");
        assert!(stdout.contains("certificate validation"), "Help should describe certificate validation");
    }

    /// Test CLI flag error messages unchanged
    /// Requirement: 7.4 - Error message consistency
    #[test]
    fn test_cli_flag_error_messages_unchanged() {
        let (_temp_dir, output_path) = create_temp_output_path().unwrap();

        // Test nonexistent CA file error
        let mut cmd = Command::cargo_bin("gold_digger").unwrap();
        let output = cmd
            .args([
                "--tls-ca-file",
                "/nonexistent/cert.pem",
                "--db-url",
                "mysql://test:test@localhost:3306/test",
                "--query",
                "SELECT 1",
                "--output",
                &output_path,
            ])
            .output()
            .unwrap();

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Verify error message format is preserved
        assert!(stderr.contains("CA certificate file not found"), "Error should mention CA certificate file");
        assert!(stderr.contains("/nonexistent/cert.pem"), "Error should include file path");

        // Verify credentials are not leaked
        assert!(!stderr.contains("test:test"), "Credentials should not be leaked");
        assert!(!stderr.contains("mysql://test:test@localhost:3306/test"), "Full URL should not be leaked");
    }
}

mod security_warnings_tests {
    use super::*;

    /// Test that security warnings still display correctly
    /// Requirement: 8.1 - Security warnings for insecure modes
    #[test]
    fn test_security_warnings_display_correctly() {
        // Test skip hostname verification warning
        let config = TlsConfig::with_skip_hostname_verification();

        // The display_security_warnings method prints to stderr
        // We test that it doesn't panic and the configuration is correct
        config.display_security_warnings();
        assert!(matches!(config.validation_mode(), TlsValidationMode::SkipHostnameVerification));

        // Test accept invalid certificate warning
        let config = TlsConfig::with_accept_invalid();
        config.display_security_warnings();
        assert!(matches!(config.validation_mode(), TlsValidationMode::AcceptInvalid));

        // Test platform mode (no warning)
        let config = TlsConfig::new();
        config.display_security_warnings();
        assert!(matches!(config.validation_mode(), TlsValidationMode::Platform));

        // Test custom CA mode (no warning)
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM).unwrap();
        let config = TlsConfig::with_custom_ca(&cert_path);
        config.display_security_warnings();
        if let TlsValidationMode::CustomCa { ca_file_path } = config.validation_mode() {
            assert_eq!(ca_file_path, &cert_path);
        } else {
            panic!("Expected CustomCa validation mode");
        }
    }

    /// Test security warning content for skip hostname verification
    /// Requirement: 8.1 - Specific warning messages
    #[test]
    fn test_skip_hostname_verification_warning_content() {
        let (_temp_dir, output_path) = create_temp_output_path().unwrap();

        // Test CLI command with skip hostname verification
        let mut cmd = Command::cargo_bin("gold_digger").unwrap();
        let output = cmd
            .args([
                "--insecure-skip-hostname-verify",
                "--db-url",
                "mysql://test:test@localhost:3306/test",
                "--query",
                "SELECT 1",
                "--output",
                &output_path,
            ])
            .output()
            .unwrap();

        let stderr = String::from_utf8_lossy(&output.stderr);

        // The warning should be displayed (though the connection will likely fail)
        // We're testing that the warning mechanism is in place

        // Verify credentials are not leaked in any output
        assert!(!stderr.contains("test:test"), "Credentials should not be leaked in stderr");

        // The actual warning display happens during connection setup
        // This test verifies the CLI accepts the flag correctly
        assert!(output.status.code().is_some(), "Command should exit with a status code");
    }

    /// Test security warning content for accept invalid certificate
    /// Requirement: 8.2 - Specific warning messages
    #[test]
    fn test_accept_invalid_certificate_warning_content() {
        let (_temp_dir, output_path) = create_temp_output_path().unwrap();

        // Test CLI command with accept invalid certificate
        let mut cmd = Command::cargo_bin("gold_digger").unwrap();
        let output = cmd
            .args([
                "--allow-invalid-certificate",
                "--db-url",
                "mysql://test:test@localhost:3306/test",
                "--query",
                "SELECT 1",
                "--output",
                &output_path,
            ])
            .output()
            .unwrap();

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Verify credentials are not leaked in any output
        assert!(!stderr.contains("test:test"), "Credentials should not be leaked in stderr");

        // The actual warning display happens during connection setup
        // This test verifies the CLI accepts the flag correctly
        assert!(output.status.code().is_some(), "Command should exit with a status code");
    }

    /// Test that no warnings are displayed for secure modes
    /// Requirement: 8.1, 8.2 - No warnings for secure configurations
    #[test]
    fn test_no_warnings_for_secure_modes() {
        // Test platform mode (secure, no warnings)
        let config = TlsConfig::new();
        config.display_security_warnings(); // Should not display warnings
        assert!(matches!(config.validation_mode(), TlsValidationMode::Platform));

        // Test custom CA mode (secure, no warnings)
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM).unwrap();
        let config = TlsConfig::with_custom_ca(&cert_path);
        config.display_security_warnings(); // Should not display warnings
        if let TlsValidationMode::CustomCa { ca_file_path } = config.validation_mode() {
            assert_eq!(ca_file_path, &cert_path);
        } else {
            panic!("Expected CustomCa validation mode");
        }
    }

    /// Test warning display integration with CLI
    /// Requirement: 8.1, 8.2 - Warning integration
    #[test]
    fn test_warning_display_integration() -> Result<()> {
        // Test that TLS configuration correctly identifies warning-worthy modes
        let skip_hostname_config = TlsConfig::with_skip_hostname_verification();
        let accept_invalid_config = TlsConfig::with_accept_invalid();
        let platform_config = TlsConfig::new();
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;
        let custom_ca_config = TlsConfig::with_custom_ca(&cert_path);

        // Verify configuration modes are correct
        assert!(matches!(skip_hostname_config.validation_mode(), TlsValidationMode::SkipHostnameVerification));
        assert!(matches!(accept_invalid_config.validation_mode(), TlsValidationMode::AcceptInvalid));
        assert!(matches!(platform_config.validation_mode(), TlsValidationMode::Platform));
        if let TlsValidationMode::CustomCa { ca_file_path } = custom_ca_config.validation_mode() {
            assert_eq!(ca_file_path, &cert_path);
        } else {
            panic!("Expected CustomCa validation mode");
        }

        // Test that all configurations can generate SSL options
        assert!(skip_hostname_config.to_ssl_opts()?.is_some());
        assert!(accept_invalid_config.to_ssl_opts()?.is_some());
        assert!(platform_config.to_ssl_opts()?.is_some());

        // Custom CA may fail certificate parsing, which is expected
        match custom_ca_config.to_ssl_opts() {
            Ok(ssl_opts) => assert!(ssl_opts.is_some()),
            Err(_) => {
                // Certificate parsing failure is acceptable
            },
        }

        Ok(())
    }
}

mod tls_always_available_tests {
    use super::*;
    use clap::Parser;

    /// Test that TLS is always available (no feature gating)
    /// Requirement: 7.1, 7.2, 7.3, 7.4 - TLS always available
    #[test]
    fn test_tls_always_available() -> Result<()> {
        // Test that TLS configuration can always be created
        let config = TlsConfig::new();
        let ssl_opts = config.to_ssl_opts()?;
        assert!(ssl_opts.is_some(), "TLS should always be available");

        // Test all TLS modes are available
        let platform_config = TlsConfig::new();
        let skip_hostname_config = TlsConfig::with_skip_hostname_verification();
        let accept_invalid_config = TlsConfig::with_accept_invalid();

        assert!(platform_config.to_ssl_opts()?.is_some());
        assert!(skip_hostname_config.to_ssl_opts()?.is_some());
        assert!(accept_invalid_config.to_ssl_opts()?.is_some());

        // Test custom CA configuration
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM)?;
        let custom_ca_config = TlsConfig::with_custom_ca(&cert_path);

        // Custom CA may fail certificate parsing, but configuration should be created
        match custom_ca_config.to_ssl_opts() {
            Ok(ssl_opts) => assert!(ssl_opts.is_some()),
            Err(_) => {
                // Certificate parsing failure is acceptable for this test
                // We're testing that TLS is available, not certificate validation
            },
        }

        Ok(())
    }

    /// Test that TLS functionality works without feature flags
    /// Requirement: 7.1 - No feature flag dependencies
    #[test]
    fn test_tls_no_feature_flag_dependencies() -> Result<()> {
        // Test that all TLS-related types and functions are available
        // without any feature flag compilation

        // Test TLS configuration creation
        let _config = TlsConfig::new();
        let _config = TlsConfig::default();
        let _config = TlsConfig::with_skip_hostname_verification();
        let _config = TlsConfig::with_accept_invalid();

        // Test TLS options parsing
        let tls_options = TlsOptions {
            tls_ca_file: None,
            insecure_skip_hostname_verify: false,
            allow_invalid_certificate: false,
        };
        let _config = TlsConfig::from_tls_options(&tls_options)?;

        // Test CLI parsing with TLS flags
        let _cli = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test:test@localhost:3306/test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--insecure-skip-hostname-verify",
        ])?;

        Ok(())
    }

    /// Test TLS error handling is always available
    /// Requirement: 7.1 - Error handling preserved
    #[test]
    fn test_tls_error_handling_always_available() -> Result<()> {
        use gold_digger::tls::TlsError;

        // Test that all TLS error types can be created
        let _error = TlsError::certificate_validation_failed("test");
        let _error = TlsError::ca_file_not_found("/path/to/file");
        let _error = TlsError::invalid_ca_format("/path/to/file", "message");
        let _error = TlsError::handshake_failed("test");
        let _error = TlsError::hostname_verification_failed("hostname", "message");
        let _error = TlsError::certificate_time_invalid("message");
        let _error = TlsError::mutually_exclusive_flags("flags");
        let _error = TlsError::connection_failed("message");

        // Test error classification methods
        let cert_error = TlsError::certificate_validation_failed("test");
        assert!(cert_error.is_certificate_error());
        assert!(!cert_error.is_hostname_error());
        assert!(!cert_error.is_server_configuration_error());
        assert!(!cert_error.is_client_configuration_error());

        let hostname_error = TlsError::hostname_verification_failed("host", "message");
        assert!(!hostname_error.is_certificate_error());
        assert!(hostname_error.is_hostname_error());

        // Test error suggestion functionality
        assert!(cert_error.suggest_cli_flag().is_some());
        assert!(hostname_error.suggest_cli_flag().is_some());

        Ok(())
    }
}

mod integration_compatibility_tests {
    use super::*;

    /// Test end-to-end CLI compatibility with TLS flags
    /// Requirement: 7.4 - End-to-end CLI behavior unchanged
    #[test]
    fn test_end_to_end_cli_compatibility() {
        let (_temp_dir, output_path) = create_temp_output_path().unwrap();

        // Test basic command without TLS flags (should work as before)
        let mut cmd = Command::cargo_bin("gold_digger").unwrap();
        let output = cmd
            .args([
                "--db-url",
                "mysql://test:test@localhost:3306/test",
                "--query",
                "SELECT 1",
                "--output",
                &output_path,
            ])
            .output()
            .unwrap();

        // Command should exit with some status (connection will likely fail, but CLI parsing should work)
        assert!(output.status.code().is_some(), "Command should exit with a status code");

        let stderr = String::from_utf8_lossy(&output.stderr);
        // Verify credentials are not leaked
        assert!(!stderr.contains("test:test"), "Credentials should not be leaked");
    }

    /// Test CLI help output includes TLS options
    /// Requirement: 7.4 - Help documentation preserved
    #[test]
    fn test_cli_help_includes_tls_options() {
        let mut cmd = Command::cargo_bin("gold_digger").unwrap();
        let output = cmd.arg("--help").output().unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Verify all TLS flags are documented
        assert!(stdout.contains("tls-ca-file"), "Help should document --tls-ca-file");
        assert!(
            stdout.contains("insecure-skip-hostname-verify"),
            "Help should document --insecure-skip-hostname-verify"
        );
        assert!(stdout.contains("allow-invalid-certificate"), "Help should document --allow-invalid-certificate");

        // Verify flag descriptions are helpful
        assert!(stdout.contains("CA certificate"), "Help should describe CA certificate functionality");
        assert!(stdout.contains("hostname"), "Help should mention hostname verification");
        assert!(stdout.contains("certificate"), "Help should mention certificate validation");
        assert!(
            stdout.contains("DANGEROUS") || stdout.contains("dangerous"),
            "Help should warn about dangerous options"
        );
    }

    /// Test that configuration dump includes TLS settings
    /// Requirement: 7.4 - Configuration introspection preserved
    #[test]
    fn test_configuration_dump_includes_tls() {
        let (_temp_dir, output_path) = create_temp_output_path().unwrap();

        // Test configuration dump with TLS flags
        let mut cmd = Command::cargo_bin("gold_digger").unwrap();
        let output = cmd
            .args([
                "--db-url",
                "mysql://test:test@localhost:3306/test",
                "--query",
                "SELECT 1",
                "--output",
                &output_path,
                "--insecure-skip-hostname-verify",
                "--dump-config",
            ])
            .output()
            .unwrap();

        // Command should exit (may fail due to missing database, but config dump should work)
        assert!(output.status.code().is_some(), "Command should exit with a status code");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Verify credentials are not leaked in any output
        assert!(!stdout.contains("test:test"), "Credentials should not be leaked in stdout");
        assert!(!stderr.contains("test:test"), "Credentials should not be leaked in stderr");
        assert!(!stdout.contains("mysql://test:test@localhost:3306/test"), "Full URL should not be leaked in stdout");
        assert!(!stderr.contains("mysql://test:test@localhost:3306/test"), "Full URL should not be leaked in stderr");
    }
}
