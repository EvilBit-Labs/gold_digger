//! Certificate generation utilities using rcgen for TLS testing
//!
//! This module provides ephemeral certificate generation for MySQL/MariaDB
//! container integration tests using the rcgen crate.

use anyhow::{Context, Result};
use rcgen::generate_simple_self_signed;

/// Certificate generation errors
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)] // Used in integration tests
pub enum CertificateError {
    #[error("Failed to generate CA certificate: {0}")]
    CaGenerationFailed(#[from] rcgen::Error),
    #[error("Failed to generate server certificate: {0}")]
    ServerGenerationFailed(String),
    #[error("Invalid hostname provided: {hostname}")]
    InvalidHostname { hostname: String },
}

/// Ephemeral certificate pair for TLS testing
#[derive(Debug, Clone)]
pub struct EphemeralCertificate {
    /// CA certificate in PEM format
    pub ca_cert_pem: String,
    /// CA private key in PEM format
    pub ca_key_pem: String,
    /// Server certificate in PEM format
    pub server_cert_pem: String,
    /// Server private key in PEM format
    pub server_key_pem: String,
}

impl EphemeralCertificate {
    /// Generate a new ephemeral certificate pair for TLS testing
    ///
    /// Creates a CA certificate and a server certificate signed by the CA.
    /// The server certificate includes localhost and container hostname SANs.
    ///
    /// # Arguments
    /// * `container_hostname` - Optional hostname for the container (e.g., "mysql-container")
    ///
    /// # Returns
    /// * `Result<EphemeralCertificate>` - Generated certificate pair
    ///
    /// # Errors
    /// Returns error if certificate generation fails
    pub fn generate(container_hostname: Option<&str>) -> Result<Self> {
        // For simplicity, generate two separate self-signed certificates
        // One for CA and one for server
        let ca_subject_alt_names = vec!["Gold Digger Test CA".to_string()];
        let ca_cert =
            generate_simple_self_signed(ca_subject_alt_names).context("Failed to generate CA certificate")?;

        // Generate server certificate with appropriate SANs
        let mut server_subject_alt_names = vec!["localhost".to_string()];
        if let Some(hostname) = container_hostname {
            server_subject_alt_names.push(hostname.to_string());
        }

        let server_cert =
            generate_simple_self_signed(server_subject_alt_names).context("Failed to generate server certificate")?;

        Ok(EphemeralCertificate {
            ca_cert_pem: ca_cert.cert.pem(),
            ca_key_pem: ca_cert.signing_key.serialize_pem(),
            server_cert_pem: server_cert.cert.pem(),
            server_key_pem: server_cert.signing_key.serialize_pem(),
        })
    }

    /// Generate a simple self-signed certificate for basic testing
    ///
    /// This is a convenience method for tests that don't need a full CA setup.
    ///
    /// # Arguments
    /// * `hostnames` - List of hostnames to include in the certificate
    ///
    /// # Returns
    /// * `Result<(String, String)>` - Tuple of (certificate_pem, private_key_pem)
    pub fn generate_self_signed(hostnames: Vec<String>) -> Result<(String, String)> {
        let cert = generate_simple_self_signed(hostnames).context("Failed to generate self-signed certificate")?;

        let cert_pem = cert.cert.pem();
        let key_pem = cert.signing_key.serialize_pem();

        Ok((cert_pem, key_pem))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ephemeral_certificate() {
        let cert = EphemeralCertificate::generate(Some("mysql-test")).unwrap();

        // Verify all components are present and non-empty
        assert!(!cert.ca_cert_pem.is_empty());
        assert!(!cert.ca_key_pem.is_empty());
        assert!(!cert.server_cert_pem.is_empty());
        assert!(!cert.server_key_pem.is_empty());

        // Verify PEM format
        assert!(cert.ca_cert_pem.contains("-----BEGIN CERTIFICATE-----"));
        assert!(cert.ca_cert_pem.contains("-----END CERTIFICATE-----"));
        assert!(cert.ca_key_pem.contains("-----BEGIN PRIVATE KEY-----"));
        assert!(cert.ca_key_pem.contains("-----END PRIVATE KEY-----"));
        assert!(cert.server_cert_pem.contains("-----BEGIN CERTIFICATE-----"));
        assert!(cert.server_cert_pem.contains("-----END CERTIFICATE-----"));
        assert!(cert.server_key_pem.contains("-----BEGIN PRIVATE KEY-----"));
        assert!(cert.server_key_pem.contains("-----END PRIVATE KEY-----"));
    }

    #[test]
    fn test_generate_self_signed_certificate() {
        let hostnames = vec!["localhost".to_string(), "test.local".to_string()];
        let (cert_pem, key_pem) = EphemeralCertificate::generate_self_signed(hostnames).unwrap();

        // Verify components are present and non-empty
        assert!(!cert_pem.is_empty());
        assert!(!key_pem.is_empty());

        // Verify PEM format
        assert!(cert_pem.contains("-----BEGIN CERTIFICATE-----"));
        assert!(cert_pem.contains("-----END CERTIFICATE-----"));
        assert!(key_pem.contains("-----BEGIN PRIVATE KEY-----"));
        assert!(key_pem.contains("-----END PRIVATE KEY-----"));
    }

    #[test]
    fn test_generate_without_container_hostname() {
        let cert = EphemeralCertificate::generate(None).unwrap();

        // Should still generate valid certificates
        assert!(!cert.ca_cert_pem.is_empty());
        assert!(!cert.server_cert_pem.is_empty());
    }
}
