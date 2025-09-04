# AGENTS.md

This file provides guidance for AI assistants working with the Gold Digger codebase.

## Project Overview

Gold Digger is a Rust-based MySQL/MariaDB query tool that outputs results in CSV, JSON, or TSV formats. It's designed for headless operation via environment variables, making it ideal for database automation workflows.

**Key Characteristics:**

- CLI-first (uses Clap) with environment variable overrides
- Outputs to structured formats based on file extension
- Fully materialized result sets (no streaming)
- Single-maintainer project by UncleSp1d3r
- Under active development toward v1.0

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
        val => from_value_opt::<String>(val.clone())
            .map(serde_json::Value::String)
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

### Other Critical Issues

1. **No Dotenv Support:** Despite README implications, there is no `.env` file support in the code. Use exported environment variables only.

2. **Non-Standard Exit Codes:** `exit(-1)` becomes exit code 255, not the standard codes specified in requirements.

3. **JSON Output:** Uses BTreeMap for deterministic key ordering as required.

4. **Pattern Matching Bug:** In `src/main.rs`, the `if let Some(url) = &cli.db_url` pattern (and similar patterns in the resolve functions) uses `Some(&_)` which should be `Some(_)` in the match arm.

### Configuration Architecture

Gold Digger uses CLI-first configuration with environment variable fallbacks:

**CLI Flags (Highest Priority):**

- `--db-url <URL>`: Database connection (overrides `DATABASE_URL`)
- `--query <SQL>`: Inline SQL (mutually exclusive with `--query-file`)
- `--query-file <FILE>`: SQL from file (mutually exclusive with `--query`)
- `--output <FILE>`: Output path (overrides `OUTPUT_FILE`)
- `--format <FORMAT>`: Force format (csv|json|tsv)

**Environment Variables (Fallback):**

- `DATABASE_URL`: MySQL/MariaDB connection string with optional SSL parameters
- `DATABASE_QUERY`: SQL query string to execute
- `OUTPUT_FILE`: Path to output file (extension determines format: .csv, .json, or defaults to TSV)

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

### Current Architecture

**Entry Point (`src/main.rs`):**

- Reads 3 required env vars, exits with 255 if missing
- Creates MySQL connection pool, fetches ALL rows into memory
- Exits with code 1 if result set is empty
- Dispatches to writer based on file extension

**Core Library (`src/lib.rs`):**

- `rows_to_strings()`: Converts `Vec<Row>` to `Vec<Vec<String>>` (PANICS on NULL/non-string)
- `get_extension_from_filename()`: Simple extension parsing

**Output Writers:**

- `csv.rs`: RFC 4180-ish with `QuoteStyle::Necessary`
- `json.rs`: `{"data": [{...}]}` using BTreeMap (deterministic ordering)
- `tab.rs`: TSV with `\t` delimiter and `QuoteStyle::Necessary`

## Development Commands

### Essential Commands

```bash
# Build (release recommended for testing)
cargo build --release

# Quality gates (see "Code Quality Standards" section below for commands)

# Run with CLI flags (preferred)
cargo run --release -- \
  --db-url "mysql://user:pass@host:3306/db" \
  --query "SELECT CAST(id AS CHAR) as id FROM table LIMIT 5" \
  --output /tmp/out.json

# Run with environment variables (fallback)
OUTPUT_FILE=/tmp/out.json \
DATABASE_URL="mysql://user:pass@host:3306/db" \
DATABASE_QUERY="SELECT CAST(id AS CHAR) as id FROM table LIMIT 5" \
cargo run --release
```

### Feature Flags

- `default`: `["json", "csv", "ssl", "additional_mysql_types", "verbose"]`
- `ssl`: MySQL native TLS support using platform-native libraries (SChannel on Windows, SecureTransport on macOS, may use OpenSSL on Linux)
- `ssl-rustls`: Pure Rust TLS implementation (alternative to native TLS)
- `additional_mysql_types`: Support for BigDecimal, Decimal, Time, Frunk
- `verbose`: Conditional logging via println!/eprintln!

**Important**: `ssl` and `ssl-rustls` are mutually exclusive features.

## Requirements Gap Analysis

The project has detailed requirements in `project_spec/requirements.md` but significant gaps exist:

### High Priority Missing Features

- **F001-F003:** CLI interface exists (clap-based); finalize CLI flag precedence and documented flags
- **F005:** Non-standard exit codes (should be 0=success, 1=no rows, 2=config error, etc.)
- **F014:** Type conversion panics on NULL/non-string values
- **Extension dispatch bug fix**

### Medium Priority

- **F007:** Streaming output (currently loads all rows into memory)
- **F008:** Structured logging with credential redaction
- **F010:** JSON output uses BTreeMap for deterministic ordering, pretty-print option

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

All public functions require comprehensive doc comments:

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

### Security Requirements

#### Critical Security Rules

- **Never log credentials:** Implement redaction for `DATABASE_URL` and secrets
- **No hardcoded secrets:** Use environment variables or GitHub OIDC
- **Vulnerability policy:** Block releases with critical vulnerabilities
- **Airgap compatibility:** No telemetry or external calls in production
- **Configure TLS programmatically:** Use `mysql::OptsBuilder` and `SslOpts` instead of URL parameters
- **TLS Implementation:** Supports both platform-native TLS via the `ssl` feature and pure Rust TLS via the `ssl-rustls` feature

#### Error Handling Patterns

- Use `anyhow::Result<T>` for all fallible functions
- Never use `from_value::<String>()` - always handle `mysql::Value::NULL`
- Implement credential redaction in all log output
- Use `?` operator for error propagation

#### Credential Redaction Example

```rust
use regex::Regex;
use std::sync::OnceLock;

static CREDENTIAL_REGEX: OnceLock<Regex> = OnceLock::new();

/// Redacts database credentials from connection URLs for safe logging
/// Replaces "user:pass@" with "****:****@" to prevent credential exposure
fn redact_database_url(url: &str) -> String {
    let regex = CREDENTIAL_REGEX.get_or_init(|| {
        Regex::new(r"([^/]+):([^@]+)@").unwrap_or_else(|_| {
            // Fallback regex that matches any credential pattern
            Regex::new(r".*@").unwrap()
        })
    });

    regex.replace(url, "****:****@").to_string()
}

// Usage example:
// let safe_url = redact_database_url("mysql://user:secret@localhost:3306/db");
// Result: "mysql://****:****@localhost:3306/db"
```

**Note:** Add `regex = "1"` to `Cargo.toml` dependencies. The `OnceLock` ensures thread-safe, one-time regex compilation.

## Common Tasks for AI Assistants

### Safe Query Testing

Always recommend casting non-string columns:

```sql
-- ❌ This will panic on NULL or non-string types
SELECT id, created_at FROM users;

-- ✅ This is safe
SELECT CAST(id AS CHAR) as id, CAST(created_at AS CHAR) as created_at FROM users;
-- Note: For BLOB/BINARY columns, use HEX(column) or TO_BASE64(column)
-- For text columns with encoding concerns, use CAST(column AS CHAR CHARACTER SET utf8mb4)
```

### Adding New Features

1. Check requirements in `project_spec/requirements.md` for context
2. Consider impact on streaming (F007 requirement)
3. Maintain backward compatibility with current env var interface
4. Add tests using recommended test crates: `criterion`, `insta`, `assert_cmd`

### Version Management

- Current discrepancy: CHANGELOG.md shows v0.2.6, Cargo.toml shows v0.2.5
- Sync versions before any releases
- Use semantic versioning with conventional commits

## Testing Strategy

### Recommended Test Dependencies

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
insta = "1"
rstest = "0.18"
assert_cmd = "2"
tempfile = "3"
testcontainers = "0.15"                                      # For real MySQL/MariaDB testing
```

### Test Categories

1. **Unit Tests:** `rows_to_strings`, output writers, extension parsing
2. **Snapshot Tests:** Golden file validation for output formats
3. **Integration Tests:** Real database connectivity with testcontainers
4. **CLI Tests:** End-to-end with environment variables
5. **Benchmarks:** Performance regression detection

## AI Assistant Best Practices

1. **Always check for the type conversion panic issue** when working with queries
2. **Recommend SQL casting** for any query involving non-string columns
3. **Never suggest .env file usage** - use exported environment variables
4. **Be aware of the single-maintainer workflow** - target small, reviewable changes
5. **Check feature flags** when suggesting new dependencies or functionality
6. **Consider streaming implications** for any changes affecting row processing
7. **Maintain offline-first principles** - no external service calls at runtime

## Quick Reference

| File                           | Purpose         | Key Issues                                |
| ------------------------------ | --------------- | ----------------------------------------- |
| `src/main.rs`                  | Entry point     | Exit codes, pattern bug, env var handling |
| `src/lib.rs`                   | Core logic      | Type conversion panics, NULL handling     |
| `src/json.rs`                  | JSON output     | Non-deterministic HashMap                 |
| `Cargo.toml`                   | Dependencies    | Version mismatch with CHANGELOG           |
| `project_spec/requirements.md` | Target features | Comprehensive feature roadmap             |

---

**Maintainer:** UncleSp1d3r
**Workflow:** Single-maintainer with CodeRabbit.ai reviews
**Status:** Active development toward v1.0
