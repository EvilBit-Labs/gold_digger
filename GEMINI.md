# Google Gemini Instructions for Gold Digger

## Project Overview

Gold Digger is a Rust-based MySQL/MariaDB query tool that outputs structured data (CSV/JSON/TSV) via environment variables. It's designed for headless database automation workflows with CLI-first architecture.

## Project File Organization

### Configuration Files

- **Cargo.toml**: Dependencies, features, release profile
- **rustfmt.toml**: Code formatting rules (100-char limit)
- **deny.toml**: Security and license compliance
- **rust-toolchain.toml**: Rust version specification

### Development Automation

- **justfile**: Cross-platform build automation and common tasks
- **.pre-commit-config.yaml**: Git hook configuration for quality gates
- **CHANGELOG.md**: Auto-generated version history (conventional commits)

### Documentation Standards

Required for all public functions with examples:

````rust
/// Converts MySQL rows to string vectors for output formatting.
///
/// # Arguments
/// * `rows` - Vector of MySQL rows from query execution
///
/// # Returns
/// * `Vec<Vec<String>>` - Converted string data ready for format modules
///
/// # Example
/// ```
/// let string_rows = rows_to_strings(mysql_rows)?;
/// csv::write(string_rows, output)?;
/// ```
pub fn rows_to_strings(rows: Vec<mysql::Row>) -> anyhow::Result<Vec<Vec<String>>> {
    // Implementation
}
````

### Error Handling Patterns

- Use `anyhow::Result<T>` for all fallible functions
- Never use `from_value::<String>()` - always handle `mysql::Value::NULL`
- Implement credential redaction in all log output
- Use `?` operator for error propagation

## 🚨 Critical Safety Rules

### Database Value Conversion (PANIC RISK)

```rust
// ❌ NEVER - causes panics on NULL/non-string types
// from_value::<String>(row[column.name_str().as_ref()])
// Use mysql_value_to_string() for CSV/TSV or mysql_value_to_json() for JSON instead

// ✅ ALWAYS - safe NULL handling with dedicated helpers

/// Converts MySQL value to String for CSV/TSV output
fn mysql_value_to_string(mysql_value: &mysql::Value) -> String {
    match mysql_value {
        mysql::Value::NULL => "".to_string(),
        val => from_value_opt::<String>(val.clone()).unwrap_or_else(|_| format!("{:?}", val)),
    }
}

/// Converts MySQL value to serde_json::Value for JSON output
fn mysql_value_to_json(mysql_value: &mysql::Value) -> serde_json::Value {
    match mysql_value {
        mysql::Value::NULL => serde_json::Value::Null,
        val => from_value_opt::<serde_json::Value>(val.clone())
            .unwrap_or_else(|_| serde_json::Value::String(format!("{:?}", val))),
    }
}

// Usage per output format:
// - CSV/TSV: mysql_value_to_string(&mysql_value)
// - JSON: mysql_value_to_json(&mysql_value)
```

### Security (NEVER VIOLATE)

- **NEVER** log `DATABASE_URL` or credentials - always redact
- **NEVER** make external service calls at runtime (offline-first)
- ⚠️ **WARNING**: `CAST(column AS CHAR)` can corrupt binary data or produce mojibake for text in lossy encodings. Use safer alternatives:
  - **BLOB/BINARY columns**: Use `HEX(column)` or `TO_BASE64(column)` for lossless binary representation
  - **Text columns**: Use `CAST(column AS CHAR CHARACTER SET utf8mb4)` or `CONVERT(column USING utf8mb4)` to specify explicit encoding
  - **Numeric/Date columns**: `CAST(column AS CHAR)` is generally safe for these types

### Configuration Architecture

Gold Digger uses CLI-first configuration with environment variable fallbacks:

**CLI Flags (Highest Priority):**

- `--db-url`: Database connection (overrides `DATABASE_URL`)
- `--query`: Inline SQL (mutually exclusive with `--query-file`)
- `--query-file`: SQL from file (mutually exclusive with `--query`)
- `--output`: Output path (overrides `OUTPUT_FILE`)
- `--format`: Force format (csv|json|tsv)

**Environment Variables (Fallback):**

- `DATABASE_URL`: MySQL connection string with optional SSL params
- `DATABASE_QUERY`: SQL query to execute
- `OUTPUT_FILE`: Determines format by extension (.csv/.json/fallback to TSV)

**Resolution Pattern:**

```rust
fn resolve_config_value(cli: &Cli) -> anyhow::Result<String> {
    if let Some(value) = &cli.field {
        Ok(value.clone()) // CLI flag (highest priority)
    } else if let Ok(value) = env::var("ENV_VAR") {
        Ok(value) // Environment variable (fallback)
    } else {
        anyhow::bail!("Missing required configuration") // Error if neither
    }
}
```

**Note:** No dotenv support - use exported environment variables only.

## Security Requirements

### Critical Security Rules

- **Never log credentials:** Implement redaction for `DATABASE_URL` and secrets
- **No hardcoded secrets:** Use environment variables or GitHub OIDC
- **Vulnerability policy:** Block releases with critical vulnerabilities
- **Airgap compatibility:** No telemetry or external calls in production

### Safe Patterns

```rust
// ❌ NEVER log credentials
println!("Connecting to {}", database_url);

// ✅ Always redact sensitive information
println!("Connecting to database...");
```

## Architecture Constraints

### Current Structure

- **Entry:** `src/main.rs` handles CLI parsing and dispatch
- **Core:** `src/lib.rs` contains `rows_to_strings()` (PANIC RISK)
- **Writers:** `src/{csv,json,tab}.rs` handle format-specific output
- **Memory:** Fully materialized results (no streaming)

### Feature Flags

```toml
default = ["json", "csv", "additional_mysql_types", "verbose"]
json = []                                  # JSON output format
csv = []                                   # CSV output format
additional_mysql_types = [...]             # BigDecimal, Decimal, etc.
verbose = []                               # Conditional logging
```

**TLS Implementation Notes:**

- TLS support is always available using rustls (no feature flags required)
- The `ssl` and `ssl-rustls` feature flags have been removed
- All builds include rustls-based TLS with platform certificate store integration

## Development Commands

### Essential Quality Checks

```bash
# Required before any commit
cargo fmt --check           # Formatting validation
cargo clippy -- -D warnings # Zero-tolerance linting
cargo nextest run           # Parallel test execution
```

### Build Variations

```bash
# Standard build with native TLS
cargo build --release

# Standard build (TLS always available)
cargo build --release --no-default-features --features "json csv additional_mysql_types verbose"

# Minimal build
cargo build --no-default-features --features "csv json"
```

### Safe Testing Pattern

```bash
# Always cast columns to avoid panics
OUTPUT_FILE=/tmp/out.json \
DATABASE_URL="mysql://user:pass@host:3306/db" \
DATABASE_QUERY="SELECT CAST(id AS CHAR) as id FROM users LIMIT 5" \
cargo run --release
```

## Known Issues to Address

1. **Pattern Bug:** `Some(&_)` should be `Some(_)` in main.rs
2. **Exit Codes:** Uses `exit(-1)` instead of proper error codes
3. **JSON Output:** Should use BTreeMap for deterministic ordering
4. **Version Sync:** CHANGELOG.md vs Cargo.toml version mismatch

## AI Assistant Guidelines

### When Suggesting Code Changes

1. **Always check for type conversion safety** - recommend SQL casting
2. **Never suggest .env file usage** - use exported environment variables
3. **Target small, reviewable changes** for single-maintainer workflow
4. **Consider streaming implications** for future compatibility
5. **Maintain offline-first principles** - no external service calls

### Testing Recommendations

```toml
[dev-dependencies]
criterion = "0.5"       # Benchmarking
insta = "1"             # Snapshot testing
assert_cmd = "2"        # CLI testing
testcontainers = "0.15" # Database integration
```

## Quick Reference

| Command                       | Purpose                      |
| ----------------------------- | ---------------------------- |
| `cargo fmt --check`           | Verify formatting (required) |
| `cargo clippy -- -D warnings` | Lint with zero tolerance     |
| `cargo nextest run`           | Run tests (preferred)        |
| `cargo tarpaulin`             | Generate coverage reports    |

---

**Maintainer:** UncleSp1d3r\
**Status:** Active development toward v1.0\
**Workflow:** Single-maintainer with CodeRabbit.ai reviews
