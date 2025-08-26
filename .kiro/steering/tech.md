---
inclusion: always
---

# Gold Digger Technology Stack

## Language & Runtime

- **Rust 2024 Edition** - Memory safety and performance
- **Cargo** - Package management and build system
- **Target platforms**: x86_64/aarch64 for Linux, macOS, Windows

## Core Dependencies

### CLI Framework

- **clap** - Command-line parsing with `derive` and `env` features
- **std::env** - Environment variable access (no dotenv support)

### Database Connectivity

- **mysql crate** - MySQL/MariaDB driver with connection pooling
- **mysql::SslOpts** - TLS configuration (programmatic only, no URL params)
- **mysql_common** - Extended type support via optional features

### Output Processing

- **serde_json** - JSON with BTreeMap for deterministic field ordering
- **csv crate** - RFC4180-compliant CSV with configurable quoting
- **Custom TSV** - Tab-separated format implementation

### Error Handling

- **anyhow** - Error propagation and context for applications
- **thiserror** - Structured error types for libraries

## Feature Configuration

### Default Feature Set

```toml
default = ["json", "csv", "ssl", "additional_mysql_types", "verbose"]
```

### TLS/SSL Implementation

- **mysql/rustls-tls** - Pure Rust TLS with platform certificate store integration (default)
  - Consistent cross-platform behavior
  - Enhanced security controls and validation options
  - Automatic system certificate store usage on all platforms
- **Simplified**: Single TLS implementation (previous dual approach consolidated)

### Extended MySQL Types

- **bigdecimal** - High-precision decimal arithmetic
- **rust_decimal** - Fixed-point decimal types
- **chrono** - Date/time handling
- **uuid** - UUID type support
- **Enabled via**: `additional_mysql_types` feature

## Build System

### Cargo Configuration

- **Release optimization**: LTO enabled, size optimization (`opt-level = 'z'`)
- **Binary stripping**: Remove debug symbols in release builds
- **Panic strategy**: `panic = "abort"` for smaller binaries

### Feature-Gated Compilation

```rust
#[cfg(feature = "verbose")]
eprintln!("Debug output");

#[cfg(feature = "csv")]
gold_digger::csv::write(rows, output)?;

#[cfg(not(feature = "csv"))]
OutputFormat::Csv => anyhow::bail!("CSV support not compiled in"),
```

### Build Variations

```bash
# Standard build (rustls TLS with platform certificate store integration)
cargo build --release

# No TLS support (insecure connections only)
cargo build --release --no-default-features --features "json csv additional_mysql_types verbose"

# Minimal feature build
cargo build --no-default-features --features "csv json"
```

## Technical Constraints

### Memory Architecture

- **Current model**: Fully materialized results - O(row_count × row_width)
- **Connection model**: Single database connection per execution
- **No streaming**: All query results loaded into memory before processing

### Database Type Safety

```rust
// DANGEROUS - causes runtime panics
from_value::<String>(row[column.name_str().as_ref()])

// SAFE - explicit NULL handling
match mysql_value {
    mysql::Value::NULL => "".to_string(),
    val => from_value_opt::<String>(val)
        .unwrap_or_else(|_| format!("{:?}", val))
}
```

### Security Implementation

- **Credential redaction**: Automatic masking of `DATABASE_URL` in all output
- **TLS configuration**: Programmatic via `mysql::SslOpts` (no URL parameters)
- **Offline operation**: No external network calls during execution
- **File permissions**: Respect system umask for output files

## Development Tools

### Code Quality Tools

- **rustfmt** - Code formatting (100-character line limit)
- **clippy** - Linting with zero-warning tolerance
- **cargo-nextest** - Parallel test execution (faster than `cargo test`)
- **cargo-llvm-cov** - Cross-platform coverage analysis
- **cargo-audit** - Security vulnerability scanning

### Testing & Coverage

```bash
# Test execution
cargo nextest run                    # Preferred parallel testing
cargo test                          # Standard test runner

# Coverage analysis
cargo llvm-cov --workspace --lcov    # CI-compatible LCOV format
cargo llvm-cov --html               # Local HTML reports
```

### Required Toolchain Components

```bash
rustup component add llvm-tools-preview rust-src
```

## Known Technical Debt

### Immediate Issues

- Non-standard exit codes (`exit(-1)` becomes 255)
- Pattern matching bug: `Some(&_)` should be `Some(_)`
- BTreeMap chosen for JSON output to ensure deterministic key ordering in serialized output

### Architecture Limitations

- No streaming support for large result sets
- Single-threaded execution model
- No configuration file support

## Deployment Considerations

### Cross-Platform Support

- **Pure Rust TLS**: Single rustls-based implementation for consistent cross-platform behavior
- **Platform Integration**: Automatic system certificate store usage (Windows/macOS/Linux)
- **Minimal builds**: Feature flags allow targeted compilation
- **Runtime dependencies**: System libraries only (no external services)
