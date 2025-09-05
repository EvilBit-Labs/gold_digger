---
inclusion: always
---

# Gold Digger Project Structure & Conventions

## Module Architecture

### Source Organization

```text
src/
├── main.rs     # CLI entry point, configuration resolution, format dispatch
├── lib.rs      # Public API, shared utilities (rows_to_strings, extensions)
├── cli.rs      # Clap CLI definitions and argument parsing
├── csv.rs      # CSV output format implementation
├── json.rs     # JSON output format implementation
└── tab.rs      # TSV output format implementation
```

### Responsibility Separation

- **main.rs**: Application entry, configuration precedence (CLI > env), format routing
- **lib.rs**: Public API surface, shared utilities, module exports
- **cli.rs**: Command-line interface definition and validation
- **Format modules**: Independent output implementations with consistent interface

### Format Module Contract

All format modules must implement:

```rust
pub fn write<W: Write>(rows: Vec<Vec<String>>, output: W) -> anyhow::Result<()>
```

## Configuration Architecture

### Resolution Pattern

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

### CLI Structure

- **Derive-based**: Use `clap` with `derive` and `env` features
- **Environment integration**: Automatic fallback to environment variables
- **Validation**: Mutually exclusive options and required parameter checking
- **Extensibility**: Support for completion generation and config dumping

## Conditional Compilation Patterns

### Feature-Gated Modules

```rust
// Enable format support conditionally
#[cfg(feature = "csv")]
OutputFormat::Csv => gold_digger::csv::write(rows, output)?,

// Graceful degradation when feature disabled
#[cfg(not(feature = "csv"))]
OutputFormat::Csv => anyhow::bail!("CSV support not compiled in"),

// Optional verbose output
#[cfg(feature = "verbose")]
eprintln!("Processing {} rows", row_count);
```

### Format Detection Logic

```rust
// File extension-based format detection
OutputFormat::from_extension(output_file)

// Explicit override support
--format csv|json|tsv
```

## Error Handling Patterns

### Consistent Error Types

```rust
// Application-level errors
pub fn function_name() -> anyhow::Result<()> {
    let result = fallible_operation()?; // Use ? for propagation
    Ok(())
}

// Library-level errors (if needed)
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FormatError {
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}
```

### Error Context Addition

```rust
// Add context to errors for better debugging
let opts = Opts::from_url(database_url)
    .map_err(|e| anyhow::anyhow!("Invalid database URL: {}", e))?;
```

## Database Interaction Patterns

### Connection Lifecycle

```rust
use mysql::prelude::Queryable;

// Single connection per execution
let opts = Opts::from_url(database_url)
    .map_err(|e| anyhow::anyhow!("Invalid database URL: {}", e))?;
let pool = Pool::new(opts)?;
let mut conn = pool.get_conn()?;
let result: Vec<mysql::Row> = conn.query(database_query)?;
```

### Safe Value Iteration

```rust
// AVOID - panic-prone indexed access
let data_row: Vec<String> = row.columns_ref()
    .iter()
    .map(|col| from_value::<String>(row[col.name_str().as_ref()]))
    .collect();

// PREFER - safe iteration with explicit NULL handling
let data_row: Vec<String> = row.as_ref()
    .iter()
    .map(|value| match value {
        mysql::Value::NULL => "".to_string(),
        val => from_value_opt::<String>(val.clone())
            .unwrap_or_else(|_| format!("{:?}", val))
    })
    .collect();
```

## Output Format Architecture

### Uniform Interface Contract

```rust
// All format modules implement this signature
pub fn write<W: Write>(rows: Vec<Vec<String>>, output: W) -> anyhow::Result<()>
```

### Format-Specific Implementation Details

- **CSV**: RFC4180 compliance, `QuoteStyle::Necessary`, CRLF line endings
- **JSON**: `{"data": [...]}` wrapper, BTreeMap for deterministic field ordering
- **TSV**: Tab delimiter, `QuoteStyle::Necessary`, fallback format

### Format Selection Strategy

```rust
// Primary: File extension detection
let format = OutputFormat::from_extension(&output_file);

// Override: Explicit format specification
if let Some(explicit_format) = cli.format {
    format = explicit_format;
}
```

## Code Organization Conventions

### Import Grouping

```rust
// Standard library (first group)
use std::{env, fs::File, path::PathBuf};

// External crates (second group)
use anyhow::Result;
use clap::{CommandFactory, Parser};
use mysql::Pool;

// Local modules (third group)
use gold_digger::cli::{Cli, Commands};
use gold_digger::rows_to_strings;
```

### Logging & Security Patterns

```rust
// Safe verbose output (never log credentials)
if cli.verbose > 0 && !cli.quiet {
    println!("Connecting to database...");  // Safe - no sensitive data
}

// Credential redaction in all output
fn redact_url(url: &str) -> String {
    "***REDACTED***".to_string()  // Always mask DATABASE_URL
}
```

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
