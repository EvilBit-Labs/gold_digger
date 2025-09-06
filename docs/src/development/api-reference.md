# API Reference

Links to detailed API documentation and developer resources.

## Rustdoc Documentation

The complete API documentation is available in the [rustdoc section](../api/gold_digger/index.html)
of this site.

## Public API Overview

### Core Functions

- [`rows_to_strings()`](../api/gold_digger/fn.rows_to_strings.html) - Convert database rows to
  string vectors
- [`get_extension_from_filename()`](../api/gold_digger/fn.get_extension_from_filename.html) -
  Extract file extensions for format detection

### Output Modules

- [`csv::write()`](../api/gold_digger/csv/fn.write.html) - CSV output generation
- [`json::write()`](../api/gold_digger/json/fn.write.html) - JSON output generation
- [`tab::write()`](../api/gold_digger/tab/fn.write.html) - TSV output generation

### CLI Interface

- [`cli::Cli`](../api/gold_digger/cli/struct.Cli.html) - Command-line argument structure
- [`cli::Commands`](../api/gold_digger/cli/enum.Commands.html) - Available subcommands

## Usage Examples

### Basic Library Usage

```rust
use gold_digger::{csv, rows_to_strings};
use mysql::{Pool, Row};
use std::fs::File;

fn example() -> anyhow::Result<()> {
    // Convert database rows and write CSV
    let rows: Vec<Row> = vec![]; // query results would go here
    let string_rows = rows_to_strings(rows)?;
    let output = File::create("output.csv")?;
    csv::write(string_rows, output)?;
    Ok(())
}
```

### Custom Format Implementation

```rust,ignore
use anyhow::Result;
use std::io::Write;

pub fn write<W: Write>(rows: Vec<Vec<String>>, mut output: W) -> Result<()> {
    for row in rows {
        writeln!(output, "{}", row.join("|"))?;
    }
    Ok(())
}
```

## Type Definitions

Key types used throughout the codebase:

- `Vec<Vec<String>>` - Standard row format for output modules
- `anyhow::Result<T>` - Error handling pattern
- `mysql::Row` - Database result row type

## Error Handling

All public functions return `anyhow::Result<T>` for consistent error handling:

```rust
use anyhow::Result;

fn example_function() -> Result<()> {
    // Function implementation
    Ok(())
}
```

## Feature Flags

Conditional compilation based on Cargo features:

```rust
#[cfg(feature = "csv")]
pub mod csv;

#[cfg(feature = "json")]
pub mod json;

#[cfg(feature = "verbose")]
println!("Debug information");
```
