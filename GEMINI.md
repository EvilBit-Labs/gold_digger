# Google Gemini Instructions for Gold Digger

## Project Overview

Gold Digger is a Rust-based MySQL/MariaDB query tool configured via CLI and environment variables that outputs query results to stdout or files in CSV/JSON/TSV formats. It defines essential architecture patterns, safety requirements, and development constraints for headless database automation workflows with CLI-first architecture.

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

**Note:** No dotenv support - use exported environment variables only.

## Security Requirements (NON-NEGOTIABLE)

### Credential Protection

- **NEVER** log `DATABASE_URL` or credentials - implement automatic redaction
- **NEVER** hardcode secrets in code or configuration
- **ALWAYS** validate user inputs before processing

### Airgap Compatibility

- **Allowed**: Configured database connections (MySQL/MariaDB only)
- **Prohibited**: Telemetry, call-home, non-essential outbound connections
- **Runtime**: No external dependencies during execution

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
additional_mysql_types = [...]             # BigDecimal, Decimal, etc.
verbose = []                               # Conditional logging
```

Note: TLS is now always available and is no longer a feature flag.

**TLS Implementation Notes:**

- Single rustls-based implementation with platform certificate store integration
- Simplified from previous dual TLS approach (native-tls vs rustls)
- Consistent cross-platform behavior with enhanced security controls

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

## Development Commands

### Build Variations

```bash
# Standard build (TLS always available)
cargo build --release

# Minimal build (TLS still available)
cargo build --no-default-features --features "csv json"

# Build without extra types
cargo build --release --no-default-features --features "json csv verbose"
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

1. **Memory:** No streaming support - O(row_count × row_width) memory usage
2. **JSON Output:** Uses BTreeMap for deterministic key ordering (implemented)
3. **Version Sync:** CHANGELOG.md vs Cargo.toml version mismatch

## Commit Standards

- **Format**: Conventional commits (`feat:`, `fix:`, `docs:`, etc.)
- **Scopes**: Use `(cli)`, `(db)`, `(output)`, `(tls)`, `(config)`
- **Automation**: cargo-dist handles versioning; git-cliff handles changelog

## CI/CD Pipeline

### GitHub Actions

- **ci.yml**: PR/push validation (format, lint, test, security)
- **release.yml**: Cross-platform artifacts via cargo-dist (auto-generated, DO NOT EDIT)

### Release Requirements

1. All CI checks pass
2. No critical vulnerabilities
3. Cross-platform binaries (x86_64/aarch64 for Linux, macOS, Windows)
4. SHA256 checksums and Cosign signatures
5. SBOM generation (CycloneDX format)

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
