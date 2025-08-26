//! CLI integration tests for TLS functionality using insta for snapshot testing
//!
//! Requirements covered: 6.1, 6.2, 6.3, 6.4, 10.7, 11.3

use assert_cmd::Command;
use insta::assert_snapshot;
use std::fs;
use tempfile::TempDir;

/// Helper function to create a temporary certificate file for testing
#[allow(dead_code)]
fn create_temp_cert_file(content: &str) -> Result<(TempDir, std::path::PathBuf), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let cert_path = temp_dir.path().join("test_cert.pem");
    fs::write(&cert_path, content)?;
    Ok((temp_dir, cert_path))
}

/// Helper function to create a cross-platform temporary output path for testing
#[cfg(feature = "ssl")]
fn create_temp_output_path() -> Result<(TempDir, String), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let output_path = temp_dir.path().join("test_output.json");
    Ok((temp_dir, output_path.to_string_lossy().to_string()))
}

/// Sample valid PEM certificate for testing
#[allow(dead_code)]
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
mod tls_cli_flag_tests {
    use super::*;

    /// Test TLS CLI help includes all TLS options
    /// Requirement: 11.3 - CLI documentation includes TLS flags
    #[test]
    fn test_tls_help_includes_all_options() {
        let mut cmd = Command::cargo_bin("gold_digger").unwrap();
        let output = cmd.arg("--help").output().unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Extract just the TLS-related help sections for snapshot testing
        let tls_help: Vec<&str> = stdout
            .lines()
            .filter(|line| {
                line.contains("tls-ca-file")
                    || line.contains("insecure-skip-hostname-verify")
                    || line.contains("allow-invalid-certificate")
            })
            .collect();

        assert_snapshot!("tls_help_options", tls_help.join("\n"));
    }

    /// Test nonexistent CA file error message
    /// Requirement: 10.7 - TLS error handling with user guidance
    #[test]
    fn test_nonexistent_ca_file_error() {
        let (_temp_dir, output_path) = create_temp_output_path().unwrap();

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

        // Assert that credentials are not leaked in error output
        // Note: This error is about CA file not found, not about database connection
        assert!(!stderr.contains("test:test"), "Credentials should not be leaked in error output");
        assert!(!stderr.contains("mysql://test:test@localhost:3306/test"), "Full DB URL should not be leaked");

        // Verify the error is about the CA file issue
        assert!(stderr.contains("CA certificate file not found"), "Error should mention CA certificate file issue");
        assert!(stderr.contains("/nonexistent/cert.pem"), "Error should mention the specific file path");

        assert_snapshot!("nonexistent_ca_file_error", stderr);
    }

    /// Test invalid CA file content error message
    /// Requirement: 10.7 - TLS error handling with user guidance
    #[test]
    fn test_invalid_ca_file_content_error() {
        let (_temp_dir, cert_path) = create_temp_cert_file("invalid certificate content").unwrap();
        let (_temp_dir2, output_path) = create_temp_output_path().unwrap();

        let mut cmd = Command::cargo_bin("gold_digger").unwrap();
        let output = cmd
            .args([
                "--tls-ca-file",
                cert_path.to_str().unwrap(),
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

        // Assert that credentials are not leaked in error output
        assert!(!stderr.contains("test:test"), "Credentials should not be leaked in error output");
        assert!(!stderr.contains("mysql://test:test@localhost:3306/test"), "Full DB URL should not be leaked");

        // Verify the error is about the CA file issue
        assert!(stderr.contains("Invalid CA certificate format"), "Error should mention invalid CA format");

        // Normalize the temporary directory path for consistent snapshots
        let normalized_stderr = stderr.replace(&cert_path.to_string_lossy().to_string(), "/tmp/test_cert.pem");
        assert_snapshot!("invalid_ca_file_content_error", normalized_stderr);
    }
}

#[cfg(feature = "ssl")]
mod tls_mutual_exclusion_tests {
    use super::*;

    /// Test mutual exclusion: tls-ca-file and insecure-skip-hostname-verify
    /// Requirement: 6.1, 6.2, 6.3 - Mutually exclusive TLS flags
    #[test]
    fn test_ca_file_and_skip_hostname_mutual_exclusion() {
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM).unwrap();
        let (_temp_dir2, output_path) = create_temp_output_path().unwrap();

        let mut cmd = Command::cargo_bin("gold_digger").unwrap();
        let output = cmd
            .args([
                "--tls-ca-file",
                cert_path.to_str().unwrap(),
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

        // Assert that credentials are not leaked in error output
        assert!(!stderr.contains("test:test"), "Credentials should not be leaked in error output");
        assert!(!stderr.contains("mysql://test:test@localhost:3306/test"), "Full DB URL should not be leaked");

        // Verify the error is about mutually exclusive flags (Clap error message)
        assert!(stderr.contains("cannot be used with"), "Error should mention mutually exclusive flags");

        assert_snapshot!("ca_file_and_skip_hostname_mutual_exclusion", stderr);
    }

    /// Test mutual exclusion: tls-ca-file and allow-invalid-certificate
    /// Requirement: 6.2, 6.4 - Mutually exclusive TLS flags
    #[test]
    fn test_ca_file_and_allow_invalid_mutual_exclusion() {
        let (_temp_dir, cert_path) = create_temp_cert_file(VALID_CERT_PEM).unwrap();
        let (_temp_dir2, output_path) = create_temp_output_path().unwrap();

        let mut cmd = Command::cargo_bin("gold_digger").unwrap();
        let output = cmd
            .args([
                "--tls-ca-file",
                cert_path.to_str().unwrap(),
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

        // Assert that credentials are not leaked in error output
        assert!(!stderr.contains("test:test"), "Credentials should not be leaked in error output");
        assert!(!stderr.contains("mysql://test:test@localhost:3306/test"), "Full DB URL should not be leaked");

        // Verify the error is about mutually exclusive flags (Clap error message)
        assert!(stderr.contains("cannot be used with"), "Error should mention mutually exclusive flags");

        assert_snapshot!("ca_file_and_allow_invalid_mutual_exclusion", stderr);
    }

    /// Test mutual exclusion: insecure-skip-hostname-verify and allow-invalid-certificate
    /// Requirement: 6.3, 6.4 - Mutually exclusive TLS flags
    #[test]
    fn test_skip_hostname_and_allow_invalid_mutual_exclusion() {
        let (_temp_dir, output_path) = create_temp_output_path().unwrap();

        let mut cmd = Command::cargo_bin("gold_digger").unwrap();
        let output = cmd
            .args([
                "--insecure-skip-hostname-verify",
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

        // Assert that credentials are not leaked in error output
        assert!(!stderr.contains("test:test"), "Credentials should not be leaked in error output");
        assert!(!stderr.contains("mysql://test:test@localhost:3306/test"), "Full DB URL should not be leaked");

        // Verify the error is about mutually exclusive flags (Clap error message)
        assert!(stderr.contains("cannot be used with"), "Error should mention mutually exclusive flags");

        assert_snapshot!("skip_hostname_and_allow_invalid_mutual_exclusion", stderr);
    }
}

#[cfg(not(feature = "ssl"))]
mod ssl_disabled_cli_tests {
    use super::*;

    /// Test TLS flags are not available when SSL feature is disabled
    /// Requirement: 11.3 - Feature-gated TLS functionality
    #[test]
    fn test_tls_flags_not_available_without_ssl() {
        let mut cmd = Command::cargo_bin("gold_digger").unwrap();
        let output = cmd.arg("--help").output().unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Verify TLS flags are not present in help
        assert!(!stdout.contains("tls-ca-file"));
        assert!(!stdout.contains("insecure-skip-hostname-verify"));
        assert!(!stdout.contains("allow-invalid-certificate"));

        assert_snapshot!("help_without_ssl_feature", stdout);
    }
}
