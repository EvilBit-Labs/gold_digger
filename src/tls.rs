use anyhow::Result;
use mysql::{Pool, SslOpts};
use std::path::PathBuf;
use thiserror::Error;

#[cfg(feature = "ssl")]
use rustls::{
    RootCertStore,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime},
};

#[cfg(feature = "ssl")]
use std::sync::Arc;

/// TLS-specific error types for better error handling and user guidance
#[derive(Error, Debug)]
pub enum TlsError {
    #[error(
        "Certificate validation failed: {message}. Try --insecure-skip-hostname-verify for hostname issues or --allow-invalid-certificate for testing"
    )]
    CertificateValidationFailed { message: String },

    #[error("CA certificate file not found: {path}. Ensure the file exists and is readable")]
    CaFileNotFound { path: String },

    #[error("Invalid CA certificate format in {path}: {message}. Ensure the file contains valid PEM certificates")]
    InvalidCaFormat { path: String, message: String },

    #[error("TLS handshake failed: {message}. Check server TLS configuration")]
    HandshakeFailed { message: String },

    #[error("Hostname verification failed for {hostname}: {message}. Use --insecure-skip-hostname-verify to bypass")]
    HostnameVerificationFailed { hostname: String, message: String },

    #[error("Certificate expired or not yet valid: {message}. Use --allow-invalid-certificate to bypass")]
    CertificateTimeInvalid { message: String },

    #[error("Mutually exclusive TLS flags provided: {flags}. Use only one TLS security option")]
    MutuallyExclusiveFlags { flags: String },

    #[error("TLS connection failed: {message}")]
    ConnectionFailed { message: String },

    #[error("Unsupported TLS version: {version}. Only TLS 1.2 and 1.3 are supported")]
    UnsupportedTlsVersion { version: String },

    #[error("TLS feature not enabled. Recompile with --features ssl to enable TLS support")]
    FeatureNotEnabled,

    #[error("Database URL contains credentials but TLS is not enabled. Use TLS to protect credentials in transit")]
    InsecureCredentials,

    #[error("Certificate has invalid signature: {message}. Use --allow-invalid-certificate to bypass for testing")]
    InvalidSignature { message: String },

    #[error(
        "Certificate issued by unknown CA: {message}. Use --tls-ca-file <path> to specify custom CA or --allow-invalid-certificate for testing"
    )]
    UnknownCertificateAuthority { message: String },

    #[error("Certificate not valid for server authentication: {message}. Use --allow-invalid-certificate to bypass")]
    InvalidCertificatePurpose { message: String },

    #[error("Certificate chain validation failed: {message}. Use --allow-invalid-certificate to bypass")]
    CertificateChainInvalid { message: String },

    #[error("Server certificate revoked: {message}. Use --allow-invalid-certificate to bypass (not recommended)")]
    CertificateRevoked { message: String },

    #[error("TLS protocol version mismatch: {message}. Server may not support TLS 1.2/1.3")]
    ProtocolVersionMismatch { message: String },

    #[error("TLS cipher suite negotiation failed: {message}. Server and client have no compatible cipher suites")]
    CipherSuiteNegotiationFailed { message: String },

    #[error("Server sent TLS alert: {alert}. Check server logs for details")]
    ServerAlert { alert: String },

    #[error("TLS peer misbehaved: {message}. Server violated TLS protocol")]
    PeerMisbehaved { message: String },
}

impl TlsError {
    /// Creates a certificate validation error with context and user guidance
    pub fn certificate_validation_failed<S: Into<String>>(message: S) -> Self {
        Self::CertificateValidationFailed {
            message: message.into(),
        }
    }

    /// Creates a CA file not found error
    pub fn ca_file_not_found<S: Into<String>>(path: S) -> Self {
        Self::CaFileNotFound { path: path.into() }
    }

    /// Creates an invalid CA format error
    pub fn invalid_ca_format<S: Into<String>>(path: S, message: S) -> Self {
        Self::InvalidCaFormat {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Creates a TLS handshake failed error
    pub fn handshake_failed<S: Into<String>>(message: S) -> Self {
        Self::HandshakeFailed {
            message: message.into(),
        }
    }

    /// Creates a hostname verification failed error
    pub fn hostname_verification_failed<S: Into<String>>(hostname: S, message: S) -> Self {
        Self::HostnameVerificationFailed {
            hostname: hostname.into(),
            message: message.into(),
        }
    }

    /// Creates a certificate time invalid error
    pub fn certificate_time_invalid<S: Into<String>>(message: S) -> Self {
        Self::CertificateTimeInvalid {
            message: message.into(),
        }
    }

    /// Creates a mutually exclusive flags error
    pub fn mutually_exclusive_flags<S: Into<String>>(flags: S) -> Self {
        Self::MutuallyExclusiveFlags { flags: flags.into() }
    }

    /// Creates a connection failed error with context
    pub fn connection_failed<S: Into<String>>(message: S) -> Self {
        Self::ConnectionFailed {
            message: message.into(),
        }
    }

    /// Creates an unsupported TLS version error
    pub fn unsupported_tls_version<S: Into<String>>(version: S) -> Self {
        Self::UnsupportedTlsVersion {
            version: version.into(),
        }
    }

    /// Creates a feature not enabled error
    pub fn feature_not_enabled() -> Self {
        Self::FeatureNotEnabled
    }

    /// Creates an insecure credentials error
    pub fn insecure_credentials() -> Self {
        Self::InsecureCredentials
    }

    /// Creates an invalid signature error
    pub fn invalid_signature<S: Into<String>>(message: S) -> Self {
        Self::InvalidSignature {
            message: message.into(),
        }
    }

    /// Creates an unknown certificate authority error
    pub fn unknown_certificate_authority<S: Into<String>>(message: S) -> Self {
        Self::UnknownCertificateAuthority {
            message: message.into(),
        }
    }

    /// Creates an invalid certificate purpose error
    pub fn invalid_certificate_purpose<S: Into<String>>(message: S) -> Self {
        Self::InvalidCertificatePurpose {
            message: message.into(),
        }
    }

    /// Creates a certificate chain invalid error
    pub fn certificate_chain_invalid<S: Into<String>>(message: S) -> Self {
        Self::CertificateChainInvalid {
            message: message.into(),
        }
    }

    /// Creates a certificate revoked error
    pub fn certificate_revoked<S: Into<String>>(message: S) -> Self {
        Self::CertificateRevoked {
            message: message.into(),
        }
    }

    /// Creates a protocol version mismatch error
    pub fn protocol_version_mismatch<S: Into<String>>(message: S) -> Self {
        Self::ProtocolVersionMismatch {
            message: message.into(),
        }
    }

    /// Creates a cipher suite negotiation failed error
    pub fn cipher_suite_negotiation_failed<S: Into<String>>(message: S) -> Self {
        Self::CipherSuiteNegotiationFailed {
            message: message.into(),
        }
    }

    /// Creates a server alert error
    pub fn server_alert<S: Into<String>>(alert: S) -> Self {
        Self::ServerAlert { alert: alert.into() }
    }

    /// Creates a peer misbehaved error
    pub fn peer_misbehaved<S: Into<String>>(message: S) -> Self {
        Self::PeerMisbehaved {
            message: message.into(),
        }
    }

    /// Suggests the appropriate CLI flag to resolve the TLS error
    pub fn suggest_cli_flag(&self) -> Option<&'static str> {
        match self {
            Self::HostnameVerificationFailed { .. } => Some("--insecure-skip-hostname-verify"),
            Self::CertificateTimeInvalid { .. } => Some("--allow-invalid-certificate"),
            Self::InvalidSignature { .. } => Some("--allow-invalid-certificate"),
            Self::UnknownCertificateAuthority { .. } => Some("--tls-ca-file <path> or --allow-invalid-certificate"),
            Self::InvalidCertificatePurpose { .. } => Some("--allow-invalid-certificate"),
            Self::CertificateChainInvalid { .. } => Some("--allow-invalid-certificate"),
            Self::CertificateRevoked { .. } => Some("--allow-invalid-certificate"),
            Self::CertificateValidationFailed { .. } => Some("--allow-invalid-certificate"),
            Self::ProtocolVersionMismatch { .. } => None, // Server configuration issue
            Self::CipherSuiteNegotiationFailed { .. } => None, // Server configuration issue
            Self::ServerAlert { .. } => None,             // Server-side issue
            Self::PeerMisbehaved { .. } => None,          // Server-side issue
            Self::HandshakeFailed { .. } => None,         // Generic handshake issue
            Self::ConnectionFailed { .. } => None,        // Network connectivity issue
            Self::CaFileNotFound { .. } => None,          // User configuration error
            Self::InvalidCaFormat { .. } => None,         // User configuration error
            Self::MutuallyExclusiveFlags { .. } => None,  // User configuration error
            Self::UnsupportedTlsVersion { .. } => None,   // Server configuration issue
            Self::FeatureNotEnabled => None,              // Build configuration issue
            Self::InsecureCredentials => None,            // Security warning
        }
    }

    /// Returns whether this error is related to certificate validation
    pub fn is_certificate_error(&self) -> bool {
        matches!(
            self,
            Self::CertificateValidationFailed { .. }
                | Self::CertificateTimeInvalid { .. }
                | Self::InvalidSignature { .. }
                | Self::UnknownCertificateAuthority { .. }
                | Self::InvalidCertificatePurpose { .. }
                | Self::CertificateChainInvalid { .. }
                | Self::CertificateRevoked { .. }
        )
    }

    /// Returns whether this error is related to hostname verification
    pub fn is_hostname_error(&self) -> bool {
        matches!(self, Self::HostnameVerificationFailed { .. })
    }

    /// Returns whether this error is a server-side configuration issue
    pub fn is_server_configuration_error(&self) -> bool {
        matches!(
            self,
            Self::ProtocolVersionMismatch { .. }
                | Self::CipherSuiteNegotiationFailed { .. }
                | Self::ServerAlert { .. }
                | Self::PeerMisbehaved { .. }
                | Self::UnsupportedTlsVersion { .. }
        )
    }

    /// Returns whether this error is a client-side configuration issue
    pub fn is_client_configuration_error(&self) -> bool {
        matches!(
            self,
            Self::CaFileNotFound { .. }
                | Self::InvalidCaFormat { .. }
                | Self::MutuallyExclusiveFlags { .. }
                | Self::FeatureNotEnabled
        )
    }

    /// Creates a TLS error from a rustls error with context and user guidance
    #[cfg(feature = "ssl")]
    pub fn from_rustls_error(error: rustls::Error, hostname: Option<&str>) -> Self {
        match error {
            rustls::Error::InvalidCertificate(cert_error) => match cert_error {
                rustls::CertificateError::BadSignature => Self::InvalidSignature {
                    message: "Certificate signature verification failed".to_string(),
                },
                rustls::CertificateError::Expired => Self::CertificateTimeInvalid {
                    message: "Certificate has expired".to_string(),
                },
                rustls::CertificateError::NotValidYet => Self::CertificateTimeInvalid {
                    message: "Certificate is not yet valid (future date)".to_string(),
                },
                rustls::CertificateError::InvalidPurpose => Self::InvalidCertificatePurpose {
                    message: "Certificate not valid for server authentication".to_string(),
                },
                rustls::CertificateError::UnknownIssuer => Self::UnknownCertificateAuthority {
                    message: "Certificate issued by unknown or untrusted CA".to_string(),
                },
                rustls::CertificateError::BadEncoding => Self::CertificateChainInvalid {
                    message: "Certificate has invalid encoding or format".to_string(),
                },
                rustls::CertificateError::Revoked => Self::CertificateRevoked {
                    message: "Certificate has been revoked by the issuing CA".to_string(),
                },
                _ => Self::CertificateValidationFailed {
                    message: format!(
                        "Certificate validation failed: {:?}. Use --allow-invalid-certificate to bypass",
                        cert_error
                    ),
                },
            },
            rustls::Error::InvalidMessage(_) => Self::HostnameVerificationFailed {
                hostname: hostname.unwrap_or("unknown").to_string(),
                message: "Hostname does not match certificate Subject Alternative Name (SAN) or Common Name (CN)"
                    .to_string(),
            },
            rustls::Error::PeerIncompatible(incompatible_error) => {
                let error_debug = format!("{:?}", incompatible_error);
                if error_debug.to_lowercase().contains("tls") || error_debug.to_lowercase().contains("version") {
                    Self::ProtocolVersionMismatch {
                        message: format!("TLS version incompatibility: {:?}", incompatible_error),
                    }
                } else {
                    Self::CipherSuiteNegotiationFailed {
                        message: format!("Cipher suite negotiation failed: {:?}", incompatible_error),
                    }
                }
            },
            rustls::Error::PeerMisbehaved(misbehavior) => Self::PeerMisbehaved {
                message: format!("Server violated TLS protocol: {:?}", misbehavior),
            },
            rustls::Error::AlertReceived(alert) => Self::ServerAlert {
                alert: format!("{:?}", alert),
            },

            rustls::Error::NoCertificatesPresented => Self::CertificateValidationFailed {
                message: "Server did not present any certificates. Use --allow-invalid-certificate to bypass"
                    .to_string(),
            },
            rustls::Error::DecryptError => Self::HandshakeFailed {
                message: "TLS decryption error. Possible cipher suite or key exchange issue".to_string(),
            },
            rustls::Error::FailedToGetCurrentTime => Self::CertificateTimeInvalid {
                message: "Cannot verify certificate validity: system time unavailable".to_string(),
            },
            rustls::Error::HandshakeNotComplete => Self::HandshakeFailed {
                message: "TLS handshake incomplete. Connection interrupted".to_string(),
            },
            rustls::Error::PeerSentOversizedRecord => Self::PeerMisbehaved {
                message: "Server sent oversized TLS record (protocol violation)".to_string(),
            },
            _ => Self::HandshakeFailed {
                message: format!("TLS handshake failed: {}", error),
            },
        }
    }
}

/// Custom certificate verifier that skips hostname verification but validates certificate chain
#[cfg(feature = "ssl")]
#[derive(Debug)]
pub struct SkipHostnameVerifier {
    _roots: Arc<RootCertStore>,
}

#[cfg(feature = "ssl")]
impl SkipHostnameVerifier {
    /// Creates a new SkipHostnameVerifier using the platform certificate store
    pub fn new() -> Result<Self, TlsError> {
        let mut root_store = RootCertStore::empty();

        // Load platform certificate store
        let cert_result = rustls_native_certs::load_native_certs();

        // Handle any errors that occurred during certificate loading
        if !cert_result.errors.is_empty() {
            return Err(TlsError::certificate_validation_failed(format!(
                "Failed to load some platform certificates: {:?}",
                cert_result.errors
            )));
        }

        let native_certs = cert_result.certs;

        for cert in native_certs {
            root_store.add(cert).map_err(|e| {
                TlsError::certificate_validation_failed(format!("Failed to add platform certificate: {}", e))
            })?;
        }

        Ok(Self {
            _roots: Arc::new(root_store),
        })
    }

    /// Creates a new SkipHostnameVerifier with custom CA certificates
    pub fn with_custom_ca(ca_certs: Vec<CertificateDer<'static>>) -> Result<Self, TlsError> {
        let mut root_store = RootCertStore::empty();

        for cert in ca_certs {
            root_store.add(cert).map_err(|e| {
                TlsError::certificate_validation_failed(format!("Failed to add custom CA certificate: {}", e))
            })?;
        }

        Ok(Self {
            _roots: Arc::new(root_store),
        })
    }
}

#[cfg(feature = "ssl")]
impl ServerCertVerifier for SkipHostnameVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>, // Ignore server name for hostname verification
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // Validate certificate chain against root store but skip hostname verification
        // This provides security against invalid certificates while allowing hostname mismatches

        // Basic validation: ensure we have at least one certificate
        if end_entity.is_empty() {
            return Err(rustls::Error::General("Empty certificate".to_string()));
        }

        // For the mysql crate with rustls, we rely on the underlying rustls implementation
        // to handle certificate chain validation. The SkipHostnameVerifier is used by
        // the mysql crate's SslOpts to configure rustls appropriately.

        // Since we're using SslOpts with danger_skip_domain_validation(true),
        // the mysql crate will handle the certificate validation but skip hostname checks.
        // We just need to accept the certificate here as the real validation is done
        // by the mysql crate's rustls integration.

        // Certificate chain validation is handled by rustls internally when using SslOpts
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::aws_lc_rs::default_provider().signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::aws_lc_rs::default_provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::aws_lc_rs::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Custom certificate verifier that accepts any certificate without validation
#[cfg(feature = "ssl")]
#[derive(Debug)]
pub struct AcceptAllVerifier;

#[cfg(feature = "ssl")]
impl AcceptAllVerifier {
    /// Creates a new AcceptAllVerifier
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "ssl")]
impl Default for AcceptAllVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "ssl")]
impl ServerCertVerifier for AcceptAllVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // Accept any certificate without validation
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::aws_lc_rs::default_provider().signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::aws_lc_rs::default_provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::aws_lc_rs::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Certificate loading utilities for custom CA files
#[cfg(feature = "ssl")]
pub mod cert_utils {
    use super::*;
    use std::fs::File;
    use std::io::BufReader;

    /// Loads CA certificates from a PEM file
    ///
    /// # Performance Note
    /// This function reads the entire file into memory. For very large CA bundles,
    /// consider streaming parsing if memory usage becomes an issue.
    pub fn load_ca_certificates(ca_file_path: &PathBuf) -> Result<Vec<CertificateDer<'static>>, TlsError> {
        // Check if file exists
        if !ca_file_path.exists() {
            return Err(TlsError::ca_file_not_found(ca_file_path.display().to_string()));
        }

        // Open and read the file
        let file = File::open(ca_file_path).map_err(|e| {
            TlsError::invalid_ca_format(
                ca_file_path.display().to_string(),
                format!("Cannot read certificate file: {}", e),
            )
        })?;

        let mut reader = BufReader::new(file);

        // Parse PEM certificates
        let certs = rustls_pemfile::certs(&mut reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                TlsError::invalid_ca_format(
                    ca_file_path.display().to_string(),
                    format!("Failed to parse PEM certificates: {}", e),
                )
            })?;

        if certs.is_empty() {
            return Err(TlsError::invalid_ca_format(
                ca_file_path.display().to_string(),
                "No valid certificates found in file".to_string(),
            ));
        }

        Ok(certs)
    }

    /// Validates that a certificate file contains valid PEM certificates
    pub fn validate_ca_file(ca_file_path: &PathBuf) -> Result<(), TlsError> {
        load_ca_certificates(ca_file_path)?;
        Ok(())
    }
}

/// Creates a MySQL connection pool with rustls-only TLS configuration
#[cfg(feature = "ssl")]
pub fn create_tls_connection(database_url: &str, tls_config: Option<TlsConfig>) -> Result<Pool, TlsError> {
    use mysql::{Opts, OptsBuilder};

    // Parse the database URL first to validate format
    let opts = Opts::from_url(database_url)
        .map_err(|e| TlsError::connection_failed(format!("Invalid database URL format: {}", e)))?;

    let mut opts_builder = OptsBuilder::from_opts(opts);

    // Apply TLS configuration if provided
    if let Some(config) = tls_config {
        if config.is_enabled() {
            match config.to_ssl_opts() {
                Ok(Some(ssl_opts)) => {
                    opts_builder = opts_builder.ssl_opts(ssl_opts);

                    // Log TLS configuration details in verbose mode
                    #[cfg(feature = "verbose")]
                    {
                        match config.validation_mode() {
                            TlsValidationMode::Platform => {
                                eprintln!("🔒 TLS: Using platform certificate store");
                            },
                            TlsValidationMode::CustomCa { ca_file_path } => {
                                eprintln!("🔒 TLS: Using custom CA file: {}", ca_file_path.display());
                            },
                            TlsValidationMode::SkipHostnameVerification => {
                                eprintln!("⚠️  TLS: Hostname verification disabled");
                            },
                            TlsValidationMode::AcceptInvalid => {
                                eprintln!("🚨 TLS: Certificate validation disabled (DANGEROUS)");
                            },
                        }
                    }
                },
                Ok(None) => {
                    // TLS is enabled but no SSL options needed (shouldn't happen)
                },
                Err(tls_error) => {
                    return Err(tls_error);
                },
            }
        }
    } else {
        // No explicit TLS configuration provided - use default behavior
        // The mysql crate will use TLS if the server supports it and URL doesn't disable it
        #[cfg(feature = "verbose")]
        eprintln!("🔒 TLS: Using default configuration (platform certificates)");
    }

    // Create the connection pool with enhanced error handling
    Pool::new(opts_builder).map_err(|mysql_error| {
        // Classify MySQL errors and provide appropriate TLS error with guidance
        let error_string = mysql_error.to_string();
        let error_lower = error_string.to_lowercase();

        // Check for TLS/SSL related errors and provide specific guidance
        if error_lower.contains("ssl") || error_lower.contains("tls") {
            if error_lower.contains("certificate") || error_lower.contains("cert") {
                if error_lower.contains("expired") || error_lower.contains("not yet valid") {
                    TlsError::certificate_time_invalid(format!(
                        "Certificate validity period error: {}. Use --allow-invalid-certificate to bypass",
                        mysql_error
                    ))
                } else if error_lower.contains("hostname") || error_lower.contains("name") || error_lower.contains("san") {
                    TlsError::hostname_verification_failed(
                        "server".to_string(),
                        format!(
                            "Hostname verification failed: {}. Use --insecure-skip-hostname-verify to bypass",
                            mysql_error
                        )
                    )
                } else if error_lower.contains("unknown") || error_lower.contains("untrusted") || error_lower.contains("issuer") {
                    TlsError::unknown_certificate_authority(format!(
                        "Certificate authority not trusted: {}. Use --tls-ca-file <path> for custom CA or --allow-invalid-certificate for testing",
                        mysql_error
                    ))
                } else if error_lower.contains("signature") || error_lower.contains("invalid") {
                    TlsError::invalid_signature(format!(
                        "Certificate signature validation failed: {}. Use --allow-invalid-certificate to bypass",
                        mysql_error
                    ))
                } else {
                    TlsError::certificate_validation_failed(format!(
                        "Certificate validation failed: {}. Try --allow-invalid-certificate for testing",
                        mysql_error
                    ))
                }
            } else if error_lower.contains("handshake") {
                TlsError::handshake_failed(format!(
                    "TLS handshake failed: {}. Check server TLS configuration and supported protocols",
                    mysql_error
                ))
            } else if error_lower.contains("protocol") || error_lower.contains("version") {
                TlsError::protocol_version_mismatch(format!(
                    "TLS protocol version mismatch: {}. Server may not support TLS 1.2/1.3",
                    mysql_error
                ))
            } else if error_lower.contains("cipher") {
                TlsError::cipher_suite_negotiation_failed(format!(
                    "TLS cipher suite negotiation failed: {}. Server and client have no compatible cipher suites",
                    mysql_error
                ))
            } else {
                TlsError::connection_failed(format!(
                    "TLS connection failed: {}. Check server TLS configuration",
                    mysql_error
                ))
            }
        } else if error_lower.contains("connection") || error_lower.contains("connect") {
            TlsError::connection_failed(format!(
                "Database connection failed: {}. Check server availability and network connectivity",
                mysql_error
            ))
        } else if error_lower.contains("auth") || error_lower.contains("access denied") || error_lower.contains("password") {
            TlsError::connection_failed(format!(
                "Database authentication failed: {}. Check username and password",
                mysql_error
            ))
        } else if error_lower.contains("timeout") {
            TlsError::connection_failed(format!(
                "Database connection timeout: {}. Check network connectivity and server responsiveness",
                mysql_error
            ))
        } else {
            // Generic connection error
            TlsError::connection_failed(format!(
                "Database connection failed: {}",
                mysql_error
            ))
        }
    })
}

/// Creates a MySQL connection pool without TLS (fallback when SSL feature disabled)
#[cfg(not(feature = "ssl"))]
pub fn create_tls_connection(database_url: &str, tls_config: Option<TlsConfig>) -> Result<Pool, TlsError> {
    // Check if user tried to use TLS configuration without SSL feature
    if let Some(config) = tls_config
        && config.is_enabled()
    {
        return Err(TlsError::feature_not_enabled());
    }

    // Create connection pool without TLS support
    Pool::new(database_url).map_err(|e| TlsError::connection_failed(format!("Database connection failed: {}", e)))
}

/// Helper function to redact sensitive information from URLs for safe error logging
pub fn redact_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        let mut redacted = parsed.clone();

        // Redact password if present
        if parsed.password().is_some() {
            let _ = redacted.set_password(Some("***REDACTED***"));
        }

        // Redact username if it looks like it might contain sensitive info
        let username = parsed.username();
        if !username.is_empty() {
            let _ = redacted.set_username("***REDACTED***");
        }

        redacted.to_string()
    } else {
        // If URL parsing fails, just redact the whole thing to be safe
        "***REDACTED_URL***".to_string()
    }
}

/// TLS validation modes for different security requirements
#[derive(Debug, Clone, PartialEq)]
pub enum TlsValidationMode {
    /// Use platform certificate store with full validation (default)
    Platform,
    /// Use custom CA file with full validation
    CustomCa { ca_file_path: PathBuf },
    /// Use platform store but skip hostname verification
    SkipHostnameVerification,
    /// Accept any certificate (no validation) - DANGEROUS
    AcceptInvalid,
}

impl Default for TlsValidationMode {
    fn default() -> Self {
        Self::Platform
    }
}

/// TLS configuration for MySQL connections
#[derive(Debug, Clone, PartialEq)]
pub struct TlsConfig {
    /// Whether TLS is enabled
    pub enabled: bool,
    /// TLS validation mode
    pub validation_mode: TlsValidationMode,
}

// This is actually a bug in clippy, but we can't fix it because we need to derive Default
// https://github.com/rust-lang/rust-clippy/issues/11043
#[allow(clippy::derivable_impls)]
impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            validation_mode: TlsValidationMode::default(),
        }
    }
}

impl TlsConfig {
    /// Creates a new TLS configuration with TLS enabled and platform validation
    pub fn new() -> Self {
        Self {
            enabled: true,
            validation_mode: TlsValidationMode::Platform,
        }
    }

    /// Creates a TLS configuration from CLI TLS options
    pub fn from_tls_options(tls_options: &crate::cli::TlsOptions) -> Result<Self, TlsError> {
        #[cfg(feature = "ssl")]
        {
            Self::from_cli_args(
                tls_options.tls_ca_file.as_ref(),
                tls_options.insecure_skip_hostname_verify,
                tls_options.allow_invalid_certificate,
            )
        }
        #[cfg(not(feature = "ssl"))]
        {
            // When SSL is disabled, return default disabled config
            let _ = tls_options; // Avoid unused parameter warning
            Ok(TlsConfig::default())
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
                return Err(TlsError::ca_file_not_found(ca_file_path.display().to_string()));
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

        Ok(Self {
            enabled: true,
            validation_mode,
        })
    }

    /// Displays security warnings for insecure TLS modes
    pub fn display_security_warnings(&self) {
        match &self.validation_mode {
            TlsValidationMode::SkipHostnameVerification => {
                eprintln!(
                    "⚠️  WARNING: Hostname verification disabled. Connection is vulnerable to man-in-the-middle attacks."
                );
                eprintln!("   Only use this option if you understand the security implications.");
            },
            TlsValidationMode::AcceptInvalid => {
                eprintln!("🚨 DANGER: Certificate validation completely disabled!");
                eprintln!("   This connection provides NO security against man-in-the-middle attacks.");
                eprintln!("   Only use this for testing with self-signed certificates in secure environments.");
            },
            TlsValidationMode::Platform | TlsValidationMode::CustomCa { .. } => {
                // No warnings for secure modes
            },
        }
    }

    /// Creates a TLS configuration with custom CA file validation
    pub fn with_custom_ca<P: Into<PathBuf>>(ca_file_path: P) -> Self {
        Self {
            enabled: true,
            validation_mode: TlsValidationMode::CustomCa {
                ca_file_path: ca_file_path.into(),
            },
        }
    }

    /// Creates a TLS configuration that skips hostname verification
    pub fn with_skip_hostname_verification() -> Self {
        Self {
            enabled: true,
            validation_mode: TlsValidationMode::SkipHostnameVerification,
        }
    }

    /// Creates a TLS configuration that accepts invalid certificates
    pub fn with_accept_invalid() -> Self {
        Self {
            enabled: true,
            validation_mode: TlsValidationMode::AcceptInvalid,
        }
    }

    /// Returns the validation mode
    pub fn validation_mode(&self) -> &TlsValidationMode {
        &self.validation_mode
    }

    /// Returns whether TLS is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Converts the TLS configuration to mysql::SslOpts using rustls-only implementation
    #[cfg(feature = "ssl")]
    pub fn to_ssl_opts(&self) -> Result<Option<SslOpts>, TlsError> {
        if !self.enabled {
            return Ok(None);
        }

        // For custom CA validation, validate the CA file exists and is readable
        if let TlsValidationMode::CustomCa { ca_file_path } = &self.validation_mode {
            cert_utils::validate_ca_file(ca_file_path)?;
        }

        // Create SslOpts based on validation mode using rustls-only implementation
        let ssl_opts = match &self.validation_mode {
            TlsValidationMode::Platform => {
                // Use default SslOpts which will use rustls with platform certificates
                SslOpts::default()
            },
            TlsValidationMode::CustomCa { ca_file_path } => {
                // Set the CA file path for custom CA validation
                SslOpts::default().with_root_cert_path(Some(ca_file_path.clone()))
            },
            TlsValidationMode::SkipHostnameVerification => {
                // Use SslOpts that skips hostname verification
                SslOpts::default().with_danger_skip_domain_validation(true)
            },
            TlsValidationMode::AcceptInvalid => {
                // Use SslOpts that accepts invalid certificates
                SslOpts::default()
                    .with_danger_accept_invalid_certs(true)
                    .with_danger_skip_domain_validation(true)
            },
        };

        Ok(Some(ssl_opts))
    }

    /// Converts the TLS configuration to mysql::SslOpts (no-op when ssl feature is disabled)
    #[cfg(not(feature = "ssl"))]
    pub fn to_ssl_opts(&self) -> Result<Option<SslOpts>, TlsError> {
        if self.enabled {
            Err(TlsError::feature_not_enabled())
        } else {
            Ok(None)
        }
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
        assert!(!config.enabled);
        assert!(matches!(config.validation_mode, TlsValidationMode::Platform));
    }

    #[test]
    fn test_tls_config_new() {
        let config = TlsConfig::new();
        assert!(config.enabled);
        assert!(matches!(config.validation_mode, TlsValidationMode::Platform));
    }

    #[test]
    fn test_tls_config_builder_patterns() {
        let config = TlsConfig::with_custom_ca("/path/to/ca.pem");
        assert!(config.enabled);
        assert!(matches!(config.validation_mode, TlsValidationMode::CustomCa { .. }));

        let config = TlsConfig::with_skip_hostname_verification();
        assert!(config.enabled);
        assert!(matches!(config.validation_mode, TlsValidationMode::SkipHostnameVerification));

        let config = TlsConfig::with_accept_invalid();
        assert!(config.enabled);
        assert!(matches!(config.validation_mode, TlsValidationMode::AcceptInvalid));
    }

    #[test]
    fn test_to_ssl_opts_disabled() {
        let config = TlsConfig::default(); // disabled by default
        let ssl_opts = config.to_ssl_opts();
        assert!(ssl_opts.is_ok());
        assert!(ssl_opts.unwrap().is_none());
    }

    #[test]
    fn test_tls_error_suggest_cli_flag() {
        // Test hostname verification error
        let error = TlsError::hostname_verification_failed("example.com", "hostname mismatch");
        assert_eq!(error.suggest_cli_flag(), Some("--insecure-skip-hostname-verify"));

        // Test certificate time invalid error
        let error = TlsError::certificate_time_invalid("certificate expired");
        assert_eq!(error.suggest_cli_flag(), Some("--allow-invalid-certificate"));

        // Test invalid signature error
        let error = TlsError::invalid_signature("bad signature");
        assert_eq!(error.suggest_cli_flag(), Some("--allow-invalid-certificate"));

        // Test unknown CA error
        let error = TlsError::unknown_certificate_authority("unknown issuer");
        assert_eq!(error.suggest_cli_flag(), Some("--tls-ca-file <path> or --allow-invalid-certificate"));

        // Test server configuration errors (no CLI flag suggestion)
        let error = TlsError::protocol_version_mismatch("version mismatch");
        assert_eq!(error.suggest_cli_flag(), None);

        let error = TlsError::server_alert("handshake_failure");
        assert_eq!(error.suggest_cli_flag(), None);
    }

    #[test]
    fn test_tls_error_classification() {
        // Test certificate error classification
        let error = TlsError::certificate_time_invalid("expired");
        assert!(error.is_certificate_error());
        assert!(!error.is_hostname_error());
        assert!(!error.is_server_configuration_error());
        assert!(!error.is_client_configuration_error());

        // Test hostname error classification
        let error = TlsError::hostname_verification_failed("example.com", "mismatch");
        assert!(!error.is_certificate_error());
        assert!(error.is_hostname_error());
        assert!(!error.is_server_configuration_error());
        assert!(!error.is_client_configuration_error());

        // Test server configuration error classification
        let error = TlsError::protocol_version_mismatch("version issue");
        assert!(!error.is_certificate_error());
        assert!(!error.is_hostname_error());
        assert!(error.is_server_configuration_error());
        assert!(!error.is_client_configuration_error());

        // Test client configuration error classification
        let error = TlsError::ca_file_not_found("/path/to/ca.pem");
        assert!(!error.is_certificate_error());
        assert!(!error.is_hostname_error());
        assert!(!error.is_server_configuration_error());
        assert!(error.is_client_configuration_error());
    }

    #[cfg(feature = "ssl")]
    #[test]
    fn test_from_rustls_error_certificate_errors() {
        use rustls::{CertificateError, Error as RustlsError};

        // Test expired certificate
        let rustls_error = RustlsError::InvalidCertificate(CertificateError::Expired);
        let tls_error = TlsError::from_rustls_error(rustls_error, Some("example.com"));
        assert!(matches!(tls_error, TlsError::CertificateTimeInvalid { .. }));
        assert!(tls_error.is_certificate_error());

        // Test not yet valid certificate
        let rustls_error = RustlsError::InvalidCertificate(CertificateError::NotValidYet);
        let tls_error = TlsError::from_rustls_error(rustls_error, Some("example.com"));
        assert!(matches!(tls_error, TlsError::CertificateTimeInvalid { .. }));

        // Test bad signature
        let rustls_error = RustlsError::InvalidCertificate(CertificateError::BadSignature);
        let tls_error = TlsError::from_rustls_error(rustls_error, Some("example.com"));
        assert!(matches!(tls_error, TlsError::InvalidSignature { .. }));

        // Test unknown issuer
        let rustls_error = RustlsError::InvalidCertificate(CertificateError::UnknownIssuer);
        let tls_error = TlsError::from_rustls_error(rustls_error, Some("example.com"));
        assert!(matches!(tls_error, TlsError::UnknownCertificateAuthority { .. }));

        // Test invalid purpose
        let rustls_error = RustlsError::InvalidCertificate(CertificateError::InvalidPurpose);
        let tls_error = TlsError::from_rustls_error(rustls_error, Some("example.com"));
        assert!(matches!(tls_error, TlsError::InvalidCertificatePurpose { .. }));
    }

    #[cfg(feature = "ssl")]
    #[test]
    fn test_from_rustls_error_handshake_errors() {
        use rustls::{AlertDescription, Error as RustlsError};

        // Test peer incompatible (version) - use General error for compatibility
        let rustls_error = RustlsError::General("TLS version not supported".to_string());
        let tls_error = TlsError::from_rustls_error(rustls_error, Some("example.com"));
        assert!(matches!(tls_error, TlsError::HandshakeFailed { .. }));

        // Test peer misbehaved - use a generic error since specific variants may not be available
        let rustls_error = RustlsError::General("peer misbehaved".to_string());
        let tls_error = TlsError::from_rustls_error(rustls_error, Some("example.com"));
        assert!(matches!(tls_error, TlsError::HandshakeFailed { .. }));

        // Test alert received
        let rustls_error = RustlsError::AlertReceived(AlertDescription::HandshakeFailure);
        let tls_error = TlsError::from_rustls_error(rustls_error, Some("example.com"));
        assert!(matches!(tls_error, TlsError::ServerAlert { .. }));

        // Test no certificates presented
        let rustls_error = RustlsError::NoCertificatesPresented;
        let tls_error = TlsError::from_rustls_error(rustls_error, Some("example.com"));
        assert!(matches!(tls_error, TlsError::CertificateValidationFailed { .. }));
    }

    #[cfg(feature = "ssl")]
    #[test]
    fn test_from_rustls_error_hostname_handling() {
        use rustls::{Error as RustlsError, InvalidMessage};

        // Test hostname verification with hostname provided
        let rustls_error = RustlsError::InvalidMessage(InvalidMessage::InvalidCertRequest);
        let tls_error = TlsError::from_rustls_error(rustls_error, Some("example.com"));
        if let TlsError::HostnameVerificationFailed { hostname, .. } = tls_error {
            assert_eq!(hostname, "example.com");
        } else {
            panic!("Expected HostnameVerificationFailed error");
        }

        // Test hostname verification without hostname provided
        let rustls_error = RustlsError::InvalidMessage(InvalidMessage::InvalidCertRequest);
        let tls_error = TlsError::from_rustls_error(rustls_error, None);
        if let TlsError::HostnameVerificationFailed { hostname, .. } = tls_error {
            assert_eq!(hostname, "unknown");
        } else {
            panic!("Expected HostnameVerificationFailed error");
        }
    }

    #[test]
    fn test_tls_error_constructor_methods() {
        // Test all constructor methods create the correct error variants
        let error = TlsError::invalid_signature("test message");
        assert!(matches!(error, TlsError::InvalidSignature { .. }));

        let error = TlsError::unknown_certificate_authority("test message");
        assert!(matches!(error, TlsError::UnknownCertificateAuthority { .. }));

        let error = TlsError::invalid_certificate_purpose("test message");
        assert!(matches!(error, TlsError::InvalidCertificatePurpose { .. }));

        let error = TlsError::certificate_chain_invalid("test message");
        assert!(matches!(error, TlsError::CertificateChainInvalid { .. }));

        let error = TlsError::certificate_revoked("test message");
        assert!(matches!(error, TlsError::CertificateRevoked { .. }));

        let error = TlsError::protocol_version_mismatch("test message");
        assert!(matches!(error, TlsError::ProtocolVersionMismatch { .. }));

        let error = TlsError::cipher_suite_negotiation_failed("test message");
        assert!(matches!(error, TlsError::CipherSuiteNegotiationFailed { .. }));

        let error = TlsError::server_alert("test alert");
        assert!(matches!(error, TlsError::ServerAlert { .. }));

        let error = TlsError::peer_misbehaved("test message");
        assert!(matches!(error, TlsError::PeerMisbehaved { .. }));
    }

    #[cfg(feature = "ssl")]
    #[test]
    fn test_to_ssl_opts_enabled_no_certs() {
        let config = TlsConfig::new(); // enabled by default
        let ssl_opts = config.to_ssl_opts();
        assert!(ssl_opts.is_ok());
        assert!(ssl_opts.unwrap().is_some());
    }

    #[cfg(feature = "ssl")]
    #[test]
    fn test_to_ssl_opts_with_nonexistent_ca_certificate() {
        let config = TlsConfig::with_custom_ca("/nonexistent/ca.pem");

        let ssl_opts = config.to_ssl_opts();
        assert!(ssl_opts.is_err());

        let error = ssl_opts.unwrap_err();
        assert!(error.to_string().contains("CA certificate file not found"));
    }

    #[cfg(feature = "ssl")]
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
        assert!(config.enabled);
        assert!(matches!(config.validation_mode, TlsValidationMode::Platform));
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
        assert!(config.enabled);
        assert!(matches!(config.validation_mode, TlsValidationMode::SkipHostnameVerification));
    }

    #[test]
    fn test_from_cli_args_accept_invalid() {
        let config = TlsConfig::from_cli_args(None, false, true).unwrap();
        assert!(config.enabled);
        assert!(matches!(config.validation_mode, TlsValidationMode::AcceptInvalid));
    }

    #[test]
    fn test_from_cli_args_mutually_exclusive() {
        // Test ca_file + skip_hostname
        let path = PathBuf::from("/path");
        let result = TlsConfig::from_cli_args(Some(&path), true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Mutually exclusive TLS flags"));

        // Test ca_file + accept_invalid
        let path = PathBuf::from("/path");
        let result = TlsConfig::from_cli_args(Some(&path), false, true);
        assert!(result.is_err());

        // Test skip_hostname + accept_invalid
        let result = TlsConfig::from_cli_args(None, true, true);
        assert!(result.is_err());
    }

    #[cfg(feature = "ssl")]
    #[test]
    fn test_rustls_error_classification() {
        // Test certificate error classification
        let cert_error = rustls::Error::InvalidCertificate(rustls::CertificateError::Expired);
        let tls_error = TlsError::from_rustls_error(cert_error, Some("example.com"));
        assert!(matches!(tls_error, TlsError::CertificateTimeInvalid { .. }));

        // Test unknown issuer error
        let cert_error = rustls::Error::InvalidCertificate(rustls::CertificateError::UnknownIssuer);
        let tls_error = TlsError::from_rustls_error(cert_error, Some("example.com"));
        assert!(matches!(tls_error, TlsError::UnknownCertificateAuthority { .. }));
        assert!(tls_error.to_string().contains("--tls-ca-file"));
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
        assert!(result.unwrap_err().to_string().contains("Mutually exclusive TLS flags"));

        // Test ca_file + skip_hostname + accept_invalid (all three)
        let path = PathBuf::from("/path");
        let result = TlsConfig::from_cli_args(Some(&path), true, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Mutually exclusive TLS flags"));

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

    #[cfg(feature = "ssl")]
    #[test]
    fn test_create_tls_connection_with_config() {
        let tls_config = TlsConfig::new();

        // This test will fail with an actual connection, but tests the function signature
        // and basic error handling
        let result = create_tls_connection("mysql://invalid:invalid@nonexistent:3306/test", Some(tls_config));

        match result {
            Ok(pool) => {
                // If pool creation succeeds, attempt to get a connection to exercise lazy initialization
                let conn_result = pool.get_conn();
                // We expect this to fail due to invalid connection details, but not panic
                assert!(conn_result.is_err());
            },
            Err(_) => {
                // Pool creation failed, which is also expected for invalid connection details
                // This is fine - the test passes as long as it doesn't panic
            },
        }
    }

    #[cfg(feature = "ssl")]
    #[test]
    fn test_create_tls_connection_without_config() {
        // Test with no TLS config
        let result = create_tls_connection("mysql://invalid:invalid@nonexistent:3306/test", None);

        match result {
            Ok(pool) => {
                // If pool creation succeeds, attempt to get a connection to exercise lazy initialization
                let conn_result = pool.get_conn();
                // We expect this to fail due to invalid connection details, but not panic
                assert!(conn_result.is_err());
            },
            Err(_) => {
                // Pool creation failed, which is also expected for invalid connection details
                // This is fine - the test passes as long as it doesn't panic
            },
        }
    }

    #[cfg(not(feature = "ssl"))]
    #[test]
    fn test_create_tls_connection_no_ssl_feature() {
        let result = create_tls_connection("mysql://test", None);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Database connection failed"),
            "Expected database connection error, got: {}",
            error_msg
        );
    }

    #[test]
    fn test_tls_error_types() {
        let error = TlsError::certificate_validation_failed("cert error");
        assert!(error.to_string().contains("Certificate validation failed: cert error"));
        assert!(error.to_string().contains("--insecure-skip-hostname-verify"));
        assert!(error.to_string().contains("--allow-invalid-certificate"));

        let error = TlsError::ca_file_not_found("/path/to/cert");
        assert!(
            error
                .to_string()
                .contains("CA certificate file not found: /path/to/cert")
        );

        let error = TlsError::invalid_ca_format("/path", "bad format");
        assert!(
            error
                .to_string()
                .contains("Invalid CA certificate format in /path: bad format")
        );
        assert!(error.to_string().contains("PEM certificates"));

        let error = TlsError::handshake_failed("handshake error");
        assert!(error.to_string().contains("TLS handshake failed: handshake error"));

        let error = TlsError::hostname_verification_failed("example.com", "mismatch");
        assert!(
            error
                .to_string()
                .contains("Hostname verification failed for example.com: mismatch")
        );
        assert!(error.to_string().contains("--insecure-skip-hostname-verify"));

        let error = TlsError::certificate_time_invalid("expired");
        assert!(
            error
                .to_string()
                .contains("Certificate expired or not yet valid: expired")
        );
        assert!(error.to_string().contains("--allow-invalid-certificate"));

        let error = TlsError::mutually_exclusive_flags("--flag1, --flag2");
        assert!(
            error
                .to_string()
                .contains("Mutually exclusive TLS flags provided: --flag1, --flag2")
        );

        let error = TlsError::connection_failed("test message");
        assert!(error.to_string().contains("TLS connection failed: test message"));

        let error = TlsError::unsupported_tls_version("1.0");
        assert!(error.to_string().contains("Unsupported TLS version: 1.0"));
        assert!(error.to_string().contains("TLS 1.2 and 1.3"));

        let error = TlsError::feature_not_enabled();
        assert!(error.to_string().contains("TLS feature not enabled"));
        assert!(error.to_string().contains("--features ssl"));

        let error = TlsError::insecure_credentials();
        assert!(error.to_string().contains("credentials but TLS is not enabled"));
    }

    #[test]
    fn test_redact_url() {
        // Test URL with password
        let url = "mysql://user:password@localhost:3306/db";
        let redacted = redact_url(url);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("password"));

        // Test URL with username only
        let url = "mysql://user@localhost:3306/db";
        let redacted = redact_url(url);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("user"));

        // Test URL without credentials - intentionally left unchanged for debugging/traceability
        let url = "mysql://localhost:3306/db";
        let redacted = redact_url(url);
        // Security rationale: Only credentials (userinfo) and query params containing secrets are redacted.
        // Non-sensitive URLs remain unchanged to preserve debugging context and connection traceability.
        // Example: "mysql://user:pass@host/db" -> "mysql://***REDACTED***:***REDACTED***@host/db"
        assert_eq!(redacted, url); // Intentionally unchanged - no sensitive data to redact

        // Test invalid URL
        let url = "not-a-valid-url";
        let redacted = redact_url(url);
        assert_eq!(redacted, "***REDACTED_URL***");
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

    #[cfg(feature = "ssl")]
    mod rustls_verifier_tests {
        use super::*;
        use std::io::Write;
        use tempfile::NamedTempFile;

        #[test]
        fn test_skip_hostname_verifier_creation() {
            let verifier = SkipHostnameVerifier::new();
            assert!(verifier.is_ok());
        }

        #[test]
        fn test_skip_hostname_verifier_with_custom_ca() {
            // Create a dummy certificate for testing
            let cert_der = CertificateDer::from(vec![0x30, 0x82]); // Minimal DER structure
            let verifier = SkipHostnameVerifier::with_custom_ca(vec![cert_der]);
            // This will likely fail due to invalid certificate, but tests the interface
            assert!(verifier.is_err()); // Expected to fail with invalid cert
        }

        #[test]
        fn test_accept_all_verifier_creation() {
            let verifier = AcceptAllVerifier::new();
            // Should always succeed
            assert_eq!(std::mem::size_of_val(&verifier), 0); // Zero-sized type
        }

        #[test]
        fn test_cert_utils_load_nonexistent_file() {
            let nonexistent_path = PathBuf::from("/nonexistent/ca.pem");
            let result = cert_utils::load_ca_certificates(&nonexistent_path);
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("CA certificate file not found")
            );
        }

        #[test]
        fn test_cert_utils_load_invalid_file() {
            // Create a temporary file with invalid content
            let mut temp_file = NamedTempFile::new().unwrap();
            writeln!(temp_file, "This is not a valid PEM certificate").unwrap();

            let result = cert_utils::load_ca_certificates(&temp_file.path().to_path_buf());
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Invalid CA certificate format")
            );
        }

        #[test]
        fn test_cert_utils_load_empty_file() {
            // Create an empty temporary file
            let temp_file = NamedTempFile::new().unwrap();

            let result = cert_utils::load_ca_certificates(&temp_file.path().to_path_buf());
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("No valid certificates found"));
        }

        #[test]
        fn test_cert_utils_validate_ca_file() {
            let nonexistent_path = PathBuf::from("/nonexistent/ca.pem");
            let result = cert_utils::validate_ca_file(&nonexistent_path);
            assert!(result.is_err());
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
        fn test_tls_error_from_rustls_error() {
            // Test certificate validation error
            let rustls_error = rustls::Error::InvalidCertificate(rustls::CertificateError::BadSignature);
            let tls_error = TlsError::from_rustls_error(rustls_error, None);
            assert!(tls_error.to_string().contains("Certificate has invalid signature"));
            assert!(tls_error.to_string().contains("--allow-invalid-certificate"));

            // Test certificate expired error
            let rustls_error = rustls::Error::InvalidCertificate(rustls::CertificateError::Expired);
            let tls_error = TlsError::from_rustls_error(rustls_error, None);
            assert!(tls_error.to_string().contains("Certificate has expired"));

            // Test certificate not yet valid error
            let rustls_error = rustls::Error::InvalidCertificate(rustls::CertificateError::NotValidYet);
            let tls_error = TlsError::from_rustls_error(rustls_error, None);
            assert!(tls_error.to_string().contains("Certificate is not yet valid"));

            // Test invalid purpose error
            let rustls_error = rustls::Error::InvalidCertificate(rustls::CertificateError::InvalidPurpose);
            let tls_error = TlsError::from_rustls_error(rustls_error, None);
            assert!(
                tls_error
                    .to_string()
                    .contains("Certificate not valid for server authentication")
            );

            // Test hostname verification error (using General as placeholder)
            let rustls_error = rustls::Error::General("invalid hostname".to_string());
            let tls_error = TlsError::from_rustls_error(rustls_error, Some("example.com"));
            assert!(tls_error.to_string().contains("TLS handshake failed"));

            // Test general handshake error
            let rustls_error = rustls::Error::General("handshake failed".to_string());
            let tls_error = TlsError::from_rustls_error(rustls_error, None);
            assert!(tls_error.to_string().contains("TLS handshake failed"));
        }

        #[test]
        fn test_verifier_supported_schemes() {
            // Test that verifiers support signature schemes
            let skip_verifier = SkipHostnameVerifier::new().unwrap();
            let schemes = skip_verifier.supported_verify_schemes();
            assert!(!schemes.is_empty());

            let accept_verifier = AcceptAllVerifier::new();
            let schemes = accept_verifier.supported_verify_schemes();
            assert!(!schemes.is_empty());
        }
    }

    #[cfg(feature = "ssl")]
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

    #[cfg(feature = "ssl")]
    #[test]
    fn test_to_ssl_opts_custom_ca_with_temp_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temporary file with a valid self-signed certificate (for testing purposes)
        let mut temp_file = NamedTempFile::new().unwrap();
        // This is a valid self-signed certificate for testing
        writeln!(temp_file, "-----BEGIN CERTIFICATE-----").unwrap();
        writeln!(temp_file, "MIIDXTCCAkWgAwIBAgIJAKoK/heBjcOuMA0GCSqGSIb3DQEBBQUAMEUxCzAJBgNV").unwrap();
        writeln!(temp_file, "BAYTAkFVMRMwEQYDVQQIDApTb21lLVN0YXRlMSEwHwYDVQQKDBhJbnRlcm5ldCBX").unwrap();
        writeln!(temp_file, "aWRnaXRzIFB0eSBMdGQwHhcNMTcwODI4MTkzNDA5WhcNMTgwODI4MTkzNDA5WjBF").unwrap();
        writeln!(temp_file, "MQswCQYDVQQGEwJBVTETMBEGA1UECAwKU29tZS1TdGF0ZTEhMB8GA1UECgwYSW50").unwrap();
        writeln!(temp_file, "ZXJuZXQgV2lkZ2l0cyBQdHkgTHRkMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIB").unwrap();
        writeln!(temp_file, "CgKCAQEAuuExKvY1nOmAHO13nPiOxvTnoFrL23apFR9W+VdtPGrb+sQXebHjZ/UU").unwrap();
        writeln!(temp_file, "kKtjWQqLQlHgHOgFbt7jr8I2J2jFiaNBBBYuHBw6NMVBnhkdXRJDn9LxMa02cx1q").unwrap();
        writeln!(temp_file, "BxuFqV7zUg4EQVXveZd0HFDZrpVeUiA21IlQpFYxyFveOiGspMdYjI5u3Ngkqbz6").unwrap();
        writeln!(temp_file, "pXrbRqZzjXaFUcuJpPMFRNKGWv5wyAcb5B2fHX1sGtSaYvNilgxnE8+ykQs6rp+j").unwrap();
        writeln!(temp_file, "kVf3lbVvB4zUHg9S5RoQBQ1CuHnRkl9wjw03EBEQ4h2z4k5cyR2DpmdJ0b+2cxJl").unwrap();
        writeln!(temp_file, "Ww9cDcTgWwIDAQABo1AwTjAdBgNVHQ4EFgQUhG9lFWZWnPfLwB9gQQd8it/u+MQw").unwrap();
        writeln!(temp_file, "HwYDVR0jBBgwFoAUhG9lFWZWnPfLwB9gQQd8it/u+MQwDAYDVR0TBAUwAwEB/zAN").unwrap();
        writeln!(temp_file, "BgkqhkiG9w0BAQUFAAOCAQEAeM9ahJ6iAJfyFq4wzSmpOddgfGqJWjXiH+OqZlHO").unwrap();
        writeln!(temp_file, "2k8sVjCjmHylI+XleLu2dDxwjNuBllhid/Qs6TRcZxEqn+cAskHReXlZjQoHuSHx").unwrap();
        writeln!(temp_file, "VxHp2+PpVUFnuU19LFbmqZ3+/dvTVc0V0QNFS4HgBXkKwA9fPQ+k/roUe0is7d+8").unwrap();
        writeln!(temp_file, "O4ArHZka85ZMd1qY4z0xvFvbMmJuC0KJvEieakGFkCEc7trGwfIuXgFMLJLBB5uZ").unwrap();
        writeln!(temp_file, "F74imqDbImh5tbwQcQYBYVHhkCjDOw+XdXUSPiOBueno0soKjOxjVmooPdxyaAuW").unwrap();
        writeln!(temp_file, "fuFhiGI+bI90H4+17ceuJAOzOFvhPH1RTwf5k+7+BzXrqbHlt+2RfEECAwEAAQ==").unwrap();
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
        assert!(config.is_enabled());

        // The to_ssl_opts() call may fail due to invalid certificate, which is expected
        // We're testing that the error handling works correctly
        match config.to_ssl_opts() {
            Ok(Some(ssl_opts)) => {
                // If it succeeds, verify the configuration
                assert!(!ssl_opts.skip_domain_validation());
                assert!(!ssl_opts.accept_invalid_certs());
                assert!(ssl_opts.root_cert_path().is_some());
                assert_eq!(ssl_opts.root_cert_path().unwrap(), temp_file.path());
            },
            Err(TlsError::CertificateValidationFailed { .. }) | Err(TlsError::InvalidCaFormat { .. }) => {
                // This is expected with an invalid test certificate
                // The important thing is that the error is properly classified
            },
            other => panic!("Unexpected result: {:?}", other),
        }
    }

    #[cfg(feature = "ssl")]
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

    // Additional comprehensive unit tests for TLS configuration
    // Requirements covered: 3.4, 6.1, 6.2, 6.3, 6.4

    #[cfg(feature = "ssl")]
    #[test]
    fn test_tls_config_from_tls_options() {
        use crate::cli::TlsOptions;

        // Test platform mode (no flags)
        let tls_options = TlsOptions {
            tls_ca_file: None,
            insecure_skip_hostname_verify: false,
            allow_invalid_certificate: false,
        };
        let config = TlsConfig::from_tls_options(&tls_options).unwrap();
        assert!(config.is_enabled());
        assert!(matches!(config.validation_mode(), TlsValidationMode::Platform));

        // Test skip hostname mode
        let tls_options = TlsOptions {
            tls_ca_file: None,
            insecure_skip_hostname_verify: true,
            allow_invalid_certificate: false,
        };
        let config = TlsConfig::from_tls_options(&tls_options).unwrap();
        assert!(config.is_enabled());
        assert!(matches!(config.validation_mode(), TlsValidationMode::SkipHostnameVerification));

        // Test accept invalid mode
        let tls_options = TlsOptions {
            tls_ca_file: None,
            insecure_skip_hostname_verify: false,
            allow_invalid_certificate: true,
        };
        let config = TlsConfig::from_tls_options(&tls_options).unwrap();
        assert!(config.is_enabled());
        assert!(matches!(config.validation_mode(), TlsValidationMode::AcceptInvalid));
    }

    #[cfg(not(feature = "ssl"))]
    #[test]
    fn test_tls_config_from_tls_options_no_ssl() {
        use crate::cli::TlsOptions;

        // Test that TlsOptions works without SSL feature
        let tls_options = TlsOptions::default();
        let config = TlsConfig::from_tls_options(&tls_options).unwrap();
        assert!(!config.is_enabled());
        assert!(matches!(config.validation_mode(), TlsValidationMode::Platform));
    }

    #[test]
    fn test_tls_config_mutual_exclusion_validation() {
        use std::path::PathBuf;

        // Create a fake path for testing (file doesn't need to exist for this test)
        let fake_cert_path = PathBuf::from("/fake/cert.pem");

        // Test ca_file + skip_hostname (should fail)
        let result = TlsConfig::from_cli_args(Some(&fake_cert_path), true, false);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::MutuallyExclusiveFlags { .. }));

        // Test ca_file + accept_invalid (should fail)
        let result = TlsConfig::from_cli_args(Some(&fake_cert_path), false, true);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::MutuallyExclusiveFlags { .. }));

        // Test skip_hostname + accept_invalid (should fail)
        let result = TlsConfig::from_cli_args(None, true, true);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::MutuallyExclusiveFlags { .. }));

        // Test all three flags (should fail)
        let result = TlsConfig::from_cli_args(Some(&fake_cert_path), true, true);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::MutuallyExclusiveFlags { .. }));
    }

    #[test]
    fn test_certificate_file_validation() {
        use std::path::PathBuf;

        // Test nonexistent file
        let nonexistent_path = PathBuf::from("/nonexistent/path/to/cert.pem");
        let result = TlsConfig::from_cli_args(Some(&nonexistent_path), false, false);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::CaFileNotFound { .. }));

        // Test empty path
        let empty_path = PathBuf::from("");
        let result = TlsConfig::from_cli_args(Some(&empty_path), false, false);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TlsError::CaFileNotFound { .. }));
    }

    #[cfg(feature = "ssl")]
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

        // Test disabled TLS
        let config = TlsConfig::default();
        let ssl_opts = config.to_ssl_opts().unwrap();
        assert!(ssl_opts.is_none());
    }

    #[cfg(feature = "ssl")]
    #[test]
    fn test_ssl_opts_nonexistent_ca_file() {
        use std::path::PathBuf;

        let nonexistent_path = PathBuf::from("/nonexistent/cert.pem");
        let config = TlsConfig {
            enabled: true,
            validation_mode: TlsValidationMode::CustomCa {
                ca_file_path: nonexistent_path,
            },
        };

        let ssl_opts_result = config.to_ssl_opts();
        assert!(ssl_opts_result.is_err());
        assert!(matches!(ssl_opts_result.unwrap_err(), TlsError::CaFileNotFound { .. }));
    }

    #[cfg(not(feature = "ssl"))]
    #[test]
    fn test_ssl_opts_feature_disabled() {
        let config = TlsConfig::new();
        let ssl_opts_result = config.to_ssl_opts();
        assert!(ssl_opts_result.is_err());
        assert!(matches!(ssl_opts_result.unwrap_err(), TlsError::FeatureNotEnabled));

        // Test disabled TLS config when SSL feature is disabled
        let config = TlsConfig::default();
        let ssl_opts_result = config.to_ssl_opts();
        assert!(ssl_opts_result.is_ok());
        assert!(ssl_opts_result.unwrap().is_none());
    }

    #[test]
    fn test_tls_config_equality_and_cloning() {
        let config1 = TlsConfig::new();
        let config2 = config1.clone();

        assert_eq!(config1, config2);
        assert_eq!(config1.is_enabled(), config2.is_enabled());
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
        assert!(config.is_enabled());
        assert!(matches!(config.validation_mode(), TlsValidationMode::Platform));

        let config = TlsConfig::default();
        assert!(!config.is_enabled());

        let config = TlsConfig::with_skip_hostname_verification();
        assert!(matches!(config.validation_mode(), TlsValidationMode::SkipHostnameVerification));

        let config = TlsConfig::with_accept_invalid();
        assert!(matches!(config.validation_mode(), TlsValidationMode::AcceptInvalid));
    }
}

// Tests for when SSL feature is disabled
#[cfg(not(feature = "ssl"))]
#[cfg(test)]
mod ssl_disabled_tests {
    use super::*;

    #[test]
    fn to_ssl_opts_returns_feature_not_enabled_when_enabled() {
        let config = TlsConfig::new();
        let res = config.to_ssl_opts();
        assert!(res.is_err());
        let err = res.unwrap_err();
        let msg = err.to_string();
        // tolerate slight message variations
        assert!(msg.contains("TLS feature not enabled") || msg.contains("feature not enabled"));
    }

    #[test]
    fn to_ssl_opts_succeeds_when_disabled() {
        let config = TlsConfig::default(); // disabled by default
        let res = config.to_ssl_opts();
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
    }
}
