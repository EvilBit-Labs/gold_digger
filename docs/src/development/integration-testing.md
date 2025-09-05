# Integration Testing Framework

Gold Digger features a comprehensive integration testing framework that validates functionality against real MySQL and MariaDB databases using testcontainers for automated container management.

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

### Test Module Structure

```
tests/
├── integration/
│   ├── mod.rs              # Common test utilities and setup functions
│   ├── common.rs           # Shared CLI execution and output parsing utilities
│   ├── containers.rs       # MySQL/MariaDB container management with health checks
│   ├── data_types.rs       # Comprehensive data type validation tests
│   ├── output_formats.rs   # Format-specific validators (CSV, JSON, TSV)
│   ├── error_scenarios.rs  # Error handling and exit code validation
│   ├── cli_integration.rs  # CLI flag precedence and configuration tests
│   ├── performance.rs      # Large dataset and memory usage tests
│   └── security.rs         # Credential protection and TLS security tests
├── fixtures/
│   ├── schema.sql          # Comprehensive test database schema
│   ├── seed_data.sql       # Test data covering all data types and edge cases
│   └── tls/                # TLS certificates for secure connection testing
└── integration_tests.rs   # Main entry point for integration tests
```

### Container Management

The framework uses testcontainers-modules for automated database lifecycle management:

```rust
use testcontainers_modules::{mariadb::Mariadb, mysql::Mysql};

pub enum TestDatabase {
    MySQL(Container<Mysql>),
    MariaDB(Container<Mariadb>),
}

impl TestDatabase {
    pub fn new(db_type: DatabaseType, tls_enabled: bool) -> anyhow::Result<Self> {
        // Automatic container startup with health checks
        // TLS configuration for secure testing
        // Connection URL generation
    }
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

### Test Execution

```bash
# Run comprehensive integration tests
cargo test --features integration_tests -- --ignored

# Run all tests including integration tests
cargo test --features integration_tests -- --include-ignored

# Run specific integration test categories
cargo test --test integration_tests data_type_validation -- --ignored
cargo test --test integration_tests output_format_validation -- --ignored
cargo test --test integration_tests error_scenario_validation -- --ignored

# Using justfile commands
just test-integration  # Run only integration tests
just test-all         # Run all tests including integration tests
```

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

This integration testing framework ensures Gold Digger's reliability and correctness across diverse database environments and usage scenarios.
