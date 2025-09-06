# Integration Testing Framework

Gold Digger features a comprehensive integration testing framework that validates functionality
against real MySQL and MariaDB databases using testcontainers for automated container management.

## Overview

The integration testing enhancement provides:

- **Multi-Database Support**: Both MySQL (8.0+) and MariaDB (10.11+) testing
- **TLS/Non-TLS Testing**: Secure and standard connection validation
- **Comprehensive Data Type Coverage**: All MySQL/MariaDB data types including edge cases
- **Output Format Validation**: CSV (RFC4180), JSON, and TSV format compliance testing
- **Error Scenario Testing**: Connection failures, SQL errors, and file I/O validation
- **Performance Testing**: Large dataset handling and memory usage validation
- **Security Testing**: Credential protection and TLS certificate validation

## Test Architecture

### Current Implementation Status

The integration testing framework is actively under development with the following components
implemented:

**✅ Completed Components:**

- Core integration test infrastructure with MySQL/MariaDB support
- TLS and non-TLS database variants (`TestDatabaseTls`, `TestDatabasePlain`)
- Container management with health checks and CI compatibility
- TLS certificate management with ephemeral certificate generation
- Comprehensive test database schema and seeding functionality
- Cross-platform support (Linux and macOS)

**🚧 In Progress:**

- Data type validation tests
- Output format validation framework
- Error scenario testing
- CLI integration tests
- Performance testing
- Security validation tests

### Test Module Structure

```
tests/
├── integration/
│   ├── mod.rs              # Common test utilities and setup functions (✅ Implemented)
│   ├── common.rs           # Shared CLI execution and output parsing utilities (✅ Implemented)
│   ├── containers.rs       # MySQL/MariaDB container management with health checks (✅ Implemented)
│   ├── data_types.rs       # Comprehensive data type validation tests (🚧 Planned)
│   ├── output_formats.rs   # Format-specific validators (CSV, JSON, TSV) (🚧 Planned)
│   ├── error_scenarios.rs  # Error handling and exit code validation (🚧 Planned)
│   ├── cli_integration.rs  # CLI flag precedence and configuration tests (🚧 Planned)
│   ├── performance.rs      # Large dataset and memory usage tests (🚧 Planned)
│   └── security.rs         # Credential protection and TLS security tests (🚧 Planned)
├── fixtures/
│   ├── schema.sql          # Comprehensive test database schema (✅ Implemented)
│   ├── seed_data.sql       # Test data covering all data types and edge cases (✅ Implemented)
│   └── tls/                # TLS certificates for secure connection testing (✅ Implemented)
├── test_support/           # Shared testing utilities (✅ Implemented)
│   ├── cli.rs              # CLI execution helpers
│   ├── containers.rs       # Container management utilities
│   ├── fixtures.rs         # Test data and schema utilities
│   └── parsing.rs          # Output parsing and validation
├── tls_variants_test.rs    # TLS and non-TLS database variant testing (✅ Implemented)
├── tls_integration.rs      # TLS connection and certificate validation (✅ Implemented)
├── integration_tests.rs    # Main integration test entry point (✅ Implemented)
├── database_seeding_test.rs # Database schema and data seeding tests (✅ Implemented)
└── container_setup_test.rs # Container lifecycle and health check tests (✅ Implemented)
```

### Container Management

The framework uses testcontainers-modules for automated database lifecycle management with
comprehensive TLS support:

```rust
use testcontainers_modules::{mariadb::Mariadb, mysql::Mysql};

// Base database types
pub enum TestDatabase {
    MySQL { tls_enabled: bool },
    MariaDB { tls_enabled: bool },
}

// TLS-enabled database variants
pub enum TestDatabaseTls {
    MySQL { tls_config: TlsContainerConfig },
    MariaDB { tls_config: TlsContainerConfig },
}

// Plain (non-TLS) database variants
pub enum TestDatabasePlain {
    MySQL,
    MariaDB,
}

// Container management with TLS support
pub struct DatabaseContainer {
    db_type: TestDatabase,
    container: Box<dyn ContainerInstance>,
    connection_url: String,
    temp_dir: TempDir,
    tls_config: ContainerTlsConfig,
}

impl DatabaseContainer {
    // Create TLS-enabled container
    pub fn new_tls(db_type: TestDatabaseTls) -> anyhow::Result<Self> {
        // Automatic container startup with health checks
        // TLS configuration with ephemeral certificates
        // Connection URL generation with SSL parameters
    }

    // Create plain (non-TLS) container
    pub fn new_plain(db_type: TestDatabasePlain) -> anyhow::Result<Self> {
        // Standard container setup without TLS
        // Connection URL generation for unencrypted connections
    }

    // Validate TLS connections
    pub fn validate_tls_connection(&self) -> anyhow::Result<TlsValidationResult> {
        // TLS handshake validation
        // Certificate verification testing
    }

    // Validate plain connections
    pub fn validate_plain_connection(&self) -> anyhow::Result<PlainValidationResult> {
        // Standard connection testing
        // Non-TLS connection validation
    }
}
```

### TLS Configuration

The framework provides comprehensive TLS configuration options:

```rust
pub struct TlsContainerConfig {
    pub require_secure_transport: bool,
    pub min_tls_version: String, // "TLSv1.2" or "TLSv1.3"
    pub cipher_suites: Vec<String>,
    pub use_ephemeral_certs: bool, // Generate certificates per test run
    pub ca_cert_path: Option<PathBuf>,
    pub server_cert_path: Option<PathBuf>,
    pub server_key_path: Option<PathBuf>,
}

impl TlsContainerConfig {
    // Secure defaults with TLS 1.2+
    pub fn new_secure() -> Self;

    // Strict security with TLS 1.3 only
    pub fn with_strict_security(self) -> anyhow::Result<Self>;

    // Custom certificate paths
    pub fn with_custom_certs(ca_cert: P, server_cert: P, server_key: P) -> Self;
}
```

## Test Categories

### 1. Data Type Validation Tests

Comprehensive testing of MySQL/MariaDB data type handling:

- **String Types**: VARCHAR, TEXT, CHAR with Unicode and special characters
- **Numeric Types**: INTEGER, BIGINT, DECIMAL, FLOAT, DOUBLE with precision testing
- **Temporal Types**: DATE, DATETIME, TIMESTAMP, TIME, YEAR with timezone handling
- **Binary Types**: BINARY, VARBINARY, BLOB with large content testing
- **Special Types**: JSON, ENUM, SET, BOOLEAN with edge case validation
- **NULL Handling**: NULL value processing across all data types

### 2. Output Format Validation Tests

Format-specific compliance and consistency testing:

- **CSV Validation**: RFC4180 compliance, quoting behavior, header validation
- **JSON Validation**: Structure validation, deterministic ordering, NULL handling
- **TSV Validation**: Tab-delimited format, special character handling
- **Cross-Format Consistency**: Identical data representation across formats

### 3. Database Integration Tests

Real database connection and query execution validation:

- **Container Management**: MySQL/MariaDB container lifecycle with health checks
- **Connection Testing**: Both TLS and non-TLS connection establishment
- **Query Execution**: SQL query processing with various data scenarios
- **Transaction Handling**: Database transaction support and rollback testing

### 4. Error Scenario Tests

Comprehensive error handling and exit code validation:

- **Connection Errors**: Authentication failures, network timeouts, unreachable hosts
- **SQL Errors**: Syntax errors, permission denied, non-existent tables/columns
- **File I/O Errors**: Permission denied, disk space exhaustion, invalid paths
- **Exit Code Validation**: Proper exit code mapping for all error scenarios

### 5. CLI Integration Tests

Command-line interface and configuration validation:

- **Flag Precedence**: CLI flags override environment variables
- **Mutual Exclusion**: Conflicting option detection and error handling
- **Configuration Resolution**: Format detection and parameter validation
- **Help and Completion**: CLI help text and shell completion generation

### 6. Performance Tests

Large dataset handling and resource usage validation:

- **Large Result Sets**: Processing 1000+ row datasets without memory issues
- **Wide Tables**: Handling 20+ column tables with various data types
- **Large Content**: Processing 1MB+ text fields and binary data
- **Memory Usage**: Validation of reasonable memory bounds for result set size

### 7. Security Tests

Credential protection and TLS validation:

- **Credential Redaction**: DATABASE_URL masking in logs and error messages
- **TLS Connection Testing**: Certificate validation and secure connection establishment
- **Connection String Security**: Special character handling in passwords
- **Security Warning Display**: Appropriate warnings for insecure TLS modes

## Running Integration Tests

### Prerequisites

- **Docker**: Required for container-based testing
- **Disk Space**: ~500MB for Docker images and test artifacts
- **Network Access**: For pulling Docker images (first run only)
- **Platform Support**: Linux and macOS (Windows support planned)

### Test Execution

```bash
# Run comprehensive integration tests
cargo test --features integration_tests -- --ignored

# Run all tests including integration tests
cargo test --features integration_tests -- --include-ignored

# Run specific integration test categories
cargo test --features integration_tests --test tls_variants_test -- --ignored
cargo test --features integration_tests --test tls_integration -- --ignored
cargo test --features integration_tests --test database_seeding_test -- --ignored
cargo test --features integration_tests --test container_setup_test -- --ignored

# Run TLS variant tests specifically
cargo test --features integration_tests test_mysql_tls_variant --test tls_variants_test -- --ignored
cargo test --features integration_tests test_mariadb_plain_variant --test tls_variants_test -- --ignored

# Using justfile commands
just test-integration  # Run only integration tests
just test-all         # Run all tests including integration tests
```

### Current Test Categories

**✅ Implemented Tests:**

1. **TLS Variant Tests** (`tests/tls_variants_test.rs`):

   - MySQL and MariaDB TLS-enabled containers
   - Plain (non-TLS) container configurations
   - Connection URL generation with SSL parameters
   - TLS configuration validation
   - Database variant conversions

2. **TLS Integration Tests** (`tests/tls_integration.rs`):

   - TLS connection establishment and validation
   - Certificate verification testing
   - SSL handshake validation
   - TLS error handling

3. **Database Seeding Tests** (`tests/database_seeding_test.rs`):

   - Schema creation and validation
   - Test data population with comprehensive data types
   - NULL value handling across all data types
   - Unicode and special character testing

4. **Container Setup Tests** (`tests/container_setup_test.rs`):

   - Container lifecycle management
   - Health check validation
   - Resource cleanup testing
   - CI environment compatibility

**🚧 Planned Tests:**

05. **Data Type Validation Tests**: Comprehensive MySQL/MariaDB data type handling
06. **Output Format Validation Tests**: CSV, JSON, and TSV format compliance
07. **Error Scenario Tests**: Connection failures, SQL errors, and file I/O issues
08. **CLI Integration Tests**: Command-line flag precedence and configuration
09. **Performance Tests**: Large dataset handling and memory usage validation
10. **Security Tests**: Credential protection and TLS certificate validation

### CI Integration

The integration testing framework is designed for CI environments:

- **GitHub Actions**: Docker service enabled with appropriate timeouts
- **Resource Limits**: Tests designed for shared CI resources
- **Container Cleanup**: Automatic cleanup prevents resource leaks
- **Retry Logic**: Configurable timeouts for container startup in CI

### Test Matrix

GitHub Actions runs integration tests across multiple configurations:

```yaml
strategy:
  matrix:
    database: [mysql-8.0, mysql-8.1, mariadb-10.11]
    connection: [tls, non-tls]
    features: [default, minimal]
```

## Test Data and Fixtures

### Database Schema

The test schema (`tests/fixtures/schema.sql`) includes comprehensive data type coverage:

```sql
-- String types with various lengths and encodings
CREATE TABLE test_strings (
    id INT PRIMARY KEY AUTO_INCREMENT,
    varchar_col VARCHAR(255),
    text_col TEXT,
    char_col CHAR(10),
    unicode_col VARCHAR(255) CHARACTER SET utf8mb4
);

-- Numeric types with precision and scale variations
CREATE TABLE test_numbers (
    id INT PRIMARY KEY AUTO_INCREMENT,
    int_col INT,
    bigint_col BIGINT,
    decimal_col DECIMAL(10,2),
    float_col FLOAT,
    double_col DOUBLE
);

-- Temporal types with timezone considerations
CREATE TABLE test_temporal (
    id INT PRIMARY KEY AUTO_INCREMENT,
    date_col DATE,
    datetime_col DATETIME,
    timestamp_col TIMESTAMP,
    time_col TIME,
    year_col YEAR
);
```

### Test Data Seeding

The seed data (`tests/fixtures/seed_data.sql`) includes:

- **Normal Values**: Standard data for each type
- **Edge Cases**: Boundary values, empty strings, zero values
- **NULL Values**: NULL handling across all columns
- **Unicode Content**: International characters and emojis
- **Large Content**: 1MB+ text fields for performance testing

### TLS Certificates

Test TLS certificates (`tests/fixtures/tls/`) provide:

- **Self-Signed CA**: Root certificate authority for testing
- **Server Certificates**: Valid certificates for container hostnames
- **Invalid Certificates**: Malformed certificates for error testing
- **Expired Certificates**: Time-based validation testing

## Development Workflow

### Adding New Integration Tests

1. **Identify Test Category**: Determine which test module the new test belongs to
2. **Create Test Function**: Add test function with appropriate attributes
3. **Use Test Utilities**: Leverage common utilities for container management
4. **Validate Output**: Use format-specific validators for output verification
5. **Handle Cleanup**: Ensure proper resource cleanup in test teardown

### Test Utilities

Common utilities provide consistent testing patterns:

```rust
use anyhow::Result;

fn test_example() -> Result<()> {
    // Container management
    let db = TestDatabase::new(DatabaseType::MySQL, true)?;
    let connection_url = db.connection_url();

    // CLI execution
    let result = GoldDiggerCli::new()
        .db_url(&connection_url)
        .query("SELECT * FROM test_table")
        .output_file(&temp_file)
        .execute()?;

    // Output validation
    let validator = CsvValidator::new();
    validator.validate_file(&temp_file)?;
    validator.validate_row_count(expected_rows)?;

    Ok(())
}
```

### Debugging Integration Tests

```bash
# Run single integration test with verbose output
RUST_LOG=debug cargo test --features integration_tests \
  test_mysql_data_type_conversion -- --ignored --nocapture

# Keep containers running for manual inspection
TESTCONTAINERS_RYUK_DISABLED=true cargo test --features integration_tests \
  test_container_setup -- --ignored

# Generate test coverage for integration tests
cargo llvm-cov --features integration_tests --html -- --ignored
```

## Best Practices

### Test Design

- **Isolation**: Each test should be independent and not affect others
- **Deterministic**: Tests should produce consistent results across runs
- **Fast Feedback**: Unit tests for quick feedback, integration tests for comprehensive validation
- **Resource Cleanup**: Always clean up containers and temporary files

### Container Management

- **Health Checks**: Wait for container readiness before running tests
- **Timeout Handling**: Configure appropriate timeouts for CI environments
- **Resource Limits**: Set memory and CPU limits for containers
- **Network Isolation**: Use container networks to avoid port conflicts

### Data Validation

- **Comprehensive Coverage**: Test all data types and edge cases
- **Format Compliance**: Validate output format standards (RFC4180 for CSV)
- **Cross-Format Consistency**: Ensure identical data across output formats
- **Error Scenarios**: Test both success and failure paths

This integration testing framework ensures Gold Digger's reliability and correctness across diverse
database environments and usage scenarios.
