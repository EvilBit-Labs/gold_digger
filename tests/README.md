# Gold Digger Integration Tests

This directory contains comprehensive integration tests for Gold Digger's MySQL/MariaDB
functionality, including TLS support, data type handling, and output format validation.

## Test Structure

### Current Test Files

- **`tls_integration.rs`**: TLS connection and certificate validation tests
- **`tls_config_unit_tests.rs`**: TLS configuration unit tests
- **`end_to_end_type_conversion.rs`**: Data type conversion and safety tests
- **`exit_codes.rs`**: Exit code validation tests
- **`type_safety.rs`**: Type safety and NULL value handling tests

### Planned Integration Test Framework

The integration testing enhancement will add:

- **`tests/integration/`**: Comprehensive integration test module structure
- **`tests/integration/mod.rs`**: Common test utilities and setup functions
- **`tests/integration/common.rs`**: Shared CLI execution and output parsing utilities
- **`tests/integration/containers.rs`**: MySQL/MariaDB container management with health checks
- **`tests/integration/data_types.rs`**: Comprehensive data type validation tests
- **`tests/integration/output_formats.rs`**: Format-specific validators (CSV, JSON, TSV)
- **`tests/integration/error_scenarios.rs`**: Error handling and exit code validation
- **`tests/integration/cli_integration.rs`**: CLI flag precedence and configuration tests
- **`tests/integration/performance.rs`**: Large dataset and memory usage tests
- **`tests/integration/security.rs`**: Credential protection and TLS security tests
- **`tests/fixtures/`**: Test database schema, seed data, and TLS certificates

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

### Integration Test Categories

#### Current TLS Integration Tests

```bash
# Run only TLS integration tests
cargo test --test tls_integration

# Release build TLS tests
cargo test --test tls_integration --release

# Run TLS tests including Docker-dependent ones (requires Docker)
cargo test --test tls_integration -- --ignored
```

#### Planned Comprehensive Integration Tests

```bash
# Run comprehensive integration tests (requires Docker)
cargo test --features integration_tests -- --ignored

# Run all tests including comprehensive integration tests
cargo test --features integration_tests -- --include-ignored

# Run specific integration test categories
cargo test --test integration_tests data_type_validation -- --ignored
cargo test --test integration_tests output_format_validation -- --ignored
cargo test --test integration_tests error_scenario_validation -- --ignored
cargo test --test integration_tests performance_validation -- --ignored

# Using justfile commands
just test-integration  # Run only integration tests
just test-all         # Run all tests including integration tests
```

### Test Execution Strategies

#### Fast Development Testing (No Docker)

```bash
# Unit tests only (no external dependencies)
cargo test --lib

# Unit tests with nextest (faster parallel execution)
cargo nextest run --lib

# Exclude Docker-dependent tests
just test-no-docker
```

#### Comprehensive Testing (Docker Required)

```bash
# All tests including Docker-dependent integration tests
cargo test --features integration_tests -- --include-ignored

# CI-equivalent comprehensive testing
just ci-check
```

#### Feature-Specific Testing

```bash
# Test with minimal features (no TLS features in minimal build)
cargo test --no-default-features --features "json csv" --lib

# Test with all features including integration tests
cargo test --features "integration_tests additional_mysql_types" -- --include-ignored
```

### Test Categories

#### Current Test Categories

1. **Unit Tests** (`tls_unit_tests`): Test TLS configuration and validation without external
   dependencies
2. **TLS Integration Tests** (`tls_integration`): Test actual TLS connections and certificate
   validation (require Docker)
3. **Type Safety Tests** (`type_safety`): Test MySQL data type conversion and NULL value handling
4. **Exit Code Tests** (`exit_codes`): Test proper exit code mapping for different error scenarios

#### Planned Integration Test Categories

1. **Data Type Validation Tests**: Comprehensive testing of all MySQL/MariaDB data types

   - String types (VARCHAR, TEXT, CHAR)
   - Numeric types (INTEGER, BIGINT, DECIMAL, FLOAT, DOUBLE)
   - Temporal types (DATE, DATETIME, TIMESTAMP, TIME, YEAR)
   - Binary types (BINARY, VARBINARY, BLOB)
   - JSON and special types (JSON, ENUM, SET, BOOLEAN)
   - NULL value handling across all types

2. **Output Format Validation Tests**: Format-specific compliance and consistency testing

   - CSV format validation (RFC4180 compliance, quoting behavior)
   - JSON format validation (structure, deterministic ordering, NULL handling)
   - TSV format validation (tab-delimited, special character handling)
   - Cross-format consistency validation

3. **Database Integration Tests**: Real database connection and query execution

   - MySQL container setup and management (versions 8.0, 8.1)
   - MariaDB container setup and management (version 10.11+)
   - TLS and non-TLS connection testing
   - Container health checks and CI compatibility

4. **Error Scenario Tests**: Comprehensive error handling validation

   - Database connection failures (authentication, network, timeout)
   - SQL execution errors (syntax errors, permission denied, non-existent tables)
   - File I/O errors (permission denied, disk space, invalid paths)
   - Exit code validation for all error scenarios

5. **CLI Integration Tests**: Command-line interface and configuration validation

   - CLI flag precedence over environment variables
   - Mutually exclusive option handling
   - Configuration resolution and format detection
   - Help text and completion generation

6. **Performance Tests**: Large dataset handling and resource usage

   - Large result set processing (1000+ rows)
   - Wide table handling (20+ columns)
   - Large content processing (1MB+ text fields)
   - Memory usage validation and performance benchmarking

7. **Security Tests**: Credential protection and TLS validation

   - Credential redaction in logs and error messages
   - TLS connection establishment and certificate validation
   - Connection string parsing with special characters
   - Security warning display for insecure TLS modes

8. **Cross-Platform Tests**: Platform-specific behavior validation

   - Path separator handling (Windows vs Unix)
   - Line ending consistency (CRLF vs LF)
   - Platform-specific TLS certificate store integration

## Requirements

### System Requirements

- **Rust**: Tests require the same Rust version as the main project (1.89.0+)
- **Docker** (optional): Required only for tests marked with `#[ignore]`
- **Disk Space**: ~500MB for Docker images and test artifacts

### Docker Images

Integration tests automatically pull the following Docker images:

- **MariaDB**: `mariadb:10.11` (current TLS integration tests)
- **MySQL**: `mysql:8.0`, `mysql:8.1` (planned comprehensive integration tests)
- **Testcontainers**: Automatic container lifecycle management

### Feature Flags

- **`integration_tests`**: Feature flag required for comprehensive integration tests
- **Default features**: Standard unit tests run without additional feature flags
- **Minimal features**: `--no-default-features --features "json csv"` for lightweight testing

### CI Environment Compatibility

- **GitHub Actions**: Docker service enabled, appropriate timeouts configured
- **Resource Limits**: Tests designed for shared CI resources with retry logic
- **Container Cleanup**: Automatic cleanup prevents resource leaks in CI
- **Timeout Handling**: Configurable timeouts for container startup in CI environments

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
