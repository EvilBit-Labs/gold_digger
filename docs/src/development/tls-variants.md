# TLS and Non-TLS Database Variants

Gold Digger's integration testing framework provides comprehensive support for testing both
TLS-enabled and plain (non-TLS) database connections through specialized database variants.

## Overview

The TLS variants system provides:

- **TestDatabaseTls**: TLS-enabled database containers with SSL certificate mounting and MySQL TLS
  settings
- **TestDatabasePlain**: Standard unencrypted database containers for basic connection testing
- **TlsContainerConfig**: Configuration for TLS settings including certificate management and
  security policies
- **Connection URL generation**: Helper methods to generate appropriate connection URLs for each
  configuration type
- **Connection validation**: Test utilities to validate TLS vs non-TLS connection establishment

## Database Variants

### TLS-Enabled Variants

```rust
use integration::{TestDatabaseTls, TlsContainerConfig, containers::DatabaseContainer};

// MySQL with secure TLS defaults
let mysql_tls = TestDatabaseTls::mysql();
let mysql_container = DatabaseContainer::new_tls(mysql_tls)?;

// MariaDB with custom TLS configuration
let tls_config = TlsContainerConfig::new_secure().with_strict_security()?;
let mariadb_tls = TestDatabaseTls::mariadb_with_config(tls_config);
let mariadb_container = DatabaseContainer::new_tls(mariadb_tls)?;
```

### Plain (Non-TLS) Variants

```rust
use integration::{TestDatabasePlain, containers::DatabaseContainer};

// MySQL without TLS
let mysql_plain = TestDatabasePlain::mysql();
let mysql_container = DatabaseContainer::new_plain(mysql_plain)?;

// MariaDB without TLS
let mariadb_plain = TestDatabasePlain::mariadb();
let mariadb_container = DatabaseContainer::new_plain(mariadb_plain)?;
```

## TLS Configuration Options

### Secure Defaults

The framework provides secure TLS defaults suitable for most testing scenarios:

```rust
let tls_config = TlsContainerConfig::new_secure();
// Configuration includes:
// - require_secure_transport: true
// - min_tls_version: "TLSv1.2"
// - use_ephemeral_certs: true
// - Strong cipher suites (ECDHE-RSA-AES256-GCM-SHA384, etc.)
```

### Strict Security

For testing with enhanced security requirements:

```rust
let tls_config = TlsContainerConfig::new_secure().with_strict_security()?;
// Enhanced configuration includes:
// - min_tls_version: "TLSv1.3"
// - TLS 1.3 cipher suites only (TLS_AES_256_GCM_SHA384, etc.)
// - Stricter security policies
```

### Custom Certificates

For testing with specific certificate requirements:

```rust
let tls_config = TlsContainerConfig::with_custom_certs(
    "/path/to/ca.pem",
    "/path/to/cert.pem",
    "/path/to/key.pem"
);
```

## Connection URL Generation

The container system provides flexible connection URL generation for different SSL modes:

```rust
// Basic connection URL (uses container defaults)
let url = container.connection_url();

// TLS-enabled URL (for TLS containers)
let tls_url = container.tls_connection_url()?;

// Explicitly disabled SSL URL
let plain_url = container.plain_connection_url()?;

// Custom SSL mode
let verify_ca_url = container.connection_url_with_ssl_mode("VERIFY_CA")?;
let required_url = container.connection_url_with_ssl_mode("REQUIRED")?;
let disabled_url = container.connection_url_with_ssl_mode("DISABLED")?;
```

### Supported SSL Modes

- **DISABLED**: No SSL encryption
- **PREFERRED**: Use SSL if available, fallback to plain
- **REQUIRED**: Require SSL connection
- **VERIFY_CA**: Require SSL and verify CA certificate
- **VERIFY_IDENTITY**: Require SSL and verify server identity

## Connection Validation

The framework provides utilities to validate connection behavior:

```rust
// For TLS containers
let tls_validation = container.validate_tls_connection()?;
assert!(tls_validation.tls_connection_success);

// For plain containers
let plain_validation = container.validate_plain_connection()?;
assert!(plain_validation.plain_connection_success);
```

### Validation Results

```rust
pub struct TlsValidationResult {
    pub tls_connection_success: bool,
    pub tls_error: Option<String>,
}

pub struct PlainValidationResult {
    pub plain_connection_success: bool,
    pub plain_error: Option<String>,
}
```

## Database Type Conversions

The variants can be converted to the base `TestDatabase` enum for compatibility with existing code:

```rust
// TLS variant conversion
let tls_db = TestDatabaseTls::mysql();
let base_db = tls_db.to_test_database(); // TestDatabase::MySQL { tls_enabled: true }

// Plain variant conversion
let plain_db = TestDatabasePlain::mysql();
let base_db = plain_db.to_test_database(); // TestDatabase::MySQL { tls_enabled: false }
```

## Running TLS Variant Tests

The TLS variants are tested in `tests/tls_variants_test.rs`:

```bash
# Run all TLS variant tests (requires Docker and integration_tests feature)
cargo test --test tls_variants_test --features integration_tests

# Run specific variant tests
cargo test test_mysql_tls_variant --test tls_variants_test --features integration_tests
cargo test test_mariadb_plain_variant --test tls_variants_test --features integration_tests
cargo test test_connection_url_generation --test tls_variants_test --features integration_tests

# Run tests that don't require Docker (no feature flag needed)
cargo test test_database_variant_conversions --test tls_variants_test
cargo test test_tls_container_config_methods --test tls_variants_test

# Using justfile commands
just test-integration  # Run integration tests with feature flag
just test-all         # Run all tests including integration tests
```

## Implementation Details

### Current Capabilities

1. **TLS Container Creation**: Basic TLS container setup with secure defaults
2. **Certificate Management**: Placeholder certificate system (ephemeral certificates planned)
3. **Connection URL Generation**: SSL parameter injection for different connection modes
4. **Connection Validation**: Basic TLS and plain connection testing
5. **Configuration Validation**: TLS configuration parameter validation

### Current Limitations

1. **Certificate Generation**: Currently uses placeholder certificates. Full integration with
   ephemeral certificate generation is planned.

2. **Container TLS Configuration**: Basic TLS container setup is implemented. Full SSL certificate
   mounting and MySQL TLS settings configuration will be enhanced in subsequent development phases.

3. **Connection Validation**: Basic connection testing is implemented. More comprehensive TLS
   handshake validation will be added as the TLS infrastructure matures.

### Future Enhancements

- **Dynamic Certificate Generation**: Integration with `rcgen` crate for ephemeral certificate
  generation
- **Full SSL Certificate Mounting**: Complete SSL certificate mounting into containers
- **MySQL/MariaDB TLS Configuration**: Comprehensive TLS configuration (require_secure_transport,
  cipher suites, etc.)
- **Advanced TLS Validation**: Comprehensive TLS handshake and certificate validation testing
- **Performance Comparison**: Performance testing with TLS vs non-TLS connections

## Best Practices

### Test Design

- **Use Appropriate Variants**: Choose TLS variants for security testing, plain variants for basic
  functionality
- **Validate Connections**: Always validate connection establishment before running tests
- **Clean Resource Management**: Ensure proper container cleanup after tests
- **Environment Detection**: Use CI-aware timeouts and resource limits

### TLS Testing

- **Certificate Validation**: Test both valid and invalid certificate scenarios
- **SSL Mode Testing**: Validate different SSL modes and their behavior
- **Error Handling**: Test TLS connection failures and error messages
- **Security Policies**: Validate TLS version and cipher suite enforcement

### Performance Considerations

- **Container Startup**: TLS containers may take longer to start due to certificate setup
- **Connection Overhead**: TLS connections have additional handshake overhead
- **Resource Usage**: TLS containers may use more memory and CPU resources

This TLS variants system provides a solid foundation for comprehensive database connection testing
that can be enhanced as the integration test infrastructure continues to mature.
