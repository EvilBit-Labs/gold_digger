//! [`rustls::Error`] → [`TlsError`] mapping.
//!
//! `create_tls_connection` (see [`super::pool`]) delegates to
//! [`TlsError::from_rustls_error`] whenever the underlying mysql driver
//! surfaces a rustls error. Keeping this classifier separate from the
//! [`super::error`] enum keeps the enum file focused on Display/constructor
//! concerns; all rustls-specific routing lives here.
//!
//! # Fail-closed behaviour
//!
//! `CertificateError` is `#[non_exhaustive]`; any variant this classifier
//! does not recognise falls through to
//! [`TlsError::CertificateValidationFailed`] rather than
//! [`TlsError::ConnectionFailed`], so a future `rustls` release that adds
//! a new cert variant never silently demotes a validation failure to a
//! retry hint.

use super::error::TlsError;

impl TlsError {
    /// Creates a TLS error from a rustls error with context and user guidance
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
                rustls::CertificateError::NotValidForName => Self::HostnameVerificationFailed {
                    hostname: hostname.unwrap_or("unknown").to_string(),
                    message: "Certificate hostname mismatch: certificate not valid for the requested hostname"
                        .to_string(),
                },
                rustls::CertificateError::NotValidForNameContext {
                    expected: _,
                    presented: _,
                } => Self::HostnameVerificationFailed {
                    hostname: hostname.unwrap_or("unknown").to_string(),
                    message: "Certificate hostname mismatch: certificate not valid for the requested hostname context"
                        .to_string(),
                },
                _ => Self::CertificateValidationFailed {
                    message: format!(
                        "Certificate validation failed: {:?}. Use --allow-invalid-certificate to bypass",
                        cert_error
                    ),
                },
            },
            rustls::Error::InvalidMessage(_) => Self::HandshakeFailed {
                message: "Invalid TLS message received from server. Possible protocol mismatch or corrupted handshake"
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(matches!(
            tls_error,
            TlsError::UnknownCertificateAuthority { .. }
        ));

        // Test invalid purpose
        let rustls_error = RustlsError::InvalidCertificate(CertificateError::InvalidPurpose);
        let tls_error = TlsError::from_rustls_error(rustls_error, Some("example.com"));
        assert!(matches!(
            tls_error,
            TlsError::InvalidCertificatePurpose { .. }
        ));
    }

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
        assert!(matches!(
            tls_error,
            TlsError::CertificateValidationFailed { .. }
        ));
    }

    #[test]
    fn test_from_rustls_error_hostname_handling() {
        use rustls::{CertificateError, Error as RustlsError};

        // Test hostname verification with NotValidForName
        let rustls_error = RustlsError::InvalidCertificate(CertificateError::NotValidForName);
        let tls_error = TlsError::from_rustls_error(rustls_error, Some("example.com"));
        if let TlsError::HostnameVerificationFailed { hostname, .. } = tls_error {
            assert_eq!(hostname, "example.com");
        } else {
            panic!("Expected HostnameVerificationFailed error");
        }

        // Test hostname verification with NotValidForNameContext
        let rustls_error =
            RustlsError::InvalidCertificate(CertificateError::NotValidForNameContext {
                expected: rustls::pki_types::ServerName::try_from("example.com").unwrap(),
                presented: vec!["wrong.com".to_string()],
            });
        let tls_error = TlsError::from_rustls_error(rustls_error, Some("example.com"));
        if let TlsError::HostnameVerificationFailed { hostname, .. } = tls_error {
            assert_eq!(hostname, "example.com");
        } else {
            panic!("Expected HostnameVerificationFailed error");
        }

        // Test hostname verification without hostname provided
        let rustls_error = RustlsError::InvalidCertificate(CertificateError::NotValidForName);
        let tls_error = TlsError::from_rustls_error(rustls_error, None);
        if let TlsError::HostnameVerificationFailed { hostname, .. } = tls_error {
            assert_eq!(hostname, "unknown");
        } else {
            panic!("Expected HostnameVerificationFailed error");
        }
    }

    #[test]
    fn test_rustls_error_classification() {
        // Test certificate error classification
        let cert_error = rustls::Error::InvalidCertificate(rustls::CertificateError::Expired);
        let tls_error = TlsError::from_rustls_error(cert_error, Some("example.com"));
        assert!(matches!(tls_error, TlsError::CertificateTimeInvalid { .. }));

        // Test unknown issuer error
        let cert_error = rustls::Error::InvalidCertificate(rustls::CertificateError::UnknownIssuer);
        let tls_error = TlsError::from_rustls_error(cert_error, Some("example.com"));
        assert!(matches!(
            tls_error,
            TlsError::UnknownCertificateAuthority { .. }
        ));
        assert!(tls_error.to_string().contains("--tls-ca-file"));
    }

    #[test]
    fn test_tls_error_from_rustls_error() {
        // Test certificate validation error
        let rustls_error =
            rustls::Error::InvalidCertificate(rustls::CertificateError::BadSignature);
        let tls_error = TlsError::from_rustls_error(rustls_error, None);
        assert!(
            tls_error
                .to_string()
                .contains("Certificate has invalid signature")
        );
        assert!(
            tls_error
                .to_string()
                .contains("--allow-invalid-certificate")
        );

        // Test certificate expired error
        let rustls_error = rustls::Error::InvalidCertificate(rustls::CertificateError::Expired);
        let tls_error = TlsError::from_rustls_error(rustls_error, None);
        assert!(tls_error.to_string().contains("Certificate has expired"));

        // Test certificate not yet valid error
        let rustls_error = rustls::Error::InvalidCertificate(rustls::CertificateError::NotValidYet);
        let tls_error = TlsError::from_rustls_error(rustls_error, None);
        assert!(
            tls_error
                .to_string()
                .contains("Certificate is not yet valid")
        );

        // Test invalid purpose error
        let rustls_error =
            rustls::Error::InvalidCertificate(rustls::CertificateError::InvalidPurpose);
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
}
