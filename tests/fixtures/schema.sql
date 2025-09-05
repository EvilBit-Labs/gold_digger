-- Basic test table
CREATE TABLE IF NOT EXISTS test_basic (
    id INT PRIMARY KEY AUTO_INCREMENT,
    item_name VARCHAR(255),
    table_value INT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Comprehensive data types test table
CREATE TABLE IF NOT EXISTS test_data_types (
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
);

-- Edge cases test table
CREATE TABLE IF NOT EXISTS test_edge_cases (
    id INT PRIMARY KEY,
    null_varchar VARCHAR(255),
    empty_string VARCHAR(255),
    unicode_text TEXT CHARACTER SET utf8mb4,
    large_text LONGTEXT,
    special_chars VARCHAR(255),
    numeric_string VARCHAR(50),
    zero_values INT,
    negative_values INT
);

-- Performance test table (for large datasets)
CREATE TABLE IF NOT EXISTS test_performance (
    id INT PRIMARY KEY AUTO_INCREMENT,
    data_column VARCHAR(1000),
    numeric_column DECIMAL(15, 5),
    timestamp_column TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
