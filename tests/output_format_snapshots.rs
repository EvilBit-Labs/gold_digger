use gold_digger::{csv, json, tab};
use insta::assert_snapshot;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::Cursor;

/// Create test data with standard values
fn create_test_data() -> Vec<Vec<String>> {
    vec![
        vec!["id".to_string(), "name".to_string(), "value".to_string()],
        vec!["1".to_string(), "Alice".to_string(), "100".to_string()],
        vec!["2".to_string(), "Bob".to_string(), "200".to_string()],
        vec!["3".to_string(), "Charlie".to_string(), "300".to_string()],
    ]
}

/// Build typed JSON maps for the JSON writer.
///
/// After todo #015 deleted the `JsonWriter`/string-inference path, JSON
/// output consumes pre-converted `BTreeMap<String, Value>` maps. These
/// helpers model what `TypeTransformer::row_to_json` would produce for the
/// corresponding scenario -- strings stay strings, numbers stay numbers,
/// NULLs are `Value::Null`.
fn typed_standard_rows() -> Vec<BTreeMap<String, Value>> {
    vec![
        build_map(&[
            ("id", json!(1)),
            ("name", json!("Alice")),
            ("value", json!(100)),
        ]),
        build_map(&[
            ("id", json!(2)),
            ("name", json!("Bob")),
            ("value", json!(200)),
        ]),
        build_map(&[
            ("id", json!(3)),
            ("name", json!("Charlie")),
            ("value", json!(300)),
        ]),
    ]
}

fn typed_null_rows() -> Vec<BTreeMap<String, Value>> {
    vec![
        build_map(&[
            ("id", json!(1)),
            ("name", json!("Alice")),
            ("value", json!(100)),
        ]),
        build_map(&[
            ("id", json!(2)),
            ("name", Value::Null),
            ("value", json!(200)),
        ]),
        build_map(&[
            ("id", json!(3)),
            ("name", json!("Charlie")),
            ("value", Value::Null),
        ]),
    ]
}

fn typed_large_number_rows() -> Vec<BTreeMap<String, Value>> {
    vec![
        build_map(&[
            ("id", json!(1)),
            ("bigint", json!(i64::MAX)),
            ("uint", json!(u64::MAX)),
        ]),
        build_map(&[
            ("id", json!(2)),
            ("bigint", json!(i64::MIN)),
            ("uint", json!(0u64)),
        ]),
    ]
}

fn build_map(entries: &[(&str, Value)]) -> BTreeMap<String, Value> {
    entries
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

/// Create test data with null values (empty strings)
fn create_test_data_with_nulls() -> Vec<Vec<String>> {
    vec![
        vec!["id".to_string(), "name".to_string(), "value".to_string()],
        vec!["1".to_string(), "Alice".to_string(), "100".to_string()],
        vec!["2".to_string(), String::new(), "200".to_string()],
        vec!["3".to_string(), "Charlie".to_string(), String::new()],
    ]
}

/// Create test data with special characters
fn create_test_data_with_special_chars() -> Vec<Vec<String>> {
    vec![
        vec!["id".to_string(), "text".to_string(), "data".to_string()],
        vec!["1".to_string(), "normal".to_string(), "value".to_string()],
        vec![
            "2".to_string(),
            "text,with,commas".to_string(),
            "value\"with\"quotes".to_string(),
        ],
        vec![
            "3".to_string(),
            "text\nwith\nnewlines".to_string(),
            "text\twith\ttabs".to_string(),
        ],
    ]
}

/// Create empty result set
fn create_empty_data() -> Vec<Vec<String>> {
    vec![vec!["id".to_string(), "name".to_string()]]
}

#[test]
fn test_csv_standard_data() {
    let data = create_test_data();
    let mut output = Cursor::new(Vec::new());
    csv::write(data.clone(), &mut output).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    insta::with_settings!({
        snapshot_path => "tests/snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("csv_standard_data", result);
    });
}

#[test]
fn test_csv_escaping_quotes_and_commas() {
    let data = create_test_data_with_special_chars();
    let mut output = Cursor::new(Vec::new());
    csv::write(data.clone(), &mut output).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    insta::with_settings!({
        snapshot_path => "tests/snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("csv_escaping_quotes_and_commas", result);
    });
}

#[test]
fn test_csv_newlines() {
    let data = create_test_data_with_special_chars();
    let mut output = Cursor::new(Vec::new());
    csv::write(data.clone(), &mut output).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    insta::with_settings!({
        snapshot_path => "tests/snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("csv_newlines", result);
    });
}

#[test]
fn test_csv_null_values() {
    let data = create_test_data_with_nulls();
    let mut output = Cursor::new(Vec::new());
    csv::write(data.clone(), &mut output).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    insta::with_settings!({
        snapshot_path => "tests/snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("csv_null_values", result);
    });
}

#[test]
fn test_csv_empty_result_set() {
    let data = create_empty_data();
    let mut output = Cursor::new(Vec::new());
    csv::write(data.clone(), &mut output).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    insta::with_settings!({
        snapshot_path => "tests/snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("csv_empty_result_set", result);
    });
}

#[test]
fn test_json_standard_data() {
    let data = typed_standard_rows();
    let mut output = Cursor::new(Vec::new());
    json::write(data, &mut output, false).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    insta::with_settings!({
        snapshot_path => "tests/snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("json_standard_data", result);
    });
}

#[test]
fn test_json_pretty_printed() {
    let data = typed_standard_rows();
    let mut output = Cursor::new(Vec::new());
    json::write(data, &mut output, true).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    insta::with_settings!({
        snapshot_path => "tests/snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("json_pretty_printed", result);
    });
}

#[test]
fn test_json_empty_result_set() {
    let mut output = Cursor::new(Vec::new());
    json::write(Vec::new(), &mut output, false).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    insta::with_settings!({
        snapshot_path => "tests/snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("json_empty_result_set", result);
    });
}

#[test]
fn test_json_null_handling() {
    let data = typed_null_rows();
    let mut output = Cursor::new(Vec::new());
    json::write(data, &mut output, false).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    insta::with_settings!({
        snapshot_path => "tests/snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("json_null_handling", result);
    });
}

#[test]
fn test_json_large_integers() {
    let data = typed_large_number_rows();
    let mut output = Cursor::new(Vec::new());
    json::write(data, &mut output, false).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    insta::with_settings!({
        snapshot_path => "tests/snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("json_large_integers", result);
    });
}

#[test]
fn test_tsv_standard_data() {
    let data = create_test_data();
    let mut output = Cursor::new(Vec::new());
    tab::write(data.clone(), &mut output).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    insta::with_settings!({
        snapshot_path => "tests/snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("tsv_standard_data", result);
    });
}

#[test]
fn test_tsv_special_characters() {
    let data = create_test_data_with_special_chars();
    let mut output = Cursor::new(Vec::new());
    tab::write(data.clone(), &mut output).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    insta::with_settings!({
        snapshot_path => "tests/snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("tsv_special_characters", result);
    });
}

#[test]
fn test_tsv_null_conversion() {
    let data = create_test_data_with_nulls();
    let mut output = Cursor::new(Vec::new());
    tab::write(data.clone(), &mut output).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    insta::with_settings!({
        snapshot_path => "tests/snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("tsv_null_conversion", result);
    });
}
