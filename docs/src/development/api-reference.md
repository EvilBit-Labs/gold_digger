# API Reference

Links to detailed API documentation and developer resources.

> Canonical API docs (latest published release): <https://docs.rs/gold_digger>

## Rustdoc Documentation

The complete API documentation is available in the [rustdoc section](../api/gold_digger/index.html) of this site, and on <https://docs.rs/gold_digger> for the most recent release tag.

## Public API Overview

### Core Functions

- [`rows_to_strings()`](../api/gold_digger/fn.rows_to_strings.html) - Convert database rows to string vectors (delegates to `TypeTransformer` internally)
- [`get_extension_from_filename()`](../api/gold_digger/fn.get_extension_from_filename.html) - Extract file extensions for format detection

### TypeTransformer

- [`TypeTransformer`](../api/gold_digger/struct.TypeTransformer.html) - Canonical hub for safe MySQL value conversion

#### Associated Functions

- [`TypeTransformer::value_to_string()`](../api/gold_digger/struct.TypeTransformer.html#method.value_to_string) - Convert `mysql::Value` to `String` for CSV/TSV output. Returns error for invalid date/time values.
- [`TypeTransformer::value_to_json()`](../api/gold_digger/struct.TypeTransformer.html#method.value_to_json) - Convert `mysql::Value` to `serde_json::Value` for JSON output. Returns error for invalid date/time values.
- [`TypeTransformer::row_to_strings()`](../api/gold_digger/struct.TypeTransformer.html#method.row_to_strings) - Convert entire row to vector of strings.
- [`TypeTransformer::row_to_json()`](../api/gold_digger/struct.TypeTransformer.html#method.row_to_json) - Convert entire row to JSON object with deterministic key ordering (`BTreeMap`).

### Output Modules

- [`csv::write()`](../api/gold_digger/csv/fn.write.html) - CSV output generation (takes `IntoIterator<Item = IntoIterator<Item = String>>`)
- [`json::write()`](../api/gold_digger/json/fn.write.html) - JSON output generation (takes pre-converted `Vec<BTreeMap<String, serde_json::Value>>` + `pretty: bool`)
- [`tab::write()`](../api/gold_digger/tab/fn.write.html) - TSV output generation (takes `IntoIterator<Item = IntoIterator<Item = String>>`)

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

### Using TypeTransformer for Value Conversion

```rust
use gold_digger::TypeTransformer;
use mysql::Value;

fn convert_value(value: &Value) -> anyhow::Result<()> {
    // Convert to string for CSV/TSV output
    let s = TypeTransformer::value_to_string(value)?;
    println!("String: {}", s);

    // Convert to JSON value
    let json = TypeTransformer::value_to_json(value)?;
    println!("JSON: {}", serde_json::to_string(&json)?);

    Ok(())
}
```

### Using JSON with Native Types

```rust
use gold_digger::{TypeTransformer, json};
use mysql::Row;
use std::collections::BTreeMap;
use std::fs::File;

fn example_json() -> anyhow::Result<()> {
    let rows: Vec<Row> = vec![]; // query results
    // Convert all rows BEFORE creating the file so a conversion error
    // never leaves behind a truncated output.
    let maps: Vec<BTreeMap<String, serde_json::Value>> = rows
        .into_iter()
        .map(TypeTransformer::row_to_json)
        .collect::<anyhow::Result<_>>()?;
    let output = File::create("output.json")?;
    json::write(maps, output, false)?; // false = compact, true = pretty
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
- `mysql::Value` - MySQL value type (used by `TypeTransformer`)
- `serde_json::Value` - JSON value type (output of `TypeTransformer::value_to_json`)
- `BTreeMap<String, serde_json::Value>` - JSON object with deterministic key ordering (output of `TypeTransformer::row_to_json`)

## Safety Guarantees

`TypeTransformer` provides the following safety guarantees for MySQL value conversion:

- **NULL handling**: NULL values convert to empty strings (CSV/TSV) or `serde_json::Value::Null` (JSON)
- **Invalid UTF-8**: Binary data that is not valid UTF-8 is hex-encoded instead of causing panics (e.g., `0xfffefd`)
- **Special floats**: NaN and Infinity values are represented as strings (`"NaN"`, `"Infinity"`, `"-Infinity"`)
- **Date/time validation**: Date and time components are validated before formatting; invalid values return errors for both `value_to_string` and `value_to_json`
- **Deterministic output**: JSON objects use `BTreeMap` for alphabetical key ordering

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

CSV and JSON output are built into the binary unconditionally -- the former `csv` and `json` feature flags were vestigial markers that never actually gated compilation and were removed in todo #011. Remaining Cargo features:

- `verbose` (default) - enables additional diagnostic output via `println!` / `eprintln!` paths guarded by `#[cfg(feature = "verbose")]`.
- `additional_mysql_types` (default) - pulls in `mysql_common` with `bigdecimal`, `rust_decimal`, `time`, and `frunk` support for extended MySQL column types.
- `integration_tests` - opt-in flag used only by heavy integration tests that require a live database container.
