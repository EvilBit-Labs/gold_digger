//! TLS certificate fixtures and utilities
//!
//! This module provides ephemeral certificate generation and TLS testing
//! utilities for MySQL/MariaDB container integration tests.

pub mod cert_generator;

// Re-export the main certificate generation functionality
pub use cert_generator::EphemeralCertificate;

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;

/// Certificate loading utilities for container configuration
#[allow(dead_code)]
pub struct CertificateLoader;

impl CertificateLoader {
    /// Create temporary certificate files from PEM strings
    ///
    /// This is useful for mounting certificates into containers that expect
    /// certificate files rather than inline PEM strings.
    ///
    /// # Arguments
    /// * `cert_pem` - Certificate in PEM format
    /// * `key_pem` - Private key in PEM format
    ///
    /// # Returns
    /// * `Result<(NamedTempFile, NamedTempFile)>` - Tuple of (cert_file, key_file)
    #[allow(dead_code)]
    pub fn create_temp_files(cert_pem: &str, key_pem: &str) -> Result<(NamedTempFile, NamedTempFile)> {
        let cert_file = NamedTempFile::new().context("Failed to create temporary certificate file")?;
        fs::write(cert_file.path(), cert_pem).context("Failed to write certificate to temporary file")?;

        let key_file = NamedTempFile::new().context("Failed to create temporary key file")?;
        fs::write(key_file.path(), key_pem).context("Failed to write key to temporary file")?;

        Ok((cert_file, key_file))
    }

    /// Load certificate from file path
    ///
    /// # Arguments
    /// * `cert_path` - Path to certificate file
    ///
    /// # Returns
    /// * `Result<String>` - Certificate content in PEM format
    #[allow(dead_code)]
    pub fn load_cert_from_file<P: AsRef<Path>>(cert_path: P) -> Result<String> {
        fs::read_to_string(cert_path.as_ref())
            .with_context(|| format!("Failed to read certificate from {}", cert_path.as_ref().display()))
    }

    /// Validate certificate PEM format
    ///
    /// # Arguments
    /// * `cert_pem` - Certificate in PEM format
    ///
    /// # Returns
    /// * `Result<()>` - Ok if certificate is valid PEM format
    #[allow(dead_code)]
    pub fn validate_cert_pem(cert_pem: &str) -> Result<()> {
        if !cert_pem.contains("-----BEGIN CERTIFICATE-----") {
            anyhow::bail!("Certificate does not contain BEGIN CERTIFICATE marker");
        }
        if !cert_pem.contains("-----END CERTIFICATE-----") {
            anyhow::bail!("Certificate does not contain END CERTIFICATE marker");
        }
        Ok(())
    }

    /// Validate private key PEM format
    ///
    /// # Arguments
    /// * `key_pem` - Private key in PEM format
    ///
    /// # Returns
    /// * `Result<()>` - Ok if private key is valid PEM format
    #[allow(dead_code)]
    pub fn validate_key_pem(key_pem: &str) -> Result<()> {
        if !key_pem.contains("-----BEGIN PRIVATE KEY-----") {
            anyhow::bail!("Private key does not contain BEGIN PRIVATE KEY marker");
        }
        if !key_pem.contains("-----END PRIVATE KEY-----") {
            anyhow::bail!("Private key does not contain END PRIVATE KEY marker");
        }
        Ok(())
    }
}

/// Certificate validation helpers for TLS connection tests
#[allow(dead_code)]
pub struct CertificateValidator;

impl CertificateValidator {
    /// Validate that a certificate pair is properly formatted
    ///
    /// # Arguments
    /// * `cert_pem` - Certificate in PEM format
    /// * `key_pem` - Private key in PEM format
    ///
    /// # Returns
    /// * `Result<()>` - Ok if both certificate and key are valid
    #[allow(dead_code)]
    pub fn validate_certificate_pair(cert_pem: &str, key_pem: &str) -> Result<()> {
        CertificateLoader::validate_cert_pem(cert_pem).context("Certificate validation failed")?;
        CertificateLoader::validate_key_pem(key_pem).context("Private key validation failed")?;
        Ok(())
    }

    /// Validate that certificates are not empty
    ///
    /// # Arguments
    /// * `ephemeral_cert` - Ephemeral certificate to validate
    ///
    /// # Returns
    /// * `Result<()>` - Ok if all certificate components are non-empty
    #[allow(dead_code)]
    pub fn validate_ephemeral_certificate(ephemeral_cert: &EphemeralCertificate) -> Result<()> {
        if ephemeral_cert.ca_cert_pem.is_empty() {
            anyhow::bail!("CA certificate is empty");
        }
        if ephemeral_cert.ca_key_pem.is_empty() {
            anyhow::bail!("CA private key is empty");
        }
        if ephemeral_cert.server_cert_pem.is_empty() {
            anyhow::bail!("Server certificate is empty");
        }
        if ephemeral_cert.server_key_pem.is_empty() {
            anyhow::bail!("Server private key is empty");
        }

        // Validate PEM format for all components
        Self::validate_certificate_pair(&ephemeral_cert.ca_cert_pem, &ephemeral_cert.ca_key_pem)
            .context("CA certificate pair validation failed")?;
        Self::validate_certificate_pair(&ephemeral_cert.server_cert_pem, &ephemeral_cert.server_key_pem)
            .context("Server certificate pair validation failed")?;

        Ok(())
    }

    /// Check if certificate contains expected hostname in Subject Alternative Names
    ///
    /// This is a basic string-based check for testing purposes.
    /// In production, you would use a proper X.509 parser.
    ///
    /// # Arguments
    /// * `cert_pem` - Certificate in PEM format
    /// * `hostname` - Expected hostname
    ///
    /// # Returns
    /// * `bool` - True if hostname appears to be in the certificate
    #[allow(dead_code)]
    pub fn certificate_contains_hostname(cert_pem: &str, hostname: &str) -> bool {
        // This is a simple string search for testing purposes
        // In production, you would decode the certificate and check SANs properly
        cert_pem.contains(hostname)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certificate_loader_temp_files() -> Result<()> {
        let cert_pem = "-----BEGIN CERTIFICATE-----\ntest cert content\n-----END CERTIFICATE-----";
        let key_pem = "-----BEGIN PRIVATE KEY-----\ntest key content\n-----END PRIVATE KEY-----";

        let (cert_file, key_file) = CertificateLoader::create_temp_files(cert_pem, key_pem)?;

        // Verify files were created and contain correct content
        let cert_content = std::fs::read_to_string(cert_file.path())?;
        let key_content = std::fs::read_to_string(key_file.path())?;

        assert_eq!(cert_content, cert_pem);
        assert_eq!(key_content, key_pem);

        Ok(())
    }

    #[test]
    fn test_certificate_validator_pem_validation() -> Result<()> {
        let valid_cert = "-----BEGIN CERTIFICATE-----\ntest cert content\n-----END CERTIFICATE-----";
        let valid_key = "-----BEGIN PRIVATE KEY-----\ntest key content\n-----END PRIVATE KEY-----";

        // Valid certificates should pass
        CertificateValidator::validate_certificate_pair(valid_cert, valid_key)?;

        // Invalid certificates should fail
        let invalid_cert = "invalid certificate content";
        let invalid_key = "invalid key content";

        assert!(CertificateValidator::validate_certificate_pair(invalid_cert, valid_key).is_err());
        assert!(CertificateValidator::validate_certificate_pair(valid_cert, invalid_key).is_err());

        Ok(())
    }

    #[test]
    fn test_certificate_validator_ephemeral_validation() -> Result<()> {
        // Generate a valid ephemeral certificate
        let ephemeral_cert = EphemeralCertificate::generate(Some("test-container"))?;

        // Should pass validation
        CertificateValidator::validate_ephemeral_certificate(&ephemeral_cert)?;

        // Test with empty certificate (should fail)
        let empty_cert = EphemeralCertificate {
            ca_cert_pem: String::new(),
            ca_key_pem: String::new(),
            server_cert_pem: String::new(),
            server_key_pem: String::new(),
        };

        assert!(CertificateValidator::validate_ephemeral_certificate(&empty_cert).is_err());

        Ok(())
    }

    #[test]
    fn test_certificate_hostname_validation() {
        let cert_with_localhost = "-----BEGIN CERTIFICATE-----\nlocalhost content\n-----END CERTIFICATE-----";
        let cert_without_localhost = "-----BEGIN CERTIFICATE-----\nother content\n-----END CERTIFICATE-----";

        assert!(CertificateValidator::certificate_contains_hostname(cert_with_localhost, "localhost"));
        assert!(!CertificateValidator::certificate_contains_hostname(cert_without_localhost, "localhost"));
    }
}
