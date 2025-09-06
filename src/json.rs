use std::{
    collections::BTreeMap,
    io::{BufWriter, Write},
};

use crate::FormatWriter;
use anyhow::Result;

/// JSON writer that implements the FormatWriter trait
pub struct JsonWriter<W: Write> {
    writer: BufWriter<W>,
    columns: Vec<String>,
    first_row: bool,
    pretty: bool,
}

impl<W: Write> JsonWriter<W> {
    /// Creates a new JsonWriter with the specified writer and pretty printing option
    pub fn new(writer: W, pretty: bool) -> Self {
        Self {
            writer: BufWriter::with_capacity(64 * 1024, writer), // 64KB buffer for better performance
            columns: Vec::new(),
            first_row: true,
            pretty,
        }
    }
}

impl<W: Write> FormatWriter for JsonWriter<W> {
    fn write_header(&mut self, columns: &[String]) -> Result<()> {
        self.columns = columns.to_vec();
        self.first_row = true;
        write!(self.writer, "{{\"data\":[")?;
        Ok(())
    }

    fn write_row(&mut self, row: &[String]) -> Result<()> {
        if !self.first_row {
            write!(self.writer, ",")?;
        }
        self.first_row = false;

        // Create ordered map for deterministic output
        let mut obj = BTreeMap::new();
        for (col, val) in self.columns.iter().zip(row.iter()) {
            // Convert string values to appropriate JSON types when possible
            let json_value = if val.is_empty() {
                // Preserve empty strings as strings, not null
                serde_json::Value::String(val.clone())
            } else if let Ok(num) = val.parse::<u64>() {
                // Try u64 first to preserve large unsigned values
                serde_json::Value::Number(num.into())
            } else if let Ok(num) = val.parse::<i64>() {
                // Then try i64 for signed integers
                serde_json::Value::Number(num.into())
            } else if let Ok(num) = val.parse::<f64>() {
                // Parse f64 and validate with from_f64
                serde_json::Number::from_f64(num)
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| serde_json::Value::String(val.clone()))
            } else if val.to_lowercase() == "true" || val.to_lowercase() == "false" {
                // Case-insensitive boolean detection
                serde_json::Value::Bool(val.to_lowercase() == "true")
            } else {
                serde_json::Value::String(val.clone())
            };
            obj.insert(col.clone(), json_value);
        }

        if self.pretty {
            serde_json::to_writer_pretty(&mut self.writer, &obj)?;
        } else {
            serde_json::to_writer(&mut self.writer, &obj)?;
        }

        Ok(())
    }

    fn finalize(mut self) -> Result<()> {
        write!(self.writer, "]}}")?;
        self.writer.flush()?;
        Ok(())
    }
}

/// Writes rows to a JSON output using the provided writer.
///
/// # Arguments
///
/// * `rows` - An iterator over records, where each record is an iterator over fields.
/// * `output` - A writer to output the JSON data.
///
/// # Returns
///
/// A Result indicating success or failure.
pub fn write<R, F, W>(rows: R, output: W) -> anyhow::Result<()>
where
    R: IntoIterator<Item = F>,
    F: IntoIterator<Item = String>,
    W: Write,
{
    write_with_pretty(rows, output, false)
}

/// Writes rows to a JSON output using the provided writer with pretty printing option.
///
/// # Arguments
///
/// * `rows` - An iterator over records, where each record is an iterator over fields.
/// * `output` - A writer to output the JSON data.
/// * `pretty` - Whether to format the JSON with pretty printing.
///
/// # Returns
///
/// A Result indicating success or failure.
pub fn write_with_pretty<R, F, W>(rows: R, output: W, pretty: bool) -> anyhow::Result<()>
where
    R: IntoIterator<Item = F>,
    F: IntoIterator<Item = String>,
    W: Write,
{
    let mut rows_iter = rows.into_iter();

    // Get the first row as headers, or use empty if no rows
    let headers = if let Some(first_row) = rows_iter.next() {
        first_row.into_iter().collect::<Vec<String>>()
    } else {
        let mut writer = JsonWriter::new(output, pretty);
        writer.write_header(&[])?;
        writer.finalize()?;
        return Ok(());
    };

    let mut writer = JsonWriter::new(output, pretty);
    writer.write_header(&headers)?;

    for row in rows_iter {
        let row_vec: Vec<String> = row.into_iter().collect();
        writer.write_row(&row_vec)?;
    }

    writer.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_json_writer_format() {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = JsonWriter::new(&mut cursor, false);

        // Test header
        writer.write_header(&["col1".to_string(), "col2".to_string()]).unwrap();

        // Test first row
        writer.write_row(&["val1".to_string(), "val2".to_string()]).unwrap();

        // Test second row
        writer.write_row(&["val3".to_string(), "val4".to_string()]).unwrap();

        // Test finalize
        writer.finalize().unwrap();

        let output = String::from_utf8(cursor.into_inner()).unwrap();
        let expected = r#"{"data":[{"col1":"val1","col2":"val2"},{"col1":"val3","col2":"val4"}]}"#;

        assert_eq!(output, expected);
    }

    #[test]
    fn test_json_type_inference() {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = JsonWriter::new(&mut cursor, false);

        writer
            .write_header(&[
                "empty".to_string(),
                "u64_max".to_string(),
                "scientific".to_string(),
                "leading_zeros".to_string(),
                "mixed_case_bool".to_string(),
            ])
            .unwrap();

        // Test various type inference scenarios
        writer
            .write_row(&[
                "".to_string(),        // Empty string (should be String, not Null)
                u64::MAX.to_string(),  // u64::MAX (should be Number)
                "1.23e-4".to_string(), // Scientific notation (should be Number)
                "00123".to_string(),   // Leading zeros (should be Number)
                "TRUE".to_string(),    // Mixed case boolean (should be Bool)
            ])
            .unwrap();

        writer.finalize().unwrap();

        let output = String::from_utf8(cursor.into_inner()).unwrap();

        // Parse the JSON to verify type inference
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        let data = json["data"][0].as_object().unwrap();

        // Test empty string is preserved as string, not null
        assert!(data["empty"].is_string());
        assert_eq!(data["empty"].as_str().unwrap(), "");

        // Test u64::MAX is preserved as number
        assert!(data["u64_max"].is_number());
        assert_eq!(data["u64_max"].as_u64().unwrap(), u64::MAX);

        // Test scientific notation is preserved as number
        assert!(data["scientific"].is_number());
        assert_eq!(data["scientific"].as_f64().unwrap(), 1.23e-4);

        // Test leading zeros are preserved as number
        assert!(data["leading_zeros"].is_number());
        assert_eq!(data["leading_zeros"].as_u64().unwrap(), 123);

        // Test mixed case boolean is detected
        assert!(data["mixed_case_bool"].is_boolean());
        assert!(data["mixed_case_bool"].as_bool().unwrap());
    }

    #[test]
    fn test_json_type_inference_edge_cases() {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = JsonWriter::new(&mut cursor, false);

        writer
            .write_header(&[
                "negative".to_string(),
                "float".to_string(),
                "bool_false".to_string(),
                "mixed_case_false".to_string(),
                "invalid_float".to_string(),
            ])
            .unwrap();

        // Test edge cases
        writer
            .write_row(&[
                "-123".to_string(),     // Negative integer (should be Number)
                "1.5".to_string(),      // Float (should be Number)
                "false".to_string(),    // Lowercase boolean (should be Bool)
                "FALSE".to_string(),    // Uppercase boolean (should be Bool)
                "1.23e999".to_string(), // Invalid float (should be String)
            ])
            .unwrap();

        writer.finalize().unwrap();

        let output = String::from_utf8(cursor.into_inner()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        let data = json["data"][0].as_object().unwrap();

        // Test negative integer
        assert!(data["negative"].is_number());
        assert_eq!(data["negative"].as_i64().unwrap(), -123);

        // Test float
        assert!(data["float"].is_number());
        assert_eq!(data["float"].as_f64().unwrap(), 1.5);

        // Test lowercase boolean
        assert!(data["bool_false"].is_boolean());
        assert!(!data["bool_false"].as_bool().unwrap());

        // Test uppercase boolean
        assert!(data["mixed_case_false"].is_boolean());
        assert!(!data["mixed_case_false"].as_bool().unwrap());

        // Test invalid float falls back to string
        assert!(data["invalid_float"].is_string());
        assert_eq!(data["invalid_float"].as_str().unwrap(), "1.23e999");
    }
}
