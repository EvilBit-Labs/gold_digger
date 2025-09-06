# Output Formats

Gold Digger supports three structured output formats: CSV, JSON, and TSV.

## Format Selection

Format is determined by this priority order:

1. **`--format` flag** (explicit override)
2. **File extension** in output path
3. **TSV as fallback** for unknown extensions

### Examples

```bash
# Format determined by extension
gold_digger --output data.csv    # CSV format
gold_digger --output data.json   # JSON format
gold_digger --output data.tsv    # TSV format

# Explicit format override
gold_digger --output data.txt --format json  # JSON despite .txt extension
```

## CSV Format

**Comma-Separated Values** - Industry standard tabular format.

### Specifications

- **Standard**: RFC4180-compliant
- **Quoting**: `QuoteStyle::Necessary` (only when required)
- **Line Endings**: CRLF (`\r\n`)
- **NULL Handling**: Empty strings
- **Encoding**: UTF-8

### Example Output

```csv
id,name,email,created_at
1,John Doe,john@example.com,2024-01-15 10:30:00
2,"Smith, Jane",jane@example.com,2024-01-16 14:22:33
3,Bob Johnson,,2024-01-17 09:15:45
```

### When to Use CSV

- **Excel compatibility** required
- **Data analysis** in spreadsheet applications
- **Legacy systems** expecting CSV input
- **Minimal file size** for large datasets

### CSV Quoting Rules

Fields are quoted only when they contain:

- Commas (`,`)
- Double quotes (`"`)
- Newlines (`\n` or `\r\n`)

## JSON Format

**JavaScript Object Notation** - Structured data format with rich type support.

### Specifications

- **Structure**: `{"data": [...]}`
- **Key Ordering**: Deterministic (BTreeMap, not HashMap)
- **NULL Handling**: JSON `null` values
- **Encoding**: UTF-8
- **Pretty Printing**: Optional with `--pretty` flag

### Example Output

**Compact (default):**

```json
{
  "data": [
    {
      "id": "1",
      "name": "John Doe",
      "email": "john@example.com",
      "created_at": "2024-01-15 10:30:00"
    },
    {
      "id": "2",
      "name": "Jane Smith",
      "email": "jane@example.com",
      "created_at": "2024-01-16 14:22:33"
    }
  ]
}
```

**Pretty-printed (`--pretty`):**

```json
{
  "data": [
    {
      "created_at": "2024-01-15 10:30:00",
      "email": "john@example.com",
      "id": "1",
      "name": "John Doe"
    },
    {
      "created_at": "2024-01-16 14:22:33",
      "email": "jane@example.com",
      "id": "2",
      "name": "Jane Smith"
    }
  ]
}
```

### When to Use JSON

- **API integration** and web services
- **Complex data structures** with nested objects
- **Type preservation** (though Gold Digger converts all to strings)
- **Modern applications** expecting JSON input

### JSON Features

- **Deterministic ordering**: Keys are always in the same order
- **NULL safety**: Database NULL values become JSON `null`
- **Unicode support**: Full UTF-8 character support

## TSV Format

**Tab-Separated Values** - Simple, reliable format for data exchange.

### Specifications

- **Delimiter**: Tab character (`\t`)
- **Quoting**: `QuoteStyle::Necessary`
- **Line Endings**: Unix (`\n`)
- **NULL Handling**: Empty strings
- **Encoding**: UTF-8

### Example Output

```tsv
id  name email created_at
1   John Doe john@example.com 2024-01-15 10:30:00
2   Jane Smith jane@example.com 2024-01-16 14:22:33
3 Bob Johnson  2024-01-17 09:15:45
```

### When to Use TSV

- **Unix/Linux tools** (awk, cut, sort)
- **Data processing pipelines**
- **Avoiding comma conflicts** in data
- **Simple parsing** requirements

### TSV Advantages

- **No comma conflicts**: Data can contain commas without quoting
- **Simple parsing**: Easy to split on tab characters
- **Unix-friendly**: Works well with command-line tools

## NULL Value Handling

Different formats handle database NULL values differently:

| Format | NULL Representation | Example                                 |
|--------|---------------------|-----------------------------------------|
| CSV    | Empty string        | `1,John,,2024-01-15`                    |
| JSON   | JSON `null`         | `{"id":"1","name":"John","email":null}` |
| TSV    | Empty string        | `1 John  2024-01-15`                    |

## Type Safety and Data Conversion

Gold Digger automatically handles all MySQL data types safely without requiring explicit casting.

### Automatic Type Conversion

All MySQL data types are converted safely:

```sql
-- ✅ Safe - Gold Digger handles all types automatically
SELECT id, name, price, created_at, is_active, description
FROM products;
```

### Type Conversion Rules

| MySQL Type         | CSV/TSV Output        | JSON Output                        | NULL Handling         |
|--------------------|-----------------------|------------------------------------|-----------------------|
| `INT`, `BIGINT`    | String representation | Number (if valid)                  | Empty string / `null` |
| `DECIMAL`, `FLOAT` | String representation | Number (if valid)                  | Empty string / `null` |
| `VARCHAR`, `TEXT`  | Direct string         | String                             | Empty string / `null` |
| `DATE`, `DATETIME` | ISO format string     | String                             | Empty string / `null` |
| `BOOLEAN`          | "0" or "1"            | `true`/`false` (if "true"/"false") | Empty string / `null` |
| `NULL`             | Empty string          | `null`                             | Always handled safely |

### JSON Type Inference

When outputting to JSON, Gold Digger attempts to preserve appropriate data types:

```json
{
  "data": [
    {
      "id": 123,           // Integer preserved
      "price": 19.99,      // Float preserved  
      "name": "Product",   // String preserved
      "active": true,      // Boolean inferred
      "description": null  // NULL preserved
    }
  ]
}
```

## Performance Considerations

### File Size Comparison

For the same dataset:

1. **TSV**: Smallest (no quotes, simple delimiters)
2. **CSV**: Medium (quotes when necessary)
3. **JSON**: Largest (structure overhead, key names repeated)

### Processing Speed

1. **TSV**: Fastest to generate and parse
2. **CSV**: Fast, with quoting overhead
3. **JSON**: Slower due to structure and key ordering

## Format-Specific Options

### CSV Options

```bash
# Standard CSV
gold_digger --output data.csv

# CSV is always RFC4180-compliant with necessary quoting
```

### JSON Options

```bash
# Compact JSON (default)
gold_digger --output data.json

# Pretty-printed JSON
gold_digger --output data.json --pretty
```

### TSV Options

```bash
# Standard TSV
gold_digger --output data.tsv

# TSV with explicit format
gold_digger --output data.txt --format tsv
```

## Integration Examples

### Excel Integration

```bash
# Generate Excel-compatible CSV
gold_digger \
  --query "SELECT CAST(id AS CHAR) as ID, name as Name FROM users" \
  --output users.csv
```

### API Integration

```bash
# Generate JSON for API consumption
gold_digger \
  --query "SELECT CAST(id AS CHAR) as id, name, email FROM users" \
  --output users.json \
  --pretty
```

### Unix Pipeline Integration

```bash
# Generate TSV for command-line processing
gold_digger \
  --query "SELECT CAST(id AS CHAR) as id, name FROM users" \
  --output users.tsv

# Process with standard Unix tools
cut -f2 users.tsv | sort | uniq -c
```

## Troubleshooting Output Formats

### Common Issues

**Malformed CSV:**

- Check for unescaped quotes in data
- Verify line ending compatibility

**Invalid JSON:**

- Ensure all columns are properly cast
- Check for NULL handling issues

**TSV parsing errors:**

- Look for tab characters in data
- Verify delimiter expectations

### Validation

Test output format validity:

```bash
# Validate CSV
csvlint data.csv

# Validate JSON
jq . data.json

# Check TSV structure
column -t -s $'\t' data.tsv | head
```
