---
inclusion: always
---

# Gold Digger Core Concepts

This document defines the essential architecture patterns, safety requirements, and development constraints for the Gold Digger MySQL/MariaDB query tool.

## Project Identity

- **Purpose**: Headless MySQL/MariaDB query tool for automation workflows
- **Design**: CLI-first with environment variable fallbacks, structured output (CSV/JSON/TSV)
- **Architecture**: Single-threaded, fully materialized results, offline-first
- **Status**: Active development toward v1.0 (currently v0.2.6)

## Critical Safety Requirements

## Security Requirements (NON-NEGOTIABLE)

### Credential Protection

- **NEVER** log `DATABASE_URL` or credentials - implement automatic redaction
- **NEVER** hardcode secrets in code or configuration
- **ALWAYS** validate user inputs before processing

### Airgap Compatibility

- **Allowed**: Configured database connections (MySQL/MariaDB only)
- **Prohibited**: Telemetry, call-home, non-essential outbound connections
- **Runtime**: No external dependencies during execution

## Configuration Architecture

### Resolution Priority: CLI → Environment → Error

```rust
// Standard pattern for all configuration values
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

### Required Configuration

| CLI Flag                   | Environment Variable | Purpose                                      |
| -------------------------- | -------------------- | -------------------------------------------- |
| `--db-url`                 | `DATABASE_URL`       | MySQL connection string                      |
| `--query` / `--query-file` | `DATABASE_QUERY`     | SQL to execute                               |
| `--output`                 | `OUTPUT_FILE`        | Output path (determines format by extension) |
| `--format`                 | -                    | Force output format (csv\|json\|tsv)         |

**Important**: No dotenv support - use exported environment variables only

## Module Architecture

```text
src/
├── main.rs     # CLI entry, config resolution, format dispatch
├── cli.rs      # Clap CLI definitions and validation
├── lib.rs      # Public API, shared utilities
├── csv.rs      # RFC4180 compliant with QuoteStyle::Necessary
├── json.rs     # {"data":[...]} with BTreeMap for deterministic ordering
└── tab.rs      # TSV fallback format
```

### Format Module Contract

All format modules must implement:

```rust
pub fn write<W: Write>(rows: Vec<Vec<String>>, output: W) -> anyhow::Result<()>
```

### Output Format Standards

- **CSV**: RFC4180 compliant, `QuoteStyle::Necessary` (quotes only when required)
- **JSON**: `{"data": [...]}` wrapper, BTreeMap ensures deterministic field ordering
- **TSV**: Tab-delimited fallback format

## Known Issues (High Priority Fixes)

1. **Memory**: No streaming support - O(row_count × row_width) memory usage

## Feature Flags

```toml
default = ["json", "csv", "additional_mysql_types", "verbose", "ssl-rustls"]
json = [] # Enable JSON output format
csv = [] # Enable CSV output format
additional_mysql_types = [ # Extended MySQL type support
  "mysql_common",
  "mysql_common?/bigdecimal",
  "mysql_common?/rust_decimal",
  "mysql_common?/time",
  "mysql_common?/frunk",
]
verbose = [] # Conditional println!/eprintln!
ssl = [
  "native-tls",
  "mysql/native-tls",
] # Native TLS implementation (OpenSSL/SecureTransport)
ssl-rustls = [
  "rustls",
  "rustls-native-certs",
  "rustls-pemfile",
  "mysql/rustls-tls",
] # Pure Rust TLS implementation (default)
```

**Note**: TLS support is provided via feature flags with `ssl-rustls` enabled by default. The `ssl` feature provides native TLS implementation (OpenSSL/SecureTransport) while `ssl-rustls` provides pure Rust TLS implementation. Only one TLS implementation can be enabled at a time.

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
- **Coverage**: Target ≥80% with `cargo tarpaulin`
- **Cross-platform**: Must pass on macOS, Windows, Linux

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

### Rust Style Guidelines

- Use `anyhow::Result<T>` for fallible functions
- Feature-gate verbose output: `#[cfg(feature = "verbose")]`
- Never log DATABASE_URL or credentials
- Handle NULL database values gracefully

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

## Memory & Performance Characteristics

- **Fully materialized results**: Loads all rows into memory (not streaming)
- **Connection pooling**: Uses mysql::Pool but no optimization
- **Memory scaling**: O(row_count × row_width)
- **Streaming requirement**: F007 in requirements.md calls for streaming support

## Requirements Gap (High Priority)

Current v0.2.6 → Target v1.0:

- **CLI present (clap-based)**: Clap-based interface exists; finalize config precedence and UX polish (F001–F003)
- **Exit code standards**: Proper error taxonomy implemented in src/exit.rs (F005 ✓)
- **Memory efficiency**: Implement streaming for large result sets (F007)
- **Streaming**: Memory-efficient large result processing (F007)

## Development Workflow

### Development Practices

- **Reviews**: CodeRabbit.ai preferred, disable GitHub Copilot auto-reviews
- **Scope**: Target small, reviewable changes for single-maintainer workflow

### Testing Strategy (Planned)

```toml
# Recommended test dependencies
criterion = "0.5"       # Benchmarking
insta = "1"             # Snapshot testing
assert_cmd = "2"        # CLI end-to-end
testcontainers = "0.15" # Database integration
```

## Safe Code Patterns

### Environment Variable Handling

```rust
let var_name = match env::var("VAR_NAME") {
    Ok(val) => val,
    Err(_) => {
        #[cfg(feature = "verbose")]
        eprintln!("couldn't find VAR_NAME in environment variable");
        std::process::exit(2);  // TODO: Use proper exit codes
    }
};
```

### Database Value Handling

```rust
// Safe NULL handling pattern
match database_value {
    mysql::Value::NULL => "".to_string(),
    val => from_value_opt::<String>(val)
        .unwrap_or_else(|_| format!("{:?}", val))
}
```

### Format Dispatch Pattern

```rust
match get_extension_from_filename(&output_file) {
    #[cfg(feature = "csv")]
    Some("csv") => gold_digger::csv::write(rows, output)?,
    #[cfg(feature = "json")]
    Some("json") => gold_digger::json::write(rows, output)?,
    Some(_) => gold_digger::tab::write(rows, output)?, // TSV fallback
    None => anyhow::bail!("missing file extension for output_file"),
}
```

## Essential Commands

```bash
# Build variations
cargo build --release                                    # Standard build (rustls TLS)
cargo build --release --no-default-features --features "json csv additional_mysql_types verbose"  # Minimal build (TLS still available)
cargo build --no-default-features --features "csv json" # Minimal build

# Development (CLI-first approach)
cargo install --path .  # Local install
cargo run --release -- \
  --db-url "mysql://u:p@h:3306/db" \
  --query "SELECT id, name FROM users" \
  --output /tmp/out.json

# Quality assurance (pipeline standards)
just fmt-check    # cargo fmt --check (100-char line limit)
just lint         # cargo clippy -- -D warnings (zero tolerance)
just test         # cargo nextest run (preferred) or cargo test
just security     # cargo audit (advisory)
```
