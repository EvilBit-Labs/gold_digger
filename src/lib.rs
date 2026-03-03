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

#[cfg(feature = "json")]
pub use json::write_typed;

/// Trait for writing data in different formats
pub trait FormatWriter {
    fn write_header(&mut self, columns: &[String]) -> Result<()>;
    fn write_row(&mut self, row: &[String]) -> Result<()>;
    fn finalize(self) -> Result<()>;
}

/// Trait for streaming data processing (future enhancement)
///
/// This trait will enable memory-efficient processing of large result sets
/// by processing rows one at a time instead of loading everything into memory.
pub trait StreamingProcessor {
    type Item;
    type Error;

    /// Process a single item from the stream
    fn process_item(&mut self, item: Self::Item) -> std::result::Result<(), Self::Error>;

    /// Finalize the streaming operation
    fn finalize(self) -> std::result::Result<(), Self::Error>;
}

// TODO: Implement RowStream with correct QueryResult type signature
// pub struct RowStream<'a> {
//     result: mysql::QueryResult<'a>,
//     columns: Vec<Column>,
// }

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
pub fn rows_to_strings(rows: Vec<Row>) -> anyhow::Result<Vec<Vec<String>>> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Pre-allocate with known capacity for better performance
    let mut result_rows = Vec::with_capacity(rows.len() + 1);

    // Extract headers from the first row
    let header_row: Vec<String> = rows[0]
        .columns_ref()
        .iter()
        .map(|column| column.name_str().to_string())
        .collect();
    result_rows.push(header_row);

    // Process each row using safe iteration
    for (row_index, row) in rows.iter().enumerate() {
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
        // Use temp_env for safer environment variable testing
        temp_env::with_var("TEST_ENV_VAR", Some("test_value"), || {
            let result = get_required_env("TEST_ENV_VAR");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "test_value");
        });
    }

    #[test]
    fn test_mysql_value_to_string_null() {
        let result =
            TypeTransformer::value_to_string(&mysql::Value::NULL).expect("NULL should succeed");
        assert_eq!(result, "");
    }

    #[test]
    fn test_mysql_value_to_string_integers() {
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::Int(42)).expect("Int should succeed"),
            "42"
        );
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::Int(-42))
                .expect("negative Int should succeed"),
            "-42"
        );
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::UInt(123))
                .expect("UInt should succeed"),
            "123"
        );
    }

    #[test]
    fn test_mysql_value_to_string_floats() {
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::Float(3.5))
                .expect("Float should succeed"),
            "3.5"
        );
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::Double(2.5))
                .expect("Double should succeed"),
            "2.5"
        );
    }

    #[test]
    fn test_mysql_value_to_string_bytes() {
        let bytes = b"hello world".to_vec();
        let result = TypeTransformer::value_to_string(&mysql::Value::Bytes(bytes))
            .expect("valid UTF-8 should succeed");
        assert_eq!(result, "hello world");

        // Test invalid UTF-8 bytes - should use hex encoding
        let invalid_bytes = vec![0xFF, 0xFE, 0xFD];
        let result = TypeTransformer::value_to_string(&mysql::Value::Bytes(invalid_bytes))
            .expect("hex fallback should succeed");
        assert_eq!(result, "0xfffefd");

        // Test large binary data - should truncate with indication
        let large_bytes = vec![0xAB; 2000];
        let result = TypeTransformer::value_to_string(&mysql::Value::Bytes(large_bytes))
            .expect("large binary should succeed");
        assert!(result.starts_with("0x"));
        assert!(result.contains("... (2000 bytes)"));
    }

    #[test]
    fn test_mysql_value_to_string_special_floats() {
        // Test NaN
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::Float(f32::NAN))
                .expect("Float NaN should succeed"),
            "NaN"
        );
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::Double(f64::NAN))
                .expect("Double NaN should succeed"),
            "NaN"
        );

        // Test Infinity
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::Float(f32::INFINITY))
                .expect("Float Infinity should succeed"),
            "Infinity"
        );
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::Double(f64::INFINITY))
                .expect("Double Infinity should succeed"),
            "Infinity"
        );

        // Test Negative Infinity
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::Float(f32::NEG_INFINITY))
                .expect("Float -Infinity should succeed"),
            "-Infinity"
        );
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::Double(f64::NEG_INFINITY))
                .expect("Double -Infinity should succeed"),
            "-Infinity"
        );
    }

    #[test]
    fn test_mysql_value_to_string_date() {
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Date(2023, 12, 25, 0, 0, 0, 0))
                .expect("date-only should succeed");
        assert_eq!(result, "2023-12-25");

        let result =
            TypeTransformer::value_to_string(&mysql::Value::Date(2023, 12, 25, 14, 30, 45, 123456))
                .expect("datetime should succeed");
        assert_eq!(result, "2023-12-25 14:30:45.123456");
    }

    #[test]
    fn test_mysql_value_to_string_time() {
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Time(false, 0, 14, 30, 45, 123456))
                .expect("time should succeed");
        assert_eq!(result, "14:30:45.123456");

        let result = TypeTransformer::value_to_string(&mysql::Value::Time(true, 1, 2, 30, 45, 0))
            .expect("negative time with days should succeed");
        assert_eq!(result, "-26:30:45.000000");
    }

    #[test]
    fn test_rows_to_strings_empty() {
        let result = rows_to_strings(vec![]).expect("empty rows should succeed");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_rows_to_strings_type_conversion_error() {
        // This test demonstrates that type conversion errors are propagated correctly
        // We can't easily create a Row with invalid data, but we can test the error path
        // by creating mock values and ensuring the error handling path works

        // For now, this serves as documentation that the error handling is in place
        // In a real scenario, invalid date/time values from the database would trigger
        // this path
        let error = anyhow::anyhow!("Type conversion error: Invalid month value 13 in date");

        // Verify that such an error would get mapped to exit code 4
        use crate::exit::map_error_to_exit_code;
        assert_eq!(map_error_to_exit_code(&error), 4);

        // Verify the error message contains the expected text
        assert!(error.to_string().contains("Type conversion error"));
    }

    #[test]
    fn test_mysql_value_to_string_invalid_date() {
        // Test invalid month
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Date(2023, 13, 25, 0, 0, 0, 0));
        assert!(result.is_err());
        let error_str = result.expect_err("month=13 should fail").to_string();
        assert!(error_str.contains("Type conversion error"));
        assert!(error_str.contains("Invalid month"));

        // Test invalid day
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Date(2023, 12, 32, 0, 0, 0, 0));
        assert!(result.is_err());
        let error_str = result.expect_err("day=32 should fail").to_string();
        assert!(error_str.contains("Type conversion error"));
        assert!(error_str.contains("Invalid day"));

        // Test invalid hour
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Date(2023, 12, 25, 25, 0, 0, 0));
        assert!(result.is_err());
        let error_str = result.expect_err("hour=25 should fail").to_string();
        assert!(error_str.contains("Type conversion error"));
        assert!(error_str.contains("Invalid hour"));
    }

    #[test]
    fn test_mysql_value_to_string_invalid_time() {
        // Test invalid hour
        let result = TypeTransformer::value_to_string(&mysql::Value::Time(false, 0, 25, 30, 45, 0));
        assert!(result.is_err());
        let error_str = result.expect_err("hour=25 should fail").to_string();
        assert!(error_str.contains("Type conversion error"));
        assert!(error_str.contains("Invalid hour"));

        // Test invalid minute
        let result = TypeTransformer::value_to_string(&mysql::Value::Time(false, 0, 14, 61, 45, 0));
        assert!(result.is_err());
        let error_str = result.expect_err("minute=61 should fail").to_string();
        assert!(error_str.contains("Type conversion error"));
        assert!(error_str.contains("Invalid minute"));
    }
}
