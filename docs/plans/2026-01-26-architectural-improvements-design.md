# Architectural Improvements Design

**Date:** 2026-01-26 **Status:** Approved **Author:** Brainstorming session

## Overview

This document captures the design decisions for addressing four architectural issues identified in the codebase review:

1. **Issue A:** String-based error classification (fragile pattern matching)
2. **Issue B:** Large TLS module (~72KB, too many responsibilities)
3. **Issue C:** No streaming support (memory issues with large result sets)
4. **Issue D:** Output format coupling (hard to add new formats)

## Implementation Order

```
1. Issue A (Typed Errors)     ─┐
2. Issue D (Format Registry)   │ Can be parallel
3. Issue B (TLS Refactor)     ─┘ Depends on A
4. Issue C (Streaming)         ← Depends on D
```

---

## Issue A: Typed Error Classification

### Problem

The `map_error_to_exit_code()` function in `exit.rs` uses string pattern matching:

```rust
if error_string.contains("access denied") || error_string.contains("authentication")
```

This is fragile and can break silently if error messages change.

### Design Decisions

| Decision        | Choice                                        |
| --------------- | --------------------------------------------- |
| Approach        | Full typed enum (`GoldDiggerError`)           |
| External errors | Convert at boundaries (semantic mapping)      |
| TlsError        | Keep separate, wrap in `GoldDiggerError::Tls` |

### New File: `src/error.rs`

```rust
use crate::tls::TlsError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GoldDiggerError {
    #[error("No records found in database")]
    NoRows,

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database authentication failed: {0}")]
    DbAuth(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("{0}")]
    Tls(#[from] TlsError),
}

impl GoldDiggerError {
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::NoRows => 1,
            Self::Config(_) => 2,
            Self::DbAuth(_) => 3,
            Self::Query(_) => 4,
            Self::Io(_) => 5,
            Self::Tls(e) => e.exit_code(),
        }
    }
}
```

### Boundary Conversion: `From<mysql::Error>`

```rust
impl From<mysql::Error> for GoldDiggerError {
    fn from(e: mysql::Error) -> Self {
        match &e {
            mysql::Error::MySqlError(mysql_err) => {
                match mysql_err.code {
                    // Authentication errors
                    1045 | 1044 | 1142 | 1143 => Self::DbAuth(e.to_string()),
                    // Query/syntax errors
                    1064 | 1146 | 1054 => Self::Query(e.to_string()),
                    // Connection errors
                    1049 | 2002 | 2003 | 2006 | 2013 => Self::DbAuth(e.to_string()),
                    _ => Self::Query(e.to_string()),
                }
            }
            mysql::Error::IoError(_) => Self::Io(e.to_string()),
            mysql::Error::UrlError(_) => Self::Config(e.to_string()),
            mysql::Error::DriverError(_) => Self::DbAuth(e.to_string()),
            _ => Self::Query(e.to_string()),
        }
    }
}
```

### TlsError Exit Code Method

Add to `src/tls.rs`:

```rust
impl TlsError {
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::MutuallyExclusiveFlags { .. } => 2,
            Self::CaFileNotFound { .. } | Self::InvalidCaFormat { .. } => 5,
            _ => 3, // All other TLS errors are connection/auth
        }
    }
}
```

### Simplified `exit.rs`

```rust
use crate::error::GoldDiggerError;

pub fn exit_with_error(error: GoldDiggerError, context: Option<&str>) -> ! {
    let exit_code = error.exit_code();
    if let Some(ctx) = context {
        eprintln!("{}: {}", ctx, error);
    } else {
        eprintln!("{}", error);
    }
    process::exit(exit_code);
}
```

### Files Changed

- `src/error.rs` (new)
- `src/tls.rs` (add `exit_code()` method)
- `src/exit.rs` (simplify, remove `map_error_to_exit_code`)
- `src/main.rs` (use `GoldDiggerError`)
- `src/lib.rs` (export error module)

---

## Issue B: TLS Module Refactoring

### Problem

The TLS module (`src/tls.rs`) is ~72KB and handles too many responsibilities.

### Design Decision

Split into submodules while preserving the public API.

### New Structure

```
src/tls/
├── mod.rs          # Re-exports public API
├── config.rs       # TlsConfig, TlsMode
├── errors.rs       # TlsError enum
├── certificates.rs # Certificate loading
└── connection.rs   # create_tls_connection()
```

### Public API (unchanged)

```rust
// src/tls/mod.rs
mod certificates;
mod config;
mod connection;
mod errors;

pub use config::{TlsConfig, TlsMode};
pub use connection::create_tls_connection;
pub use errors::TlsError;

pub(crate) use certificates::{load_ca_certificates, load_platform_certificates};
```

### Submodule Contents

| File              | Contents                                                                             |
| ----------------- | ------------------------------------------------------------------------------------ |
| `errors.rs`       | `TlsError` enum, `Display`, `Error`, `suggest_cli_flag()`, `exit_code()`             |
| `config.rs`       | `TlsMode`, `TlsConfig`, `from_tls_options()`, `display_security_warnings()`          |
| `certificates.rs` | `load_ca_certificates()`, `load_platform_certificates()`, `create_root_cert_store()` |
| `connection.rs`   | `create_tls_connection()`, `build_client_config()`, `create_mysql_pool()`            |

### Migration Steps

1. Create `src/tls/` directory
2. Extract `errors.rs`
3. Extract `config.rs`
4. Extract `certificates.rs`
5. Extract `connection.rs`
6. Create `mod.rs` with re-exports
7. Delete `src/tls.rs`
8. Run tests to verify

---

## Issue C: Streaming Support

### Problem

`rows_to_strings()` loads all rows into memory, causing issues with large result sets in memory-constrained environments (Docker containers).

### Design Decisions

| Decision        | Choice                                   |
| --------------- | ---------------------------------------- |
| Approach        | Iterator-based (single row at a time)    |
| Memory model    | Constant ~64KB regardless of result size |
| Backward compat | Not needed - remove old functions        |

### Enhanced FormatWriter Trait

```rust
pub trait FormatWriter {
    type Error: std::error::Error;

    fn write_header(&mut self, columns: &[String]) -> Result<(), Self::Error>;
    fn write_row(&mut self, row: &[String]) -> Result<(), Self::Error>;
    fn finalize(self) -> Result<(), Self::Error>;
}
```

### Streaming Writers

Each format module gets a streaming writer:

- `CsvStreamWriter` - rows written directly via csv crate
- `JsonStreamWriter` - writes `[`, rows with commas, `]`
- `TsvStreamWriter` - rows written with tab delimiters

### Core Streaming Function

```rust
pub fn stream_rows<W, T>(
    mut result: QueryResult<'_, '_, '_, T>,
    writer: &mut W,
) -> Result<u64, GoldDiggerError>
where
    W: FormatWriter<Error = GoldDiggerError>,
    T: mysql::prelude::Protocol,
{
    let mut count = 0u64;
    let mut columns_written = false;

    for row_result in result.iter() {
        let row: Row = row_result?;

        if !columns_written {
            let columns: Vec<String> = row
                .columns_ref()
                .iter()
                .map(|c| c.name_str().to_string())
                .collect();
            writer.write_header(&columns)?;
            columns_written = true;
        }

        let string_row = row_to_strings(&row)?;
        writer.write_row(&string_row)?;
        count += 1;
    }

    writer.finalize()?;
    Ok(count)
}
```

### main.rs Changes

- Use `query_iter()` instead of `query()`
- Create writer before query execution
- Count returned instead of checking `Vec::is_empty()`
- Remove `rows_to_strings()` entirely

---

## Issue D: Output Format Extensibility

### Problem

Adding a new format requires changes to multiple files (enum, module, match statement).

### Design Decision

Factory registry pattern - adding a format requires only:

1. Implement `FormatWriter` in new module
2. Register it (one line)
3. Add CLI enum variant

### Format Registry

```rust
// src/formats/mod.rs
pub type WriterFactory = fn(File, &FormatOptions) -> Box<dyn FormatWriter<Error = GoldDiggerError>>;

pub struct FormatRegistry {
    formats: HashMap<&'static str, WriterFactory>,
}

impl FormatRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            formats: HashMap::new(),
        };
        registry.register("csv", csv::create_writer);
        registry.register("json", json::create_writer);
        registry.register("tsv", tab::create_writer);
        registry
    }

    pub fn create_writer(
        &self,
        format: &str,
        output: File,
        options: &FormatOptions,
    ) -> Result<Box<dyn FormatWriter<Error = GoldDiggerError>>, GoldDiggerError> {
        let factory = self
            .formats
            .get(format)
            .ok_or_else(|| GoldDiggerError::Config(format!("Unknown format: {}", format)))?;
        Ok(factory(output, options))
    }
}
```

### Format Options

```rust
pub struct FormatOptions {
    pub pretty: bool,
    // Future: delimiter, root_element, etc.
}
```

### New Directory Structure

```
src/formats/
├── mod.rs    # Registry and FormatOptions
├── csv.rs    # CsvStreamWriter + create_writer
├── json.rs   # JsonStreamWriter + create_writer
└── tab.rs    # TsvStreamWriter + create_writer
```

### Adding a New Format (Example: XML)

1. Create `src/formats/xml.rs` implementing `FormatWriter`
2. Add `registry.register("xml", xml::create_writer);` in `mod.rs`
3. Add `Xml` variant to `OutputFormat` enum in `cli.rs`

---

## Summary

| Issue              | Files Changed                                              | Effort |
| ------------------ | ---------------------------------------------------------- | ------ |
| A: Typed Errors    | `error.rs` (new), `tls.rs`, `exit.rs`, `main.rs`, `lib.rs` | Medium |
| B: TLS Refactor    | `src/tls/` directory (5 files)                             | Low    |
| C: Streaming       | `lib.rs`, `formats/*.rs`, `main.rs`                        | High   |
| D: Format Registry | `src/formats/` directory, `cli.rs`, `main.rs`              | Medium |

**Total estimated scope:** ~1500-2000 lines changed/added
