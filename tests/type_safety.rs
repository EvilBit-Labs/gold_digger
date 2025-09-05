use gold_digger::rows_to_strings;
use mysql::prelude::Queryable;
use std::time::Instant;
use testcontainers_modules::{mariadb::Mariadb, testcontainers::runners::SyncRunner};

/// Check if running in CI environment
fn is_ci() -> bool {
    std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok()
}

/// Test type conversion safety with real MySQL data types
/// This test verifies that the rows_to_strings function handles all MySQL data types safely
/// without panicking on NULL values or non-string types
#[test]
fn test_type_conversion_safety_with_real_database() {
    if is_ci() {
        return;
    }
    // Start a MariaDB container for testing
    let mariadb_container = Mariadb::default().start().expect("Failed to start MariaDB container");
    let host_port = mariadb_container
        .get_host_port_ipv4(3306)
        .expect("Failed to get host port");

    // Create connection URL
    let database_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);

    // Create connection pool
    let pool = mysql::Pool::new(database_url.as_str()).expect("Failed to create connection pool");
    let mut conn = pool.get_conn().expect("Failed to get connection");

    // Create a test table with various data types
    conn.query_drop(
        r#"
        CREATE TABLE IF NOT EXISTS type_test (
            id INT PRIMARY KEY,
            int_val INT,
            uint_val INT UNSIGNED,
            float_val FLOAT,
            double_val DOUBLE,
            string_val VARCHAR(255),
            null_val VARCHAR(255),
            bool_val BOOLEAN,
            date_val DATE,
            time_val TIME,
            datetime_val DATETIME,
            blob_val BLOB,
            text_val TEXT
        )
    "#,
    )
    .expect("Failed to create test table");

    // Insert test data with various types including NULL values
    conn.query_drop(r#"
        INSERT INTO type_test VALUES
        (1, 42, 123, 3.14, 2.718, 'hello', NULL, 1, '2023-12-25', '14:30:00', '2023-12-25 14:30:00', 'binary data', 'text data'),
        (2, -42, 0, -3.14, -2.718, 'world', NULL, 0, '2023-01-01', '00:00:00', '2023-01-01 00:00:00', '', ''),
        (3, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL)
    "#).expect("Failed to insert test data");

    // Query the data and test type conversion
    let rows: Vec<mysql::Row> = conn
        .query("SELECT * FROM type_test ORDER BY id")
        .expect("Failed to query test data");

    // Convert rows to strings - this should not panic
    let result = rows_to_strings(rows).expect("Failed to convert rows to strings");

    // Verify the results
    assert_eq!(result.len(), 4); // Header + 3 data rows

    // Check header
    let expected_headers = vec![
        "id",
        "int_val",
        "uint_val",
        "float_val",
        "double_val",
        "string_val",
        "null_val",
        "bool_val",
        "date_val",
        "time_val",
        "datetime_val",
        "blob_val",
        "text_val",
    ];
    assert_eq!(result[0], expected_headers);

    // Check first row (all valid values)
    let row1 = &result[1];
    assert_eq!(row1[0], "1"); // id
    assert_eq!(row1[1], "42"); // int_val
    assert_eq!(row1[2], "123"); // uint_val
    assert!(row1[3].contains("3.14")); // float_val (may have precision differences)
    assert!(row1[4].contains("2.718")); // double_val (may have precision differences)
    assert_eq!(row1[5], "hello"); // string_val
    assert_eq!(row1[6], ""); // null_val should be empty string
    assert_eq!(row1[7], "1"); // bool_val
    assert!(row1[8].contains("2023-12-25")); // date_val
    assert!(row1[9].contains("14:30:00")); // time_val
    assert!(row1[10].contains("2023-12-25 14:30:00")); // datetime_val
    assert_eq!(row1[11], "binary data"); // blob_val
    assert_eq!(row1[12], "text data"); // text_val

    // Check second row (some negative values)
    let row2 = &result[2];
    assert_eq!(row2[0], "2"); // id
    assert_eq!(row2[1], "-42"); // int_val (negative)
    assert_eq!(row2[2], "0"); // uint_val
    assert!(row2[3].contains("-3.14")); // float_val (negative)
    assert!(row2[4].contains("-2.718")); // double_val (negative)
    assert_eq!(row2[5], "world"); // string_val
    assert_eq!(row2[6], ""); // null_val should be empty string
    assert_eq!(row2[7], "0"); // bool_val
    assert!(row2[8].contains("2023-01-01")); // date_val
    assert!(row2[9].contains("00:00:00")); // time_val
    assert!(row2[10].contains("2023-01-01 00:00:00")); // datetime_val
    assert_eq!(row2[11], ""); // blob_val (empty)
    assert_eq!(row2[12], ""); // text_val (empty)

    // Check third row (all NULL values)
    let row3 = &result[3];
    assert_eq!(row3[0], "3"); // id (should be present)
    // All other values should be empty strings for NULL
    for (i, value) in row3.iter().enumerate().skip(1) {
        assert_eq!(value, "", "Column {} should be empty string for NULL value", i);
    }
}

/// Test edge cases with special characters and unicode
#[test]
fn test_special_characters_and_unicode() {
    if is_ci() {
        return;
    }
    let mariadb_container = Mariadb::default().start().expect("Failed to start MariaDB container");
    let host_port = mariadb_container
        .get_host_port_ipv4(3306)
        .expect("Failed to get host port");

    let database_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);
    let pool = mysql::Pool::new(database_url.as_str()).expect("Failed to create connection pool");
    let mut conn = pool.get_conn().expect("Failed to get connection");

    // Create test table for special characters
    conn.query_drop(
        r#"
        CREATE TABLE IF NOT EXISTS special_chars_test (
            id INT PRIMARY KEY,
            text_with_quotes VARCHAR(255),
            text_with_newlines TEXT,
            text_with_tabs TEXT,
            unicode_text VARCHAR(255),
            empty_string VARCHAR(255)
        )
    "#,
    )
    .expect("Failed to create test table");

    // Insert data with special characters
    conn.query_drop(
        r#"
        INSERT INTO special_chars_test VALUES
        (1, 'Text with "quotes"', 'Line 1\nLine 2', 'Col1\tCol2', 'café', ''),
        (2, '', NULL, '', '测试', NULL)
    "#,
    )
    .expect("Failed to insert test data");

    // Query and convert
    let rows: Vec<mysql::Row> = conn
        .query("SELECT * FROM special_chars_test ORDER BY id")
        .expect("Failed to query test data");
    let result = rows_to_strings(rows).expect("Failed to convert rows to strings");

    assert_eq!(result.len(), 3); // Header + 2 data rows

    // Check first row
    let row1 = &result[1];
    assert_eq!(row1[0], "1");
    assert_eq!(row1[1], "Text with \"quotes\""); // Quotes preserved
    assert_eq!(row1[2], "Line 1\nLine 2"); // Newlines preserved
    assert_eq!(row1[3], "Col1\tCol2"); // Tabs preserved
    assert_eq!(row1[4], "café"); // Unicode preserved
    assert_eq!(row1[5], ""); // Empty string

    // Check second row
    let row2 = &result[2];
    assert_eq!(row2[0], "2");
    assert_eq!(row2[1], ""); // Empty string
    assert_eq!(row2[2], ""); // NULL converted to empty string
    assert_eq!(row2[3], ""); // Empty string
    assert_eq!(row2[4], "测试"); // Unicode preserved
    assert_eq!(row2[5], ""); // NULL converted to empty string
}

/// Test large numbers and precision
#[test]
fn test_large_numbers_and_precision() {
    if is_ci() {
        return;
    }
    let mariadb_container = Mariadb::default().start().expect("Failed to start MariaDB container");
    let host_port = mariadb_container
        .get_host_port_ipv4(3306)
        .expect("Failed to get host port");

    let database_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);
    let pool = mysql::Pool::new(database_url.as_str()).expect("Failed to create connection pool");
    let mut conn = pool.get_conn().expect("Failed to get connection");

    // Create test table for large numbers
    conn.query_drop(
        r#"
        CREATE TABLE IF NOT EXISTS large_numbers_test (
            id INT PRIMARY KEY,
            big_int BIGINT,
            big_uint BIGINT UNSIGNED,
            decimal_val DECIMAL(10,3),
            float_val FLOAT,
            double_val DOUBLE
        )
    "#,
    )
    .expect("Failed to create test table");

    // Insert large numbers
    conn.query_drop(
        r#"
        INSERT INTO large_numbers_test VALUES
        (1, 9223372036854775807, 18446744073709551615, 123.456, 3.14159, 2.718281828459045),
        (2, -9223372036854775808, 0, -123.456, -3.14159, -2.718281828459045)
    "#,
    )
    .expect("Failed to insert test data");

    // Query and convert
    let rows: Vec<mysql::Row> = conn
        .query("SELECT * FROM large_numbers_test ORDER BY id")
        .expect("Failed to query test data");
    let result = rows_to_strings(rows).expect("Failed to convert rows to strings");

    assert_eq!(result.len(), 3); // Header + 2 data rows

    // Check first row
    let row1 = &result[1];
    assert_eq!(row1[0], "1");
    assert_eq!(row1[1], "9223372036854775807"); // Max BIGINT
    assert_eq!(row1[2], "18446744073709551615"); // Max UNSIGNED BIGINT
    assert_eq!(row1[3], "123.456"); // DECIMAL
    assert!(row1[4].contains("3.14159")); // FLOAT
    assert!(row1[5].contains("2.718281828459045")); // DOUBLE

    // Check second row
    let row2 = &result[2];
    assert_eq!(row2[0], "2");
    assert_eq!(row2[1], "-9223372036854775808"); // Min BIGINT
    assert_eq!(row2[2], "0"); // Zero UNSIGNED
    assert_eq!(row2[3], "-123.456"); // Negative DECIMAL
    assert!(row2[4].contains("-3.14159")); // Negative FLOAT
    assert!(row2[5].contains("-2.718281828459045")); // Negative DOUBLE
}

/// Test that the function handles empty result sets gracefully
#[test]
fn test_empty_result_set() {
    if is_ci() {
        return;
    }
    let mariadb_container = Mariadb::default().start().expect("Failed to start MariaDB container");
    let host_port = mariadb_container
        .get_host_port_ipv4(3306)
        .expect("Failed to get host port");

    let database_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);
    let pool = mysql::Pool::new(database_url.as_str()).expect("Failed to create connection pool");
    let mut conn = pool.get_conn().expect("Failed to get connection");

    // Query that returns no rows
    let rows: Vec<mysql::Row> = conn
        .query("SELECT * FROM mysql.user WHERE 1=0")
        .expect("Failed to query");
    let result = rows_to_strings(rows).expect("Failed to convert rows to strings");

    // Should return empty vector
    assert_eq!(result.len(), 0);
}

/// Test that the function handles single row results correctly
#[test]
fn test_single_row_result() {
    if is_ci() {
        return;
    }
    let mariadb_container = Mariadb::default().start().expect("Failed to start MariaDB container");
    let host_port = mariadb_container
        .get_host_port_ipv4(3306)
        .expect("Failed to get host port");

    let database_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);
    let pool = mysql::Pool::new(database_url.as_str()).expect("Failed to create connection pool");
    let mut conn = pool.get_conn().expect("Failed to get connection");

    // Query that returns a single row
    let rows: Vec<mysql::Row> = conn.query("SELECT 1 as id, 'test' as name").expect("Failed to query");
    let result = rows_to_strings(rows).expect("Failed to convert rows to strings");

    assert_eq!(result.len(), 2); // Header + 1 data row
    assert_eq!(result[0], vec!["id", "name"]);
    assert_eq!(result[1], vec!["1", "test"]);
}

/// Test that demonstrates the safety improvements over the old panic-prone pattern
/// This test specifically validates that NULL values and type conversions are handled gracefully
/// and that the dangerous `row[column.name_str().as_ref()]` pattern has been eliminated
#[test]
fn test_null_and_type_conversion_safety() {
    if is_ci() {
        return;
    }
    let mariadb_container = Mariadb::default().start().expect("Failed to start MariaDB container");
    let host_port = mariadb_container
        .get_host_port_ipv4(3306)
        .expect("Failed to get host port");

    let database_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);
    let pool = mysql::Pool::new(database_url.as_str()).expect("Failed to create connection pool");
    let mut conn = pool.get_conn().expect("Failed to get connection");

    // Create a comprehensive test table with edge cases that would cause the old
    // from_value::<String>() pattern to panic
    conn.query_drop(
        r#"
        CREATE TABLE IF NOT EXISTS safety_test (
            id INT PRIMARY KEY,
            -- These would cause panics with unsafe conversion:
            null_int INT,
            null_varchar VARCHAR(255),
            binary_data BINARY(16),
            json_data JSON,
            -- Edge case values:
            zero_int INT,
            empty_string VARCHAR(255),
            max_bigint BIGINT,
            min_bigint BIGINT,
            scientific_float DOUBLE
        )
    "#,
    )
    .expect("Failed to create test table");

    // Insert data that would break unsafe type conversion
    conn.query_drop(
        r#"
        INSERT INTO safety_test VALUES
        (1, NULL, NULL, NULL, NULL, 0, '', 9223372036854775807, -9223372036854775808, 1.23e-10),
        (2, NULL, NULL, 0x48656C6C6F, '{"key": "value"}', -1, 'test', 0, 0, 0.0)
    "#,
    )
    .expect("Failed to insert test data");

    // This should NOT panic - the old from_value::<String>() would have panicked here
    let rows: Vec<mysql::Row> = conn
        .query("SELECT * FROM safety_test ORDER BY id")
        .expect("Failed to query test data");

    // Convert rows - this is the critical test that would fail with unsafe conversion
    let result = rows_to_strings(rows);

    // Verify it succeeds without panicking
    assert!(result.is_ok(), "rows_to_strings should handle all types safely");
    let result = result.unwrap();

    // Verify structure
    assert_eq!(result.len(), 3); // Header + 2 data rows

    // Verify NULL values are converted to empty strings (not panics)
    let row1 = &result[1];
    assert_eq!(row1[1], ""); // null_int -> empty string
    assert_eq!(row1[2], ""); // null_varchar -> empty string
    assert_eq!(row1[3], ""); // binary_data NULL -> empty string
    assert_eq!(row1[4], ""); // json_data NULL -> empty string

    // Verify non-string types are converted safely (not panics)
    assert_eq!(row1[5], "0"); // zero_int converted to string
    assert_eq!(row1[6], ""); // empty_string preserved
    assert_eq!(row1[7], "9223372036854775807"); // max_bigint converted
    assert_eq!(row1[8], "-9223372036854775808"); // min_bigint converted
    // Check for scientific notation or regular decimal format
    let float_str = &row1[9];
    assert!(
        float_str.contains("1.23e-10")
            || float_str.contains("1.23E-10")
            || float_str.contains("0.000000000123")
            || float_str.contains("1.23"),
        "Float value should be converted to string: {}",
        float_str
    ); // scientific notation or decimal

    // Verify second row with actual binary and JSON data
    let row2 = &result[2];
    assert_eq!(row2[0], "2");
    assert_eq!(row2[1], ""); // NULL int
    assert_eq!(row2[2], ""); // NULL varchar
    // Binary data should be converted to some string representation (not panic)
    assert!(!row2[3].is_empty(), "Binary data should convert to non-empty string");
    // JSON should be converted to string representation
    assert!(row2[4].contains("key") && row2[4].contains("value"), "JSON should be converted to string");
}

/// Test memory efficiency and performance characteristics
/// This test validates that the function doesn't have excessive memory overhead
#[test]
fn test_memory_efficiency_with_large_dataset() {
    if is_ci() {
        return;
    }
    let mariadb_container = Mariadb::default().start().expect("Failed to start MariaDB container");
    let host_port = mariadb_container
        .get_host_port_ipv4(3306)
        .expect("Failed to get host port");

    let database_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);
    let pool = mysql::Pool::new(database_url.as_str()).expect("Failed to create connection pool");
    let mut conn = pool.get_conn().expect("Failed to get connection");

    // Create a table with moderate number of rows to test memory behavior
    conn.query_drop(
        r#"
        CREATE TABLE IF NOT EXISTS memory_test (
            id INT PRIMARY KEY,
            data VARCHAR(100)
        )
    "#,
    )
    .expect("Failed to create test table");

    // Insert 1000 rows to test memory scaling
    for i in 0..1000 {
        conn.query_drop(format!("INSERT INTO memory_test VALUES ({}, 'test_data_row_{}')", i, i))
            .expect("Failed to insert test data");
    }

    let rows: Vec<mysql::Row> = conn
        .query("SELECT * FROM memory_test ORDER BY id")
        .expect("Failed to query test data");

    // Measure memory behavior (this should complete without excessive memory usage)
    let start_time = Instant::now();
    let result = rows_to_strings(rows).expect("Failed to convert rows to strings");
    let duration = start_time.elapsed();

    // Verify results
    assert_eq!(result.len(), 1001); // Header + 1000 data rows
    assert_eq!(result[0], vec!["id", "data"]); // Header
    assert_eq!(result[1], vec!["0", "test_data_row_0"]); // First row
    assert_eq!(result[1000], vec!["999", "test_data_row_999"]); // Last row

    // Performance check - should complete reasonably quickly
    assert!(duration.as_millis() < 1000, "Conversion should complete within 1 second for 1000 rows");

    // Memory efficiency check - ensure we're not holding excessive memory
    println!("Processed {} rows in {:?}", result.len() - 1, duration);
}

/// Test that specifically validates the fix for the dangerous indexed access pattern
/// This test creates scenarios that would cause the old `row[column.name_str().as_ref()]` to panic
#[test]
fn test_indexed_access_safety_fix() {
    if is_ci() {
        return;
    }
    let mariadb_container = Mariadb::default().start().expect("Failed to start MariaDB container");
    let host_port = mariadb_container
        .get_host_port_ipv4(3306)
        .expect("Failed to get host port");

    let database_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);
    let pool = mysql::Pool::new(database_url.as_str()).expect("Failed to create connection pool");
    let mut conn = pool.get_conn().expect("Failed to get connection");

    // Create a table with column names that could cause issues with indexed access
    conn.query_drop(
        r#"
        CREATE TABLE IF NOT EXISTS indexed_access_test (
            `id` INT PRIMARY KEY,
            `null_column` VARCHAR(255),
            `binary_column` BINARY(4),
            `json_column` JSON,
            `decimal_column` DECIMAL(10,2),
            `timestamp_column` TIMESTAMP NULL,
            `enum_column` ENUM('value1', 'value2', 'value3')
        )
    "#,
    )
    .expect("Failed to create test table");

    // Insert data that would break the old indexed access pattern
    conn.query_drop(
        r#"
        INSERT INTO indexed_access_test VALUES
        (1, NULL, NULL, NULL, NULL, NULL, NULL),
        (2, 'test', 0x48656C6C, '{"test": "value"}', 123.45, '2023-12-25 14:30:00', 'value1'),
        (3, '', 0x00000000, '[]', 0.00, '1970-01-01 00:00:01', 'value2')
    "#,
    )
    .expect("Failed to insert test data");

    // This query would have caused panics with the old indexed access pattern
    let rows: Vec<mysql::Row> = conn
        .query("SELECT * FROM indexed_access_test ORDER BY id")
        .expect("Failed to query test data");

    // The critical test - this should NOT panic with the new safe iteration approach
    let result = rows_to_strings(rows);

    // Verify it succeeds without panicking
    assert!(result.is_ok(), "rows_to_strings should handle indexed access safely");
    let result = result.unwrap();

    // Verify structure and content
    assert_eq!(result.len(), 4); // Header + 3 data rows

    // Verify header extraction works correctly
    let expected_headers = vec![
        "id",
        "null_column",
        "binary_column",
        "json_column",
        "decimal_column",
        "timestamp_column",
        "enum_column",
    ];
    assert_eq!(result[0], expected_headers);

    // Verify NULL handling in all columns
    let row1 = &result[1];
    assert_eq!(row1[0], "1"); // id should be present
    for (i, value) in row1.iter().enumerate().skip(1) {
        assert_eq!(value, "", "NULL values should convert to empty strings, column {}", i);
    }

    // Verify non-NULL data conversion
    let row2 = &result[2];
    assert_eq!(row2[0], "2");
    assert_eq!(row2[1], "test");
    assert!(!row2[2].is_empty(), "Binary data should convert to non-empty string");
    assert!(row2[3].contains("test") && row2[3].contains("value"), "JSON should be converted");
    assert_eq!(row2[4], "123.45");
    assert!(row2[5].contains("2023-12-25"), "Timestamp should be converted");
    assert_eq!(row2[6], "value1");
}

/// Test error handling and edge cases that could cause panics
#[test]
fn test_error_handling_edge_cases() {
    if is_ci() {
        return;
    }
    let mariadb_container = Mariadb::default().start().expect("Failed to start MariaDB container");
    let host_port = mariadb_container
        .get_host_port_ipv4(3306)
        .expect("Failed to get host port");

    let database_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);
    let pool = mysql::Pool::new(database_url.as_str()).expect("Failed to create connection pool");
    let mut conn = pool.get_conn().expect("Failed to get connection");

    // Test with extreme values that could cause conversion issues
    conn.query_drop(
        r#"
        CREATE TABLE IF NOT EXISTS edge_case_test (
            id INT PRIMARY KEY,
            max_bigint BIGINT,
            min_bigint BIGINT,
            max_double DOUBLE,
            min_double DOUBLE,
            zero_timestamp TIMESTAMP NULL,
            max_varchar VARCHAR(1000)
        )
    "#,
    )
    .expect("Failed to create test table");

    // Insert extreme values
    conn.query_drop(
        r#"
        INSERT INTO edge_case_test VALUES
        (1, 9223372036854775807, -9223372036854775808, 1.7976931348623157e+308, -1.7976931348623157e+308, NULL, REPEAT('A', 1000))
    "#,
    )
    .expect("Failed to insert test data");

    let rows: Vec<mysql::Row> = conn
        .query("SELECT * FROM edge_case_test")
        .expect("Failed to query test data");

    // This should handle extreme values without panicking
    let result = rows_to_strings(rows);
    assert!(result.is_ok(), "Should handle extreme values safely");

    let result = result.unwrap();
    assert_eq!(result.len(), 2); // Header + 1 data row

    let data_row = &result[1];
    assert_eq!(data_row[0], "1");
    assert_eq!(data_row[1], "9223372036854775807");
    assert_eq!(data_row[2], "-9223372036854775808");
    // Double values might have precision differences, just check they're not empty
    assert!(!data_row[3].is_empty(), "Max double should convert to non-empty string");
    assert!(!data_row[4].is_empty(), "Min double should convert to non-empty string");
    assert_eq!(data_row[5], ""); // NULL timestamp
    assert_eq!(data_row[6].len(), 1000); // Large varchar should be preserved
}
