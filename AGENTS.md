# AGENTS.md

This file provides guidance for AI coding assistants working with the Gold Digger Rust codebase.

## Rules of Engagement for AI Assistants

### File Priority Order

Always consult these files in order when working with this codebase:

1. **AGENTS.md** (this file) - Primary AI assistant guidance
2. **GEMINI.md** - Gemini-specific overrides (if present)
3. **.cursor/rules/**/\*.mdc\*\* - Cursor-specific rules (if present)
4. **.github/copilot-instructions.md** - Copilot-specific guidance

### Critical Restrictions

- **NEVER** commit code, switch branches, or alter git settings without explicit maintainer permission
- **NEVER** log raw `DATABASE_URL`, connection strings, or credentials
- **NEVER** use direct MySQL row indexing: `row[index]` or `mysql::from_value::<String>()`
- **ALWAYS** ask clarifying questions before making risky changes
- **ALWAYS** run `just check` and validate changes before proposing them
- **ALWAYS** verify CLI subcommands and flags (`--help`) before writing them into workflows or scripts

### Change Proposals

- Present changes as unified diffs, not direct file modifications
- Include test updates when adding features
- Run `just ci-full` locally when feasible to validate changes
- Use context7 website or MCP tool to get current documentation for APIs and crates

### Review Process

- This project prefers **CodeRabbit.ai** for code review
- Do **NOT** enable GitHub Copilot auto-review in pull requests
- Maintainer: **UncleSp1d3r** (single-maintainer workflow)

## Build/Lint/Test Commands

### Testing Gotchas

- **assert_cmd**: `Command::cargo_bin()` is deprecated; use `cargo::cargo_bin_cmd!("gold_digger")` macro instead
- **Clap env var leakage**: `#[arg(env = "...")]` reads real env vars at parse time; wrap `Cli::parse_from` inside `temp_env::with_var` blocks, not just the function under test
- **Insta snapshots**: Update with `INSTA_UPDATE=always cargo test --test <test_name>`; `cargo insta review --accept` is not valid
- **Env isolation in integration tests**: Always use `.env_remove("DATABASE_URL")` etc. on `Command` to prevent user shell env leakage

### Tool Management

All dev tools are managed via `mise.toml`. Commands in justfile use `{{ mise_exec }}` prefix.

- `just setup` - Install all tools via mise
- `mise use <tool>` - Add a new tool (e.g., `mise use cargo:cargo-watch`)
- `cargo-dist` binary is named `dist` (not `cargo dist`) when installed via mise. Valid subcommands: `build`, `init`, `plan`, `generate`, `manifest`, `host` (no `check` subcommand -- use `dist plan` to validate config)
- `mise install` - Reinstall all configured tools

### Pre-commit Hooks

Pre-commit hooks run automatically and may modify files. If commit fails with formatting changes:

1. Review the auto-fixed files
2. `git add -A` to stage fixes
3. Commit again

```bash
# Quick development cycle
just check                    # fmt + lint + test-no-docker
just fmt                      # cargo fmt
just lint                     # cargo clippy -- -D warnings
just test                     # cargo nextest run (preferred) or cargo test
just test-no-docker           # cargo nextest run (excludes Docker tests)

# Single test execution
cargo nextest run --test test_name
cargo test --test test_name
cargo test --lib module::function
cargo test --bin gold_digger

# Quality gates
just ci-check                 # fmt-check + lint + lint-sql + test + deny-check
just ci-full                  # Complete CI workflow equivalent
just fmt-check                # cargo fmt --check
just deny-check               # cargo deny check

# Build variants
cargo build                   # Debug build
cargo build --release         # Release build
cargo build --no-default-features --features "json csv additional_mysql_types verbose"  # Minimal build
```

## Code Style Guidelines

### Formatting & Imports

- **Line limit**: 100 characters (enforced by `rustfmt.toml`)
- **Clippy**: Zero tolerance warnings (`-D warnings`)
- **Imports**: Group by std, external crates, local modules (separated by newlines)
- **Formatting**: Use `cargo fmt` (Rustfmt conventions)
- Avoid using emojis and other non-ASCII characters in code, comments, or documentation, except when the code is handling non-plaintext characters (for example: em dash, en dash, or other non-ASCII symbols).

### Types & Naming

- **snake_case** for functions/variables, **CamelCase** for types/structs
- Use explicit types for public APIs
- Prefer `anyhow::Result<T>` for applications, `thiserror` for libraries
- Use `?` operator for error propagation

### Documentation

- All public functions require doc comments (`///`)
- Use proper markdown formatting with Arguments/Returns/Example sections
- Keep files ≤1000 lines, preferably ≤500 lines

### Error Handling

- Never use `from_value::<String>()` - always handle `mysql::Value::NULL`
- Use safe conversion helpers: `mysql_value_to_string()` for CSV/TSV, `mysql_value_to_json()` for JSON
- Redact credentials in all log output
- Use context with `.map_err()` for better debugging

### Cursor Rules Compliance

- Follow `.cursor/rules/rust-best-practices.mdc` for module organization
- Use format module contract: `fn write<W: Write>(rows: impl IntoIterator<Item = impl IntoIterator<Item = impl AsRef<str>>>, output: &mut W) -> anyhow::Result<()>`
- Implement streaming support with generic writers
- Use `#[cfg(feature = "...")]` for conditional compilation

## Project Overview

Gold Digger is a production-ready Rust CLI tool for MySQL/MariaDB database queries with structured output (CSV, JSON, TSV). It features comprehensive CLI interface, rustls-only TLS, and safe data type handling.

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

## 🚨 Critical Safety Rules

### Database Value Conversion (PANIC RISK)

```rust
// ❌ NEVER - causes panics on NULL/non-string types
// from_value::<String>(row[column.name_str().as_ref()])
// Use TypeTransformer::value_to_string() for CSV/TSV or TypeTransformer::value_to_json() for JSON

// ✅ ALWAYS - use TypeTransformer (src/type_transformer.rs)
use gold_digger::TypeTransformer;

// Usage per output format:
// - CSV/TSV: TypeTransformer::value_to_string(&value)?
// - JSON:    TypeTransformer::value_to_json(&value)
// - Full row to strings: TypeTransformer::row_to_strings(row)?
// - Full row to JSON map: TypeTransformer::row_to_json(row)?
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

- `rows_to_strings()`: Converts `Vec<Row>` to `Vec<Vec<String>>` (delegates to `TypeTransformer`)
- `get_extension_from_filename()`: Simple extension parsing

**Type Conversion (`src/type_transformer.rs`):**

- `TypeTransformer::value_to_string()`: Safe MySQL value to String (CSV/TSV)
- `TypeTransformer::value_to_json()`: Safe MySQL value to serde_json::Value (JSON, ISO-8601 datetimes with `T` separator)
- `TypeTransformer::row_to_strings()`: Full row to Vec<String>
- `TypeTransformer::row_to_json()`: Full row to BTreeMap\<String, serde_json::Value>

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

- `default`: `["json", "csv", "additional_mysql_types", "verbose"]`
- TLS: Always enabled with rustls (no feature flags)
- `additional_mysql_types`: Support for BigDecimal, Decimal, Time, Frunk
- `verbose`: Conditional logging via println!/eprintln!

**Important**: TLS is always enabled with rustls - no feature flags needed.

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

- **SBOM generation**: Use `cargo cyclonedx --format json` (not Syft). Syft scans the filesystem and picks up stale `Cargo.lock` files in `megalinter-reports/` producing false positives. cargo-cyclonedx reads only the project's `Cargo.lock`.
- **Never log credentials:** Implement redaction for `DATABASE_URL` and secrets
- **No hardcoded secrets:** Use environment variables or GitHub OIDC
- **Vulnerability policy:** Block releases with critical vulnerabilities
- **Airgap compatibility:** No telemetry or external calls in production
- **Configure TLS programmatically:** Use `mysql::OptsBuilder` and `SslOpts` instead of URL parameters
- **TLS Implementation:** Always enabled with rustls (no feature flags)

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

| File                           | Purpose          | Key Issues                                 |
| ------------------------------ | ---------------- | ------------------------------------------ |
| `src/main.rs`                  | Entry point      | Exit codes, pattern bug, env var handling  |
| `src/lib.rs`                   | Core logic       | Delegates to TypeTransformer               |
| `src/type_transformer.rs`      | Value conversion | TypeTransformer: safe MySQL value handling |
| `src/json.rs`                  | JSON output      | BTreeMap for deterministic ordering        |
| `Cargo.toml`                   | Dependencies     | Version mismatch with CHANGELOG            |
| `project_spec/requirements.md` | Target features  | Comprehensive feature roadmap              |

---

**Maintainer:** UncleSp1d3r\
**Workflow:** Single-maintainer with CodeRabbit.ai reviews\
**Status:** Active development toward v1.0
