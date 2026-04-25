use gold_digger::TypeTransformer;
use insta::assert_snapshot;
use mysql::Value;

// ---------------------------------------------------------------
// value_to_string snapshot tests
// ---------------------------------------------------------------

#[test]
fn test_value_to_string_null() {
    let result = TypeTransformer::value_to_string(&Value::NULL).expect("NULL should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_null", result);
    });
}

#[test]
fn test_value_to_string_int() {
    let result = TypeTransformer::value_to_string(&Value::Int(42)).expect("Int should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_int", result);
    });
}

#[test]
fn test_value_to_string_uint() {
    let result =
        TypeTransformer::value_to_string(&Value::UInt(u64::MAX)).expect("UInt should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_uint", result);
    });
}

#[test]
fn test_value_to_string_float() {
    let result =
        TypeTransformer::value_to_string(&Value::Float(3.15)).expect("Float should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_float", result);
    });
}

#[test]
fn test_value_to_string_double() {
    let result = TypeTransformer::value_to_string(&Value::Double(2.719_281_828))
        .expect("Double should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_double", result);
    });
}

#[test]
fn test_value_to_string_nan() {
    let result =
        TypeTransformer::value_to_string(&Value::Float(f32::NAN)).expect("NaN should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_nan", result);
    });
}

#[test]
fn test_value_to_string_infinity() {
    let result = TypeTransformer::value_to_string(&Value::Float(f32::INFINITY))
        .expect("Infinity should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_infinity", result);
    });
}

#[test]
fn test_value_to_string_neg_infinity() {
    let result = TypeTransformer::value_to_string(&Value::Double(f64::NEG_INFINITY))
        .expect("-Infinity should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_neg_infinity", result);
    });
}

#[test]
fn test_value_to_string_utf8_bytes() {
    let result = TypeTransformer::value_to_string(&Value::Bytes(b"hello world".to_vec()))
        .expect("valid UTF-8 should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_utf8_bytes", result);
    });
}

#[test]
fn test_value_to_string_invalid_utf8() {
    let result = TypeTransformer::value_to_string(&Value::Bytes(vec![0xFF, 0xFE, 0xFD]))
        .expect("hex fallback should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_invalid_utf8", result);
    });
}

#[test]
fn test_value_to_string_large_binary() {
    let result = TypeTransformer::value_to_string(&Value::Bytes(vec![0xAB; 2000]))
        .expect("large binary should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_large_binary", result);
    });
}

#[test]
fn test_value_to_string_date() {
    let result = TypeTransformer::value_to_string(&Value::Date(2023, 12, 25, 0, 0, 0, 0))
        .expect("date should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_date", result);
    });
}

#[test]
fn test_value_to_string_datetime() {
    let result = TypeTransformer::value_to_string(&Value::Date(2023, 12, 25, 14, 30, 45, 123456))
        .expect("datetime should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_datetime", result);
    });
}

#[test]
fn test_value_to_string_time() {
    let result = TypeTransformer::value_to_string(&Value::Time(false, 0, 14, 30, 45, 0))
        .expect("time should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_time", result);
    });
}

#[test]
fn test_value_to_string_time_with_days() {
    let result = TypeTransformer::value_to_string(&Value::Time(true, 1, 2, 30, 45, 0))
        .expect("time with days should succeed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_string_time_with_days", result);
    });
}

// ---------------------------------------------------------------
// value_to_json snapshot tests
// ---------------------------------------------------------------

#[test]
fn test_value_to_json_null() {
    let result = TypeTransformer::value_to_json(&Value::NULL).expect("NULL should succeed");
    let serialized = serde_json::to_string_pretty(&result).expect("JSON serialization failed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_json_null", serialized);
    });
}

#[test]
fn test_value_to_json_int() {
    let result = TypeTransformer::value_to_json(&Value::Int(42)).expect("Int should succeed");
    let serialized = serde_json::to_string_pretty(&result).expect("JSON serialization failed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_json_int", serialized);
    });
}

#[test]
fn test_value_to_json_uint_max() {
    let result =
        TypeTransformer::value_to_json(&Value::UInt(u64::MAX)).expect("UInt should succeed");
    let serialized = serde_json::to_string_pretty(&result).expect("JSON serialization failed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_json_uint_max", serialized);
    });
}

#[test]
fn test_value_to_json_float_nan() {
    let result =
        TypeTransformer::value_to_json(&Value::Float(f32::NAN)).expect("Float NaN should succeed");
    let serialized = serde_json::to_string_pretty(&result).expect("JSON serialization failed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_json_float_nan", serialized);
    });
}

#[test]
fn test_value_to_json_float_infinity() {
    let result = TypeTransformer::value_to_json(&Value::Float(f32::INFINITY))
        .expect("Float Infinity should succeed");
    let serialized = serde_json::to_string_pretty(&result).expect("JSON serialization failed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_json_float_infinity", serialized);
    });
}

#[test]
fn test_value_to_json_double() {
    let result =
        TypeTransformer::value_to_json(&Value::Double(2.5)).expect("Double should succeed");
    let serialized = serde_json::to_string_pretty(&result).expect("JSON serialization failed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_json_double", serialized);
    });
}

#[test]
fn test_value_to_json_bytes_valid_utf8() {
    let result = TypeTransformer::value_to_json(&Value::Bytes(b"hello".to_vec()))
        .expect("Bytes should succeed");
    let serialized = serde_json::to_string_pretty(&result).expect("JSON serialization failed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_json_bytes_valid_utf8", serialized);
    });
}

#[test]
fn test_value_to_json_large_binary() {
    let result = TypeTransformer::value_to_json(&Value::Bytes(vec![0xAB; 2000]))
        .expect("large binary should succeed");
    let serialized = serde_json::to_string_pretty(&result).expect("JSON serialization failed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_json_large_binary", serialized);
    });
}

#[test]
fn test_value_to_json_date() {
    let result = TypeTransformer::value_to_json(&Value::Date(2023, 12, 25, 0, 0, 0, 0))
        .expect("date should succeed");
    let serialized = serde_json::to_string_pretty(&result).expect("JSON serialization failed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_json_date", serialized);
    });
}

#[test]
fn test_value_to_json_datetime_with_microseconds() {
    let result = TypeTransformer::value_to_json(&Value::Date(2023, 12, 25, 14, 30, 45, 123456))
        .expect("datetime should succeed");
    let serialized = serde_json::to_string_pretty(&result).expect("JSON serialization failed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_json_datetime_with_microseconds", serialized);
    });
}

#[test]
fn test_value_to_json_invalid_date_returns_error() {
    let result = TypeTransformer::value_to_json(&Value::Date(2023, 13, 25, 0, 0, 0, 0));
    let error_msg = result
        .expect_err("invalid date should return Err")
        .to_string();
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_json_invalid_date_error", error_msg);
    });
}

#[test]
fn test_value_to_json_time() {
    let result = TypeTransformer::value_to_json(&Value::Time(false, 0, 14, 30, 45, 0))
        .expect("time should succeed");
    let serialized = serde_json::to_string_pretty(&result).expect("JSON serialization failed");
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_snapshot!("value_to_json_time", serialized);
    });
}
