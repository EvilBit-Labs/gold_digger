use gold_digger::{csv, json, tab};
use insta::assert_snapshot;
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

/// Create test data with large integers and boundary values
fn create_test_data_with_large_numbers() -> Vec<Vec<String>> {
    vec![
        vec!["id".to_string(), "bigint".to_string(), "uint".to_string()],
        vec![
            "1".to_string(),
            "9223372036854775807".to_string(),
            "18446744073709551615".to_string(),
        ],
        vec![
            "2".to_string(),
            "-9223372036854775808".to_string(),
            "0".to_string(),
        ],
    ]
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
    let data = create_test_data();
    let mut output = Cursor::new(Vec::new());
    json::write(data.clone(), &mut output).unwrap();
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
    let data = create_test_data();
    let mut output = Cursor::new(Vec::new());
    json::write_with_pretty(data.clone(), &mut output, true).unwrap();
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
    let data = create_empty_data();
    let mut output = Cursor::new(Vec::new());
    json::write(data.clone(), &mut output).unwrap();
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
    let data = create_test_data_with_nulls();
    let mut output = Cursor::new(Vec::new());
    json::write(data.clone(), &mut output).unwrap();
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
    let data = create_test_data_with_large_numbers();
    let mut output = Cursor::new(Vec::new());
    json::write(data.clone(), &mut output).unwrap();
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
