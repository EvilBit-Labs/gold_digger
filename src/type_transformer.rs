use std::collections::BTreeMap;

use anyhow::Context;

/// Maximum total hours for MySQL TIME type (838).
///
/// MySQL TIME range is -838:59:59.000000 to 838:59:59.000000.
/// When days are present, the total hours are computed as `days * 24 + hours`.
const MAX_TIME_TOTAL_HOURS: u32 = 838;

/// Canonical hub for converting MySQL values to Rust types.
///
/// `TypeTransformer` provides safe, panic-free conversion of `mysql::Value` variants
/// into `String` and `serde_json::Value` representations. All methods are associated
/// functions (no `self`) on this zero-sized struct, keeping the API stateless and
/// easy to call from any context.
///
/// # Safety guarantees
///
/// - NULL values are handled gracefully (empty string for CSV/TSV, `Null` for JSON).
/// - Binary data that is not valid UTF-8 is hex-encoded instead of causing panics.
/// - Special float values (NaN, Infinity) are represented as strings.
/// - Date/time values are validated before formatting.
pub struct TypeTransformer;

// ---------------------------------------------------------------------------
// Private validation helpers
// ---------------------------------------------------------------------------

/// Validates MySQL DATE/DATETIME components.
///
/// Returns `Ok(())` when all components are within their valid ranges,
/// or an error describing the first invalid component found.
fn validate_date(
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    microsecond: u32,
) -> anyhow::Result<()> {
    if month == 0 || month > 12 {
        anyhow::bail!(
            "Type conversion error: Invalid month value {} in date",
            month
        );
    }
    if day == 0 || day > 31 {
        anyhow::bail!("Type conversion error: Invalid day value {} in date", day);
    }
    if hour > 23 {
        anyhow::bail!(
            "Type conversion error: Invalid hour value {} in datetime",
            hour
        );
    }
    validate_minute_second_micro(minute, second, microsecond, "datetime")
}

/// Validates MySQL TIME components.
///
/// MySQL TIME range is -838:59:59.000000 to 838:59:59.000000.
/// The `hours` field from the wire protocol is limited to u8 (0-255),
/// but when combined with `days` (`days * 24 + hours`) the total can
/// reach up to 838.
fn validate_time(
    days: u32,
    hours: u8,
    minutes: u8,
    seconds: u8,
    microseconds: u32,
) -> anyhow::Result<()> {
    let total_hours = days * 24 + hours as u32;
    if total_hours > MAX_TIME_TOTAL_HOURS {
        anyhow::bail!(
            "Type conversion error: Invalid total hour value {} in time \
             (max {})",
            total_hours,
            MAX_TIME_TOTAL_HOURS
        );
    }
    validate_minute_second_micro(minutes, seconds, microseconds, "time")
}

/// Validates minute, second, and microsecond components shared by both
/// DATE/DATETIME and TIME types.
fn validate_minute_second_micro(
    minute: u8,
    second: u8,
    microsecond: u32,
    context: &str,
) -> anyhow::Result<()> {
    if minute > 59 {
        anyhow::bail!(
            "Type conversion error: Invalid minute value {} in {}",
            minute,
            context
        );
    }
    if second > 59 {
        anyhow::bail!(
            "Type conversion error: Invalid second value {} in {}",
            second,
            context
        );
    }
    if microsecond > 999999 {
        anyhow::bail!(
            "Type conversion error: Invalid microsecond value {} in {}",
            microsecond,
            context
        );
    }
    Ok(())
}

/// Converts bytes to a string, falling back to hex encoding for non-UTF-8
/// data. Large binary payloads (> 1024 bytes) are truncated with a size
/// indicator.
fn bytes_to_string(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            if bytes.len() > 1024 {
                let hex_prefix: String = bytes
                    .iter()
                    .take(32)
                    .map(|b| format!("{:02x}", b))
                    .collect();
                format!("0x{}... ({} bytes)", hex_prefix, bytes.len())
            } else {
                let hex_string: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
                format!("0x{}", hex_string)
            }
        }
    }
}

/// Formats a float/double value as a string, handling NaN and Infinity
/// specially.
fn format_special_float(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value.is_infinite() {
        if value.is_sign_positive() {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        }
    } else {
        value.to_string()
    }
}

impl TypeTransformer {
    /// Safely converts a MySQL `Value` to its `String` representation.
    ///
    /// This function handles all MySQL value types including NULL values,
    /// binary data, and numeric types without panicking. It returns an error
    /// only for genuinely invalid date/time components.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to a `mysql::Value`.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `String` representation of the value, or an
    /// error for invalid date/time values. NULL values become empty strings.
    ///
    /// # Errors
    ///
    /// Returns an error when date or time components are out of range
    /// (e.g. month > 12, total hours > 838).
    pub fn value_to_string(value: &mysql::Value) -> anyhow::Result<String> {
        match value {
            mysql::Value::NULL => Ok(String::new()),
            mysql::Value::Bytes(bytes) => Ok(bytes_to_string(bytes)),
            mysql::Value::Int(i) => Ok(i.to_string()),
            mysql::Value::UInt(u) => Ok(u.to_string()),
            mysql::Value::Float(f) => {
                if f.is_nan() {
                    Ok("NaN".to_string())
                } else if f.is_infinite() {
                    Ok(if f.is_sign_positive() {
                        "Infinity"
                    } else {
                        "-Infinity"
                    }
                    .to_string())
                } else {
                    Ok(f.to_string())
                }
            }
            mysql::Value::Double(d) => Ok(format_special_float(*d)),
            mysql::Value::Date(year, month, day, hour, minute, second, microsecond) => {
                validate_date(*month, *day, *hour, *minute, *second, *microsecond)?;

                if *hour == 0 && *minute == 0 && *second == 0 && *microsecond == 0 {
                    Ok(format!("{:04}-{:02}-{:02}", year, month, day))
                } else {
                    Ok(format!(
                        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
                        year, month, day, hour, minute, second, microsecond
                    ))
                }
            }
            mysql::Value::Time(negative, days, hours, minutes, seconds, microseconds) => {
                validate_time(*days, *hours, *minutes, *seconds, *microseconds)?;

                let sign = if *negative { "-" } else { "" };
                let total_hours = *days * 24 + *hours as u32;
                Ok(format!(
                    "{}{:02}:{:02}:{:02}.{:06}",
                    sign, total_hours, minutes, seconds, microseconds
                ))
            }
        }
    }

    /// Converts a MySQL `Value` to a `serde_json::Value` with native JSON
    /// types.
    ///
    /// Maps each MySQL variant to the most appropriate JSON type:
    /// - `NULL` becomes `Null`
    /// - Integers become `Number`
    /// - Floats/Doubles become `Number` (or `String` for NaN/Infinity)
    /// - Bytes become `String` (UTF-8 or hex-encoded)
    /// - Dates become `String` (ISO-8601 format with `T` separator)
    /// - Times become `String` (HH:MM:SS.ffffff)
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to a `mysql::Value`.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `serde_json::Value`, or an error for invalid
    /// date/time components.
    ///
    /// # Errors
    ///
    /// Returns an error when date or time components are out of valid range.
    pub fn value_to_json(value: &mysql::Value) -> anyhow::Result<serde_json::Value> {
        match value {
            mysql::Value::NULL => Ok(serde_json::Value::Null),
            mysql::Value::Int(i) => Ok(serde_json::Value::Number((*i).into())),
            mysql::Value::UInt(u) => Ok(serde_json::Value::Number((*u).into())),
            mysql::Value::Float(f) => {
                let f64_val = f64::from(*f);
                Ok(serde_json::Number::from_f64(f64_val)
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| serde_json::Value::String(format_special_float(f64_val))))
            }
            mysql::Value::Double(d) => Ok(serde_json::Number::from_f64(*d)
                .map(serde_json::Value::Number)
                .unwrap_or_else(|| serde_json::Value::String(format_special_float(*d)))),
            mysql::Value::Bytes(bytes) => Ok(serde_json::Value::String(bytes_to_string(bytes))),
            mysql::Value::Date(year, month, day, hour, minute, second, microsecond) => {
                validate_date(*month, *day, *hour, *minute, *second, *microsecond)?;

                if *hour == 0 && *minute == 0 && *second == 0 && *microsecond == 0 {
                    Ok(serde_json::Value::String(format!(
                        "{:04}-{:02}-{:02}",
                        year, month, day
                    )))
                } else {
                    Ok(serde_json::Value::String(format!(
                        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}",
                        year, month, day, hour, minute, second, microsecond
                    )))
                }
            }
            mysql::Value::Time(negative, days, hours, minutes, seconds, microseconds) => {
                validate_time(*days, *hours, *minutes, *seconds, *microseconds)?;

                let sign = if *negative { "-" } else { "" };
                let total_hours = *days * 24 + *hours as u32;
                Ok(serde_json::Value::String(format!(
                    "{}{:02}:{:02}:{:02}.{:06}",
                    sign, total_hours, minutes, seconds, microseconds
                )))
            }
        }
    }

    /// Converts a single MySQL `Row` into a vector of strings.
    ///
    /// Iterates over every column in the row and delegates to
    /// [`Self::value_to_string`] for each value.
    ///
    /// # Arguments
    ///
    /// * `row` - A `mysql::Row` to convert.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<String>` with one entry per column, or
    /// an error if any value conversion fails or a column is missing.
    pub fn row_to_strings(row: mysql::Row) -> anyhow::Result<Vec<String>> {
        let mut values = Vec::with_capacity(row.len());
        for i in 0..row.len() {
            match row.as_ref(i) {
                Some(value) => {
                    let s = Self::value_to_string(value)
                        .with_context(|| format!("Failed to convert column {} to string", i + 1))?;
                    values.push(s);
                }
                None => {
                    anyhow::bail!("Unexpected missing value at column index {}", i);
                }
            }
        }
        Ok(values)
    }

    /// Converts a single MySQL `Row` into a `BTreeMap` of column names to
    /// JSON values.
    ///
    /// Column names are extracted from the row metadata and values are
    /// converted via [`Self::value_to_json`]. The `BTreeMap` guarantees
    /// deterministic (alphabetical) key ordering in serialised output.
    ///
    /// # Arguments
    ///
    /// * `row` - A `mysql::Row` to convert.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `BTreeMap<String, serde_json::Value>`, or an
    /// error if a column index is out of range or value conversion fails.
    pub fn row_to_json(row: mysql::Row) -> anyhow::Result<BTreeMap<String, serde_json::Value>> {
        let columns: Vec<String> = row
            .columns_ref()
            .iter()
            .map(|col| col.name_str().to_string())
            .collect();

        let mut map = BTreeMap::new();
        for (i, col_name) in columns.into_iter().enumerate() {
            match row.as_ref(i) {
                Some(value) => {
                    let json_val = Self::value_to_json(value).with_context(|| {
                        format!("Failed to convert column '{}' to JSON", col_name)
                    })?;
                    map.insert(col_name, json_val);
                }
                None => {
                    anyhow::bail!("Unexpected missing value at column index {}", i);
                }
            }
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // value_to_string tests
    // ---------------------------------------------------------------

    #[test]
    fn test_value_to_string_null() {
        let result = TypeTransformer::value_to_string(&mysql::Value::NULL);
        assert!(result.is_ok(), "NULL conversion should succeed");
        assert_eq!(result.expect("checked above"), "");
    }

    #[test]
    fn test_value_to_string_integers() {
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::Int(42))
                .expect("Int conversion should succeed"),
            "42"
        );
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::Int(-42))
                .expect("negative Int conversion should succeed"),
            "-42"
        );
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::UInt(123))
                .expect("UInt conversion should succeed"),
            "123"
        );
    }

    #[test]
    fn test_value_to_string_floats() {
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::Float(3.5))
                .expect("Float conversion should succeed"),
            "3.5"
        );
        assert_eq!(
            TypeTransformer::value_to_string(&mysql::Value::Double(2.5))
                .expect("Double conversion should succeed"),
            "2.5"
        );
    }

    #[test]
    fn test_value_to_string_special_floats() {
        // NaN
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
        // Infinity
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
        // Negative Infinity
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
    fn test_value_to_string_bytes_valid_utf8() {
        let bytes = b"hello world".to_vec();
        let result = TypeTransformer::value_to_string(&mysql::Value::Bytes(bytes));
        assert_eq!(result.expect("valid UTF-8 should succeed"), "hello world");
    }

    #[test]
    fn test_value_to_string_bytes_empty() {
        let result = TypeTransformer::value_to_string(&mysql::Value::Bytes(vec![]));
        assert_eq!(result.expect("empty bytes should succeed"), "");
    }

    #[test]
    fn test_value_to_string_bytes_invalid_utf8() {
        let invalid_bytes = vec![0xFF, 0xFE, 0xFD];
        let result = TypeTransformer::value_to_string(&mysql::Value::Bytes(invalid_bytes));
        assert_eq!(result.expect("hex fallback should succeed"), "0xfffefd");
    }

    #[test]
    fn test_value_to_string_bytes_at_truncation_boundary() {
        // Exactly 1024 bytes - should NOT truncate
        let bytes_1024 = vec![0xAB; 1024];
        let result = TypeTransformer::value_to_string(&mysql::Value::Bytes(bytes_1024));
        let s = result.expect("1024 bytes should succeed");
        assert!(s.starts_with("0x"));
        assert!(!s.contains("..."), "1024 bytes should not truncate");

        // 1025 bytes - SHOULD truncate
        let bytes_1025 = vec![0xAB; 1025];
        let result = TypeTransformer::value_to_string(&mysql::Value::Bytes(bytes_1025));
        let s = result.expect("1025 bytes should succeed");
        assert!(s.starts_with("0x"));
        assert!(s.contains("... (1025 bytes)"), "1025 bytes should truncate");
    }

    #[test]
    fn test_value_to_string_bytes_large_binary() {
        let large_bytes = vec![0xAB; 2000];
        let result = TypeTransformer::value_to_string(&mysql::Value::Bytes(large_bytes));
        let s = result.expect("large binary should succeed");
        assert!(s.starts_with("0x"));
        assert!(s.contains("... (2000 bytes)"));
    }

    #[test]
    fn test_value_to_string_date_only() {
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Date(2023, 12, 25, 0, 0, 0, 0));
        assert_eq!(result.expect("date-only should succeed"), "2023-12-25");
    }

    #[test]
    fn test_value_to_string_datetime() {
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Date(2023, 12, 25, 14, 30, 45, 123456));
        assert_eq!(
            result.expect("datetime should succeed"),
            "2023-12-25 14:30:45.123456"
        );
    }

    #[test]
    fn test_value_to_string_invalid_date() {
        // Invalid month (0)
        let result = TypeTransformer::value_to_string(&mysql::Value::Date(2023, 0, 25, 0, 0, 0, 0));
        assert!(result.is_err());
        assert!(
            result
                .expect_err("month=0 should fail")
                .to_string()
                .contains("Invalid month")
        );

        // Invalid month (13)
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Date(2023, 13, 25, 0, 0, 0, 0));
        assert!(result.is_err());
        assert!(
            result
                .expect_err("month=13 should fail")
                .to_string()
                .contains("Invalid month")
        );

        // Invalid day (0)
        let result = TypeTransformer::value_to_string(&mysql::Value::Date(2023, 12, 0, 0, 0, 0, 0));
        assert!(result.is_err());
        assert!(
            result
                .expect_err("day=0 should fail")
                .to_string()
                .contains("Invalid day")
        );

        // Invalid day (32)
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Date(2023, 12, 32, 0, 0, 0, 0));
        assert!(result.is_err());
        assert!(
            result
                .expect_err("day=32 should fail")
                .to_string()
                .contains("Invalid day")
        );

        // Invalid hour (25 in datetime context)
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Date(2023, 12, 25, 25, 0, 0, 0));
        assert!(result.is_err());
        assert!(
            result
                .expect_err("hour=25 should fail")
                .to_string()
                .contains("Invalid hour")
        );

        // Invalid microsecond
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Date(2023, 12, 25, 0, 0, 0, 1_000_000));
        assert!(result.is_err());
        assert!(
            result
                .expect_err("microsecond=1000000 should fail")
                .to_string()
                .contains("Invalid microsecond")
        );
    }

    #[test]
    fn test_value_to_string_time() {
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Time(false, 0, 14, 30, 45, 123456));
        assert_eq!(result.expect("time should succeed"), "14:30:45.123456");
    }

    #[test]
    fn test_value_to_string_time_negative_with_days() {
        let result = TypeTransformer::value_to_string(&mysql::Value::Time(true, 1, 2, 30, 45, 0));
        assert_eq!(
            result.expect("negative time with days should succeed"),
            "-26:30:45.000000"
        );
    }

    #[test]
    fn test_value_to_string_time_large_hours() {
        // MySQL TIME supports up to 838:59:59
        // 34 days * 24 + 22 hours = 838 total hours
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Time(false, 34, 22, 59, 59, 999999));
        assert_eq!(
            result.expect("838 hours should succeed"),
            "838:59:59.999999"
        );

        // Negative max
        let result = TypeTransformer::value_to_string(&mysql::Value::Time(true, 34, 22, 59, 59, 0));
        assert_eq!(
            result.expect("negative 838 hours should succeed"),
            "-838:59:59.000000"
        );
    }

    #[test]
    fn test_value_to_string_time_exceeds_max() {
        // 35 days * 24 = 840 total hours, exceeds 838
        let result = TypeTransformer::value_to_string(&mysql::Value::Time(false, 35, 0, 0, 0, 0));
        assert!(result.is_err());
        assert!(
            result
                .expect_err("840 hours should fail")
                .to_string()
                .contains("Invalid total hour")
        );
    }

    #[test]
    fn test_value_to_string_invalid_time() {
        // Invalid minute
        let result = TypeTransformer::value_to_string(&mysql::Value::Time(false, 0, 14, 61, 45, 0));
        assert!(result.is_err());
        assert!(
            result
                .expect_err("minute=61 should fail")
                .to_string()
                .contains("Invalid minute")
        );

        // Invalid second
        let result = TypeTransformer::value_to_string(&mysql::Value::Time(false, 0, 14, 30, 60, 0));
        assert!(result.is_err());
        assert!(
            result
                .expect_err("second=60 should fail")
                .to_string()
                .contains("Invalid second")
        );

        // Invalid microsecond
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Time(false, 0, 14, 30, 45, 1_000_000));
        assert!(result.is_err());
        assert!(
            result
                .expect_err("microsecond=1000000 should fail")
                .to_string()
                .contains("Invalid microsecond")
        );
    }

    // ---------------------------------------------------------------
    // value_to_json tests
    // ---------------------------------------------------------------

    #[test]
    fn test_value_to_json_null() {
        let result =
            TypeTransformer::value_to_json(&mysql::Value::NULL).expect("NULL should succeed");
        assert_eq!(result, serde_json::Value::Null);
    }

    #[test]
    fn test_value_to_json_int() {
        let result =
            TypeTransformer::value_to_json(&mysql::Value::Int(42)).expect("Int should succeed");
        assert_eq!(result, serde_json::json!(42));

        let result = TypeTransformer::value_to_json(&mysql::Value::Int(-1))
            .expect("negative Int should succeed");
        assert_eq!(result, serde_json::json!(-1));
    }

    #[test]
    fn test_value_to_json_uint() {
        let result = TypeTransformer::value_to_json(&mysql::Value::UInt(u64::MAX))
            .expect("UInt should succeed");
        assert_eq!(result, serde_json::json!(u64::MAX));
    }

    #[test]
    fn test_value_to_json_float_normal() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Float(3.5))
            .expect("Float should succeed");
        assert!(result.is_number());
        assert_eq!(
            result.as_f64().expect("should be a number"),
            f64::from(3.5_f32)
        );
    }

    #[test]
    fn test_value_to_json_float_nan() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Float(f32::NAN))
            .expect("Float NaN should succeed");
        assert_eq!(result, serde_json::Value::String("NaN".to_string()));
    }

    #[test]
    fn test_value_to_json_float_infinity() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Float(f32::INFINITY))
            .expect("Float Infinity should succeed");
        assert_eq!(result, serde_json::Value::String("Infinity".to_string()));
    }

    #[test]
    fn test_value_to_json_float_neg_infinity() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Float(f32::NEG_INFINITY))
            .expect("Float -Infinity should succeed");
        assert_eq!(result, serde_json::Value::String("-Infinity".to_string()));
    }

    #[test]
    fn test_value_to_json_double_normal() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Double(2.5))
            .expect("Double should succeed");
        assert!(result.is_number());
        assert_eq!(result.as_f64().expect("should be a number"), 2.5);
    }

    #[test]
    fn test_value_to_json_double_nan() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Double(f64::NAN))
            .expect("Double NaN should succeed");
        assert_eq!(result, serde_json::Value::String("NaN".to_string()));
    }

    #[test]
    fn test_value_to_json_double_infinity() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Double(f64::INFINITY))
            .expect("Double Infinity should succeed");
        assert_eq!(result, serde_json::Value::String("Infinity".to_string()));
    }

    #[test]
    fn test_value_to_json_double_neg_infinity() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Double(f64::NEG_INFINITY))
            .expect("Double -Infinity should succeed");
        assert_eq!(result, serde_json::Value::String("-Infinity".to_string()));
    }

    #[test]
    fn test_value_to_json_bytes_valid_utf8() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Bytes(b"hello".to_vec()))
            .expect("Bytes should succeed");
        assert_eq!(result, serde_json::Value::String("hello".to_string()));
    }

    #[test]
    fn test_value_to_json_bytes_invalid_utf8() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Bytes(vec![0xFF, 0xFE]))
            .expect("hex Bytes should succeed");
        let s = result.as_str().expect("should be a string");
        assert!(s.starts_with("0x"));
    }

    #[test]
    fn test_value_to_json_date() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Date(2023, 12, 25, 0, 0, 0, 0))
            .expect("date should succeed");
        assert_eq!(result, serde_json::Value::String("2023-12-25".to_string()));
    }

    #[test]
    fn test_value_to_json_datetime() {
        let result =
            TypeTransformer::value_to_json(&mysql::Value::Date(2023, 12, 25, 14, 30, 45, 0))
                .expect("datetime should succeed");
        assert_eq!(
            result,
            serde_json::Value::String("2023-12-25T14:30:45.000000".to_string())
        );
    }

    #[test]
    fn test_value_to_json_datetime_with_microseconds() {
        let result =
            TypeTransformer::value_to_json(&mysql::Value::Date(2023, 12, 25, 14, 30, 45, 123456))
                .expect("datetime with microseconds should succeed");
        assert_eq!(
            result,
            serde_json::Value::String("2023-12-25T14:30:45.123456".to_string())
        );
    }

    #[test]
    fn test_value_to_json_invalid_date_returns_error() {
        // Invalid month -- should now return Err, not a string
        let result = TypeTransformer::value_to_json(&mysql::Value::Date(2023, 13, 25, 0, 0, 0, 0));
        assert!(result.is_err());
        assert!(
            result
                .expect_err("month=13 should fail")
                .to_string()
                .contains("Invalid month")
        );
    }

    #[test]
    fn test_value_to_json_invalid_time_returns_error() {
        // Invalid minute
        let result = TypeTransformer::value_to_json(&mysql::Value::Time(false, 0, 14, 61, 45, 0));
        assert!(result.is_err());
        assert!(
            result
                .expect_err("minute=61 should fail")
                .to_string()
                .contains("Invalid minute")
        );
    }

    #[test]
    fn test_value_to_json_time() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Time(false, 0, 14, 30, 45, 0))
            .expect("time should succeed");
        assert_eq!(
            result,
            serde_json::Value::String("14:30:45.000000".to_string())
        );
    }

    #[test]
    fn test_value_to_json_time_large_hours() {
        // MySQL TIME supports hours up to 838
        let result = TypeTransformer::value_to_json(&mysql::Value::Time(false, 34, 22, 0, 0, 0))
            .expect("838 hours JSON should succeed");
        assert_eq!(
            result,
            serde_json::Value::String("838:00:00.000000".to_string())
        );
    }

    // ---------------------------------------------------------------
    // Helper function tests
    // ---------------------------------------------------------------

    #[test]
    fn test_validate_date_valid() {
        assert!(validate_date(1, 1, 0, 0, 0, 0).is_ok());
        assert!(validate_date(12, 31, 23, 59, 59, 999999).is_ok());
    }

    #[test]
    fn test_validate_date_invalid() {
        assert!(validate_date(0, 1, 0, 0, 0, 0).is_err());
        assert!(validate_date(13, 1, 0, 0, 0, 0).is_err());
        assert!(validate_date(1, 0, 0, 0, 0, 0).is_err());
        assert!(validate_date(1, 32, 0, 0, 0, 0).is_err());
        assert!(validate_date(1, 1, 24, 0, 0, 0).is_err());
        assert!(validate_date(1, 1, 0, 60, 0, 0).is_err());
        assert!(validate_date(1, 1, 0, 0, 60, 0).is_err());
        assert!(validate_date(1, 1, 0, 0, 0, 1_000_000).is_err());
    }

    #[test]
    fn test_validate_time_valid() {
        assert!(validate_time(0, 0, 0, 0, 0).is_ok());
        assert!(validate_time(0, 23, 59, 59, 999999).is_ok());
        // Max: 34*24 + 22 = 838
        assert!(validate_time(34, 22, 59, 59, 999999).is_ok());
    }

    #[test]
    fn test_validate_time_invalid() {
        // 35*24 = 840 > 838
        assert!(validate_time(35, 0, 0, 0, 0).is_err());
        assert!(validate_time(0, 0, 60, 0, 0).is_err());
        assert!(validate_time(0, 0, 0, 60, 0).is_err());
        assert!(validate_time(0, 0, 0, 0, 1_000_000).is_err());
    }

    #[test]
    fn test_bytes_to_string_empty() {
        assert_eq!(bytes_to_string(&[]), "");
    }

    #[test]
    fn test_format_special_float() {
        assert_eq!(format_special_float(f64::NAN), "NaN");
        assert_eq!(format_special_float(f64::INFINITY), "Infinity");
        assert_eq!(format_special_float(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(format_special_float(3.5), "3.5");
    }
}
