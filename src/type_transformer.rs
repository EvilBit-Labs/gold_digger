use std::collections::BTreeMap;

use anyhow::Context;

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
    /// A `Result` containing the `String` representation of the value, or an error
    /// for invalid date/time values. NULL values become empty strings.
    ///
    /// # Errors
    ///
    /// Returns an error when date or time components are out of range
    /// (e.g. month > 12, hour > 23).
    pub fn value_to_string(value: &mysql::Value) -> anyhow::Result<String> {
        match value {
            mysql::Value::NULL => Ok(String::new()),
            mysql::Value::Bytes(bytes) => {
                // Try to convert bytes to UTF-8 string, fallback to hex encoding
                // for binary data
                match std::str::from_utf8(bytes) {
                    Ok(s) => Ok(s.to_string()),
                    Err(_) => {
                        // For binary data that's not valid UTF-8, use hex encoding.
                        // This prevents data corruption and provides deterministic
                        // output.
                        if bytes.len() > 1024 {
                            // For large binary data, truncate and indicate
                            let hex_prefix = bytes
                                .iter()
                                .take(32)
                                .map(|b| format!("{:02x}", b))
                                .collect::<String>();
                            Ok(format!("0x{}... ({} bytes)", hex_prefix, bytes.len()))
                        } else {
                            let hex_string = bytes
                                .iter()
                                .map(|b| format!("{:02x}", b))
                                .collect::<String>();
                            Ok(format!("0x{}", hex_string))
                        }
                    }
                }
            }
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
            mysql::Value::Double(d) => {
                if d.is_nan() {
                    Ok("NaN".to_string())
                } else if d.is_infinite() {
                    Ok(if d.is_sign_positive() {
                        "Infinity"
                    } else {
                        "-Infinity"
                    }
                    .to_string())
                } else {
                    Ok(d.to_string())
                }
            }
            mysql::Value::Date(year, month, day, hour, minute, second, microsecond) => {
                // Validate date components (requirement 10.3)
                if *month == 0 || *month > 12 {
                    anyhow::bail!(
                        "Type conversion error: Invalid month value {} in date",
                        month
                    );
                }
                if *day == 0 || *day > 31 {
                    anyhow::bail!("Type conversion error: Invalid day value {} in date", day);
                }
                if *hour > 23 {
                    anyhow::bail!(
                        "Type conversion error: Invalid hour value {} in datetime",
                        hour
                    );
                }
                if *minute > 59 {
                    anyhow::bail!(
                        "Type conversion error: Invalid minute value {} in datetime",
                        minute
                    );
                }
                if *second > 59 {
                    anyhow::bail!(
                        "Type conversion error: Invalid second value {} in datetime",
                        second
                    );
                }
                if *microsecond > 999999 {
                    anyhow::bail!(
                        "Type conversion error: Invalid microsecond value {} in datetime",
                        microsecond
                    );
                }

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
                // Validate time components (requirement 10.3)
                if *hours > 23 {
                    anyhow::bail!(
                        "Type conversion error: Invalid hour value {} in time",
                        hours
                    );
                }
                if *minutes > 59 {
                    anyhow::bail!(
                        "Type conversion error: Invalid minute value {} in time",
                        minutes
                    );
                }
                if *seconds > 59 {
                    anyhow::bail!(
                        "Type conversion error: Invalid second value {} in time",
                        seconds
                    );
                }
                if *microseconds > 999999 {
                    anyhow::bail!(
                        "Type conversion error: Invalid microsecond value {} in time",
                        microseconds
                    );
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
                    Ok(format!(
                        "{}{:02}:{:02}:{:02}.{:06}",
                        sign, hours, minutes, seconds, microseconds
                    ))
                }
            }
        }
    }

    /// Converts a MySQL `Value` to a `serde_json::Value` with native JSON types.
    ///
    /// Maps each MySQL variant to the most appropriate JSON type:
    /// - `NULL` becomes `Null`
    /// - Integers become `Number`
    /// - Floats/Doubles become `Number` (or `String` for NaN/Infinity)
    /// - Bytes become `String` (UTF-8 or hex-encoded)
    /// - Dates/Times become `String` (ISO-8601 format)
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to a `mysql::Value`.
    ///
    /// # Returns
    ///
    /// A `serde_json::Value` representing the MySQL value. This function is
    /// infallible -- invalid dates/times produce a `String` with the error
    /// message rather than panicking.
    pub fn value_to_json(value: &mysql::Value) -> serde_json::Value {
        match value {
            mysql::Value::NULL => serde_json::Value::Null,
            mysql::Value::Int(i) => serde_json::Value::Number((*i).into()),
            mysql::Value::UInt(u) => serde_json::Value::Number((*u).into()),
            mysql::Value::Float(f) => {
                serde_json::Number::from_f64(f64::from(*f))
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| {
                        // NaN / Infinity cannot be represented as JSON numbers
                        serde_json::Value::String(Self::value_to_string(value).unwrap_or_default())
                    })
            }
            mysql::Value::Double(d) => serde_json::Number::from_f64(*d)
                .map(serde_json::Value::Number)
                .unwrap_or_else(|| {
                    serde_json::Value::String(Self::value_to_string(value).unwrap_or_default())
                }),
            mysql::Value::Date(year, month, day, hour, minute, second, microsecond) => {
                // Validate date components inline (same rules as value_to_string)
                if *month == 0 || *month > 12 {
                    return serde_json::Value::String(format!(
                        "Type conversion error: Invalid month value {} in date",
                        month
                    ));
                }
                if *day == 0 || *day > 31 {
                    return serde_json::Value::String(format!(
                        "Type conversion error: Invalid day value {} in date",
                        day
                    ));
                }
                if *hour > 23 {
                    return serde_json::Value::String(format!(
                        "Type conversion error: Invalid hour value {} in datetime",
                        hour
                    ));
                }
                if *minute > 59 {
                    return serde_json::Value::String(format!(
                        "Type conversion error: Invalid minute value {} in datetime",
                        minute
                    ));
                }
                if *second > 59 {
                    return serde_json::Value::String(format!(
                        "Type conversion error: Invalid second value {} in datetime",
                        second
                    ));
                }
                if *microsecond > 999999 {
                    return serde_json::Value::String(format!(
                        "Type conversion error: Invalid microsecond value {} in datetime",
                        microsecond
                    ));
                }

                // Date-only: YYYY-MM-DD
                if *hour == 0 && *minute == 0 && *second == 0 && *microsecond == 0 {
                    serde_json::Value::String(format!("{:04}-{:02}-{:02}", year, month, day))
                } else {
                    // ISO-8601 datetime: YYYY-MM-DDTHH:MM:SS.ffffff
                    serde_json::Value::String(format!(
                        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}",
                        year, month, day, hour, minute, second, microsecond
                    ))
                }
            }
            mysql::Value::Bytes(_) | mysql::Value::Time(..) => {
                // Delegate to value_to_string; on validation error, capture the
                // error message as a string rather than panicking.
                serde_json::Value::String(
                    Self::value_to_string(value).unwrap_or_else(|e| e.to_string()),
                )
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
    /// A `Result` containing a `Vec<String>` with one entry per column, or an
    /// error if any value conversion fails.
    pub fn row_to_strings(row: mysql::Row) -> anyhow::Result<Vec<String>> {
        let mut values = Vec::with_capacity(row.len());
        for i in 0..row.len() {
            match row.as_ref(i) {
                Some(value) => {
                    let s = Self::value_to_string(value)
                        .with_context(|| format!("Failed to convert column {} to string", i + 1))?;
                    values.push(s);
                }
                None => values.push(String::new()),
            }
        }
        Ok(values)
    }

    /// Converts a single MySQL `Row` into a `BTreeMap` of column names to JSON
    /// values.
    ///
    /// Column names are extracted from the row metadata and values are converted
    /// via [`Self::value_to_json`]. The `BTreeMap` guarantees deterministic
    /// (alphabetical) key ordering in serialised output.
    ///
    /// # Arguments
    ///
    /// * `row` - A `mysql::Row` to convert.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `BTreeMap<String, serde_json::Value>`, or an
    /// error if a column index is unexpectedly out of range.
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
                    map.insert(col_name, Self::value_to_json(value));
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
    fn test_value_to_string_bytes_invalid_utf8() {
        let invalid_bytes = vec![0xFF, 0xFE, 0xFD];
        let result = TypeTransformer::value_to_string(&mysql::Value::Bytes(invalid_bytes));
        assert_eq!(result.expect("hex fallback should succeed"), "0xfffefd");
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
        // Invalid month
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Date(2023, 13, 25, 0, 0, 0, 0));
        assert!(result.is_err());
        let err = result.expect_err("month=13 should fail").to_string();
        assert!(err.contains("Invalid month"));

        // Invalid day
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Date(2023, 12, 32, 0, 0, 0, 0));
        assert!(result.is_err());
        let err = result.expect_err("day=32 should fail").to_string();
        assert!(err.contains("Invalid day"));

        // Invalid hour
        let result =
            TypeTransformer::value_to_string(&mysql::Value::Date(2023, 12, 25, 25, 0, 0, 0));
        assert!(result.is_err());
        let err = result.expect_err("hour=25 should fail").to_string();
        assert!(err.contains("Invalid hour"));
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
    fn test_value_to_string_invalid_time() {
        // Invalid hour
        let result = TypeTransformer::value_to_string(&mysql::Value::Time(false, 0, 25, 30, 45, 0));
        assert!(result.is_err());
        let err = result.expect_err("hour=25 should fail").to_string();
        assert!(err.contains("Invalid hour"));

        // Invalid minute
        let result = TypeTransformer::value_to_string(&mysql::Value::Time(false, 0, 14, 61, 45, 0));
        assert!(result.is_err());
        let err = result.expect_err("minute=61 should fail").to_string();
        assert!(err.contains("Invalid minute"));
    }

    // ---------------------------------------------------------------
    // value_to_json tests
    // ---------------------------------------------------------------

    #[test]
    fn test_value_to_json_null() {
        let result = TypeTransformer::value_to_json(&mysql::Value::NULL);
        assert_eq!(result, serde_json::Value::Null);
    }

    #[test]
    fn test_value_to_json_int() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Int(42));
        assert_eq!(result, serde_json::json!(42));

        let result = TypeTransformer::value_to_json(&mysql::Value::Int(-1));
        assert_eq!(result, serde_json::json!(-1));
    }

    #[test]
    fn test_value_to_json_uint() {
        let result = TypeTransformer::value_to_json(&mysql::Value::UInt(u64::MAX));
        assert_eq!(result, serde_json::json!(u64::MAX));
    }

    #[test]
    fn test_value_to_json_float_normal() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Float(3.5));
        assert!(result.is_number());
        // f32 3.5 promotes to f64 3.5 exactly
        assert_eq!(
            result.as_f64().expect("should be a number"),
            f64::from(3.5_f32)
        );
    }

    #[test]
    fn test_value_to_json_float_nan() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Float(f32::NAN));
        assert_eq!(result, serde_json::Value::String("NaN".to_string()));
    }

    #[test]
    fn test_value_to_json_float_infinity() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Float(f32::INFINITY));
        assert_eq!(result, serde_json::Value::String("Infinity".to_string()));
    }

    #[test]
    fn test_value_to_json_float_neg_infinity() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Float(f32::NEG_INFINITY));
        assert_eq!(result, serde_json::Value::String("-Infinity".to_string()));
    }

    #[test]
    fn test_value_to_json_double_normal() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Double(2.5));
        assert!(result.is_number());
        assert_eq!(result.as_f64().expect("should be a number"), 2.5);
    }

    #[test]
    fn test_value_to_json_double_nan() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Double(f64::NAN));
        assert_eq!(result, serde_json::Value::String("NaN".to_string()));
    }

    #[test]
    fn test_value_to_json_double_infinity() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Double(f64::INFINITY));
        assert_eq!(result, serde_json::Value::String("Infinity".to_string()));
    }

    #[test]
    fn test_value_to_json_double_neg_infinity() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Double(f64::NEG_INFINITY));
        assert_eq!(result, serde_json::Value::String("-Infinity".to_string()));
    }

    #[test]
    fn test_value_to_json_bytes_valid_utf8() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Bytes(b"hello".to_vec()));
        assert_eq!(result, serde_json::Value::String("hello".to_string()));
    }

    #[test]
    fn test_value_to_json_bytes_invalid_utf8() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Bytes(vec![0xFF, 0xFE]));
        // Should be a hex-encoded string
        let s = result.as_str().expect("should be a string");
        assert!(s.starts_with("0x"));
    }

    #[test]
    fn test_value_to_json_date() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Date(2023, 12, 25, 0, 0, 0, 0));
        assert_eq!(result, serde_json::Value::String("2023-12-25".to_string()));
    }

    #[test]
    fn test_value_to_json_datetime() {
        let result =
            TypeTransformer::value_to_json(&mysql::Value::Date(2023, 12, 25, 14, 30, 45, 0));
        assert_eq!(
            result,
            serde_json::Value::String("2023-12-25T14:30:45.000000".to_string())
        );
    }

    #[test]
    fn test_value_to_json_datetime_with_microseconds() {
        let result =
            TypeTransformer::value_to_json(&mysql::Value::Date(2023, 12, 25, 14, 30, 45, 123456));
        assert_eq!(
            result,
            serde_json::Value::String("2023-12-25T14:30:45.123456".to_string())
        );
    }

    #[test]
    fn test_value_to_json_invalid_date_fallback() {
        // Invalid month -- should produce a String with the error message, not panic
        let result = TypeTransformer::value_to_json(&mysql::Value::Date(2023, 13, 25, 0, 0, 0, 0));
        let s = result
            .as_str()
            .expect("should be a string on validation error");
        assert!(
            s.contains("Invalid month"),
            "error string should mention invalid month"
        );
    }

    #[test]
    fn test_value_to_json_time() {
        let result = TypeTransformer::value_to_json(&mysql::Value::Time(false, 0, 14, 30, 45, 0));
        assert_eq!(
            result,
            serde_json::Value::String("14:30:45.000000".to_string())
        );
    }
}
