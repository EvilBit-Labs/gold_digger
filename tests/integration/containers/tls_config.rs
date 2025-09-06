//! TLS configuration for Gold Digger integration tests
//!
//! This module provides TLS configuration structures and validation for secure
//! database connections in container-based tests.

use anyhow::Result;

/// Configuration for TLS-enabled database containers
#[derive(Debug, Clone)]
pub struct ContainerTlsConfig {
    /// Whether TLS is enabled
    pub enabled: bool,
    /// Path to CA certificate file
    pub ca_cert_path: Option<std::path::PathBuf>,
    /// Whether to require secure transport
    pub require_secure_transport: bool,
    /// Minimum TLS version (TLSv1.2 or TLSv1.3)
    pub min_tls_version: String,
    /// Allowed cipher suites
    pub cipher_suites: Vec<String>,
    /// Whether to generate ephemeral certificates per test run
    pub use_ephemeral_certs: bool,
    /// Custom server certificate path (if not using ephemeral certificates)
    pub server_cert_path: Option<std::path::PathBuf>,
    /// Custom server key path (if not using ephemeral certificates)
    pub server_key_path: Option<std::path::PathBuf>,
}

impl Default for ContainerTlsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ca_cert_path: None,
            require_secure_transport: false,
            min_tls_version: "TLSv1.2".to_string(),
            cipher_suites: vec![
                "ECDHE-RSA-AES256-GCM-SHA384".to_string(),
                "ECDHE-RSA-AES128-GCM-SHA256".to_string(),
            ],
            use_ephemeral_certs: false,
            server_cert_path: None,
            server_key_path: None,
        }
    }
}

impl ContainerTlsConfig {
    /// Create a new TLS configuration with secure defaults
    pub fn new_secure() -> Self {
        Self {
            enabled: true,
            ca_cert_path: None,
            require_secure_transport: true,
            min_tls_version: "TLSv1.2".to_string(),
            cipher_suites: vec![
                "ECDHE-RSA-AES256-GCM-SHA384".to_string(),
                "ECDHE-RSA-AES128-GCM-SHA256".to_string(),
            ],
            use_ephemeral_certs: true,
            server_cert_path: None,
            server_key_path: None,
        }
    }

    /// Create a new TLS configuration with custom CA certificate path
    pub fn new_with_ca_cert<P: AsRef<std::path::Path>>(ca_cert_path: P) -> Self {
        Self {
            enabled: true,
            ca_cert_path: Some(ca_cert_path.as_ref().to_path_buf()),
            require_secure_transport: true,
            min_tls_version: "TLSv1.2".to_string(),
            cipher_suites: vec![
                "ECDHE-RSA-AES256-GCM-SHA384".to_string(),
                "ECDHE-RSA-AES128-GCM-SHA256".to_string(),
            ],
            use_ephemeral_certs: false,
            server_cert_path: None,
            server_key_path: None,
        }
    }

    /// Create a TLS configuration with minimum TLS version enforcement
    pub fn with_min_tls_version(mut self, version: &str) -> Result<Self> {
        match version {
            "TLSv1.2" | "TLSv1.3" => {
                self.min_tls_version = version.to_string();
                Ok(self)
            },
            _ => Err(anyhow::anyhow!("Invalid TLS version: {}. Must be TLSv1.2 or TLSv1.3", version)),
        }
    }

    /// Create a TLS configuration with strict cipher suite policy
    pub fn with_strict_ciphers(mut self) -> Self {
        self.cipher_suites = vec![
            "ECDHE-RSA-AES256-GCM-SHA384".to_string(),
            "ECDHE-RSA-AES128-GCM-SHA256".to_string(),
            "ECDHE-ECDSA-AES256-GCM-SHA384".to_string(),
            "ECDHE-ECDSA-AES128-GCM-SHA256".to_string(),
        ];
        self
    }

    /// Disable older TLS versions and weak ciphers
    pub fn with_security_hardening(mut self) -> Result<Self> {
        self.min_tls_version = "TLSv1.3".to_string();
        self.cipher_suites = vec![
            "TLS_AES_256_GCM_SHA384".to_string(),
            "TLS_AES_128_GCM_SHA256".to_string(),
            "TLS_CHACHA20_POLY1305_SHA256".to_string(),
        ];
        Ok(self)
    }

    /// Set the CA certificate path with validation
    pub fn with_ca_cert<P: AsRef<std::path::Path>>(mut self, path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Err(anyhow::anyhow!("CA certificate file does not exist: {}", path_ref.display()));
        }
        self.ca_cert_path = Some(path_ref.to_path_buf());
        Ok(self)
    }

    /// Validate TLS configuration
    pub fn validate(&self) -> Result<()> {
        if self.enabled {
            // Validate TLS version
            match self.min_tls_version.as_str() {
                "TLSv1.2" | "TLSv1.3" => {},
                _ => {
                    return Err(anyhow::anyhow!(
                        "Invalid TLS version: {}. Must be TLSv1.2 or TLSv1.3",
                        self.min_tls_version
                    ));
                },
            }

            // Validate cipher suites are not empty for secure configurations
            if self.cipher_suites.is_empty() && self.require_secure_transport {
                return Err(anyhow::anyhow!("Cipher suites cannot be empty when secure transport is required"));
            }

            // Validate CA certificate path if provided
            if let Some(ca_path) = &self.ca_cert_path
                && !ca_path.exists()
            {
                return Err(anyhow::anyhow!("CA certificate file does not exist: {}", ca_path.display()));
            }
        }
        Ok(())
    }
}

/// TLS connection validation result
#[derive(Debug, Clone)]
pub struct TlsValidationResult {
    /// Whether TLS connection was successful
    pub tls_connection_success: bool,
    /// TLS error message if connection failed
    pub tls_error: Option<String>,
}

/// Plain connection validation result
#[derive(Debug, Clone)]
pub struct PlainValidationResult {
    /// Whether plain connection was successful
    pub plain_connection_success: bool,
    /// Plain connection error message if connection failed
    pub plain_error: Option<String>,
}
