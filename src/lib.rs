//! Gold Digger: MySQL/MariaDB query tool with structured output.
//!
//! The crate is organised into the [`cli`] argument parser, output writers
//! ([`csv`], [`json`], [`tab`]), the panic-free value converter
//! ([`type_transformer`]), the exit-code contract ([`exit`]), rustls-backed
//! TLS ([`tls`]), and shared helpers ([`utils`]). The library exposes
//! [`rows_to_strings`] and [`get_extension_from_filename`] for callers.
//!
//! Each writer module exposes a single `write(...)` free function. CSV and
//! TSV take `IntoIterator<Item = IntoIterator<Item = String>>`; JSON takes
//! pre-converted `BTreeMap<String, serde_json::Value>` maps so the caller
//! can validate every row before truncating the output file.
//!
//! TLS is always rustls; consumers must invoke [`init_crypto_provider`] once
//! before opening a connection pool so rustls has a default crypto provider.
//! Exit codes follow the 0-5 contract defined in [`exit`].

use std::{env, ffi::OsStr, path::Path, sync::Once};

use anyhow::{Context, Result};
use mysql::Row;

static INIT: Once = Once::new();

/// Initialize crypto provider for rustls
pub fn init_crypto_provider() {
    INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// CLI interface module.
pub mod cli;
/// CSV output module.
pub mod csv;
/// Exit code helper module.
pub mod exit;
/// JSON output module.
pub mod json;
/// Tab-delimited output module.
pub mod tab;
/// TLS configuration module.
pub mod tls;
/// Type transformation module for safe MySQL value conversion.
pub mod type_transformer;
/// Utility functions module.
pub mod utils;

pub use type_transformer::TypeTransformer;

/// Converts MySQL rows to a vector of string vectors, with the first row as headers.
///
/// This function safely handles all MySQL data types including NULL values without panicking.
/// It uses safe iteration over row values instead of indexed access to prevent runtime panics.
///
/// # Arguments
///
/// * `rows` - A vector of MySQL rows.
///
/// # Returns
///
/// A Result containing a vector of string vectors, or an error.
///
/// # Safety
///
/// This function replaces the dangerous pattern of using `row[column.name_str().as_ref()]`
/// which can panic on NULL values or type mismatches. Instead, it uses safe iteration
/// over `row.as_ref()` to handle all value types gracefully.
pub fn rows_to_strings(mut rows: Vec<Row>) -> anyhow::Result<Vec<Vec<String>>> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Pre-allocate with known capacity for better performance.
    // +1 accounts for the header row prepended below.
    let mut result_rows = Vec::with_capacity(rows.len().saturating_add(1));

    // Extract headers from the first row before draining so we retain access
    // to column metadata while the row values are still owned by `rows`.
    let header_row: Vec<String> = rows[0]
        .columns_ref()
        .iter()
        .map(|column| column.name_str().to_string())
        .collect();
    result_rows.push(header_row);

    // Drain each row by value so the `Row` is dropped as soon as its string
    // representation has been extracted. This halves peak memory during
    // conversion compared to `rows.iter()` which kept every source row live
    // alongside the fully-materialised result set.
    for (row_index, row) in rows.drain(..).enumerate() {
        let mut data_row = Vec::with_capacity(row.len());
        for i in 0..row.len() {
            match row.as_ref(i) {
                Some(value) => match TypeTransformer::value_to_string(value) {
                    Ok(string_value) => data_row.push(string_value),
                    Err(e) => {
                        return Err(e.context(format!(
                            "Type conversion failed at row {} column {}",
                            row_index + 1,
                            i + 1
                        )));
                    }
                },
                // Within 0..row.len(), None indicates a SQL NULL
                None => data_row.push(String::new()),
            }
        }
        result_rows.push(data_row);
    }

    Ok(result_rows)
}

/// Extracts the file extension from a filename, if present.
///
/// # Arguments
///
/// * `filename` - The filename as a string slice.
///
/// # Returns
///
/// An Option containing the extension as a string slice, or None if not found.
pub fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension().and_then(OsStr::to_str)
}

/// Gets a required environment variable with contextual error information.
///
/// # Arguments
///
/// * `var_name` - The name of the environment variable to retrieve.
///
/// # Returns
///
/// A Result containing the environment variable value as a String, or an error with context.
pub fn get_required_env(var_name: &str) -> Result<String> {
    env::var(var_name)
        .with_context(|| format!("Missing required environment variable: {}", var_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_required_env_missing() {
        let result = get_required_env("NONEXISTENT_ENV_VAR");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Missing required environment variable: NONEXISTENT_ENV_VAR")
        );
    }

    #[test]
    fn test_get_required_env_present() {
        temp_env::with_var("TEST_ENV_VAR", Some("test_value"), || {
            let result = get_required_env("TEST_ENV_VAR");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "test_value");
        });
    }

    #[test]
    fn test_rows_to_strings_empty() {
        let result = rows_to_strings(vec![]).expect("empty rows should succeed");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_rows_to_strings_type_conversion_error() {
        // Verify that type conversion errors map to exit code 4
        let error = anyhow::anyhow!("Type conversion error: Invalid month value 13 in date");
        use crate::exit::map_error_to_exit_code;
        assert_eq!(map_error_to_exit_code(&error), 4);
        assert!(error.to_string().contains("Type conversion error"));
    }

    #[test]
    fn test_get_extension_from_filename() {
        assert_eq!(get_extension_from_filename("test.json"), Some("json"));
        assert_eq!(get_extension_from_filename("test.csv"), Some("csv"));
        assert_eq!(get_extension_from_filename("test.tsv"), Some("tsv"));
        assert_eq!(get_extension_from_filename("test"), None);
        assert_eq!(get_extension_from_filename(""), None);
        assert_eq!(get_extension_from_filename(".hidden"), None);
    }
}
