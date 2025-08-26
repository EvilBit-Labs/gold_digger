# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Purpose and Quick Start

Gold Digger is a Rust-based MySQL/MariaDB query tool that outputs results in CSV, JSON, or TSV formats. It defines essential architecture patterns, safety requirements, and development constraints for headless database automation workflows.

**Basic usage:**

```bash
export OUTPUT_FILE="/tmp/output.json"
export DATABASE_URL="mysql://user:pass@host:3306/database"
export DATABASE_QUERY="SELECT id, name FROM users LIMIT 10"
cargo run --release
```

The output format is determined by file extension: `.csv`, `.json`, or anything else defaults to TSV.

## Essential Development Commands

### Build and Install

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Standard build (TLS always available)
cargo build --release

# Minimal build (no default features)
cargo build --no-default-features --features "csv json"

# Install locally from workspace
cargo install --path .

# Install from crates.io (when published)
cargo install gold_digger
```

### Lint and Format (Required for PRs)

```bash
# Check formatting (enforced)
cargo fmt --check

# Run clippy with warnings as errors (enforced)
cargo clippy -- -D warnings

# Fix formatting
cargo fmt
```

### Testing

```bash
# Run tests (currently minimal)
cargo test
```

### Running with Environment Variables

**Linux/macOS:**

```bash
OUTPUT_FILE=/tmp/out.json \
DATABASE_URL="mysql://user:pass@host:3306/db" \
DATABASE_QUERY="SELECT 1 as x" \
cargo run --release
```

**Windows PowerShell:**

```powershell
$env:OUTPUT_FILE="C:\temp\out.json"
$env:DATABASE_URL="mysql://user:pass@host:3306/db"
$env:DATABASE_QUERY="SELECT 1 as x"
cargo run --release
```

**⚠️ Important:** Despite README mentions, there is NO dotenv support in the code. Use exported environment variables or an external env loader.

## Architecture and Data Flow

### Current Implementation (v0.2.6)

**Entry Point (`src/main.rs`):**

- Uses CLI-first configuration with environment variable fallbacks
- Configuration resolution pattern: CLI flags override environment variables
- Reads required config: `--db-url`/`DATABASE_URL`, `--query`/`DATABASE_QUERY`, `--output`/`OUTPUT_FILE`
- Exits with code 255 (due to `exit(-1)`) if any are missing
- Creates MySQL connection pool and fetches ALL rows into memory (`Vec<Row>`)
- Exits with code 1 if result set is empty
- Dispatches to writer based on output file extension

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

**Core Library (`src/lib.rs`):**

- `rows_to_strings()`: Converts `Vec<Row>` to `Vec<Vec<String>>`, building header from first row metadata
- `get_extension_from_filename()`: Simple extension parsing
- **⚠️ Critical:** Uses `mysql::from_value::<String>()` which **WILL PANIC** on NULL or non-string values

**Output Writers:**

- `csv.rs`: RFC 4180-ish with `QuoteStyle::Necessary`
- `json.rs`: Produces `{"data": [{...}]}` structure using BTreeMap (deterministic key order)
- `tab.rs`: TSV with `\t` delimiter and `QuoteStyle::Necessary`

**Performance Characteristics:**

- Fully materialized result sets (not streaming)
- Memory usage scales linearly with row count
- No connection pooling optimization

### Feature Flags (Cargo.toml)

- `default`: `["json", "csv", "additional_mysql_types", "verbose"]`
- `additional_mysql_types`: Support for BigDecimal, Decimal, Time, Frunk
- `verbose`: Conditional logging via println!/eprintln!

Note: TLS is now always available and is no longer a feature flag.

## Output Format Dispatch and Edge Cases

### Extension Dispatch Logic

```rust
match get_extension_from_filename(&output_file) {
    Some("csv") => gold_digger::csv::write(rows, output)?,
    Some("json") => gold_digger::json::write(rows, output)?,
    Some(_) => gold_digger::tab::write(rows, output)?,
    None => { /* exits 255 */ }
}
```

**Note:** The extension dispatch logic has been corrected to use proper pattern matching for fallback cases.

### Known Issues

1. **Memory:** No streaming support - O(row_count × row_width) memory usage
2. **Extension Confusion:** `.txt` mentioned in README but dispatches to TSV
3. **Missing Features:** No `--pretty` JSON flag, no format override option

### Output Schemas

- **CSV:** Headers in first row, `QuoteStyle::Necessary`
- **JSON:** `{"data": [{"col1": "val1", "col2": "val2"}, ...]}` with BTreeMap for deterministic key ordering
- **TSV:** Tab-delimited, `QuoteStyle::Necessary`

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

## Critical Gotchas and Invariants

### Memory and Performance

- All rows loaded into memory before processing
- No streaming support (required by F007 in requirements)
- Use `conn.query_iter()` for streaming when implementing

### Exit Codes

- `exit(-1)` becomes exit code 255 (not standard)
- Requirements call for specific exit codes: 0 (success), 1 (no rows), 2 (config error), etc.

### README vs. Code Mismatches

- **No dotenv support** despite README implications
- Install command should be `cargo install --path .` not `cargo install`
- Verbose logging is feature-gated, not always available

## Current vs. Target Requirements Gap Analysis

Based on `project_spec/requirements.md`, major missing features:

### High Priority (Blocking)

- **F001-F003:** No CLI interface (clap), no config precedence, no `--query-file`, `--format` flags
- **F005:** Non-standard exit codes
- **F014:** Type conversion panics on NULL/non-string values
- **Extension dispatch bug fix**

### Medium Priority

- **F007:** Streaming output for large result sets
- **F008:** Structured logging with credential redaction
- **F010:** Pretty-print JSON option (deterministic ordering implemented via BTreeMap)

### Low Priority

- **F009:** Shell completion generation
- **F012:** Machine-readable `--dump-config`
- **F013:** `--allow-empty` flag

## Development Workflow and Conventions

### Project File Organization

**Configuration Files:**

- **Cargo.toml**: Dependencies, features, release profile
- **rustfmt.toml**: Code formatting rules (100-char limit)
- **deny.toml**: Security and license compliance
- **rust-toolchain.toml**: Rust version specification

**Development Automation:**

- **justfile**: Cross-platform build automation and common tasks
- **.pre-commit-config.yaml**: Git hook configuration for quality gates
- **CHANGELOG.md**: Auto-generated version history (conventional commits)

**Documentation Standards:**
All public functions require doc comments with examples:

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

### Recommended Justfile

```justfile
default: lint

setup:
    cd {{justfile_dir()}}
    rustup component add rustfmt clippy

fmt:
    cd {{justfile_dir()}}
    cargo fmt

fmt-check:
    cd {{justfile_dir()}}
    cargo fmt --check

lint:
    cd {{justfile_dir()}}
    cargo clippy -- -D warnings

build:
    cd {{justfile_dir()}}
    cargo build --release

run OUTPUT_FILE DATABASE_URL DATABASE_QUERY:
    cd {{justfile_dir()}}
    OUTPUT_FILE={{OUTPUT_FILE}} DATABASE_URL={{DATABASE_URL}} DATABASE_QUERY={{DATABASE_QUERY}} cargo run --release

test:
    cd {{justfile_dir()}}
    cargo nextest run

ci-check: fmt-check lint test

security:
    cd {{justfile_dir()}}
    cargo audit
```

## Testing Strategy

### Current State

- Minimal/no existing tests
- No integration test suite

### Recommended Test Architecture

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
2. **Snapshot Tests (insta):** Golden file validation for output formats
3. **Integration Tests (testcontainers):** Real database connectivity
4. **CLI Tests (assert_cmd):** End-to-end with environment variables
5. **Benchmarks (criterion):** Performance regression detection

## CI/CD and Release Management

- **GitHub Actions:** CI/CD pipeline
- **cargo-dist:** Release management and distribution
- **GitHub Releases:** Release artifacts
- **GitHub Pages:** Documentation deployment
- NOTE: `.github/workflows/release.yml` is automatically generated and should not be altered.

## Security and Operational Guidelines

### Critical Security Requirements

- **Never log credentials:** Implement redaction for `DATABASE_URL` and secrets
- **No hardcoded secrets:** Use environment variables or GitHub OIDC
- **Vulnerability policy:** Block releases with critical vulnerabilities
- **Airgap compatibility:** No telemetry or external calls in production
- **Respect system umask** for output files

### Error Handling Patterns

- Use `anyhow::Result<T>` for all fallible functions
- Never use `from_value::<String>()` - always handle `mysql::Value::NULL`
- Implement credential redaction in all log output
- Use `?` operator for error propagation

## Technology Stack

### Core Dependencies

- **CLI**: `clap` with `derive` and `env` features
- **Database**: `mysql` crate with `rustls-tls` (pure Rust TLS)
- **Output**: `serde_json` (BTreeMap), `csv` crate (RFC4180)
- **Errors**: `anyhow` (applications), `thiserror` (libraries)

### Feature Flags

```toml
default = ["json", "csv", "additional_mysql_types", "verbose"]
additional_mysql_types = ["bigdecimal", "rust_decimal", "chrono", "uuid"]
```

Note: TLS is now always available and is no longer a feature flag.

## GitHub Interactions

**⚠️ Important:** When directed to interact with GitHub (issues, pull requests, repositories, etc.), prioritize using the `gh` CLI tool if available. The `gh` tool provides comprehensive GitHub functionality including:

- Creating and managing issues and pull requests
- Repository operations (cloning, forking, etc.)
- GitHub Actions workflow management
- Release management
- Authentication with GitHub API

**Usage examples:**

```bash
# Check if gh is available
gh --version

# Common operations
gh issue create --title "Bug: Type conversion panic" --body "Details..."
gh pr create --title "Fix: Extension dispatch pattern" --body "Fixes the Some(&_) bug"
gh repo view UncleSp1d3r/gold_digger
gh workflow list
```

Fall back to other GitHub integration methods only if `gh` is not available or doesn't support the required functionality.

## First PR Checklist for AI Agents

Before submitting any changes:

- [ ] Run `cargo fmt --check` and `cargo clippy -- -D warnings` locally
- [ ] Avoid logging secrets or connection details
- [ ] Target small, reviewable changes
- [ ] Use conventional commit messages
- [ ] Add/update snapshot tests when touching output formats
- [ ] Test with various data types if modifying row conversion
- [ ] Document any new environment variables or flags

## Appendix: Feature Flags and Build Matrix

### Feature Combinations

```bash
# Default build (TLS always available)
cargo build --release

# Minimal build (no extra types, TLS still available)
cargo build --no-default-features --features "csv json"

# Database admin build (all MySQL types)
cargo build --release --features "default additional_mysql_types"
```

### Dependencies by Feature

- **Base:** `mysql`, `anyhow`, `csv`, `serde_json`, `clap`
- **TLS:** `mysql/rustls-tls`, `rustls`, `rustls-native-certs`, `rustls-pemfile` (always included - pure Rust implementation with platform certificate store integration)
- **Types:** `mysql_common` with bigdecimal, rust_decimal, time, frunk
- **No native TLS dependencies** in any configuration

---

**Note:** This project is under active development toward v1.0. Refer to `project_spec/requirements.md` for the complete roadmap. Maintainer handle: `UncleSp1d3r`. Single-maintainer workflow with CodeRabbit.ai reviews.
