//! CLI integration tests for TLS functionality using insta for snapshot testing
//!
//! Requirements covered: 6.1, 6.2, 6.3, 6.4, 10.7, 11.3

use assert_cmd::Command;
use insta::assert_snapshot;
use std::fs;
use tempfile::TempDir;

// Import the new certificate generation functionality
mod fixtures;
use fixtures::tls::EphemeralCertificate;

/// Helper function to create a temporary certificate file for testing
#[allow(dead_code)]
fn create_temp_cert_file(content: &str) -> Result<(TempDir, std::path::PathBuf), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let cert_path = temp_dir.path().join("test_cert.pem");
    fs::write(&cert_path, content)?;
    Ok((temp_dir, cert_path))
}

/// Helper function to create a cross-platform temporary output path for testing
fn create_temp_output_path() -> Result<(TempDir, String), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let output_path = temp_dir.path().join("test_output.json");
    Ok((temp_dir, output_path.to_string_lossy().to_string()))
}

/// Generate a valid PEM certificate for testing using rcgen
/// This replaces the hardcoded certificate with dynamic generation
#[allow(dead_code)]
fn generate_test_certificate() -> Result<String, Box<dyn std::error::Error>> {
    let (cert_pem, _key_pem) =
        EphemeralCertificate::generate_self_signed(vec!["localhost".to_string(), "test.local".to_string()])?;
    Ok(cert_pem)
}

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

mod tls_mutual_exclusion_tests {
    use super::*;

    /// Test mutual exclusion: tls-ca-file and insecure-skip-hostname-verify
    /// Requirement: 6.1, 6.2, 6.3 - Mutually exclusive TLS flags
    #[test]
    fn test_ca_file_and_skip_hostname_mutual_exclusion() {
        let cert_pem = generate_test_certificate().unwrap();
        let (_temp_dir, cert_path) = create_temp_cert_file(&cert_pem).unwrap();
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
        let cert_pem = generate_test_certificate().unwrap();
        let (_temp_dir, cert_path) = create_temp_cert_file(&cert_pem).unwrap();
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
