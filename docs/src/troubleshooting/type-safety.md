# Type Safety and Data Conversion

Gold Digger handles MySQL data types safely without panicking on NULL values or type mismatches.

## Safe Type Handling

### Automatic Type Conversion

Gold Digger automatically converts all MySQL data types to string representations:

- **NULL values** → Empty strings (`""`)
- **Integers** → String representation (`42` → `"42"`)
- **Floats/Doubles** → String representation (`3.14` → `"3.14"`)
- **Dates/Times** → ISO format strings (`2023-12-25 14:30:45.123456`)
- **Binary data** → UTF-8 conversion (with lossy conversion for invalid UTF-8)

### Special Value Handling

Gold Digger handles special floating-point values:

- **NaN** → `"NaN"`
- **Positive Infinity** → `"Infinity"`
- **Negative Infinity** → `"-Infinity"`

### JSON Output Type Inference

When outputting to JSON format, Gold Digger attempts to preserve data types:

```json,ignore
{
  "data": [
    {
      "id": 123,           // Integer preserved
      "price": 19.99,      // Float preserved
      "name": "Product",   // String preserved
      "active": true,      // Boolean preserved
      "description": null  // NULL preserved as JSON null
    }
  ]
}
```

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

**Solution**: Binary data is converted to UTF-8 with lossy conversion:

```sql
-- Binary columns are handled safely
SELECT id, binary_data FROM files;
```

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

**Solution**: Use SQL functions to handle binary data:

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

Gold Digger's type conversion is optimized for safety and performance:

- **Zero-copy string conversion** where possible
- **Efficient NULL handling** without allocations
- **Streaming-friendly design** for large result sets
- **Memory-efficient** binary data handling

The safe type handling adds minimal overhead while preventing crashes and data corruption.
