# Type Errors

Solutions for data type conversion and NULL handling issues.

## Common Type Conversion Errors

### NULL Value Panics

**Problem**: Gold Digger crashes with a panic when encountering NULL values.

**Error Message**:

```console
thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value'
```

**Solution**: Always cast columns to CHAR in your SQL queries:

```sql
-- ❌ Dangerous - can panic on NULL
SELECT id, name, created_at FROM users;

-- ✅ Safe - handles NULL values properly
SELECT
  CAST(id AS CHAR) as id,
  CAST(name AS CHAR) as name,
  CAST(created_at AS CHAR) as created_at
FROM users;
```

### Non-String Type Errors

**Problem**: Numeric, date, or binary columns cause conversion errors.

**Solution**: Cast all non-string columns:

```sql
SELECT
  CAST(user_id AS CHAR) as user_id,
  username,  -- Already string, no cast needed
  CAST(balance AS CHAR) as balance,
  CAST(last_login AS CHAR) as last_login,
  CAST(is_active AS CHAR) as is_active
FROM accounts;
```

## Best Practices for Type Safety

### 1. Always Use CAST

```sql
-- For all numeric types
CAST(price AS CHAR) as price,
CAST(quantity AS CHAR) as quantity,

-- For dates and timestamps
CAST(created_at AS CHAR) as created_at,
CAST(updated_at AS CHAR) as updated_at,

-- For boolean values
CAST(is_enabled AS CHAR) as is_enabled
```

### 2. Handle NULL Values Explicitly

```sql
-- Use COALESCE for default values
SELECT
  CAST(COALESCE(phone, '') AS CHAR) as phone,
  CAST(COALESCE(address, 'No address') AS CHAR) as address
FROM contacts;
```

### 3. Test Queries First

Before running large exports, test with a small subset:

```sql
-- Test with LIMIT first
SELECT CAST(id AS CHAR) as id, name
FROM large_table
LIMIT 5;
```

## Output Format Considerations

### JSON Output

NULL values in JSON output appear as `null`:

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

### CSV/TSV Output

NULL values in CSV/TSV appear as empty strings:

```csv
id,name,phone
1,John,
2,Jane,555-1234
```

## Debugging Type Issues

### Enable Verbose Output

```bash
gold_digger --verbose \
  --db-url "mysql://..." \
  --query "SELECT ..." \
  --output debug.json
```

### Check Column Types

Query your database schema first:

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

## Advanced Type Handling

### Custom Formatting

```sql
-- Format numbers with specific precision
SELECT
  CAST(FORMAT(price, 2) AS CHAR) as price,
  CAST(FORMAT(tax_rate, 4) AS CHAR) as tax_rate
FROM products;
```

### Date Formatting

```sql
-- Custom date formats
SELECT
  CAST(DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s') AS CHAR) as created_at,
  CAST(DATE_FORMAT(updated_at, '%Y-%m-%d') AS CHAR) as updated_date
FROM records;
```

## When to Contact Support

If you continue experiencing type errors after following these guidelines:

1. Provide the exact SQL query causing issues
2. Include the table schema (`DESCRIBE table_name`)
3. Share the complete error message
4. Specify which output format you're using

> [!NOTE]
> The type conversion system is being improved in future versions to handle these cases more
> gracefully.
