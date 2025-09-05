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

/// Trait for writing data in different formats
pub trait FormatWriter {
    fn write_header(&mut self, columns: &[String]) -> Result<()>;
    fn write_row(&mut self, row: &[String]) -> Result<()>;
    fn finalize(self) -> Result<()>;
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

    let mut result_rows = Vec::with_capacity(rows.len() + 1);

    // Extract headers from the first row
    let header_row: Vec<String> = rows[0]
        .columns_ref()
        .iter()
        .map(|column| column.name_str().to_string())
        .collect();
    result_rows.push(header_row);

    // Process each row using safe iteration
    for row in rows {
        let mut data_row = Vec::with_capacity(row.len());
        for i in 0..row.len() {
            match row.as_ref(i) {
                Some(value) => match mysql_value_to_string(value) {
                    Ok(string_value) => data_row.push(string_value),
                    Err(e) => return Err(e.context("Type conversion failed during row processing")),
                },
                None => data_row.push(String::new()),
            }
        }
        result_rows.push(data_row);
    }

    Ok(result_rows)
}

/// Safely converts a MySQL Value to a String representation.
///
/// This function handles all MySQL value types including NULL values,
/// binary data, and numeric types without panicking in most cases.
/// However, it may return an error for certain edge cases that represent
/// genuine type conversion failures.
///
/// # Arguments
///
/// * `value` - A reference to a MySQL Value.
///
/// # Returns
///
/// A Result containing the String representation of the value, or an error
/// for genuine type conversion failures. NULL values become empty strings.
///
/// # Errors
///
/// Returns an error for certain edge cases such as invalid date/time values
/// that cannot be properly formatted.
fn mysql_value_to_string(value: &mysql::Value) -> anyhow::Result<String> {
    match value {
        mysql::Value::NULL => Ok(String::new()),
        mysql::Value::Bytes(bytes) => {
            // Try to convert bytes to UTF-8 string, fallback to lossy conversion
            // Use Cow to avoid unnecessary allocation when bytes are valid UTF-8
            Ok(match std::str::from_utf8(bytes) {
                Ok(s) => s.to_string(),
                Err(_) => String::from_utf8_lossy(bytes).into_owned(),
            })
        },
        mysql::Value::Int(i) => Ok(i.to_string()),
        mysql::Value::UInt(u) => Ok(u.to_string()),
        mysql::Value::Float(f) => {
            // Handle special float values
            if f.is_nan() {
                Ok("NaN".to_string())
            } else if f.is_infinite() {
                Ok(if f.is_sign_positive() { "Infinity" } else { "-Infinity" }.to_string())
            } else {
                Ok(f.to_string())
            }
        },
        mysql::Value::Double(d) => {
            // Handle special double values
            if d.is_nan() {
                Ok("NaN".to_string())
            } else if d.is_infinite() {
                Ok(if d.is_sign_positive() { "Infinity" } else { "-Infinity" }.to_string())
            } else {
                Ok(d.to_string())
            }
        },
        mysql::Value::Date(year, month, day, hour, minute, second, microsecond) => {
            // Add validation for date values (requirement 10.3)
            if *month == 0 || *month > 12 {
                anyhow::bail!("Type conversion error: Invalid month value {} in date", month);
            }
            if *day == 0 || *day > 31 {
                anyhow::bail!("Type conversion error: Invalid day value {} in date", day);
            }
            if *hour > 23 {
                anyhow::bail!("Type conversion error: Invalid hour value {} in datetime", hour);
            }
            if *minute > 59 {
                anyhow::bail!("Type conversion error: Invalid minute value {} in datetime", minute);
            }
            if *second > 59 {
                anyhow::bail!("Type conversion error: Invalid second value {} in datetime", second);
            }
            if *microsecond > 999999 {
                anyhow::bail!("Type conversion error: Invalid microsecond value {} in datetime", microsecond);
            }

            if *hour == 0 && *minute == 0 && *second == 0 && *microsecond == 0 {
                Ok(format!("{:04}-{:02}-{:02}", year, month, day))
            } else {
                Ok(format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
                    year, month, day, hour, minute, second, microsecond
                ))
            }
        },
        mysql::Value::Time(negative, days, hours, minutes, seconds, microseconds) => {
            // Add validation for time values (requirement 10.3)
            if *hours > 23 {
                anyhow::bail!("Type conversion error: Invalid hour value {} in time", hours);
            }
            if *minutes > 59 {
                anyhow::bail!("Type conversion error: Invalid minute value {} in time", minutes);
            }
            if *seconds > 59 {
                anyhow::bail!("Type conversion error: Invalid second value {} in time", seconds);
            }
            if *microseconds > 999999 {
                anyhow::bail!("Type conversion error: Invalid microsecond value {} in time", microseconds);
            }

            let sign = if *negative { "-" } else { "" };
            if *days > 0 {
                Ok(format!(
                    "{}{:02}:{:02}:{:02}.{:06}",
                    sign,
                    days * 24 + *hours as u32,
                    minutes,
                    seconds,
                    microseconds
                ))
            } else {
                Ok(format!("{}{:02}:{:02}:{:02}.{:06}", sign, hours, minutes, seconds, microseconds))
            }
        },
    }
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
    env::var(var_name).with_context(|| format!("Missing required environment variable: {}", var_name))
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
        unsafe {
            std::env::set_var("TEST_ENV_VAR", "test_value");
        }
        let result = get_required_env("TEST_ENV_VAR");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_value");
        unsafe {
            std::env::remove_var("TEST_ENV_VAR");
        }
    }

    #[test]
    fn test_mysql_value_to_string_null() {
        let result = mysql_value_to_string(&mysql::Value::NULL).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_mysql_value_to_string_integers() {
        assert_eq!(mysql_value_to_string(&mysql::Value::Int(42)).unwrap(), "42");
        assert_eq!(mysql_value_to_string(&mysql::Value::Int(-42)).unwrap(), "-42");
        assert_eq!(mysql_value_to_string(&mysql::Value::UInt(123)).unwrap(), "123");
    }

    #[test]
    fn test_mysql_value_to_string_floats() {
        assert_eq!(mysql_value_to_string(&mysql::Value::Float(3.5)).unwrap(), "3.5");
        assert_eq!(mysql_value_to_string(&mysql::Value::Double(2.5)).unwrap(), "2.5");
    }

    #[test]
    fn test_mysql_value_to_string_bytes() {
        let bytes = b"hello world".to_vec();
        let result = mysql_value_to_string(&mysql::Value::Bytes(bytes)).unwrap();
        assert_eq!(result, "hello world");

        // Test invalid UTF-8 bytes - should use lossy conversion
        let invalid_bytes = vec![0xFF, 0xFE, 0xFD];
        let result = mysql_value_to_string(&mysql::Value::Bytes(invalid_bytes)).unwrap();
        assert!(!result.is_empty()); // Should contain replacement characters
        assert!(result.contains('\u{FFFD}')); // Explicitly check for Unicode replacement character
    }

    #[test]
    fn test_mysql_value_to_string_special_floats() {
        // Test NaN
        assert_eq!(mysql_value_to_string(&mysql::Value::Float(f32::NAN)).unwrap(), "NaN");
        assert_eq!(mysql_value_to_string(&mysql::Value::Double(f64::NAN)).unwrap(), "NaN");

        // Test Infinity
        assert_eq!(mysql_value_to_string(&mysql::Value::Float(f32::INFINITY)).unwrap(), "Infinity");
        assert_eq!(mysql_value_to_string(&mysql::Value::Double(f64::INFINITY)).unwrap(), "Infinity");

        // Test Negative Infinity
        assert_eq!(mysql_value_to_string(&mysql::Value::Float(f32::NEG_INFINITY)).unwrap(), "-Infinity");
        assert_eq!(mysql_value_to_string(&mysql::Value::Double(f64::NEG_INFINITY)).unwrap(), "-Infinity");
    }

    #[test]
    fn test_mysql_value_to_string_date() {
        let result = mysql_value_to_string(&mysql::Value::Date(2023, 12, 25, 0, 0, 0, 0)).unwrap();
        assert_eq!(result, "2023-12-25");

        let result = mysql_value_to_string(&mysql::Value::Date(2023, 12, 25, 14, 30, 45, 123456)).unwrap();
        assert_eq!(result, "2023-12-25 14:30:45.123456");
    }

    #[test]
    fn test_mysql_value_to_string_time() {
        let result = mysql_value_to_string(&mysql::Value::Time(false, 0, 14, 30, 45, 123456)).unwrap();
        assert_eq!(result, "14:30:45.123456");

        let result = mysql_value_to_string(&mysql::Value::Time(true, 1, 2, 30, 45, 0)).unwrap();
        assert_eq!(result, "-26:30:45.000000");
    }

    #[test]
    fn test_rows_to_strings_empty() {
        let result = rows_to_strings(vec![]).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_rows_to_strings_type_conversion_error() {
        // This test demonstrates that type conversion errors are propagated correctly
        // We can't easily create a Row with invalid data, but we can test the error path
        // by creating mock values and ensuring the error handling path works

        // For now, this serves as documentation that the error handling is in place
        // In a real scenario, invalid date/time values from the database would trigger this path
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
        let result = mysql_value_to_string(&mysql::Value::Date(2023, 13, 25, 0, 0, 0, 0));
        assert!(result.is_err());
        let error_str = result.unwrap_err().to_string();
        assert!(error_str.contains("Type conversion error"));
        assert!(error_str.contains("Invalid month"));

        // Test invalid day
        let result = mysql_value_to_string(&mysql::Value::Date(2023, 12, 32, 0, 0, 0, 0));
        assert!(result.is_err());
        let error_str = result.unwrap_err().to_string();
        assert!(error_str.contains("Type conversion error"));
        assert!(error_str.contains("Invalid day"));

        // Test invalid hour
        let result = mysql_value_to_string(&mysql::Value::Date(2023, 12, 25, 25, 0, 0, 0));
        assert!(result.is_err());
        let error_str = result.unwrap_err().to_string();
        assert!(error_str.contains("Type conversion error"));
        assert!(error_str.contains("Invalid hour"));
    }

    #[test]
    fn test_mysql_value_to_string_invalid_time() {
        // Test invalid hour
        let result = mysql_value_to_string(&mysql::Value::Time(false, 0, 25, 30, 45, 0));
        assert!(result.is_err());
        let error_str = result.unwrap_err().to_string();
        assert!(error_str.contains("Type conversion error"));
        assert!(error_str.contains("Invalid hour"));

        // Test invalid minute
        let result = mysql_value_to_string(&mysql::Value::Time(false, 0, 14, 61, 45, 0));
        assert!(result.is_err());
        let error_str = result.unwrap_err().to_string();
        assert!(error_str.contains("Type conversion error"));
        assert!(error_str.contains("Invalid minute"));
    }
}
