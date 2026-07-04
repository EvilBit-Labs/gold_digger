//! JSON writer producing `{"data": [ ... ]}` output.
//!
//! The writer consumes a pre-converted `Vec<BTreeMap<String, serde_json::Value>>`
//! produced by [`TypeTransformer::row_to_json`](crate::TypeTransformer::row_to_json).
//! Splitting "convert rows" from "write bytes" lets callers surface conversion
//! errors before creating / truncating the output file. `BTreeMap` guarantees
//! deterministic key ordering across runs — required for snapshot tests and
//! diffable automation. Pretty-printing is controlled by `--pretty`.

use std::{
    collections::BTreeMap,
    io::{BufWriter, Write},
};

use crate::OUTPUT_BUFFER_CAPACITY;

/// Writes pre-converted JSON maps to the provided output.
///
/// This function accepts maps that have already been converted from MySQL rows
/// via [`TypeTransformer::row_to_json`](crate::TypeTransformer::row_to_json).
/// That separation allows callers to validate every row before
/// creating / truncating the destination file, so a type-conversion failure
/// on row N never leaves behind a half-written output.
///
/// # Arguments
///
/// * `maps` - Pre-converted JSON object maps (one per row, ordered as the query returned them).
/// * `output` - A writer to output the JSON data.
/// * `pretty` - Whether to format the JSON with pretty printing.
///
/// # Returns
///
/// A Result indicating success or failure.
pub fn write<W: Write>(
    maps: Vec<BTreeMap<String, serde_json::Value>>,
    output: W,
    pretty: bool,
) -> anyhow::Result<()> {
    let mut writer = BufWriter::with_capacity(OUTPUT_BUFFER_CAPACITY, output);

    if maps.is_empty() {
        write!(writer, "{{\"data\":[]}}")?;
        writer.flush()?;
        return Ok(());
    }

    write!(writer, "{{\"data\":[")?;

    for (i, map) in maps.iter().enumerate() {
        if i > 0 {
            write!(writer, ",")?;
        }
        if pretty {
            serde_json::to_writer_pretty(&mut writer, map)?;
        } else {
            serde_json::to_writer(&mut writer, map)?;
        }
    }

    write!(writer, "]}}")?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_write_empty() {
        let mut cursor = Cursor::new(Vec::new());
        write(vec![], &mut cursor, false).unwrap();
        let output = String::from_utf8(cursor.into_inner()).unwrap();
        assert_eq!(output, r#"{"data":[]}"#);
    }

    #[test]
    fn test_write_single_row() {
        let mut map = BTreeMap::new();
        map.insert("id".to_string(), serde_json::json!(1));
        map.insert("name".to_string(), serde_json::json!("Alice"));

        let mut cursor = Cursor::new(Vec::new());
        write(vec![map], &mut cursor, false).unwrap();
        let output = String::from_utf8(cursor.into_inner()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(json["data"][0]["id"], 1);
        assert_eq!(json["data"][0]["name"], "Alice");
    }

    #[test]
    fn test_write_multiple_rows() {
        let mut map1 = BTreeMap::new();
        map1.insert("id".to_string(), serde_json::json!(1));
        let mut map2 = BTreeMap::new();
        map2.insert("id".to_string(), serde_json::json!(2));

        let mut cursor = Cursor::new(Vec::new());
        write(vec![map1, map2], &mut cursor, false).unwrap();
        let output = String::from_utf8(cursor.into_inner()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(json["data"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_write_pretty() {
        let mut map = BTreeMap::new();
        map.insert("id".to_string(), serde_json::json!(1));

        let mut cursor = Cursor::new(Vec::new());
        write(vec![map], &mut cursor, true).unwrap();
        let output = String::from_utf8(cursor.into_inner()).unwrap();
        // Pretty output contains newlines and indentation
        assert!(output.contains('\n'));
    }

    #[test]
    fn test_write_preserves_types() {
        let mut map = BTreeMap::new();
        map.insert("int_val".to_string(), serde_json::json!(42));
        map.insert("null_val".to_string(), serde_json::Value::Null);
        map.insert("str_val".to_string(), serde_json::json!("hello"));
        map.insert("float_val".to_string(), serde_json::json!(3.25));

        let mut cursor = Cursor::new(Vec::new());
        write(vec![map], &mut cursor, false).unwrap();
        let output = String::from_utf8(cursor.into_inner()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        let data = &json["data"][0];

        assert_eq!(data["int_val"], 42);
        assert!(data["null_val"].is_null());
        assert_eq!(data["str_val"], "hello");
        assert_eq!(data["float_val"], 3.25);
    }

    /// Regression: leading-zero strings must stay strings once the
    /// typed-JSON path is the only path. The deleted `JsonWriter` path used
    /// to parse `"00123"` as the integer `123`, corrupting ZIP codes, phone
    /// numbers, and similar opaque identifiers. The typed path carries the
    /// value through as a string via `TypeTransformer::row_to_json`.
    #[test]
    fn test_leading_zeros_preserved_as_strings() {
        let mut map = BTreeMap::new();
        map.insert(
            "zip".to_string(),
            serde_json::Value::String("00123".to_string()),
        );

        let mut cursor = Cursor::new(Vec::new());
        write(vec![map], &mut cursor, false).unwrap();
        let output = String::from_utf8(cursor.into_inner()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(
            json["data"][0]["zip"].is_string(),
            "leading-zero identifier must remain a JSON string"
        );
        assert_eq!(json["data"][0]["zip"].as_str().unwrap(), "00123");
    }

    /// Regression: mixed-case `"TRUE"` / `"FALSE"` must stay strings. The
    /// deleted `JsonWriter` path coerced them into JSON booleans, which
    /// silently altered feature flags, audit log strings, and country
    /// codes. The typed path preserves the original string value.
    #[test]
    fn test_mixed_case_bool_strings_preserved() {
        let mut map = BTreeMap::new();
        map.insert(
            "flag".to_string(),
            serde_json::Value::String("TRUE".to_string()),
        );

        let mut cursor = Cursor::new(Vec::new());
        write(vec![map], &mut cursor, false).unwrap();
        let output = String::from_utf8(cursor.into_inner()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(
            json["data"][0]["flag"].is_string(),
            "string 'TRUE' must not be coerced into a JSON bool"
        );
        assert_eq!(json["data"][0]["flag"].as_str().unwrap(), "TRUE");
    }
}
