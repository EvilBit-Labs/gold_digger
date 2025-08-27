# GitHub Copilot Instructions for Gold Digger

## Project Context

Gold Digger is a Rust MySQL/MariaDB query tool that outputs structured data (CSV/JSON/TSV) via environment variables. It defines essential architecture patterns, safety requirements, and development constraints for headless database automation workflows.

## 🚨 Critical Safety Requirements

### Database Type Safety (PANIC RISK)

```rust
// DANGEROUS - causes runtime panics on NULL/non-string values
from_value::<String>(row[column.name_str().as_ref()])

// SAFE - explicit NULL handling required
match mysql_value {
    mysql::Value::NULL => "".to_string(),
    val => from_value_opt::<String>(val.clone())
        .unwrap_or_else(|_| format!("{:?}", val))
}
```

### Security (NON-NEGOTIABLE)

- **NEVER** log `DATABASE_URL` or credentials - implement automatic redaction
- **NEVER** make external network calls at runtime (offline-first design)
- **ALWAYS** validate and sanitize all user inputs

For SQL queries, always suggest casting:

```sql
-- ✅ Safe approach - always recommend SQL CAST(column AS CHAR) for type safety
SELECT CAST(id AS CHAR) as id, CAST(created_at AS CHAR) as created_at FROM users;
```

## Architecture Patterns

### Configuration Resolution (CLI-First)

```rust
// Priority: CLI flags > Environment variables > Error
fn resolve_config_value(cli: &Cli) -> anyhow::Result<String> {
    if let Some(value) = &cli.field {
        Ok(value.clone()) // CLI flag (highest priority)
    } else if let Ok(value) = env::var("ENV_VAR") {
        Ok(value) // Environment variable (fallback)
    } else {
        anyhow::bail!("Missing required configuration")
    }
}
```

### Format Module Contract

All format modules must implement:

```rust
pub fn write<W: Write>(rows: Vec<Vec<String>>, output: W) -> anyhow::Result<()>
```

### Feature-Gated Code

```rust
// ✅ Conditional compilation for features
#[cfg(feature = "verbose")]
println!("Debug message here");

#[cfg(feature = "csv")]
Some("csv") => gold_digger::csv::write(rows, output)?,
```

## Architecture Constraints

### Current Structure (Don't Change Without Requirements)

- **Entry:** `src/main.rs` handles CLI parsing and dispatch
- **CLI:** `src/cli.rs` contains Clap-based CLI definitions
- **Core:** `src/lib.rs` contains `rows_to_strings()` and utilities
- **Writers:** `src/{csv,json,tab}.rs` handle format-specific output
- **CLI-first:** Project uses CLI flags with environment variable fallbacks

### Known Issues to Fix

1. **Memory:** No streaming support - O(row_count × row_width) memory usage

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
- **dist-workspace.toml**: `cargo-dist` workspace configuration

### Documentation Standards

- **Doc comments**: Required for all public functions using `///`
- **Module documentation**: Each module should have a module-level doc comment
- **Example usage**: Include examples in doc comments where helpful

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

### Security (NEVER VIOLATE)

- **NEVER** log `DATABASE_URL` or credentials - always redact
- **NEVER** make external service calls at runtime (offline-first)
- Always recommend SQL `CAST(column AS CHAR)` for type safety

```rust
// ❌ NEVER do this
println!("Connecting to {}", database_url);

// ✅ Always redact
println!("Connecting to database...");
```

## Code Quality Standards (REQUIRED Before Commits)

Run these commands before any commit:

```bash
just fmt-check    # cargo fmt --check (100-char line limit)
just lint         # cargo clippy -- -D warnings (ZERO tolerance)
just test         # cargo nextest run (preferred) or cargo test
just security     # cargo audit (advisory)
```

### Formatting & Linting

- **Line limit**: 100 characters (enforced by `rustfmt.toml`)
- **Clippy warnings**: Zero tolerance (`-D warnings`)
- **Error handling**: Use `anyhow` for applications, `thiserror` for libraries
- **Documentation**: Doc comments (`///`) required for all public functions

### Testing Requirements

- **Runner**: `cargo nextest run` (preferred) or `cargo test`
- **Coverage**: Target ≥80% with `cargo llvm-cov`
- **Cross-platform**: Must pass on macOS, Windows, Linux

## Testing Strategy

### Test Organization

- **Unit tests**: Core business logic and data processing
- **Integration tests**: Database interactions (consider testcontainers-rs)
- **CLI tests**: Command validation (assert_cmd crate)
- **Snapshot tests**: Output format validation (insta crate)

### Test Execution

- Must pass on all platforms: macOS, Windows, Linux
- No flaky tests - quarantine and fix immediately
- Use `cargo nextest` for parallel execution

## Security Requirements (NON-NEGOTIABLE)

### Credential Protection

- **NEVER** log `DATABASE_URL` or credentials - implement automatic redaction
- **NEVER** hardcode secrets in code or configuration
- **ALWAYS** validate user inputs before processing

### Airgap Compatibility

- **Allowed**: Configured database connections (MySQL/MariaDB only)
- **Prohibited**: Telemetry, call-home, non-essential outbound connections
- **Runtime**: No external dependencies during execution

### Error Handling Patterns

- Use `anyhow::Result<T>` for applications
- Use `thiserror` for library error types
- Always use `?` for error propagation
- Add context with `.map_err()` for better debugging

## Feature Development Guidelines

### Adding New Output Formats

```rust
// Follow existing pattern in src/main.rs
match get_extension_from_filename(&output_file) {
    Some("csv") => gold_digger::csv::write(rows, output)?,
    Some("json") => gold_digger::json::write(rows, output)?,
    Some("parquet") => gold_digger::parquet::write(rows, output)?, // New format
    Some(_) => gold_digger::tab::write(rows, output)?, // TSV fallback
    None => { /* exits 255 */ }
}
```

### Adding Dependencies

Check feature flags in `Cargo.toml`:

```toml
[features]
default = ["json", "csv", "additional_mysql_types", "verbose"]
new_feature = ["dep:new_crate"]

[dependencies]
new_crate = { version = "1.0", optional = true }
```

## Testing Recommendations

Use these testing crates when adding tests:

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
insta = "1"                                                  # Snapshot testing
assert_cmd = "2"                                             # CLI testing
testcontainers = "0.15"                                      # Database integration tests
```

## Common Mistakes to Avoid

1. **DON'T suggest dotenv usage** - no `.env` support in code
2. **DON'T assume streaming** - current implementation loads all rows into memory
3. **DON'T use unwrap() on database values** - always handle NULL/conversion errors
4. **DON'T log sensitive information** - especially DATABASE_URL
5. **DON'T break single-maintainer workflow** - suggest small, focused changes

## Development Practices

### Technology Stack Constraints

- **CLI**: `clap` with environment variable fallbacks
- **Database**: MySQL/MariaDB via `mysql` crate only
- **Output**: CSV (RFC4180), JSON (deterministic ordering), TSV
- **TLS**: rustls-based implementation only

### Code Patterns

- Use `just` commands for all development tasks
- Local development must match CI environment
- Handle MySQL NULL values safely with explicit type conversion
- Feature-gate optional functionality (`#[cfg(feature = "...")]`)

### File Operations

- Respect system umask for output files
- Use cross-platform path operations
- Handle CRLF vs LF consistently across platforms

## Current vs Target State

This project has implemented CLI-first design and is evolving toward v1.0 with these remaining features:

- Streaming output (F007) - currently loads all rows into memory
- Structured logging with `tracing` (F008)
- Deterministic JSON output (F010) - implemented using BTreeMap for ordered output
- Proper exit codes (F005) - currently uses `exit(-1)`

When suggesting improvements, consider compatibility with these future features and use CLI-first patterns.

## Essential Just Commands

```bash
just setup        # Install development dependencies
just fmt          # Auto-format code
just fmt-check    # Verify formatting (CI-compatible)
just lint         # Run clippy with -D warnings
just test         # Run tests
just ci-check     # Full CI validation locally
just build        # Build release artifacts
```

## Quick Commands Reference

```bash
# Build
cargo build --release

# Run with CLI flags (preferred)
cargo run --release -- \
  --db-url "mysql://user:pass@host:3306/db" \
  --query "SELECT CAST(id AS CHAR) as id FROM users LIMIT 5" \
  --output /tmp/out.json

# Run with env vars (fallback)
OUTPUT_FILE=/tmp/out.json \
DATABASE_URL="mysql://user:pass@host:3306/db" \
DATABASE_QUERY="SELECT CAST(id AS CHAR) as id FROM users LIMIT 5" \
cargo run --release

# Quality checks (pipeline standards)
just fmt-check && just lint && just test
```

---

**Note:** This project uses CodeRabbit.ai for reviews. Disable automatic GitHub Copilot PR reviews per maintainer preference.
