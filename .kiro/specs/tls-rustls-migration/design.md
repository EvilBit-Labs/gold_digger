# Design Document

## Overview

This design outlines the migration from Gold Digger's current feature-gated rustls TLS implementation to an always-available rustls-only approach. The current implementation already uses rustls exclusively with enhanced certificate validation controls and three CLI flags for different security modes.

The key architectural change is removing the `ssl` feature flag and making rustls TLS support always available, eliminating the need for conditional compilation while maintaining all existing TLS functionality.

## Architecture

### Current TLS Architecture

```mermaid
graph TD
    A[CLI Input] --> B{Feature Check}
    B -->|ssl feature enabled| C[rustls-tls]
    B -->|ssl feature disabled| D[No TLS Support]
    C --> E[Pure Rust TLS]
    E --> F[MySQL Connection]
    D --> G[Plain Connection Only]
```

### New TLS Architecture (Always Available)

```mermaid
graph TD
    A[CLI Input] --> B{TLS Flags}
    B -->|--tls-ca-file| C[Custom CA Trust]
    B -->|--insecure-skip-hostname-verify| D[Skip Hostname Validation]
    B -->|--allow-invalid-certificate| E[Skip All Validation]
    B -->|No flags| F[Platform Certificate Store]

    C --> G[rustls + Custom CA]
    D --> H[rustls + Skip Hostname]
    E --> I[rustls + Accept Invalid]
    F --> J[rustls + Native Certs]

    G --> K[MySQL Connection]
    H --> K
    I --> K
    J --> K
```

### TLS Configuration Flow

```mermaid
sequenceDiagram
    participant CLI as CLI Parser
    participant TLS as TLS Config
    participant Rustls as Rustls Builder
    participant MySQL as MySQL Pool

    CLI->>TLS: Parse TLS flags
    TLS->>TLS: Validate mutual exclusion
    TLS->>Rustls: Configure certificate validation

    alt Custom CA File
        Rustls->>Rustls: Load custom CA certificates
        Rustls->>Rustls: Enable hostname + time validation
    else Skip Hostname Verify
        Rustls->>Rustls: Use platform CA store
        Rustls->>Rustls: Disable hostname validation
    else Allow Invalid Certificates
        Rustls->>Rustls: Disable all validation
    else Default (Platform Trust)
        Rustls->>Rustls: Use platform CA store
        Rustls->>Rustls: Enable all validation
    end

    Rustls->>MySQL: Create TLS connector
    MySQL->>MySQL: Establish connection
```

## Components and Interfaces

### CLI Interface Changes

#### Current TLS Flags Implementation (Already Implemented)

The TLS flags are already implemented and working correctly. The current implementation uses feature-gated compilation:

```rust
/// TLS configuration options (mutually exclusive)
#[cfg_attr(not(feature = "ssl"), derive(Args, Debug, Clone, Default))]
#[cfg_attr(feature = "ssl", derive(Args, Debug, Clone))]
#[group(id = "tls_mode", multiple = false)]
pub struct TlsOptions {
    /// Path to CA certificate file for trust anchor pinning
    #[cfg(feature = "ssl")]
    #[arg(long, group = "tls_mode")]
    pub tls_ca_file: Option<PathBuf>,

    /// Skip hostname verification (keeps chain and time validation)
    #[cfg(feature = "ssl")]
    #[arg(long, group = "tls_mode")]
    pub insecure_skip_hostname_verify: bool,

    /// Disable certificate validation entirely (DANGEROUS)
    #[cfg(feature = "ssl")]
    #[arg(long, group = "tls_mode")]
    pub allow_invalid_certificate: bool,
}
```

#### Required Changes for Always-Available TLS

Remove the `#[cfg(feature = "ssl")]` attributes to make TLS flags always available:

```rust
/// TLS configuration options (mutually exclusive)
#[derive(Args, Debug, Clone)]
#[group(id = "tls_mode", multiple = false)]
pub struct TlsOptions {
    /// Path to CA certificate file for trust anchor pinning
    #[arg(long, group = "tls_mode")]
    pub tls_ca_file: Option<PathBuf>,

    /// Skip hostname verification (keeps chain and time validation)
    #[arg(long, group = "tls_mode")]
    pub insecure_skip_hostname_verify: bool,

    /// Disable certificate validation entirely (DANGEROUS)
    #[arg(long, group = "tls_mode")]
    pub allow_invalid_certificate: bool,
}
```

### TLS Configuration Module

#### Current TlsConfig Implementation (Already Implemented)

The TLS configuration is already well-implemented in `src/tls.rs`. The current structure includes:

```rust
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

/// TLS configuration for MySQL connections
#[derive(Debug, Clone, PartialEq)]
pub struct TlsConfig {
    /// Whether TLS is enabled
    pub enabled: bool,
    /// TLS validation mode
    pub validation_mode: TlsValidationMode,
}
```

#### Required Changes for Always-Available TLS

Remove the `enabled` field since TLS will always be available:

```rust
/// TLS configuration for MySQL connections
#[derive(Debug, Clone, PartialEq)]
pub struct TlsConfig {
    /// TLS validation mode
    pub validation_mode: TlsValidationMode,
}
```

The `from_tls_options` method already exists and works correctly, but needs to be updated to remove feature gating.

#### Current Rustls Integration (Already Implemented)

The rustls integration is already fully implemented in `src/tls.rs` with custom certificate verifiers and comprehensive error handling. The current implementation includes:

- Platform certificate store integration via `rustls-native-certs`
- Custom CA file loading with PEM parsing
- Custom certificate verifiers for hostname skipping and accepting invalid certificates
- Comprehensive error classification and user guidance
- Security warning system

The `to_ssl_opts()` method is already implemented and working correctly. The main change needed is removing the feature gating from the function calls.

### Custom Certificate Verifiers

#### Hostname Skip Verifier

```rust
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use std::sync::Arc;
use webpki::{EndEntityCert, Time, TrustAnchor};

struct SkipHostnameVerifier {
    roots: Vec<TrustAnchor<'static>>,
}

impl SkipHostnameVerifier {
    fn new() -> Result<Self, rustls::Error> {
        // Load system root certificates
        let mut roots = Vec::new();
        for cert in rustls_native_certs::load_native_certs()? {
            if let Ok(ta) = TrustAnchor::from_der(&cert.0) {
                roots.push(ta);
            }
        }

        Ok(Self { roots })
    }
}

impl ServerCertVerifier for SkipHostnameVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>, // Ignore server name for hostname verification
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // Convert rustls time to webpki time
        let time = Time::from_secs_since_unix_epoch(now.as_secs());

        // Parse end entity certificate
        let cert = EndEntityCert::try_from(end_entity.as_ref())
            .map_err(|e| rustls::Error::InvalidCertificate(rustls::CertificateError::BadEncoding))?;

        // Build certificate chain
        let mut chain = Vec::new();
        for intermediate in intermediates {
            let intermediate_cert = webpki::Cert::from_der(intermediate.as_ref())
                .map_err(|e| rustls::Error::InvalidCertificate(rustls::CertificateError::BadEncoding))?;
            chain.push(intermediate_cert);
        }

        // Verify certificate chain and validity period using webpki
        // This performs signature verification, chain validation, and time validation
        // but explicitly skips DNS-ID/name validation
        cert.verify_for_usage(
            &[&self.roots],
            &chain,
            time,
            webpki::KeyUsage::server_auth(),
            None, // No DNS names to verify against
            None, // No IP addresses to verify against
        )
        .map_err(|e| rustls::Error::InvalidCertificate(rustls::CertificateError::BadEncoding))?;

        // Handle OCSP response if present
        if !ocsp_response.is_empty() {
            // OCSP validation would go here if needed
            // For now, we just accept the response as-is
        }

        Ok(ServerCertVerified::assertion())
    }
}
```

#### Accept All Verifier

```rust
struct AcceptAllVerifier;

impl AcceptAllVerifier {
    fn new() -> Self {
        Self
    }
}

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
}
```

## Data Models

### Feature Flag Changes

#### Cargo.toml Updates

```toml
[features]
default = ["json", "csv", "additional_mysql_types"]
json = []
csv = []
additional_mysql_types = [
  "mysql_common",
  "mysql_common?/bigdecimal",
  "mysql_common?/rust_decimal",
  "mysql_common?/time",
  "mysql_common?/frunk",
]

# Remove ssl and ssl-rustls features (TLS always available)
```

#### Dependency Updates

```toml
[dependencies]
mysql = { version = "26.0.1", features = [
  "rustls-tls",
], default-features = false }
rustls-native-certs = "0.7"
# Remove native-tls related dependencies
# TLS dependencies are now always included
```

### Error Handling Updates

#### Enhanced TLS Error Types

```rust
#[derive(Error, Debug)]
pub enum TlsError {
    #[error("Certificate validation failed: {message}. Try --insecure-skip-hostname-verify for hostname issues or --allow-invalid-certificate for testing")]
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
}
```

## Error Handling

### TLS Error Classification and User Guidance

#### Error Detection and Suggestion Logic

```rust
impl TlsError {
    pub fn from_rustls_error(error: rustls::Error, hostname: Option<&str>) -> Self {
        match error {
            rustls::Error::InvalidCertificate(cert_error) => match cert_error {
                rustls::CertificateError::BadSignature => Self::CertificateValidationFailed {
                    message: "Certificate has invalid signature. Use --allow-invalid-certificate for testing"
                        .to_string(),
                },
                rustls::CertificateError::CertExpired => Self::CertificateTimeInvalid {
                    message: "Certificate has expired".to_string(),
                },
                rustls::CertificateError::CertNotYetValid => Self::CertificateTimeInvalid {
                    message: "Certificate is not yet valid".to_string(),
                },
                rustls::CertificateError::InvalidPurpose => Self::CertificateValidationFailed {
                    message: "Certificate not valid for server authentication".to_string(),
                },
                _ => Self::CertificateValidationFailed {
                    message: format!("Certificate validation failed: {:?}", cert_error),
                },
            },
            rustls::Error::InvalidDnsName(_) => Self::HostnameVerificationFailed {
                hostname: hostname.unwrap_or("unknown").to_string(),
                message: "Hostname does not match certificate".to_string(),
            },
            _ => Self::HandshakeFailed {
                message: error.to_string(),
            },
        }
    }
}
```

### Warning System

#### Security Warning Display

```rust
pub fn display_security_warnings(tls_config: &TlsConfig) {
    match &tls_config.validation_mode {
        TlsValidationMode::SkipHostnameVerification => {
            eprintln!(
                "WARNING: Hostname verification disabled. Connection is vulnerable to man-in-the-middle attacks."
            );
        },
        TlsValidationMode::AcceptInvalid => {
            eprintln!("WARNING: Certificate validation disabled. Connection is NOT secure.");
            eprintln!("This should ONLY be used for testing. Never use in production.");
        },
        TlsValidationMode::CustomCa { ca_file_path } => {
            if cli.verbose {
                eprintln!("Using custom CA file: {}", ca_file_path.display());
            }
        },
        TlsValidationMode::Platform => {
            if cli.verbose {
                eprintln!("Using platform certificate store for TLS validation");
            }
        },
    }
}
```

## Testing Strategy

### Unit Tests

#### TLS Configuration Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_mutually_exclusive_flags() {
        // Test that clap rejects mutually exclusive flags
        use clap::Parser;

        let result = Cli::try_parse_from([
            "gold_digger",
            "--tls-ca-file",
            "/path/to/ca.pem",
            "--insecure-skip-hostname-verify",
        ]);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be used with"));
    }

    #[test]
    fn test_custom_ca_config() {
        let temp_file = NamedTempFile::new().unwrap();
        let cli = Cli::try_parse_from([
            "gold_digger",
            "--tls-ca-file",
            temp_file.path().to_str().unwrap(),
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
        ])
        .unwrap();

        let config = TlsConfig::from_cli(&cli).unwrap();
        assert!(matches!(config.validation_mode, TlsValidationMode::CustomCa { .. }));
    }

    #[test]
    fn test_platform_default_config() {
        let cli = Cli::try_parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
        ])
        .unwrap();

        let config = TlsConfig::from_cli(&cli).unwrap();
        assert!(matches!(config.validation_mode, TlsValidationMode::Platform));
    }
}
```

### Integration Tests

#### TLS Connection Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use testcontainers::*;

    #[test]
    fn test_tls_connection_with_valid_cert() {
        // Test connection to MySQL with valid certificate
        // This would use testcontainers with TLS-enabled MySQL
    }

    #[test]
    fn test_tls_connection_with_self_signed_cert() {
        // Test that connection fails with self-signed cert by default
        // Test that --allow-invalid-certificate allows connection
    }

    #[test]
    fn test_hostname_mismatch_handling() {
        // Test connection to server with hostname mismatch
        // Test that --insecure-skip-hostname-verify allows connection
    }
}
```

### Compatibility Tests

#### Migration Validation

```rust
#[test]
fn test_backward_compatibility() {
    // Ensure existing DATABASE_URL formats still work
    let database_urls = vec![
        "mysql://user:pass@localhost:3306/db",
        "mysql://user:pass@localhost:3306/db?ssl-mode=required",
        "mysql://user:pass@localhost:3306/db?ssl-mode=disabled",
    ];

    for url in database_urls {
        // Test that URL parsing and connection creation works
        // with new rustls implementation
    }
}
```

## Migration Strategy

### Phase 1: Feature Flag Removal

1. Update `Cargo.toml` to remove the `ssl` feature and make TLS dependencies standard
2. Remove all `#[cfg(feature = "ssl")]` conditional compilation directives
3. Update CLI structure to always include TLS options

### Phase 2: CLI Interface Updates

1. Remove `#[cfg(feature = "ssl")]` attributes from TLS flags
2. Update TLS option derivation to always include the fields
3. Ensure mutual exclusion validation works without feature gating

### Phase 3: TLS Configuration Updates

1. Remove the `enabled` field from `TlsConfig` (TLS always available)
2. Update `from_tls_options` method to remove feature gating
3. Update connection creation logic to always use TLS configuration

### Phase 4: Conditional Compilation Cleanup

1. Remove `#[cfg(feature = "ssl")]` from all TLS-related code
2. Remove the non-SSL fallback implementation of `create_tls_connection`
3. Update verbose logging to remove feature gating

### Phase 5: Testing and Documentation Updates

1. Update tests to remove feature-gated TLS testing
2. Update documentation to reflect always-available TLS
3. Update CI workflows to remove SSL feature variations

## Backward Compatibility

### Existing Behavior Preservation

- All existing `DATABASE_URL` formats continue to work
- Default TLS behavior (when no flags specified) uses platform certificate store
- TLS support is always available in all builds
- Exit codes and error handling patterns remain consistent

### Breaking Changes

- Remove both `ssl` and `ssl-rustls` feature flags (TLS always available)
- TLS implementation changes from native-tls to rustls (may affect certificate validation behavior)
- Some certificate validation errors may have different messages

### Migration Guide

```markdown
## Migrating from native-tls to rustls

### Feature Flags
- Remove any `--features ssl` or `--features ssl-rustls` from build commands
- TLS support is now always available in all builds

### Certificate Issues
- If you encounter hostname verification errors, use `--insecure-skip-hostname-verify`
- If you have self-signed certificates, use `--allow-invalid-certificate`
- For internal CAs, use `--tls-ca-file /path/to/ca.pem`

### Build Changes
- No changes needed for most users (TLS always available)
- Minimal builds: use `--no-default-features --features "csv json"`
```
