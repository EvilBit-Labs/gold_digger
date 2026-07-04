# Type Errors

Solutions for data type conversion and NULL handling issues.

> [!NOTE]
> TypeTransformer provides safe MySQL value conversion in gold_digger. NULL values and type mismatches no longer cause panics -- all MySQL value types are handled gracefully. The guidance below covers historical issues and edge cases.

## Common Type Conversion Errors

### NULL Value Panics (Historical Issue)

**Problem**: Before the TypeTransformer implementation, gold_digger could crash with a panic when encountering NULL values.

**Error Message** (historical):

```console
thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value'
```

**Current Status**: TypeTransformer handles NULL values gracefully:

- `TypeTransformer::value_to_string` converts NULL to an empty string (for CSV/TSV output)
- `TypeTransformer::value_to_json` converts NULL to JSON `null` (for JSON output)

No CAST workarounds are required. You can query columns directly:

```sql
-- ✅ Safe with TypeTransformer
SELECT id, name, created_at FROM users;
```

### Type Conversion Safety

**Historical Problem**: Numeric, date, or binary columns could cause conversion errors.

**Current Status**: TypeTransformer safely handles all MySQL value types:

- Integers (`INT`, `BIGINT`, `UNSIGNED`) convert to numbers (JSON) or strings (CSV/TSV)
- Floats and doubles handle special values (`NaN`, `Infinity`) as strings
- Date and time values format to ISO-8601 strings
- Binary data (BLOBs) that is not valid UTF-8 is hex-encoded
- NULL values convert to empty strings (CSV/TSV) or JSON `null` (JSON)

Example query requiring no special handling:

```sql
SELECT
  user_id,       -- numeric type
  username,      -- string type
  balance,       -- decimal type
  last_login,    -- datetime type
  is_active      -- boolean type
FROM accounts;
```

## Best Practices for Type Safety

### TypeTransformer Handles Type Conversion

TypeTransformer (used automatically by gold_digger's output modules) provides safe, panic-free conversion for all MySQL value types. The output modules (`csv::write`, `json::write`, `tab::write`) delegate to TypeTransformer for value conversion.

You do not need defensive CAST statements in most cases.

### When to Use CAST (Optional)

CAST remains useful for specific scenarios:

1. **Explicit type consistency**: Ensure a column has the same data type across all result rows.
2. **Backward compatibility**: Maintain compatibility with existing scripts that expect CAST.
3. **Custom formatting**: Apply database-side formatting (e.g., `FORMAT()` for decimals, `DATE_FORMAT()` for dates).

Example of optional CAST usage:

```sql
-- Optional: ensures all values are strings at query time
SELECT
  CAST(price AS CHAR) as price,
  CAST(quantity AS CHAR) as quantity
FROM products;
```

TypeTransformer will handle the result either way.

### Handle NULL Values (Optional)

TypeTransformer converts NULL values to empty strings (CSV/TSV) or JSON `null` (JSON) automatically. You can use `COALESCE` if you need custom default values:

```sql
-- Optional: provide custom defaults for NULL values
SELECT
  CAST(COALESCE(phone, 'N/A') AS CHAR) as phone,
  CAST(COALESCE(address, 'No address') AS CHAR) as address
FROM contacts;
```

### Test Queries First

Before running large exports, test with a small subset:

```sql
-- Test with LIMIT first
SELECT id, name, created_at
FROM large_table
LIMIT 5;
```

## Output Format Considerations

TypeTransformer handles NULL values and type conversion differently based on the output format.

### JSON Output

NULL values appear as JSON `null`:

```json
{
  "data": [
    {
      "id": "1",
      "name": "John",
      "phone": null
    }
  ]
}
```

Numeric values appear as JSON numbers:

```json
{
  "data": [
    {
      "id": 1,
      "balance": 123.45
    }
  ]
}
```

### CSV/TSV Output

NULL values appear as empty strings:

```csv
id,name,phone
1,John,
2,Jane,555-1234
```

All values are converted to strings for CSV/TSV output.

## Debugging Type Issues

### Check Error Messages

TypeTransformer validates date and time components. If you encounter errors, they will reference specific invalid values:

```console
Type conversion error: Invalid month value 13 in date
Type conversion error: Invalid hour value 25 in datetime
```

These errors indicate data quality issues in the source database that require attention.

### Check Column Types

Query your database schema to understand column types:

```sql
DESCRIBE your_table;
-- or
SHOW COLUMNS FROM your_table;
```

### Use Information Schema

```sql
SELECT
  COLUMN_NAME,
  DATA_TYPE,
  IS_NULLABLE
FROM INFORMATION_SCHEMA.COLUMNS
WHERE TABLE_NAME = 'your_table';
```

### Enable Verbose Output

Use `--verbose` to see detailed query execution information:

```bash
gold_digger --verbose \
  --db-url "mysql://..." \
  --query "SELECT ..." \
  --output debug.json
```

## Advanced Type Handling

### Custom Formatting (Database-Side)

Use database functions for custom formatting at query time:

```sql
-- Format numbers with specific precision
SELECT
  FORMAT(price, 2) as price,
  FORMAT(tax_rate, 4) as tax_rate
FROM products;
```

### Date Formatting (Database-Side)

```sql
-- Custom date formats
SELECT
  DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s') as created_at,
  DATE_FORMAT(updated_at, '%Y-%m-%d') as updated_date
FROM records;
```

TypeTransformer will convert the formatted result to the appropriate output type.

## When to Contact Support

If you encounter type conversion errors despite TypeTransformer's safety guarantees:

1. Provide the exact SQL query causing issues
2. Include the table schema (`DESCRIBE table_name`)
3. Share the complete error message
4. Specify which output format you're using (`--format json`, `--format csv`, `--format tsv`)
5. Note the MySQL server version and any non-standard data types

TypeTransformer handles all standard MySQL value types. Errors typically indicate:

- Invalid date/time component values in the database (e.g., month > 12)
- Custom or vendor-specific MySQL extensions not covered by the `mysql` crate
