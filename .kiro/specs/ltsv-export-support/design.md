# LTSV Export Support Design Document

## Overview

This design implements LTSV (Labeled Tab-Separated Values) export support for Gold Digger through a
two-tier architecture: a standalone `ltsv-rs` workspace crate that provides general-purpose LTSV
reading/writing capabilities, and integration within Gold Digger's existing output format system.

The design follows Gold Digger's established patterns for output formats while creating a reusable
library that benefits the broader Rust ecosystem. The LTSV format specification follows the standard
where each line contains `label:value` pairs separated by tab characters.

## Architecture

### Workspace Structure

```structured text
gold_digger/
├── Cargo.toml (workspace root)
├── src/ (main binary)
├── crates/
│   └── ltsv-rs/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   ├── reader.rs
│       │   ├── writer.rs
│       │   ├── error.rs
│       │   └── escape.rs
│       └── tests/
└── src/ltsv.rs (Gold Digger integration)
```

### Integration Points

1. **Workspace Crate (`ltsv-rs`)**: Standalone library following csv crate patterns
2. **Gold Digger Integration (`src/ltsv.rs`)**: Thin wrapper implementing Gold Digger's format
   interface
3. **CLI Integration**: Extension detection (`.ltsv`) and format flag (`--format ltsv`)
4. **Feature Flag**: `ltsv` feature included in default features

## Components and Interfaces

### LTSV-RS Crate Public API

```rust
// High-level convenience API
pub fn write_records<W, I, R>(writer: W, records: I) -> Result<()>
where
    W: Write,
    I: IntoIterator<Item = R>,
    R: IntoIterator<Item = (String, String)>;

pub fn read_records<R>(reader: R) -> Result<Vec<HashMap<String, String>>>
where
    R: Read;

// Builder pattern API (csv crate style)
pub struct WriterBuilder {
    escape_style: EscapeStyle,
    // Configuration options
}

impl WriterBuilder {
    pub fn new() -> Self;
    pub fn escape_style(mut self, style: EscapeStyle) -> Self;
    pub fn build<W: Write>(self, writer: W) -> Writer<W>;
}

pub struct Writer<W: Write> {
    // Internal writer state
}

impl<W: Write> Writer<W> {
    pub fn write_record<I>(&mut self, record: I) -> Result<()>
    where
        I: IntoIterator<Item = (String, String)>;

    pub fn flush(&mut self) -> Result<()>;
}

// Reader API
pub struct ReaderBuilder {
    // Configuration options
}

pub struct Reader<R: Read> {
    // Internal reader state
}

impl<R: Read> Reader<R> {
    pub fn records(&mut self) -> RecordIterator<R>;
}

// Error handling
#[derive(Error, Debug)]
pub enum LtsvError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error at line {line}: {message}")]
    Parse { line: usize, message: String },
    #[error("Invalid field name: {0}")]
    InvalidFieldName(String),
}
```

### Gold Digger Integration Interface

```rust
// src/ltsv.rs - Gold Digger integration module
use ltsv_rs::{EscapeStyle, WriterBuilder};

pub fn write<W: Write>(rows: Vec<Vec<String>>, output: W) -> anyhow::Result<()> {
    // Convert Gold Digger's row format to LTSV records
    // Use first row as headers, subsequent rows as data
    // Handle NULL values and escaping
}
```

### CLI Integration Points

```rust
// Extension detection in lib.rs
pub fn get_extension_from_filename(filename: &str) -> Option<&str> {
    // Add "ltsv" case
}

// Format dispatch in main.rs
match format {
    #[cfg(feature = "ltsv")]
    OutputFormat::Ltsv => gold_digger::ltsv::write(rows, output)?,
    // ... other formats
}

// CLI enum extension
#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Csv,
    Json,
    Tsv,
    #[cfg(feature = "ltsv")]
    Ltsv,
}
```

## Data Models

### LTSV Format Specification

```structured text
# Standard LTSV format
field1:value1<TAB>field2:value2<TAB>field3:value3<LF>
field1:value4<TAB>field2:value5<TAB>field3:value6<LF>

# Escaping rules
- Colon in value: field:val\:ue
- Tab in value: field:val\tue  
- Newline in value: field:val\nue
- Backslash in value: field:val\\ue
```

### Data Transformation Pipeline

```rust
// Gold Digger row format
Vec<Vec<String>> // [headers, row1, row2, ...]

// Transform to LTSV records
Vec<HashMap<String, String>> // [{field1: value1, field2: value2}, ...]

// Serialize to LTSV format
String // "field1:value1\tfield2:value2\n..."
```

### Escape Handling

```rust
pub enum EscapeStyle {
    Standard, // \t, \n, \r, \\, \:
    Minimal,  // Only escape when necessary
    None,     // No escaping (unsafe but fast)
}

pub fn escape_value(value: &str, style: EscapeStyle) -> String {
    match style {
        EscapeStyle::Standard => value
            .replace('\\', "\\\\")
            .replace('\t', "\\t")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace(':', "\\:"),
        EscapeStyle::Minimal => {
            // Only escape if value contains problematic characters
        },
        EscapeStyle::None => value.to_string(),
    }
}
```

## Error Handling

### Error Categories

1. **IO Errors**: File system operations, network issues
2. **Parse Errors**: Malformed LTSV input during reading
3. **Validation Errors**: Invalid field names, encoding issues
4. **Configuration Errors**: Invalid escape styles, builder options

### Error Propagation Strategy

```rust
// ltsv-rs crate uses thiserror for structured errors
#[derive(Error, Debug)]
pub enum LtsvError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error at line {line}: {message}")]
    Parse { line: usize, message: String },

    #[error("Invalid field name '{name}': {reason}")]
    InvalidFieldName { name: String, reason: String },
}

// Gold Digger integration uses anyhow for application errors
pub fn write<W: Write>(rows: Vec<Vec<String>>, output: W) -> anyhow::Result<()> {
    let mut writer = WriterBuilder::new().escape_style(EscapeStyle::Standard).build(output);

    // Convert and handle errors with context
    for row in rows {
        writer
            .write_record(row.into_iter().enumerate().map(|(i, v)| (format!("field_{}", i), v)))
            .with_context(|| "Failed to write LTSV record")?;
    }

    writer.flush().with_context(|| "Failed to flush LTSV output")?;

    Ok(())
}
```

## Testing Strategy

### Unit Tests (ltsv-rs crate)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_write() {
        let mut output = Vec::new();
        let records = vec![
            vec![("name".to_string(), "Alice".to_string())],
            vec![("name".to_string(), "Bob".to_string())],
        ];

        write_records(&mut output, records).unwrap();

        let expected = "name:Alice\nname:Bob\n";
        assert_eq!(String::from_utf8(output).unwrap(), expected);
    }

    #[test]
    fn test_escaping() {
        let mut output = Vec::new();
        let records = vec![vec![("field".to_string(), "val:ue\twith\ttabs".to_string())]];

        write_records(&mut output, records).unwrap();

        let expected = "field:val\\:ue\\twith\\ttabs\n";
        assert_eq!(String::from_utf8(output).unwrap(), expected);
    }

    #[test]
    fn test_read_write_roundtrip() {
        // Test that reading and writing produces identical results
    }

    #[test]
    fn test_malformed_input() {
        // Test error handling for invalid LTSV input
    }
}
```

### Integration Tests (Gold Digger)

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_gold_digger_ltsv_integration() {
        let rows = vec![
            vec!["name".to_string(), "age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];

        let mut output = Vec::new();
        write(rows, &mut output).unwrap();

        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("name:Alice\tage:30"));
        assert!(result.contains("name:Bob\tage:25"));
    }

    #[test]
    fn test_null_value_handling() {
        // Test NULL database values become empty strings
    }

    #[test]
    fn test_cli_integration() {
        // Test file extension detection and format flag
    }
}
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_escape_unescape_roundtrip(value in ".*") {
        let escaped = escape_value(&value, EscapeStyle::Standard);
        let unescaped = unescape_value(&escaped).unwrap();
        assert_eq!(value, unescaped);
    }

    #[test]
    fn test_write_read_roundtrip(records in prop::collection::vec(
        prop::collection::vec((".*", ".*"), 1..10), 1..100
    )) {
        let mut buffer = Vec::new();
        write_records(&mut buffer, records.clone()).unwrap();

        let parsed = read_records(&buffer[..]).unwrap();
        assert_eq!(records.len(), parsed.len());
    }
}
```

## Implementation Phases

### Phase 1: Core LTSV-RS Crate

- Basic writer implementation with escaping
- Error handling and validation
- Unit tests for core functionality

### Phase 2: Reader Implementation

- LTSV parsing with error recovery
- Iterator-based API following csv crate patterns
- Comprehensive parsing tests

### Phase 3: Gold Digger Integration

- Format module implementation
- CLI integration (extension detection, format flag)
- Feature flag configuration

### Phase 4: Documentation and Polish

- API documentation with examples
- Integration tests
- Performance benchmarks
- README and usage examples
