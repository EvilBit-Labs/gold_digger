# Gold Digger Integration Tests

This directory contains comprehensive integration tests for Gold Digger's MySQL/MariaDB functionality, including TLS support, data type handling, and output format validation.

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

1. **Unit Tests** (`tls_unit_tests`): Test TLS configuration and validation without external dependencies
2. **TLS Integration Tests** (`tls_integration`): Test actual TLS connections and certificate validation (require Docker)
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

## Performance Benchmarking

The project includes comprehensive performance benchmarks using Criterion to measure and track performance characteristics of core functionality.

### Benchmark Suite

The benchmark suite is located in the `benches/` directory and includes:

- **`rows_processing.rs`**: Benchmarks for `rows_to_strings` function with various dataset sizes (small, medium, large, wide, null-heavy, mixed types)
- **`output_formats.rs`**: Benchmarks for CSV, JSON (compact and pretty), and TSV writers with different data characteristics
- **`value_conversion.rs`**: Benchmarks for MySQL value to string conversion across all value types
- **`memory_usage.rs`**: End-to-end memory and throughput benchmarks comparing formats on large datasets

### Running Benchmarks

#### Basic Commands

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark suite
cargo bench --bench rows_processing
cargo bench --bench output_formats
cargo bench --bench value_conversion
cargo bench --bench memory_usage
```

#### Using Just Recipes

```bash
# Run full benchmark suite (mirrors CI)
just bench

# Run with reduced sample size for faster feedback
just bench-quick

# Save current performance as a named baseline
just bench-baseline main-branch

# Compare against a saved baseline
just bench-compare main-branch

# Run a specific benchmark by name
just bench-specific rows_processing

# Open generated HTML report in browser
just bench-report
```

### Interpreting Results

Criterion provides detailed performance metrics:

- **Throughput**: Measured in rows/second or elements/second
- **Time per iteration**: Average time for each benchmark iteration
- **HTML Reports**: Detailed reports with statistical analysis in `target/criterion/`

The HTML reports include:

- Performance trends over time
- Statistical significance of changes
- Comparison with saved baselines
- Distribution plots and outlier detection

### Baseline Management

Baselines allow tracking performance regressions:

1. **Create baseline**: `just bench-baseline baseline-name`
2. **Compare**: `just bench-compare baseline-name`
3. **Update**: Re-run `bench-baseline` with the same name to update

Baselines are stored in `target/criterion/` and should be committed for main branch performance tracking.

## Parameterized Testing with rstest

The project uses `rstest` for parameterized testing, allowing comprehensive test coverage with less code duplication.

### rstest Features

- **Fixtures**: Reusable test setup code (containers, connections, CLI commands)
- **Parameterized Tests**: Run the same test with different inputs using `#[case]` or `#[values]`
- **Descriptive Case Names**: Each test case has a clear name for easy identification in output

### Using rstest

#### Fixtures

Fixtures encapsulate common setup logic:

```rust
use rstest::{fixture, rstest};

#[fixture]
fn db_pool() -> Pool {
    // Setup database connection
    // ...
}

#[test]
fn test_with_fixture(db_pool: Pool) {
    // Use the fixture
}
```

#### Parameterized Tests

Use `#[case]` for specific test scenarios:

```rust
#[rstest]
#[case("scenario_1")]
#[case("scenario_2")]
fn test_multiple_scenarios(db_pool: Pool, #[case] scenario: &str) {
    // Test logic varies based on scenario
}
```

### Files Using rstest

- **`tests/type_safety.rs`**: Parameterized tests for data types, special characters, and numeric ranges
- **`tests/cli_testing_example.rs`**: Parameterized tests for formats, environment variables, and flag precedence
- **`tests/tls_integration.rs`**: Parameterized tests for TLS configuration matrices

### Adding New Parameterized Tests

1. Import rstest: `use rstest::{fixture, rstest};`
2. Create fixtures for common setup
3. Use `#[rstest]` instead of `#[test]`
4. Add `#[case]` attributes for each test scenario
5. Ensure case names clearly describe the scenario

## Snapshot Testing with insta

The project uses `insta` for snapshot testing to ensure output format consistency and catch regressions.

### Snapshot Test Files

- **`tests/output_format_snapshots.rs`**: Snapshot tests for CSV, JSON, and TSV output formats
- **`tests/snapshots/`**: Directory containing snapshot files (`.snap` format)

### Running Snapshot Tests

```bash
# Run snapshot tests
cargo test output_format_snapshots

# Review and accept snapshot changes
cargo insta review

# Accept all snapshot changes (use with caution)
cargo insta accept
```

### Snapshot Coverage

The snapshot tests cover:

- **CSV**: Standard data, escaping (quotes/commas), newlines, null values, empty result sets
- **JSON**: Standard data, pretty-printed output, empty result sets, null handling, large integers, boundary values
- **TSV**: Standard tab-delimited data, special characters, null conversion

### Working with Snapshots

1. **Run tests**: `cargo test output_format_snapshots`
2. **Review changes**: `cargo insta review` (opens interactive review)
3. **Accept changes**: Review and accept if changes are expected
4. **Commit snapshots**: Commit updated `.snap` files to version control

Snapshots are stored in `tests/snapshots/output_formats.snap` and should be committed alongside code changes.

## CI Integration

The tests are designed to work in CI environments:

- Docker-dependent tests are ignored by default
- Unit tests run without external dependencies
- Tests work with always-available rustls TLS implementation
- Tests validate that TLS is available in both standard and minimal builds
- **Benchmarks**: Run on main branch pushes, results uploaded as artifacts
- **Snapshot tests**: Run as part of standard test suite, failures require review

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
