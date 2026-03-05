# Type Safety and Data Conversion

Gold Digger handles MySQL data types safely without panicking on NULL values or type mismatches.

## Safe Type Handling

### Automatic Type Conversion

Gold Digger automatically converts all MySQL data types to string representations:

- **NULL values** → Empty strings (`""`)
- **Integers** → String representation (`42` → `"42"`)
- **Floats/Doubles** → String representation (`3.14` → `"3.14"`)
- **Dates/Times** → ISO format strings (`2023-12-25 14:30:45.123456`)
- **Binary data** → UTF-8 string or hex encoding (`0x...`) for invalid UTF-8

### Special Value Handling

Gold Digger handles special floating-point values:

- **NaN** → `"NaN"`
- **Positive Infinity** → `"Infinity"`
- **Negative Infinity** → `"-Infinity"`

### JSON Output Type Preservation

When outputting to JSON format, Gold Digger preserves native MySQL types:

```json,ignore
{
  "data": [
    {
      "id": 123,           // Integer preserved as JSON number
      "price": 19.99,      // Float preserved as JSON number
      "name": "Product",   // String preserved
      "active": 1,          // Boolean-like TINYINT(1) preserved as JSON number (0/1)
      "description": null  // NULL preserved as JSON null
    }
  ]
}
```

Dates and times are formatted as ISO-8601 strings in JSON output, with `T` separator for datetimes (`2023-12-25T14:30:45.123456`).

## Common Type Issues

### NULL Value Handling

**Problem**: Database contains NULL values

**Solution**: Gold Digger handles NULLs automatically:

- **CSV/TSV**: NULL becomes empty string
- **JSON**: NULL becomes `null` value

```sql
-- This query works safely with NULLs
SELECT id, name, description FROM products WHERE id <= 10;
```

### Mixed Data Types

**Problem**: Column contains mixed data types

**Solution**: All values are converted to strings safely:

```sql
-- This works even if 'value' column has mixed types
SELECT id, value FROM mixed_data_table;
```

### Binary Data

**Problem**: Column contains binary data (BLOB, BINARY)

**Solution**: Binary data is converted to UTF-8 if valid, otherwise hex-encoded:

```sql
-- Binary columns are handled safely
SELECT id, binary_data FROM files;
```

Invalid UTF-8 bytes are encoded as `0x<hexstring>` (e.g., `0xfffefd`) to prevent data corruption. Large binary data (>1024 bytes) is truncated with indication: `0x<prefix>... (N bytes)`.

### Date and Time Formats

**Problem**: Need consistent date formatting

**Solution**: Gold Digger uses ISO format for all date/time values:

```sql
-- Date/time columns are formatted consistently
SELECT created_at, updated_at FROM events;
```

Output format:

- **Date only**: `2023-12-25`
- **DateTime**: `2023-12-25 14:30:45.123456`
- **Time only**: `14:30:45.123456`

## Best Practices

### Query Writing

1. **No casting required**: Unlike previous versions, you don't need to cast columns to CHAR
2. **Use appropriate data types**: Let MySQL handle the data types naturally
3. **Handle NULLs in SQL if needed**: Use `COALESCE()` or `IFNULL()` for custom NULL handling

```sql
-- Good: Let Gold Digger handle type conversion
SELECT id, name, price, created_at FROM products;

-- Also good: Custom NULL handling in SQL
SELECT id, COALESCE(name, 'Unknown') as name FROM products;
```

### Output Format Selection

Choose the appropriate output format based on your needs:

- **CSV**: Best for spreadsheet import, preserves all data as strings
- **JSON**: Best for APIs, preserves data types where possible
- **TSV**: Best for tab-delimited processing, similar to CSV

### Error Prevention

Gold Digger's safe type handling prevents common errors:

- **No panics on NULL values**
- **No crashes on type mismatches**
- **Graceful handling of special values (NaN, Infinity)**
- **Safe binary data conversion**

## Migration from Previous Versions

### Removing CAST Statements

If you have queries with explicit casting from previous versions:

```sql
-- Old approach (still works but unnecessary)
SELECT CAST(id AS CHAR) as id, CAST(name AS CHAR) as name FROM users;

-- New approach (recommended)
SELECT id, name FROM users;
```

### Handling Type-Specific Requirements

If you need specific type handling, use SQL functions:

```sql
-- Format numbers with specific precision
SELECT id, ROUND(price, 2) as price FROM products;

-- Format dates in specific format
SELECT id, DATE_FORMAT(created_at, '%Y-%m-%d') as created_date FROM events;

-- Handle NULLs with custom values
SELECT id, COALESCE(description, 'No description') as description FROM items;
```

## Troubleshooting Type Issues

### Unexpected Output Format

**Issue**: Numbers appearing as strings in JSON

**Cause**: Value contains non-numeric characters or formatting

**Solution**: Clean the data in SQL:

```sql
SELECT id, CAST(TRIM(price_string) AS DECIMAL(10,2)) as price FROM products;
```

### Binary Data Display Issues

**Issue**: Binary data showing as garbled text

**Cause**: Binary column being converted to string

**Solution**: Gold Digger automatically hex-encodes invalid UTF-8. For custom formats, use SQL functions:

```sql
-- Convert binary to hex representation
SELECT id, HEX(binary_data) as binary_hex FROM files;

-- Or encode as base64 (MySQL 5.6+)
SELECT id, TO_BASE64(binary_data) as binary_b64 FROM files;
```

### Date Format Consistency

**Issue**: Need different date format

**Solution**: Format dates in SQL:

```sql
-- US format
SELECT id, DATE_FORMAT(created_at, '%m/%d/%Y') as created_date FROM events;

-- European format
SELECT id, DATE_FORMAT(created_at, '%d.%m.%Y') as created_date FROM events;
```

## Performance Considerations

Gold Digger's type conversion uses the `TypeTransformer` API for safety and performance:

- **Zero-copy string conversion** where possible
- **Efficient NULL handling** without allocations
- **Streaming-friendly design** for large result sets
- **Deterministic hex encoding** for binary data

The `TypeTransformer` implementation in `src/type_transformer.rs` provides the canonical safe MySQL value conversion, preventing crashes and data corruption with minimal overhead.

## TypeTransformer API Reference

Developers extending Gold Digger should use the `TypeTransformer` API:

```rust,ignore
use std::collections::BTreeMap;
use gold_digger::TypeTransformer;

// Convert single value to string (CSV/TSV):
let s: String = TypeTransformer::value_to_string(&value)?;

// Convert single value to JSON:
let json_value: serde_json::Value = TypeTransformer::value_to_json(&value)?;

// Convert full row to Vec<String>:
let strings: Vec<String> = TypeTransformer::row_to_strings(row)?;

// Convert full row to JSON map (deterministic ordering):
let map: BTreeMap<String, serde_json::Value> = TypeTransformer::row_to_json(row)?;
```

See `src/type_transformer.rs` for implementation details and safety guarantees.
