# Gold Digger TLS Integration Tests

This directory contains comprehensive TLS integration tests for Gold Digger's MySQL/MariaDB TLS functionality.

## Test Structure

### `tls_integration.rs`

Contains testcontainers-based TLS integration tests that validate:

- Basic TLS connection establishment
- Certificate validation scenarios (valid, invalid, self-signed)
- Custom CA certificate configuration
- Programmatic TLS configuration via SslOpts
- TLS error handling and messaging
- Cross-platform TLS functionality

## Running Tests

### Full Test Suite

```bash
# Run all tests (unit, integration, and TLS tests)
cargo test

# Release build tests
cargo test --release

# Run all tests including Docker-dependent ones (requires Docker)
cargo test -- --ignored
```

### TLS Integration Tests Only

```bash
# Run only TLS integration tests
cargo test --test tls_integration

# Release build TLS tests
cargo test --test tls_integration --release

# Run TLS tests including Docker-dependent ones (requires Docker)
cargo test --test tls_integration -- --ignored
```

### Minimal Build Tests (without TLS)

```bash
# Run library unit tests only (no TLS features)
cargo test --no-default-features --features json,csv --lib

# Run all unit tests (no TLS features)
cargo test --no-default-features --features json,csv --tests
```

### Docker-dependent Tests

Some tests require Docker and MySQL containers. These are marked with `#[ignore]` by default:

```bash
# Run all tests including Docker-dependent ones (requires Docker)
cargo test --test tls_integration -- --ignored

# Run specific Docker test
cargo test --test tls_integration test_basic_tls_connection_establishment -- --ignored
```

### Heavy Integration Tests

For comprehensive testing with real database connections, enable the `integration_tests` feature:

```bash
# Run integration tests (requires Docker and integration_tests feature)
cargo test --test tls_integration --features "integration_tests" -- --ignored

# Run all tests including integration tests
cargo test --test tls_integration --features "integration_tests" -- --include-ignored

# Using justfile commands
just test-integration  # Run only integration tests
just test-all         # Run all tests including integration tests
```

### Test Categories

1. **Unit Tests** (`tls_unit_tests`): Test TLS configuration and validation without external
   dependencies
2. **Validation Tests** (`tls_validation_tests`): Test certificate validation and error scenarios
3. **Integration Tests** (`tls_tests`): Test actual TLS connections (require Docker)
4. **Performance Tests** (`tls_performance_tests`): Test connection pooling and concurrency (require
   Docker)
5. **No-TLS Tests** (`no_tls_tests`): Test behavior when TLS features are disabled
6. **Heavy Integration Tests** (`integration_tests`): Comprehensive database integration tests (require Docker, TEST_DATABASE_URL, and `integration_tests` feature)

## Requirements

- **Rust**: Tests require the same Rust version as the main project
- **Docker** (optional): Required only for tests marked with `#[ignore]`
- **MariaDB Image**: Docker tests will automatically pull `mariadb:10.11` image
- **integration_tests feature** (optional): Feature flag required for heavy integration tests

## Test Features

### TLS Configuration Testing

- Tests all TLS configuration options
- Validates certificate file handling
- Tests programmatic SslOpts configuration
- Verifies error handling for invalid configurations

### Certificate Validation

- Tests with valid PEM certificates
- Tests with invalid certificate content
- Tests with nonexistent certificate files
- Tests with self-signed certificates

### Connection Testing

- Tests basic TLS connection establishment
- Tests connection with various authentication scenarios
- Tests connection to different MySQL databases
- Tests connection pooling and reuse

### Error Handling

- Tests connection failures with helpful error messages
- Tests certificate validation errors
- Tests malformed URL handling
- Tests unreachable host scenarios

## CI Integration

The tests are designed to work in CI environments:

- Docker-dependent tests are ignored by default
- Unit tests run without external dependencies
- Tests work with always-available rustls TLS implementation
- Tests validate that TLS is available in both standard and minimal builds

## Troubleshooting

### Docker Issues

If Docker tests fail:

1. Ensure Docker is running
2. Check Docker has internet access to pull images
3. Run tests with `--ignored` flag only if Docker is available

### Certificate Issues

If certificate tests fail:

1. Check file permissions on temporary directories
2. Verify certificate content is valid PEM format
3. Ensure test has write access to create temporary files

### Feature Issues

If feature-related tests fail:

1. Verify TLS is available in standard builds
2. Check that rustls TLS implementation works correctly
3. Ensure TLS is properly excluded in minimal builds
