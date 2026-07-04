//! [`TlsError`] enum — the typed error surface for the TLS layer.
//!
//! Every variant carries actionable guidance in its `Display` message so
//! `main.rs` can surface the error directly to the user without additional
//! formatting. Classifiers live alongside the enum (`suggest_cli_flag`,
//! `is_certificate_error`, etc.); the [`rustls::Error`] → [`TlsError`]
//! mapping lives in the sibling [`super::classifier`] module.

use thiserror::Error;

/// TLS-specific error types for better error handling and user guidance
#[derive(Error, Debug)]
pub enum TlsError {
    #[error(
        "Certificate validation failed: {message}. Try --insecure-skip-hostname-verify for hostname issues or --allow-invalid-certificate for testing"
    )]
    CertificateValidationFailed { message: String },

    #[error("CA certificate file not found: {path}. Ensure the file exists and is readable")]
    CaFileNotFound { path: String },

    #[error(
        "Invalid CA certificate format in {path}: {message}. Ensure the file contains valid PEM certificates"
    )]
    InvalidCaFormat { path: String, message: String },

    #[error("TLS handshake failed: {message}. Check server TLS configuration")]
    HandshakeFailed { message: String },

    #[error(
        "Hostname verification failed for {hostname}: {message}. Use --insecure-skip-hostname-verify to bypass"
    )]
    HostnameVerificationFailed { hostname: String, message: String },

    #[error(
        "Certificate expired or not yet valid: {message}. Use --allow-invalid-certificate to bypass"
    )]
    CertificateTimeInvalid { message: String },

    #[error("Mutually exclusive TLS flags provided: {flags}. Use only one TLS security option")]
    MutuallyExclusiveFlags { flags: String },

    #[error("TLS connection failed: {message}")]
    ConnectionFailed { message: String },

    #[error("Unsupported TLS version: {version}. Only TLS 1.2 and 1.3 are supported")]
    UnsupportedTlsVersion { version: String },

    #[error(
        "Database URL contains credentials but TLS is not enabled. Use TLS to protect credentials in transit"
    )]
    InsecureCredentials,

    #[error(
        "Certificate has invalid signature: {message}. Use --allow-invalid-certificate to bypass for testing"
    )]
    InvalidSignature { message: String },

    #[error(
        "Certificate issued by unknown CA: {message}. Use --tls-ca-file <path> to specify custom CA or --allow-invalid-certificate for testing"
    )]
    UnknownCertificateAuthority { message: String },

    #[error(
        "Certificate not valid for server authentication: {message}. Use --allow-invalid-certificate to bypass"
    )]
    InvalidCertificatePurpose { message: String },

    #[error(
        "Certificate chain validation failed: {message}. Use --allow-invalid-certificate to bypass"
    )]
    CertificateChainInvalid { message: String },

    #[error(
        "Server certificate revoked: {message}. Use --allow-invalid-certificate to bypass (not recommended)"
    )]
    CertificateRevoked { message: String },

    #[error("TLS protocol version mismatch: {message}. Server may not support TLS 1.2/1.3")]
    ProtocolVersionMismatch { message: String },

    #[error(
        "TLS cipher suite negotiation failed: {message}. Server and client have no compatible cipher suites"
    )]
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
        Self::MutuallyExclusiveFlags {
            flags: flags.into(),
        }
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
        Self::ServerAlert {
            alert: alert.into(),
        }
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
            Self::UnknownCertificateAuthority { .. } => {
                Some("--tls-ca-file <path> or --allow-invalid-certificate")
            }
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
                | Self::MutuallyExclusiveFlags { .. },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_error_suggest_cli_flag() {
        // Test hostname verification error
        let error = TlsError::hostname_verification_failed("example.com", "hostname mismatch");
        assert_eq!(
            error.suggest_cli_flag(),
            Some("--insecure-skip-hostname-verify")
        );

        // Test certificate time invalid error
        let error = TlsError::certificate_time_invalid("certificate expired");
        assert_eq!(
            error.suggest_cli_flag(),
            Some("--allow-invalid-certificate")
        );

        // Test invalid signature error
        let error = TlsError::invalid_signature("bad signature");
        assert_eq!(
            error.suggest_cli_flag(),
            Some("--allow-invalid-certificate")
        );

        // Test unknown CA error
        let error = TlsError::unknown_certificate_authority("unknown issuer");
        assert_eq!(
            error.suggest_cli_flag(),
            Some("--tls-ca-file <path> or --allow-invalid-certificate")
        );

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

    #[test]
    fn test_tls_error_constructor_methods() {
        // Test all constructor methods create the correct error variants
        let error = TlsError::invalid_signature("test message");
        assert!(matches!(error, TlsError::InvalidSignature { .. }));

        let error = TlsError::unknown_certificate_authority("test message");
        assert!(matches!(
            error,
            TlsError::UnknownCertificateAuthority { .. }
        ));

        let error = TlsError::invalid_certificate_purpose("test message");
        assert!(matches!(error, TlsError::InvalidCertificatePurpose { .. }));

        let error = TlsError::certificate_chain_invalid("test message");
        assert!(matches!(error, TlsError::CertificateChainInvalid { .. }));

        let error = TlsError::certificate_revoked("test message");
        assert!(matches!(error, TlsError::CertificateRevoked { .. }));

        let error = TlsError::protocol_version_mismatch("test message");
        assert!(matches!(error, TlsError::ProtocolVersionMismatch { .. }));

        let error = TlsError::cipher_suite_negotiation_failed("test message");
        assert!(matches!(
            error,
            TlsError::CipherSuiteNegotiationFailed { .. }
        ));

        let error = TlsError::server_alert("test alert");
        assert!(matches!(error, TlsError::ServerAlert { .. }));

        let error = TlsError::peer_misbehaved("test message");
        assert!(matches!(error, TlsError::PeerMisbehaved { .. }));
    }

    #[test]
    fn test_tls_error_types() {
        let error = TlsError::certificate_validation_failed("cert error");
        assert!(
            error
                .to_string()
                .contains("Certificate validation failed: cert error")
        );
        assert!(
            error
                .to_string()
                .contains("--insecure-skip-hostname-verify")
        );
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
        assert!(
            error
                .to_string()
                .contains("TLS handshake failed: handshake error")
        );

        let error = TlsError::hostname_verification_failed("example.com", "mismatch");
        assert!(
            error
                .to_string()
                .contains("Hostname verification failed for example.com: mismatch")
        );
        assert!(
            error
                .to_string()
                .contains("--insecure-skip-hostname-verify")
        );

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
        assert!(
            error
                .to_string()
                .contains("TLS connection failed: test message")
        );

        let error = TlsError::unsupported_tls_version("1.0");
        assert!(error.to_string().contains("Unsupported TLS version: 1.0"));
        assert!(error.to_string().contains("TLS 1.2 and 1.3"));

        let error = TlsError::insecure_credentials();
        assert!(
            error
                .to_string()
                .contains("credentials but TLS is not enabled")
        );
    }
}
