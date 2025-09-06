-- ============================================================================
-- Gold Digger Integration Test Data Seeding
-- ============================================================================
-- Comprehensive test data covering all MySQL/MariaDB data types, edge cases,
-- Unicode characters, and performance testing scenarios.
-- Uses INSERT ... ON DUPLICATE KEY UPDATE for idempotent seeding.

-- ============================================================================
-- Basic Test Data
-- ============================================================================

INSERT INTO test_basic (item_name, table_value)
VALUES ('test1', 100) AS new
ON DUPLICATE KEY UPDATE table_value = new.table_value;

INSERT INTO test_basic (item_name, table_value)
VALUES ('test2', 200) AS new
ON DUPLICATE KEY UPDATE table_value = new.table_value;

INSERT INTO test_basic (item_name, table_value)
VALUES ('test3', 300) AS new
ON DUPLICATE KEY UPDATE table_value = new.table_value;

INSERT INTO test_basic (item_name, table_value)
VALUES ('empty_test', 0) AS new
ON DUPLICATE KEY UPDATE table_value = new.table_value;

INSERT INTO test_basic (item_name, table_value)
VALUES ('negative_test', -50) AS new
ON DUPLICATE KEY UPDATE table_value = new.table_value;

-- ============================================================================
-- Comprehensive Data Types Test Data
-- ============================================================================

-- Normal values covering all data types
INSERT INTO test_data_types (
    varchar_col, char_col, text_col, mediumtext_col, longtext_col, tinytext_col,
    tinyint_col, smallint_col, mediumint_col, int_col, bigint_col,
    decimal_col, numeric_col, float_col, double_col, real_col, bit_col,
    date_col, datetime_col, timestamp_col, time_col, year_col,
    binary_col, varbinary_col, tinyblob_col, blob_col, mediumblob_col, longblob_col,
    json_col, enum_col, set_col, bool_col, boolean_col
)
VALUES (
    'Sample varchar text', 'char10    ', 'This is a TEXT column with more content',
    'Medium text content for testing', 'Long text content for comprehensive testing',
    'Tiny text', 127, 32767, 8388607, 2147483647, 9223372036854775807,
    99999.99, 12345.67890, 3.14159, 2.718281828459045, 1.414213562,
    b'10101010', '2024-01-15', '2024-01-15 14:30:00', '2024-01-15 14:30:00',
    '14:30:00', 2024, UNHEX('48656C6C6F20576F726C64210000000000'),
    UNHEX('48656C6C6F20576F726C6421'), UNHEX('54696E79'), UNHEX('426C6F6220636F6E74656E74'),
    UNHEX('4D656469756D20626C6F6220636F6E74656E74'), UNHEX('4C6F6E6720626C6F6220636F6E74656E74'),
    '{"name": "test", "value": 42, "active": true, "tags": ["mysql", "testing"]}',
    'medium', 'red,blue', TRUE, FALSE
) AS new
ON DUPLICATE KEY UPDATE
    varchar_col = new.varchar_col, char_col = new.char_col, text_col = new.text_col,
    mediumtext_col = new.mediumtext_col, longtext_col = new.longtext_col, tinytext_col = new.tinytext_col,
    tinyint_col = new.tinyint_col, smallint_col = new.smallint_col, mediumint_col = new.mediumint_col,
    int_col = new.int_col, bigint_col = new.bigint_col, decimal_col = new.decimal_col,
    numeric_col = new.numeric_col, float_col = new.float_col, double_col = new.double_col,
    real_col = new.real_col, bit_col = new.bit_col, date_col = new.date_col,
    datetime_col = new.datetime_col, timestamp_col = new.timestamp_col, time_col = new.time_col,
    year_col = new.year_col, binary_col = new.binary_col, varbinary_col = new.varbinary_col,
    tinyblob_col = new.tinyblob_col, blob_col = new.blob_col, mediumblob_col = new.mediumblob_col,
    longblob_col = new.longblob_col, json_col = new.json_col, enum_col = new.enum_col,
    set_col = new.set_col, bool_col = new.bool_col, boolean_col = new.boolean_col;

-- Edge case values: minimum values
INSERT INTO test_data_types (
    varchar_col, char_col, text_col, mediumtext_col, longtext_col, tinytext_col,
    tinyint_col, smallint_col, mediumint_col, int_col, bigint_col,
    decimal_col, numeric_col, float_col, double_col, real_col, bit_col,
    date_col, datetime_col, timestamp_col, time_col, year_col,
    binary_col, varbinary_col, tinyblob_col, blob_col, mediumblob_col, longblob_col,
    json_col, enum_col, set_col, bool_col, boolean_col
)
VALUES (
    '', '', '', '', '', '', -128, -32768, -8388608, -2147483648, -9223372036854775808,
    -99999.99, -12345.67890, -3.14159, -2.718281828459045, -1.414213562,
    b'00000000', '1000-01-01', '1000-01-01 00:00:00', '1970-01-01 00:00:01',
    '-838:59:59', 1901, UNHEX('00000000000000000000000000000000'),
    UNHEX(''), UNHEX(''), UNHEX(''), UNHEX(''), UNHEX(''),
    '{}', 'small', '', FALSE, TRUE
) AS new
ON DUPLICATE KEY UPDATE
    varchar_col = new.varchar_col, char_col = new.char_col, text_col = new.text_col,
    mediumtext_col = new.mediumtext_col, longtext_col = new.longtext_col, tinytext_col = new.tinytext_col,
    tinyint_col = new.tinyint_col, smallint_col = new.smallint_col, mediumint_col = new.mediumint_col,
    int_col = new.int_col, bigint_col = new.bigint_col, decimal_col = new.decimal_col,
    numeric_col = new.numeric_col, float_col = new.float_col, double_col = new.double_col,
    real_col = new.real_col, bit_col = new.bit_col, date_col = new.date_col,
    datetime_col = new.datetime_col, timestamp_col = new.timestamp_col, time_col = new.time_col,
    year_col = new.year_col, binary_col = new.binary_col, varbinary_col = new.varbinary_col,
    tinyblob_col = new.tinyblob_col, blob_col = new.blob_col, mediumblob_col = new.mediumblob_col,
    longblob_col = new.longblob_col, json_col = new.json_col, enum_col = new.enum_col,
    set_col = new.set_col, bool_col = new.bool_col, boolean_col = new.boolean_col;

-- Edge case values: maximum values
INSERT INTO test_data_types (
    varchar_col, char_col, text_col, mediumtext_col, longtext_col, tinytext_col,
    tinyint_col, smallint_col, mediumint_col, int_col, bigint_col,
    decimal_col, numeric_col, float_col, double_col, real_col, bit_col,
    date_col, datetime_col, timestamp_col, time_col, year_col,
    binary_col, varbinary_col, tinyblob_col, blob_col, mediumblob_col, longblob_col,
    json_col, enum_col, set_col, bool_col, boolean_col
)
VALUES (
    REPEAT('A', 255), 'MAXCHAR123', REPEAT('Maximum text content. ', 100),
    REPEAT('Medium maximum content. ', 200), REPEAT('Long maximum content. ', 500),
    REPEAT('T', 255), 127, 32767, 8388607, 2147483647, 9223372036854775807,
    99999.99, 99999.99999, 3.402823466E+38, 1.7976931348623157E+308, 3.402823466E+38,
    b'11111111', '9999-12-31', '9999-12-31 23:59:59', '2038-01-19 03:14:07',
    '838:59:59', 2155, UNHEX('FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF'),
    REPEAT(UNHEX('FF'), 255), REPEAT(UNHEX('FF'), 255), REPEAT(UNHEX('FF'), 1000),
    REPEAT(UNHEX('FF'), 2000), REPEAT(UNHEX('FF'), 5000),
    '{"max": true, "array": [1,2,3,4,5], "nested": {"deep": {"value": "maximum"}}}',
    'large', 'red,green,blue,yellow', TRUE, FALSE
) AS new
ON DUPLICATE KEY UPDATE
    varchar_col = new.varchar_col, char_col = new.char_col, text_col = new.text_col,
    mediumtext_col = new.mediumtext_col, longtext_col = new.longtext_col, tinytext_col = new.tinytext_col,
    tinyint_col = new.tinyint_col, smallint_col = new.smallint_col, mediumint_col = new.mediumint_col,
    int_col = new.int_col, bigint_col = new.bigint_col, decimal_col = new.decimal_col,
    numeric_col = new.numeric_col, float_col = new.float_col, double_col = new.double_col,
    real_col = new.real_col, bit_col = new.bit_col, date_col = new.date_col,
    datetime_col = new.datetime_col, timestamp_col = new.timestamp_col, time_col = new.time_col,
    year_col = new.year_col, binary_col = new.binary_col, varbinary_col = new.varbinary_col,
    tinyblob_col = new.tinyblob_col, blob_col = new.blob_col, mediumblob_col = new.mediumblob_col,
    longblob_col = new.longblob_col, json_col = new.json_col, enum_col = new.enum_col,
    set_col = new.set_col, bool_col = new.bool_col, boolean_col = new.boolean_col;

-- NULL values test data
INSERT INTO test_data_types (
    varchar_col, char_col, text_col, mediumtext_col, longtext_col, tinytext_col,
    tinyint_col, smallint_col, mediumint_col, int_col, bigint_col,
    decimal_col, numeric_col, float_col, double_col, real_col, bit_col,
    date_col, datetime_col, time_col, year_col,
    binary_col, varbinary_col, tinyblob_col, blob_col, mediumblob_col, longblob_col,
    json_col, enum_col, set_col, bool_col, boolean_col
)
VALUES (
    NULL, NULL, NULL, NULL, NULL, NULL,
    NULL, NULL, NULL, NULL, NULL,
    NULL, NULL, NULL, NULL, NULL, NULL,
    NULL, NULL, NULL, NULL,
    NULL, NULL, NULL, NULL, NULL, NULL,
    NULL, NULL, NULL, NULL, NULL
) AS new
ON DUPLICATE KEY UPDATE
    varchar_col = new.varchar_col, char_col = new.char_col, text_col = new.text_col,
    mediumtext_col = new.mediumtext_col, longtext_col = new.longtext_col, tinytext_col = new.tinytext_col,
    tinyint_col = new.tinyint_col, smallint_col = new.smallint_col, mediumint_col = new.mediumint_col,
    int_col = new.int_col, bigint_col = new.bigint_col, decimal_col = new.decimal_col,
    numeric_col = new.numeric_col, float_col = new.float_col, double_col = new.double_col,
    real_col = new.real_col, bit_col = new.bit_col, date_col = new.date_col,
    datetime_col = new.datetime_col, time_col = new.time_col, year_col = new.year_col,
    binary_col = new.binary_col, varbinary_col = new.varbinary_col,
    tinyblob_col = new.tinyblob_col, blob_col = new.blob_col, mediumblob_col = new.mediumblob_col,
    longblob_col = new.longblob_col, json_col = new.json_col, enum_col = new.enum_col,
    set_col = new.set_col, bool_col = new.bool_col, boolean_col = new.boolean_col;

-- ============================================================================
-- Edge Cases and Special Values Test Data
-- ============================================================================

-- Comprehensive edge cases with NULL values across all types
INSERT INTO test_edge_cases (
    id, null_varchar, null_int, null_decimal, null_date, null_datetime, null_json,
    empty_string, zero_int, zero_decimal, negative_int, negative_decimal,
    special_chars, numeric_string, unicode_text, emoji_text, control_chars,
    sql_injection, path_traversal
)
VALUES (
    1, NULL, NULL, NULL, NULL, NULL, NULL,
    '', 0, 0.00, -42, -123.45,
    'Special chars: ",\n\t\r\\''`', '12345',
    'Hello 世界 🚀 Testing Unicode', '🎉🚀🌟💻🔥',
    CONCAT(CHAR(9), CHAR(10), CHAR(13), CHAR(0)),
    'SELECT * FROM users; DROP TABLE users; --',
    '../../../etc/passwd'
) AS new
ON DUPLICATE KEY UPDATE
    unicode_text = new.unicode_text, emoji_text = new.emoji_text,
    special_chars = new.special_chars, sql_injection = new.sql_injection;

-- More edge cases with different patterns
INSERT INTO test_edge_cases (
    id, null_varchar, null_int, null_decimal, null_date, null_datetime, null_json,
    empty_string, zero_int, zero_decimal, negative_int, negative_decimal,
    special_chars, numeric_string, unicode_text, emoji_text, control_chars,
    sql_injection, path_traversal
)
VALUES (
    2, 'Not NULL', 42, 99.99, '2024-02-01', '2024-02-01 10:00:00', '{"test": true}',
    '', 0, 0.00, -999, -999.99,
    'Quotes: "Hello" ''World'' `Backtick`', '67890',
    'Café Ñoño résumé naïve', '🇺🇸🇯🇵🇩🇪🇫🇷🇬🇧',
    'Control\x00\x01\x02\x03\x04\x05',
    'UNION SELECT password FROM admin_users',
    '..\\..\\..\\windows\\system32\\config\\sam'
) AS new
ON DUPLICATE KEY UPDATE
    unicode_text = new.unicode_text, emoji_text = new.emoji_text,
    special_chars = new.special_chars, sql_injection = new.sql_injection;

-- Large text content edge case
INSERT INTO test_edge_cases (
    id, null_varchar, null_int, null_decimal, null_date, null_datetime, null_json,
    empty_string, zero_int, zero_decimal, negative_int, negative_decimal,
    special_chars, numeric_string, unicode_text, emoji_text, control_chars,
    sql_injection, path_traversal
)
VALUES (
    3, 'Large content test', 1000, 1000.00, '2024-03-01', '2024-03-01 15:00:00', '{"size": "large"}',
    '', 0, 0.00, -1000, -1000.00,
    REPEAT('Large text content with special chars: ",\n\t ', 100), '99999',
    REPEAT('Unicode test 世界 🚀 ', 50), REPEAT('🎯🎪🎨🎭🎪 ', 20),
    REPEAT(CONCAT(CHAR(9), CHAR(10), CHAR(13)), 10),
    REPEAT('SELECT * FROM sensitive_data; ', 10),
    REPEAT('../', 50)
) AS new
ON DUPLICATE KEY UPDATE
    unicode_text = new.unicode_text, emoji_text = new.emoji_text,
    special_chars = new.special_chars, sql_injection = new.sql_injection;

-- ============================================================================
-- Unicode and Character Encoding Test Data
-- ============================================================================

-- ASCII text
INSERT INTO test_unicode (ascii_text, latin1_text, utf8_text, utf8mb4_text, chinese_text, japanese_text, arabic_text, cyrillic_text, emoji_text, mixed_unicode)
VALUES (
    'Basic ASCII text 123',
    'Latin1 text with accents: café résumé naïve',
    'UTF8 text: Hello World',
    'UTF8MB4 text with emojis: Hello 🌍',
    '你好世界 - 中文测试',
    'こんにちは世界 - 日本語テスト',
    'مرحبا بالعالم - اختبار عربي',
    'Привет мир - русский тест',
    '🚀🌟💻🔥🎉🎯🎪🎨🎭🎪',
    'Mixed: Hello 世界 🚀 مرحبا Привет こんにちは café'
) AS new
ON DUPLICATE KEY UPDATE
    ascii_text = new.ascii_text, latin1_text = new.latin1_text, utf8_text = new.utf8_text,
    utf8mb4_text = new.utf8mb4_text, chinese_text = new.chinese_text, japanese_text = new.japanese_text,
    arabic_text = new.arabic_text, cyrillic_text = new.cyrillic_text, emoji_text = new.emoji_text,
    mixed_unicode = new.mixed_unicode;

-- More Unicode variations
INSERT INTO test_unicode (ascii_text, latin1_text, utf8_text, utf8mb4_text, chinese_text, japanese_text, arabic_text, cyrillic_text, emoji_text, mixed_unicode)
VALUES (
    'Numbers and symbols: 0123456789 !@#$%^&*()',
    'Extended Latin: àáâãäåæçèéêëìíîïðñòóôõöøùúûüýþÿ',
    'UTF8 symbols: ©®™€£¥§¶†‡•…‰‹›""''–—',
    'Mathematical symbols: ∀∂∃∅∇∈∉∋∌∏∑−∕∗∘∙√∝∞∠∡∢∣∤∥∦∧∨∩∪∫∬∭∮∯∰∱∲∳',
    '繁體中文測試 - 香港台灣',
    'ひらがな: あいうえお カタカナ: アイウエオ',
    'أرقام عربية: ٠١٢٣٤٥٦٧٨٩',
    'Кириллица: АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ',
    '🏠🏡🏢🏣🏤🏥🏦🏧🏨🏩🏪🏫🏬🏭🏮🏯',
    'Currency: $€£¥₹₽₩₪₫₱₡₦₨₩₪₫₱₡₦₨'
) AS new
ON DUPLICATE KEY UPDATE
    ascii_text = new.ascii_text, latin1_text = new.latin1_text, utf8_text = new.utf8_text,
    utf8mb4_text = new.utf8mb4_text, chinese_text = new.chinese_text, japanese_text = new.japanese_text,
    arabic_text = new.arabic_text, cyrillic_text = new.cyrillic_text, emoji_text = new.emoji_text,
    mixed_unicode = new.mixed_unicode;

-- ============================================================================
-- Large Content Test Data
-- ============================================================================

-- Small content
INSERT INTO test_large_content (small_text, medium_text, large_text, small_blob, medium_blob, large_blob, json_data)
VALUES (
    'Small text content',
    REPEAT('Medium text content for testing. ', 100),
    REPEAT('Large text content for comprehensive testing and validation. ', 1000),
    UNHEX('536D616C6C20626C6F6220636F6E74656E74'),
    REPEAT(UNHEX('4D656469756D20626C6F6220636F6E74656E7420'), 100),
    REPEAT(UNHEX('4C6172676520626C6F6220636F6E74656E7420666F722074657374696E6720'), 500),
    '{"type": "small", "size": 1024, "content": "test data"}'
) AS new
ON DUPLICATE KEY UPDATE
    small_text = new.small_text, medium_text = new.medium_text, large_text = new.large_text,
    small_blob = new.small_blob, medium_blob = new.medium_blob, large_blob = new.large_blob,
    json_data = new.json_data;

-- Medium content
INSERT INTO test_large_content (small_text, medium_text, large_text, small_blob, medium_blob, large_blob, json_data)
VALUES (
    'Medium test',
    REPEAT('This is medium text content with more data for testing purposes. ', 500),
    REPEAT('This is large text content with extensive data for comprehensive testing and validation of memory handling. ', 2000),
    REPEAT(UNHEX('4D656469756D'), 50),
    REPEAT(UNHEX('4D656469756D20626C6F6220636F6E74656E7420666F722074657374696E6720'), 200),
    REPEAT(UNHEX('4C6172676520626C6F6220636F6E74656E7420666F7220657874656E736976652074657374696E6720'), 1000),
    JSON_OBJECT('type', 'medium', 'size', 65536, 'data', REPEAT('test', 100), 'timestamp', NOW())
) AS new
ON DUPLICATE KEY UPDATE
    small_text = new.small_text, medium_text = new.medium_text, large_text = new.large_text,
    small_blob = new.small_blob, medium_blob = new.medium_blob, large_blob = new.large_blob,
    json_data = new.json_data;

-- Large content (1MB+ text)
INSERT INTO test_large_content (small_text, medium_text, large_text, small_blob, medium_blob, large_blob, json_data)
VALUES (
    'Large test',
    REPEAT('Large medium text content with substantial data for comprehensive testing. ', 1000),
    REPEAT('This is very large text content designed to test memory handling and performance with substantial amounts of data. It includes various characters and patterns to ensure comprehensive validation of text processing capabilities. ', 5000),
    REPEAT(UNHEX('4C61726765'), 100),
    REPEAT(UNHEX('4C6172676520626C6F6220636F6E74656E7420666F7220706572666F726D616E63652074657374696E6720'), 500),
    REPEAT(UNHEX('56657279206C6172676520626C6F6220636F6E74656E7420666F7220657874656E736976652074657374696E6720616E642076616C69646174696F6E20'), 2000),
    JSON_OBJECT('type', 'large', 'size', 1048576, 'data', REPEAT('large_test_data', 1000), 'metadata', JSON_OBJECT('created', NOW(), 'version', '1.0'))
) AS new
ON DUPLICATE KEY UPDATE
    small_text = new.small_text, medium_text = new.medium_text, large_text = new.large_text,
    small_blob = new.small_blob, medium_blob = new.medium_blob, large_blob = new.large_blob,
    json_data = new.json_data;

-- ============================================================================
-- Performance Test Data (1000+ rows)
-- ============================================================================

-- Generate numbers table for large result sets
INSERT INTO test_numbers (n) VALUES (1), (2), (3), (4), (5), (6), (7), (8), (9), (10) AS new
ON DUPLICATE KEY UPDATE n = new.n;

-- Use the numbers table to generate more numbers (up to 100)
INSERT INTO test_numbers (n)
SELECT a.n + b.n * 10 AS n
FROM test_numbers a
CROSS JOIN test_numbers b
WHERE a.n <= 10 AND b.n <= 10 AND a.n + b.n * 10 <= 100
ON DUPLICATE KEY UPDATE n = VALUES(n);

-- Generate up to 1000 numbers
INSERT INTO test_numbers (n)
SELECT a.n + b.n * 100 AS n
FROM test_numbers a
CROSS JOIN test_numbers b
WHERE a.n <= 100 AND b.n <= 10 AND a.n + b.n * 100 <= 1000
ON DUPLICATE KEY UPDATE n = VALUES(n);

-- Generate performance test data using the numbers table
INSERT INTO test_performance (data_column, numeric_column, index_column, text_column, json_column)
SELECT
    CONCAT('Performance test data row ', n, ' with additional content for testing') AS data_column,
    ROUND(n * 3.14159 + (n % 100) * 0.01, 2) AS numeric_column,
    n AS index_column,
    CONCAT('This is test text for row ', n, '. It contains various content to test text handling: ',
           'numbers (', n, '), special chars (!@#$%^&*()), and unicode (世界🚀). ',
           'Additional padding: ', REPEAT('test ', n % 10 + 1)) AS text_column,
    JSON_OBJECT(
        'id', n,
        'value', ROUND(n * 2.71828, 3),
        'category', CASE
            WHEN n % 4 = 0 THEN 'alpha'
            WHEN n % 4 = 1 THEN 'beta'
            WHEN n % 4 = 2 THEN 'gamma'
            ELSE 'delta'
        END,
        'active', n % 2 = 1,
        'metadata', JSON_OBJECT('created_at', NOW(), 'row_number', n)
    ) AS json_column
FROM test_numbers
WHERE n <= 1000
ON DUPLICATE KEY UPDATE
    data_column = VALUES(data_column),
    numeric_column = VALUES(numeric_column),
    index_column = VALUES(index_column),
    text_column = VALUES(text_column),
    json_column = VALUES(json_column);

-- ============================================================================
-- Wide Table Test Data (20+ columns)
-- ============================================================================

-- Generate wide table test data
INSERT INTO test_wide_table (
    col_01, col_02, col_03, col_04, col_05, col_06, col_07, col_08, col_09, col_10,
    col_11, col_12, col_13, col_14, col_15, col_16, col_17, col_18, col_19, col_20,
    col_21, col_22, col_23, col_24, col_25,
    numeric_01, numeric_02, numeric_03, numeric_04,
    date_01, datetime_01, json_01, enum_01, bool_01
)
SELECT
    CONCAT('Col01_', n) AS col_01, CONCAT('Col02_', n) AS col_02, CONCAT('Col03_', n) AS col_03,
    CONCAT('Col04_', n) AS col_04, CONCAT('Col05_', n) AS col_05, CONCAT('Col06_', n) AS col_06,
    CONCAT('Col07_', n) AS col_07, CONCAT('Col08_', n) AS col_08, CONCAT('Col09_', n) AS col_09,
    CONCAT('Col10_', n) AS col_10, CONCAT('Col11_', n) AS col_11, CONCAT('Col12_', n) AS col_12,
    CONCAT('Col13_', n) AS col_13, CONCAT('Col14_', n) AS col_14, CONCAT('Col15_', n) AS col_15,
    CONCAT('Col16_', n) AS col_16, CONCAT('Col17_', n) AS col_17, CONCAT('Col18_', n) AS col_18,
    CONCAT('Col19_', n) AS col_19, CONCAT('Col20_', n) AS col_20, CONCAT('Col21_', n) AS col_21,
    CONCAT('Col22_', n) AS col_22, CONCAT('Col23_', n) AS col_23, CONCAT('Col24_', n) AS col_24,
    CONCAT('Col25_', n) AS col_25,
    n AS numeric_01, ROUND(n * 1.5, 2) AS numeric_02, n * 2.5 AS numeric_03, n * 3.7 AS numeric_04,
    DATE_ADD('2024-01-01', INTERVAL n DAY) AS date_01,
    DATE_ADD('2024-01-01 00:00:00', INTERVAL n * 3600 SECOND) AS datetime_01,
    JSON_OBJECT('row', n, 'wide_table', true, 'columns', 35) AS json_01,
    CASE WHEN n % 3 = 0 THEN 'a' WHEN n % 3 = 1 THEN 'b' ELSE 'c' END AS enum_01,
    n % 2 = 1 AS bool_01
FROM test_numbers
WHERE n <= 100
ON DUPLICATE KEY UPDATE
    col_01 = VALUES(col_01), col_02 = VALUES(col_02), col_03 = VALUES(col_03),
    col_04 = VALUES(col_04), col_05 = VALUES(col_05), col_06 = VALUES(col_06),
    col_07 = VALUES(col_07), col_08 = VALUES(col_08), col_09 = VALUES(col_09),
    col_10 = VALUES(col_10), col_11 = VALUES(col_11), col_12 = VALUES(col_12),
    col_13 = VALUES(col_13), col_14 = VALUES(col_14), col_15 = VALUES(col_15),
    col_16 = VALUES(col_16), col_17 = VALUES(col_17), col_18 = VALUES(col_18),
    col_19 = VALUES(col_19), col_20 = VALUES(col_20), col_21 = VALUES(col_21),
    col_22 = VALUES(col_22), col_23 = VALUES(col_23), col_24 = VALUES(col_24),
    col_25 = VALUES(col_25), numeric_01 = VALUES(numeric_01), numeric_02 = VALUES(numeric_02),
    numeric_03 = VALUES(numeric_03), numeric_04 = VALUES(numeric_04), date_01 = VALUES(date_01),
    datetime_01 = VALUES(datetime_01), json_01 = VALUES(json_01), enum_01 = VALUES(enum_01),
    bool_01 = VALUES(bool_01);

-- ============================================================================
-- MySQL-Specific Features Test Data
-- ============================================================================

-- MySQL functions and expressions test data
INSERT INTO test_mysql_functions (base_value, concat_result, math_result, date_result, string_result, json_result)
VALUES
    (1, CONCAT('Test_', 1), ROUND(1 * PI(), 2), CURDATE(), UPPER('mysql_test_1'), JSON_OBJECT('func_test', 1, 'pi_value', PI())),
    (2, CONCAT('Test_', 2), ROUND(2 * PI(), 2), DATE_ADD(CURDATE(), INTERVAL 1 DAY), LOWER('MYSQL_TEST_2'), JSON_OBJECT('func_test', 2, 'sqrt_value', SQRT(2))),
    (3, CONCAT('Test_', 3), ROUND(3 * PI(), 2), DATE_SUB(CURDATE(), INTERVAL 1 DAY), REVERSE('3_tset_lqsym'), JSON_OBJECT('func_test', 3, 'pow_value', POW(3, 2))),
    (4, CONCAT('Test_', 4), ROUND(4 * PI(), 2), DATE_ADD(CURDATE(), INTERVAL 1 WEEK), SUBSTRING('mysql_test_4', 1, 5), JSON_OBJECT('func_test', 4, 'log_value', LOG(4))),
    (5, CONCAT('Test_', 5), ROUND(5 * PI(), 2), DATE_SUB(CURDATE(), INTERVAL 1 MONTH), REPLACE('mysql_test_5', 'test', 'demo'), JSON_OBJECT('func_test', 5, 'exp_value', EXP(1)))
AS new
ON DUPLICATE KEY UPDATE
    concat_result = new.concat_result, math_result = new.math_result, date_result = new.date_result,
    string_result = new.string_result, json_result = new.json_result;

-- ============================================================================
-- Character Set and Collation Test Data
-- ============================================================================

INSERT INTO test_charsets (
    utf8_general_ci, utf8_unicode_ci, utf8mb4_general_ci, utf8mb4_unicode_ci,
    latin1_swedish_ci, ascii_bin
)
VALUES
    ('UTF8 General CI: café résumé', 'UTF8 Unicode CI: café résumé', 'UTF8MB4 General: café 🚀', 'UTF8MB4 Unicode: café 🚀', 'Latin1: cafe resume', 'ASCII: cafe resume'),
    ('Sorting test: apple', 'Sorting test: apple', 'Emoji sort: 🍎 apple', 'Emoji sort: 🍎 apple', 'Sort: apple', 'Sort: apple'),
    ('Sorting test: Äpfel', 'Sorting test: Äpfel', 'Unicode sort: Äpfel 🍏', 'Unicode sort: Äpfel 🍏', 'Sort: Apfel', 'Sort: Apfel'),
    ('Case test: CAFÉ', 'Case test: CAFÉ', 'Case emoji: CAFÉ 🏪', 'Case emoji: CAFÉ 🏪', 'Case: CAFE', 'Case: CAFE'),
    ('Accent test: naïve', 'Accent test: naïve', 'Accent emoji: naïve 😊', 'Accent emoji: naïve 😊', 'Accent: naive', 'Accent: naive')
AS new
ON DUPLICATE KEY UPDATE
    utf8_general_ci = new.utf8_general_ci, utf8_unicode_ci = new.utf8_unicode_ci,
    utf8mb4_general_ci = new.utf8mb4_general_ci, utf8mb4_unicode_ci = new.utf8mb4_unicode_ci,
    latin1_swedish_ci = new.latin1_swedish_ci, ascii_bin = new.ascii_bin;

-- ============================================================================
-- Timezone and Temporal Test Data
-- ============================================================================

INSERT INTO test_timezones (utc_timestamp, local_timestamp, date_only, time_only, datetime_with_tz, year_only)
VALUES
    ('2024-01-01 00:00:00', '2024-01-01 00:00:00', '2024-01-01', '00:00:00', '2024-01-01 00:00:00', 2024),
    ('2024-06-15 12:30:45', '2024-06-15 12:30:45', '2024-06-15', '12:30:45', '2024-06-15 12:30:45', 2024),
    ('2024-12-31 23:59:59', '2024-12-31 23:59:59', '2024-12-31', '23:59:59', '2024-12-31 23:59:59', 2024),
    ('2025-02-29 06:15:30', '2025-02-29 06:15:30', '2025-02-29', '06:15:30', '2025-02-29 06:15:30', 2025),
    ('1970-01-01 00:00:01', '1970-01-01 00:00:01', '1970-01-01', '00:00:01', '1970-01-01 00:00:01', 1970)
AS new
ON DUPLICATE KEY UPDATE
    utc_timestamp = new.utc_timestamp, local_timestamp = new.local_timestamp,
    date_only = new.date_only, time_only = new.time_only,
    datetime_with_tz = new.datetime_with_tz, year_only = new.year_only;

-- ============================================================================
-- Database-Specific Test Data (MySQL vs MariaDB differences)
-- ============================================================================

-- Test data that may behave differently between MySQL and MariaDB
INSERT INTO test_mysql_functions (base_value, concat_result, math_result, date_result, string_result, json_result)
VALUES
    -- MySQL 8.0+ specific functions (may not work in older versions or MariaDB)
    (100, 'MySQL 8.0 test', 100.0, CURDATE(), 'mysql8_specific', '{"mysql_version": "8.0", "feature": "window_functions"}'),
    -- MariaDB specific test (should work in both but may have different behavior)
    (200, 'MariaDB test', 200.0, CURDATE(), 'mariadb_specific', '{"mariadb_version": "10.x", "feature": "sequences"}'),
    -- Common functionality that should work in both
    (300, 'Common test', 300.0, CURDATE(), 'common_feature', '{"compatibility": "both", "feature": "standard_sql"}')
AS new
ON DUPLICATE KEY UPDATE
    concat_result = new.concat_result, math_result = new.math_result, date_result = new.date_result,
    string_result = new.string_result, json_result = new.json_result;

-- JSON data with different complexity levels for MySQL vs MariaDB JSON handling
INSERT INTO test_data_types (json_col)
VALUES
    ('{"simple": "value"}'),
    ('{"nested": {"deep": {"value": "test"}}}'),
    ('{"array": [1, 2, 3, "string", true, null]}'),
    ('{"mixed": {"numbers": [1, 2.5, -3], "strings": ["hello", "world"], "boolean": true, "null_value": null}}'),
    ('[{"id": 1, "name": "first"}, {"id": 2, "name": "second"}]')
AS new
ON DUPLICATE KEY UPDATE json_col = new.json_col;

-- ============================================================================
-- Error Scenario Test Data
-- ============================================================================

-- Data for error scenario testing (will be used in error tests)
INSERT INTO test_error_scenarios (id, error_data)
VALUES
    (1, 'Valid data for error testing'),
    (2, 'Another valid row'),
    (3, 'Third test row')
AS new
ON DUPLICATE KEY UPDATE error_data = new.error_data;

-- ============================================================================
-- Final Data Validation
-- ============================================================================

-- Insert a summary record to validate seeding completion
INSERT INTO test_basic (item_name, table_value)
VALUES ('seeding_complete', 1) AS new
ON DUPLICATE KEY UPDATE table_value = new.table_value;
