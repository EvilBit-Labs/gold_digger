//! Test fixtures and data utilities
//!
//! This module provides utilities for managing test fixtures, sample data,
//! and test database schemas.

#![allow(dead_code)]

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Test fixtures manager
pub struct TestFixtures;

impl TestFixtures {
    /// Get the path to the fixtures directory
    pub fn fixtures_dir() -> PathBuf {
        PathBuf::from("tests/fixtures")
    }

    /// Get the path to a specific fixture file
    pub fn fixture_path(name: &str) -> PathBuf {
        Self::fixtures_dir().join(name)
    }

    /// Load fixture content as string
    pub fn load_fixture(name: &str) -> Result<String> {
        let path = Self::fixture_path(name);
        std::fs::read_to_string(&path).with_context(|| format!("Failed to load fixture: {}", path.display()))
    }

    /// Check if a fixture exists
    pub fn fixture_exists(name: &str) -> bool {
        Self::fixture_path(name).exists()
    }

    /// Get all available fixtures
    pub fn list_fixtures() -> Result<Vec<String>> {
        let fixtures_dir = Self::fixtures_dir();
        if !fixtures_dir.exists() {
            return Ok(Vec::new());
        }

        let mut fixtures = Vec::new();
        for entry in std::fs::read_dir(fixtures_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && let Some(name) = entry.file_name().to_str()
            {
                fixtures.push(name.to_string());
            }
        }

        fixtures.sort();
        Ok(fixtures)
    }
}

/// Sample SQL queries for testing
pub struct SampleQueries;

impl SampleQueries {
    /// Basic SELECT query
    pub fn basic_select() -> &'static str {
        "SELECT 1 as id, 'test' as item_name, NOW() as created_at"
    }

    /// Query with multiple data types
    pub fn data_types_query() -> &'static str {
        r"SELECT
            1 as int_col,
            'test string' as varchar_col,
            123.45 as decimal_col,
            TRUE as bool_col,
            NOW() as datetime_col,
            NULL as null_col"
    }

    /// Query that returns empty result set
    pub fn empty_result_query() -> &'static str {
        "SELECT 1 as id WHERE 1 = 0"
    }

    /// Query with special characters
    pub fn special_chars_query() -> &'static str {
        r#"SELECT
            'Hello, World!' as greeting,
            'Line 1\nLine 2' as multiline,
            'Tab\tSeparated' as tabs,
            'Quote"Test' as quotes,
            'Comma,Test' as commas"#
    }

    /// Query with Unicode characters
    pub fn unicode_query() -> &'static str {
        r"SELECT
            'Hello 世界' as chinese,
            'Café' as french,
            '🚀 Rocket' as emoji,
            'Ñoño' as spanish"
    }

    /// Invalid SQL query for error testing
    pub fn invalid_sql() -> &'static str {
        "SELECT * FROM non_existent_table WHERE invalid syntax"
    }

    /// Query that would cause a timeout (for testing)
    pub fn slow_query() -> &'static str {
        "SELECT SLEEP(10) as slow_result"
    }
}

/// Test database schema utilities
pub struct TestSchema;

impl TestSchema {
    /// Get basic test table creation SQL
    pub fn basic_test_table() -> &'static str {
        r"CREATE TABLE IF NOT EXISTS test_basic (
            id INT PRIMARY KEY AUTO_INCREMENT,
            item_name VARCHAR(255),
            table_value INT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )"
    }

    /// Get comprehensive data types test table
    pub fn data_types_table() -> &'static str {
        r"CREATE TABLE IF NOT EXISTS test_data_types (
            id INT PRIMARY KEY AUTO_INCREMENT,
            varchar_col VARCHAR(255),
            text_col TEXT,
            int_col INT,
            bigint_col BIGINT,
            decimal_col DECIMAL(10, 2),
            float_col FLOAT,
            double_col DOUBLE,
            date_col DATE,
            datetime_col DATETIME,
            timestamp_col TIMESTAMP,
            time_col TIME,
            year_col YEAR,
            binary_col BINARY(16),
            varbinary_col VARBINARY(255),
            blob_col BLOB,
            json_col JSON,
            enum_col ENUM('small', 'medium', 'large'),
            set_col SET('red', 'green', 'blue'),
            bool_col BOOLEAN
        )"
    }

    /// Get edge cases test table
    pub fn edge_cases_table() -> &'static str {
        r"CREATE TABLE IF NOT EXISTS test_edge_cases (
            id INT PRIMARY KEY,
            null_varchar VARCHAR(255),
            empty_string VARCHAR(255),
            unicode_text TEXT CHARACTER SET utf8mb4,
            large_text LONGTEXT,
            special_chars VARCHAR(255),
            numeric_string VARCHAR(50),
            zero_values INT,
            negative_values INT
        )"
    }

    /// Get performance test table (for large datasets)
    pub fn performance_table() -> &'static str {
        r"CREATE TABLE IF NOT EXISTS test_performance (
            id INT PRIMARY KEY AUTO_INCREMENT,
            data_column VARCHAR(1000),
            numeric_column DECIMAL(15, 5),
            timestamp_column TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )"
    }
}

/// Test data seeding utilities
pub struct TestData;

impl TestData {
    /// Get basic test data inserts
    pub fn basic_test_data() -> Vec<&'static str> {
        vec![
            "INSERT INTO test_basic (item_name, table_value) VALUES ('test1', 100) ON DUPLICATE KEY UPDATE table_value = VALUES (table_value)",
            "INSERT INTO test_basic (item_name, table_value) VALUES ('test2', 200) ON DUPLICATE KEY UPDATE table_value = VALUES (table_value)",
            "INSERT INTO test_basic (item_name, table_value) VALUES ('test3', 300) ON DUPLICATE KEY UPDATE table_value = VALUES (table_value)",
        ]
    }

    /// Get comprehensive data types test data
    pub fn data_types_test_data() -> Vec<&'static str> {
        vec![
            r#"INSERT INTO test_data_types (
                varchar_col, text_col, int_col, bigint_col, decimal_col, float_col, double_col,
                date_col, datetime_col, timestamp_col, time_col, year_col,
                binary_col, varbinary_col, blob_col, json_col,
                enum_col, set_col, bool_col
            ) VALUES (
                'Sample text', 'Longer text content', 42, 9223372036854775807, 123.45, 3.14159, 2.718281828,
                '2024-01-01', '2024-01-01 12:00:00', '2024-01-01 12:00:00', '12:00:00', 2024,
                UNHEX('48656C6C6F20576F726C64210000000000'), UNHEX('48656C6C6F'), UNHEX('48656C6C6F20576F726C6421'),
                '{"key": "value", "number": 42}',
                'medium', 'red,blue', TRUE
            ) ON DUPLICATE KEY UPDATE varchar_col = VALUES (varchar_col)"#,
        ]
    }

    /// Get edge cases test data
    pub fn edge_cases_test_data() -> Vec<&'static str> {
        vec![
            "INSERT INTO test_edge_cases (id, null_varchar, empty_string, unicode_text, large_text, special_chars, numeric_string, zero_values, negative_values) VALUES (1, NULL, '', 'Hello 世界 🚀', REPEAT('Large text content ', 1000), 'Special: \",\\n\\t', '12345', 0, -42) ON DUPLICATE KEY UPDATE unicode_text = VALUES (unicode_text)",
            "INSERT INTO test_edge_cases (id, null_varchar, empty_string, unicode_text, large_text, special_chars, numeric_string, zero_values, negative_values) VALUES (2, NULL, '', 'Café Ñoño', 'Normal text', 'Quotes: \"Hello\"', '67890', 0, -100) ON DUPLICATE KEY UPDATE unicode_text = VALUES (unicode_text)",
        ]
    }

    /// Generate performance test data (large dataset)
    pub fn generate_performance_data(count: usize) -> Vec<String> {
        let mut statements = Vec::new();

        for i in 1..=count {
            let statement = format!(
                "INSERT INTO test_performance (data_column, numeric_column) VALUES ('Performance test data row {}', {}.{:02}) ON DUPLICATE KEY UPDATE data_column = VALUES (data_column)",
                i,
                i,
                i % 100
            );
            statements.push(statement);
        }

        statements
    }
}

/// Fixture file utilities
pub struct FixtureFiles;

impl FixtureFiles {
    /// Create fixtures directory if it doesn't exist
    pub fn ensure_fixtures_dir() -> Result<()> {
        let fixtures_dir = TestFixtures::fixtures_dir();
        if !fixtures_dir.exists() {
            std::fs::create_dir_all(&fixtures_dir)
                .with_context(|| format!("Failed to create fixtures directory: {}", fixtures_dir.display()))?;
        }
        Ok(())
    }

    /// Write a fixture file
    pub fn write_fixture(name: &str, content: &str) -> Result<()> {
        Self::ensure_fixtures_dir()?;
        let path = TestFixtures::fixture_path(name);
        std::fs::write(&path, content).with_context(|| format!("Failed to write fixture: {}", path.display()))?;
        Ok(())
    }

    /// Create schema.sql fixture
    pub fn create_schema_fixture() -> Result<()> {
        let schema_content = format!(
            "{}\n\n{}\n\n{}\n\n{}",
            TestSchema::basic_test_table(),
            TestSchema::data_types_table(),
            TestSchema::edge_cases_table(),
            TestSchema::performance_table()
        );
        Self::write_fixture("schema.sql", &schema_content)
    }

    /// Create seed_data.sql fixture
    pub fn create_seed_data_fixture() -> Result<()> {
        let mut seed_content = String::new();

        // Add basic test data
        for statement in TestData::basic_test_data() {
            seed_content.push_str(statement);
            seed_content.push_str(";\n");
        }
        seed_content.push('\n');

        // Add data types test data
        for statement in TestData::data_types_test_data() {
            seed_content.push_str(statement);
            seed_content.push_str(";\n");
        }
        seed_content.push('\n');

        // Add edge cases test data
        for statement in TestData::edge_cases_test_data() {
            seed_content.push_str(statement);
            seed_content.push_str(";\n");
        }

        Self::write_fixture("seed_data.sql", &seed_content)
    }

    /// Create sample query files
    pub fn create_query_fixtures() -> Result<()> {
        Self::ensure_fixtures_dir()?;

        let queries_dir = TestFixtures::fixtures_dir().join("test_queries");
        if !queries_dir.exists() {
            std::fs::create_dir_all(&queries_dir)?;
        }

        // Create individual query files
        std::fs::write(queries_dir.join("basic_select.sql"), SampleQueries::basic_select())?;
        std::fs::write(queries_dir.join("data_types.sql"), SampleQueries::data_types_query())?;
        std::fs::write(queries_dir.join("empty_result.sql"), SampleQueries::empty_result_query())?;
        std::fs::write(queries_dir.join("special_chars.sql"), SampleQueries::special_chars_query())?;
        std::fs::write(queries_dir.join("unicode.sql"), SampleQueries::unicode_query())?;
        std::fs::write(queries_dir.join("invalid.sql"), SampleQueries::invalid_sql())?;

        Ok(())
    }
}
