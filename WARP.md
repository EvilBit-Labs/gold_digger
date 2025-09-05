# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

Gold Digger is a production-ready Rust CLI tool for MySQL/MariaDB database queries with structured
output (CSV, JSON, TSV). It features comprehensive CLI interface, rustls-only TLS, and safe data
type handling.

**Current Architecture (v0.2.6):**

- CLI-first with environment variable fallbacks using `clap`
- Rustls-only TLS implementation (no OpenSSL dependencies)
- Safe MySQL value conversion with NULL handling
- Structured exit codes and error handling
- Modular output format system

**Command Examples:**

```bash
# CLI interface (preferred)
gold_digger --db-url "mysql://user:pass@host:3306/db" \
            --query "SELECT id, name FROM users" \
            --output results.json --pretty

# Environment variables (legacy support)
DATABASE_URL="mysql://user:pass@host:3306/db" \
DATABASE_QUERY="SELECT * FROM table" \
OUTPUT_FILE="/tmp/data.csv" \
cargo run --release
```

## Development Workflow

**Use the justfile for all development tasks:**

```bash
# Setup development environment
just setup

# Run quality checks (format, lint, test)
just check

# Build release version
just build-release

# Run comprehensive CI checks locally
just ci-full

# Run tests (includes integration tests with Docker)
just test

# Run tests without Docker dependencies
just test-no-docker

# Generate coverage reports
just coverage

# Security auditing
just audit
```

**Direct cargo commands when needed:**

```bash
# Format code (enforced)
cargo fmt

# Lint with strict warnings (enforced)
cargo clippy -- -D warnings

# Test all features
cargo test --all-features

# Build with specific features
cargo build --release --features "json csv additional_mysql_types verbose"
```

## Code Architecture

### Module Structure

```text
src/
├── main.rs           # CLI entry point, argument parsing, execution flow
├── lib.rs            # Core library, safe value conversion, module exports
├── cli.rs            # Clap CLI definitions, argument structures
├── exit.rs           # Structured exit codes and error handling
├── tls.rs            # Rustls TLS configuration and certificate management
├── csv.rs            # CSV output writer (RFC 4180 compliant)
├── json.rs           # JSON output writer (deterministic, pretty-print support)
└── tab.rs            # TSV output writer (tab-delimited)
```

### Key Functions and Safety Rules

**Data Conversion (`src/lib.rs`)**:

```rust,ignore
// ✅ SAFE - Always use this pattern
pub fn rows_to_strings(rows: Vec<Row>) -> anyhow::Result<Vec<Vec<String>>> {
    // Safe iteration with proper NULL handling
}

fn mysql_value_to_string(value: &mysql::Value) -> anyhow::Result<String> {
    match value {
        mysql::Value::NULL => Ok(String::new()),
        // ... handles all types safely
    }
}

// ❌ NEVER USE - Will panic on NULL/type mismatches
// mysql::from_value::<String>(row[column.name_str().as_ref()])
```

**Configuration Resolution (`src/main.rs`)**:

```rust,ignore
// CLI flags override environment variables
let database_url = cli.db_url
    .or_else(|| env::var("DATABASE_URL").ok())
    .ok_or_else(|| anyhow::anyhow!("Missing DATABASE_URL"))?;
```

**Exit Codes (`src/exit.rs`)**:

- 0: Success
- 1: No rows found (unless `--allow-empty`)
- 2: Configuration error
- 3: Database connection error
- 4: Query execution error
- 5: File I/O error
- 6: TLS configuration error

## Output Format System

### Format Selection Logic

```rust
// Priority: CLI flag > file extension > default TSV
let format = cli.format.unwrap_or_else(|| {
    OutputFormat::from_extension(&output_file)
});

match format {
    OutputFormat::Csv => csv::write(rows, output)?,
    OutputFormat::Json => json::write(rows, output, cli.pretty)?,
    OutputFormat::Tsv => tab::write(rows, output)?,
}
```

### Output Format Specifications

**CSV (`src/csv.rs`)**:

- RFC 4180 compliant
- Headers in first row
- `QuoteStyle::Necessary` for minimal escaping

**JSON (`src/json.rs`)**:

- Schema: `{"data": [{"col": "value", ...}, ...]}`
- Deterministic field ordering (BTreeMap)
- Pretty-print support via `--pretty` flag
- NULL values preserved as `null`

**TSV (`src/tab.rs`)**:

- Tab-delimited fields
- Headers in first row
- Minimal escaping for tab/newline characters

## 🚨 Critical Coding Rules

### Database Value Conversion

```rust
// ✅ CORRECT - Use the safe conversion in lib.rs
let rows = rows_to_strings(mysql_rows)?;  // Always safe

// ✅ CORRECT - When adding new value handling
fn mysql_value_to_string(value: &mysql::Value) -> anyhow::Result<String> {
    match value {
        mysql::Value::NULL => Ok(String::new()),
        mysql::Value::Bytes(bytes) => {
            // Handle binary data safely
            match std::str::from_utf8(bytes) {
                Ok(s) => Ok(s.to_string()),
                Err(_) => Ok(format!("[BINARY:{}bytes]", bytes.len())),
            }
        },
        // ... handle all variants explicitly
    }
}

// ❌ NEVER USE - Will panic
// mysql::from_value::<String>(row[index])
// row[column.name_str().as_ref()]  // Direct indexing
```

### Security Requirements

**Credential Protection**:

```rust
// ✅ ALWAYS redact credentials in error messages
fn redact_sql_error(message: &str) -> String {
    // Use the regex patterns in main.rs for credential redaction
    // Patterns handle passwords, tokens, connection strings
}

// ✅ NEVER log raw DATABASE_URL
if cli.verbose > 0 {
    println!("Connecting to database...");  // Generic message only
}
```

**Query Safety**:

- **AVOID**: `CAST(column AS CHAR)` on binary data (causes corruption)
- **USE**: `HEX(column)` or `TO_BASE64(column)` for binary columns
- **USE**: `CAST(column AS CHAR CHARACTER SET utf8mb4)` for text with encoding issues

## Implementation Notes

### Memory Model

- **Current**: Fully materialized result sets (`Vec<Row>`)
- **Future**: Streaming support will use `conn.query_iter()` for large datasets
- **Implication**: Memory usage scales with result set size

### Error Handling Patterns

```rust
// ✅ ALWAYS use structured exit codes
use gold_digger::exit::{exit_no_rows, exit_success, exit_with_error};

// ✅ ALWAYS use anyhow::Result for error propagation
fn function_that_can_fail() -> anyhow::Result<T> {
    // Use .context() to add error context
    operation().context("Descriptive error context")?;
    Ok(result)
}
```

## Feature Status (v0.2.6)

### ✅ Implemented

- CLI interface with clap (`src/cli.rs`)
- Configuration precedence (CLI > env vars)
- Format override (`--format` flag)
- Proper exit codes (`src/exit.rs`)
- Safe type conversion (`src/lib.rs`)
- Shell completion generation
- Pretty-print JSON (`--pretty`)
- Allow empty results (`--allow-empty`)
- Configuration dumping (`--dump-config`)
- Credential redaction in errors
- Rustls-only TLS with certificate options

### 🚧 Planned

- Query file support (`--query-file`)
- Streaming output for large datasets
- Enhanced structured logging
- Connection pooling optimization

## Development Standards

### Code Style

- **Formatting**: Use `cargo fmt` (rustfmt.toml defines 100-char limit)
- **Linting**: `cargo clippy -- -D warnings` (zero warnings policy)
- **Documentation**: All public functions require doc comments with examples

### Testing Strategy

```rust
// Integration tests use testcontainers for real MySQL/MariaDB
#[cfg(test)]
mod integration_tests {
    use testcontainers_modules::{mysql::Mysql, testcontainers::runners::SyncRunner};

    #[test]
    fn test_mysql_connection() {
        let mysql = Mysql::default().start().unwrap();
        // Test against real MySQL instance
    }
}
```

### Security Practices

- All credentials must be redacted in logs/errors
- TLS verification is default (disable only for testing)
- No hardcoded secrets in code
- Regular security audits via `just audit`

### TLS Configuration (Rustls-Only)

**Architecture**: Gold Digger uses rustls exclusively (no OpenSSL dependencies)

```rust
// TLS options are mutually exclusive (enforced by clap)
#[derive(Args, Debug, Clone)]
#[group(id = "tls_mode", multiple = false)]
pub struct TlsOptions {
    /// Custom CA certificate file
    #[arg(long, group = "tls_mode")]
    pub tls_ca_file: Option<PathBuf>,

    /// Skip hostname verification (testing only)
    #[arg(long, group = "tls_mode")]
    pub insecure_skip_hostname_verify: bool,

    /// Disable all certificate validation (dangerous)
    #[arg(long, group = "tls_mode")]
    pub allow_invalid_certificate: bool,
}
```

**Usage Examples**:

```bash
# Production (default) - uses system certificate store
gold_digger --db-url "mysql://user:pass@secure.db:3306/app"

# Custom CA certificate
gold_digger --tls-ca-file /path/to/ca.pem --db-url "mysql://..."

# Testing with self-signed certificates (WARNING: insecure)
gold_digger --allow-invalid-certificate --db-url "mysql://..."
```

## Testing Approach

### Test Categories

**Unit Tests**: Core functionality in each module

```rust
#[test]
fn test_mysql_value_conversion() {
    use mysql::Value;
    let result = mysql_value_to_string(&Value::NULL).unwrap();
    assert_eq!(result, "");
}
```

**Integration Tests**: Real databases via testcontainers

```rust
#[test]
#[ignore] // Requires Docker
fn test_mysql_integration() {
    let mysql = Mysql::default().start().unwrap();
    // Test against real MySQL instance
}
```

**CLI Tests**: End-to-end command testing

```rust
use assert_cmd::Command;

#[test]
fn test_cli_help() {
    Command::cargo_bin("gold_digger")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}
```

**Running Tests**:

```bash
just test-no-docker  # Fast unit tests
just test            # All tests including Docker integration
```

## Common Development Tasks

### Adding New CLI Options

1. Add to `src/cli.rs` Cli struct with appropriate clap attributes
2. Handle the option in `src/main.rs` configuration resolution
3. Add tests in `tests/cli_tests.rs`
4. Update help text and documentation

### Adding New Output Formats

1. Create new module `src/format_name.rs`
2. Implement writer function following existing patterns
3. Add format to `OutputFormat` enum in `src/cli.rs`
4. Add dispatch case in `src/main.rs`
5. Add integration tests

### Modifying Value Conversion

1. **NEVER** use direct indexing or `from_value::<T>()`
2. Modify `mysql_value_to_string()` in `src/lib.rs`
3. Add comprehensive test cases for new types
4. Ensure NULL handling remains safe

## AI Assistant Guidelines

### When Writing Code

**Always**:

- Use the safe conversion functions in `src/lib.rs`
- Follow the structured exit codes from `src/exit.rs`
- Use `anyhow::Result<T>` for error handling
- Add appropriate error context with `.context()`
- Redact credentials in error messages
- Follow the CLI patterns established in `src/cli.rs`

**Never**:

- Use direct row indexing: `row[index]`
- Use `mysql::from_value::<String>()` directly
- Log raw connection strings or credentials
- Make assumptions about data types without NULL checking

### Testing New Features

```bash
# Always run before proposing changes
just check           # Fast validation
just ci-full        # Complete CI equivalent
just test-no-docker # Unit tests
```

### Project Context

**Maintainer**: UncleSp1d3r (single-maintainer workflow)\
**Repository**: <https://github.com/EvilBit-Labs/gold_digger>\
**License**: MIT\
**Current Version**: v0.2.6\
**Target**: v1.0 with streaming support and enhanced performance\
**Dependencies**:

- **Core**: mysql, anyhow, clap, serde_json
- **TLS**: rustls, rustls-native-certs (no OpenSSL)
- **Output**: csv crate for CSV, custom JSON/TSV writers
- **Development**: testcontainers, assert_cmd, insta for testing

**Build Targets**:

- Cross-platform (Windows, macOS, Linux)
- Static binaries with cargo-dist
- GitHub Actions CI/CD

**Dependencies**:

- **Core**: mysql, anyhow, clap, serde_json
- **TLS**: rustls, rustls-native-certs (no OpenSSL)
- **Output**: csv crate for CSV, custom JSON/TSV writers
- **Development**: testcontainers, assert_cmd, insta for testing

**Build Targets**:

- Cross-platform (Windows, macOS, Linux)
- Static binaries with cargo-dist
- GitHub Actions CI/CD

---

**This guidance document should be updated when major architectural changes occur. For current
project status, see Cargo.toml version and recent commit history.**
