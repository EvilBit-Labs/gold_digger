-- Basic test data
INSERT INTO test_basic (item_name, table_value)
VALUES ('test1', 100)
ON DUPLICATE KEY UPDATE table_value = VALUES (table_value);

INSERT INTO test_basic (item_name, table_value)
VALUES ('test2', 200)
ON DUPLICATE KEY UPDATE table_value = VALUES (table_value);

INSERT INTO test_basic (item_name, table_value)
VALUES ('test3', 300)
ON DUPLICATE KEY UPDATE table_value = VALUES (table_value);

-- Comprehensive data types test data
INSERT INTO test_data_types (
    varchar_col,
    text_col,
    int_col,
    bigint_col,
    decimal_col,
    float_col,
    double_col,
    date_col,
    datetime_col,
    timestamp_col,
    time_col,
    year_col,
    binary_col,
    varbinary_col,
    blob_col,
    json_col,
    enum_col,
    set_col,
    bool_col
)
VALUES (
    'Sample text',
    'Longer text content',
    42,
    9223372036854775807,
    123.45,
    3.14159,
    2.718281828,
    '2024-01-01',
    '2024-01-01 12:00:00',
    '2024-01-01 12:00:00',
    '12:00:00',
    2024,
    UNHEX('48656C6C6F20576F726C64210000000000'),
    UNHEX('48656C6C6F'),
    UNHEX('48656C6C6F20576F726C6421'),
    '{"key": "value", "number": 42}',
    'medium',
    'red,blue',
    TRUE
)
ON DUPLICATE KEY UPDATE varchar_col = VALUES (varchar_col);

-- Edge cases test data
INSERT INTO test_edge_cases (
    id,
    null_varchar,
    empty_string,
    unicode_text,
    large_text,
    special_chars,
    numeric_string,
    zero_values,
    negative_values
)
VALUES (
    1,
    NULL,
    '',
    'Hello 世界 🚀',
    REPEAT('Large text content ', 1000),
    'Special: ",\n\t',
    '12345',
    0,
    -42
)
ON DUPLICATE KEY UPDATE unicode_text = VALUES (unicode_text);

INSERT INTO test_edge_cases (
    id,
    null_varchar,
    empty_string,
    unicode_text,
    large_text,
    special_chars,
    numeric_string,
    zero_values,
    negative_values
)
VALUES (
    2,
    NULL,
    '',
    'Café Ñoño',
    'Normal text',
    'Quotes: "Hello"',
    '67890',
    0,
    -100
)
ON DUPLICATE KEY UPDATE unicode_text = VALUES (unicode_text);
