-- ============================================================================
-- Gold Digger Integration Test Database Schema
-- ============================================================================
-- This schema provides comprehensive test data for validating Gold Digger's
-- MySQL/MariaDB data type handling, output format compliance, and edge cases.
-- All tables use IF NOT EXISTS for idempotent schema creation.

-- ============================================================================
-- Basic Test Tables
-- ============================================================================

-- Simple test table for basic functionality validation
CREATE TABLE IF NOT EXISTS test_basic (
    id INT PRIMARY KEY AUTO_INCREMENT,
    item_name VARCHAR(255),
    table_value INT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================================
-- Comprehensive Data Types Test Tables
-- ============================================================================

-- Core data types table covering all MySQL/MariaDB data types
CREATE TABLE IF NOT EXISTS test_data_types (
    id INT PRIMARY KEY AUTO_INCREMENT,

    -- String types
    varchar_col VARCHAR(255),
    char_col CHAR(10),
    text_col TEXT,
    mediumtext_col MEDIUMTEXT,
    longtext_col LONGTEXT,
    tinytext_col TINYTEXT,

    -- Numeric types
    tinyint_col TINYINT,
    smallint_col SMALLINT,
    mediumint_col MEDIUMINT,
    int_col INT,
    bigint_col BIGINT,
    decimal_col DECIMAL(10, 2),
    numeric_col NUMERIC(15, 5),
    float_col FLOAT,
    double_col DOUBLE,
    real_col REAL,
    bit_col BIT(8),

    -- Date and time types
    date_col DATE,
    datetime_col DATETIME,
    timestamp_col TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    time_col TIME,
    year_col YEAR,

    -- Binary types
    binary_col BINARY(16),
    varbinary_col VARBINARY(255),
    tinyblob_col TINYBLOB,
    blob_col BLOB,
    mediumblob_col MEDIUMBLOB,
    longblob_col LONGBLOB,

    -- JSON and structured types
    json_col JSON,
    enum_col ENUM('small', 'medium', 'large'),
    set_col SET('red', 'green', 'blue', 'yellow'),
    bool_col BOOLEAN,
    boolean_col BOOLEAN
);

-- ============================================================================
-- Edge Cases and Special Values Test Tables
-- ============================================================================

-- Edge cases table for NULL values, empty strings, and special characters
CREATE TABLE IF NOT EXISTS test_edge_cases (
    id INT PRIMARY KEY,
    null_varchar VARCHAR(255),
    null_int INT,
    null_decimal DECIMAL(10, 2),
    null_date DATE,
    null_datetime DATETIME,
    null_json JSON,
    empty_string VARCHAR(255),
    zero_int INT,
    zero_decimal DECIMAL(10, 2),
    negative_int INT,
    negative_decimal DECIMAL(10, 2),
    special_chars VARCHAR(255),
    numeric_string VARCHAR(50),
    unicode_text TEXT CHARACTER SET utf8mb4,
    emoji_text VARCHAR(255) CHARACTER SET utf8mb4,
    control_chars VARCHAR(255),
    sql_injection VARCHAR(255),
    path_traversal VARCHAR(255)
);

-- Unicode and character encoding test table
CREATE TABLE IF NOT EXISTS test_unicode (
    id INT PRIMARY KEY AUTO_INCREMENT,
    ascii_text VARCHAR(255),
    latin1_text VARCHAR(255) CHARACTER SET latin1,
    utf8_text VARCHAR(255) CHARACTER SET utf8,
    utf8mb4_text VARCHAR(255) CHARACTER SET utf8mb4,
    chinese_text VARCHAR(255) CHARACTER SET utf8mb4,
    japanese_text VARCHAR(255) CHARACTER SET utf8mb4,
    arabic_text VARCHAR(255) CHARACTER SET utf8mb4,
    cyrillic_text VARCHAR(255) CHARACTER SET utf8mb4,
    emoji_text VARCHAR(255) CHARACTER SET utf8mb4,
    mixed_unicode TEXT CHARACTER SET utf8mb4
);

-- Large content test table for performance and memory testing
CREATE TABLE IF NOT EXISTS test_large_content (
    id INT PRIMARY KEY AUTO_INCREMENT,
    small_text VARCHAR(100),
    medium_text TEXT,
    large_text LONGTEXT,
    small_blob TINYBLOB,
    medium_blob BLOB,
    large_blob LONGBLOB,
    json_data JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================================
-- Performance and Scale Test Tables
-- ============================================================================

-- Performance test table for large datasets (1000+ rows)
CREATE TABLE IF NOT EXISTS test_performance (
    id INT PRIMARY KEY AUTO_INCREMENT,
    data_column VARCHAR(1000),
    numeric_column DECIMAL(15, 5),
    timestamp_column TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    index_column INT,
    text_column TEXT,
    json_column JSON
);

-- Wide table test (20+ columns) for column handling validation
CREATE TABLE IF NOT EXISTS test_wide_table (
    id INT PRIMARY KEY AUTO_INCREMENT,
    col_01 VARCHAR(100), col_02 VARCHAR(100), col_03 VARCHAR(100), col_04 VARCHAR(100), col_05 VARCHAR(100),
    col_06 VARCHAR(100), col_07 VARCHAR(100), col_08 VARCHAR(100), col_09 VARCHAR(100), col_10 VARCHAR(100),
    col_11 VARCHAR(100), col_12 VARCHAR(100), col_13 VARCHAR(100), col_14 VARCHAR(100), col_15 VARCHAR(100),
    col_16 VARCHAR(100), col_17 VARCHAR(100), col_18 VARCHAR(100), col_19 VARCHAR(100), col_20 VARCHAR(100),
    col_21 VARCHAR(100), col_22 VARCHAR(100), col_23 VARCHAR(100), col_24 VARCHAR(100), col_25 VARCHAR(100),
    numeric_01 INT, numeric_02 DECIMAL(10,2), numeric_03 FLOAT, numeric_04 DOUBLE,
    date_01 DATE, datetime_01 DATETIME, timestamp_01 TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    json_01 JSON, enum_01 ENUM('a', 'b', 'c'), bool_01 BOOLEAN
);

-- Numbers table for generating large result sets
CREATE TABLE IF NOT EXISTS test_numbers (
    n INT PRIMARY KEY
);

-- ============================================================================
-- MySQL-Specific Features Test Tables
-- ============================================================================

-- MySQL functions and expressions test table
CREATE TABLE IF NOT EXISTS test_mysql_functions (
    id INT PRIMARY KEY AUTO_INCREMENT,
    base_value INT,
    concat_result VARCHAR(255),
    math_result DECIMAL(10, 2),
    date_result DATE,
    string_result VARCHAR(255),
    json_result JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Character set and collation test table
CREATE TABLE IF NOT EXISTS test_charsets (
    id INT PRIMARY KEY AUTO_INCREMENT,
    utf8_general_ci VARCHAR(255) CHARACTER SET utf8 COLLATE utf8_general_ci,
    utf8_unicode_ci VARCHAR(255) CHARACTER SET utf8 COLLATE utf8_unicode_ci,
    utf8mb4_general_ci VARCHAR(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci,
    utf8mb4_unicode_ci VARCHAR(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci,
    latin1_swedish_ci VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_swedish_ci,
    ascii_bin VARCHAR(255) CHARACTER SET ascii COLLATE ascii_bin
);

-- Timezone and temporal test table
CREATE TABLE IF NOT EXISTS test_timezones (
    id INT PRIMARY KEY AUTO_INCREMENT,
    utc_timestamp_col TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    local_timestamp_col TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    date_only DATE,
    time_only TIME,
    datetime_with_tz DATETIME,
    year_only YEAR
);

-- ============================================================================
-- Error Scenario Test Tables
-- ============================================================================

-- Table for testing non-existent table scenarios (referenced in error tests)
-- This table will be dropped in error scenario tests
CREATE TABLE IF NOT EXISTS test_error_scenarios (
    id INT PRIMARY KEY,
    error_data VARCHAR(255)
);

-- ============================================================================
-- Indexes for Performance Testing
-- ============================================================================

-- Create indexes on performance test tables
CREATE INDEX IF NOT EXISTS idx_test_performance_timestamp ON test_performance(timestamp_column);
CREATE INDEX IF NOT EXISTS idx_test_performance_numeric ON test_performance(numeric_column);
CREATE INDEX IF NOT EXISTS idx_test_wide_table_numeric ON test_wide_table(numeric_01);
CREATE INDEX IF NOT EXISTS idx_test_mysql_functions_base ON test_mysql_functions(base_value);

-- ============================================================================
-- Views for Complex Query Testing
-- ============================================================================

-- Complex view for testing JOIN operations and subqueries
CREATE OR REPLACE VIEW test_complex_view AS
SELECT
    dt.id,
    dt.varchar_col,
    dt.int_col,
    dt.decimal_col,
    dt.date_col,
    p.data_column,
    p.numeric_column,
    CONCAT(dt.varchar_col, ' - ', p.data_column) AS combined_text,
    dt.decimal_col * p.numeric_column AS calculated_value
FROM test_data_types dt
LEFT JOIN test_performance p ON dt.id = p.id;

-- ============================================================================
-- Stored Procedures for Advanced Testing (MySQL 5.7+)
-- ============================================================================

DELIMITER $$

-- Procedure to generate test data
CREATE PROCEDURE IF NOT EXISTS GenerateTestData(IN row_count INT)
BEGIN
    DECLARE i INT DEFAULT 1;
    WHILE i <= row_count DO
        INSERT INTO test_performance (data_column, numeric_column, index_column, text_column, json_column)
        VALUES (
            CONCAT('Test data row ', i),
            ROUND(RAND() * 1000, 2),
            i,
            CONCAT('This is test text for row ', i, ' with some additional content to make it longer.'),
            JSON_OBJECT('id', i, 'value', ROUND(RAND() * 100, 2), 'timestamp', NOW())
        );
        SET i = i + 1;
    END WHILE;
END$$

-- Procedure to populate numbers table
CREATE PROCEDURE IF NOT EXISTS PopulateNumbers(IN max_number INT)
BEGIN
    DECLARE i INT DEFAULT 1;
    WHILE i <= max_number DO
        INSERT IGNORE INTO test_numbers (n) VALUES (i);
        SET i = i + 1;
    END WHILE;
END$$

DELIMITER ;
