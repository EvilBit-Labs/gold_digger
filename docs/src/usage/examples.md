# Examples

Practical examples for common Gold Digger use cases.

## Basic Data Export

### Simple User Export

```bash
gold_digger \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT id, name, email FROM users LIMIT 100" \
  --output users.csv
```

### Pretty JSON Output

```bash
gold_digger \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT id, name, email FROM users LIMIT 10" \
  --output users.json \
  --pretty
```

## Complex Queries

### Joins and Aggregations

```bash
gold_digger \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT u.name, COUNT(p.id) as post_count
           FROM users u LEFT JOIN posts p ON u.id = p.user_id
           WHERE u.active = 1
           GROUP BY u.id, u.name
           ORDER BY post_count DESC" \
  --output user_stats.json
```

### Date Range Queries

```bash
gold_digger \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT DATE(created_at) as date, COUNT(*) as orders
           FROM orders
           WHERE created_at >= '2023-01-01'
           GROUP BY DATE(created_at)
           ORDER BY date" \
  --output daily_orders.csv
```

## Using Query Files

### Complex Query from File

Create a query file:

```sql
-- analytics_query.sql
SELECT
    p.category,
    COUNT(*) as product_count,
    AVG(p.price) as avg_price,
    SUM(oi.quantity) as total_sold
FROM products p
LEFT JOIN order_items oi ON p.id = oi.product_id
LEFT JOIN orders o ON oi.order_id = o.id
WHERE o.created_at >= DATE_SUB(NOW(), INTERVAL 30 DAY)
GROUP BY p.category
ORDER BY total_sold DESC;
```

Use the query file:

```bash
gold_digger \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query-file analytics_query.sql \
  --output monthly_analytics.json \
  --pretty
```

## Environment Variables

### Basic Environment Setup

```bash
export DATABASE_URL="mysql://user:pass@localhost:3306/mydb"
export DATABASE_QUERY="SELECT * FROM products WHERE price > 100"
export OUTPUT_FILE="expensive_products.json"

gold_digger
```

### Windows PowerShell

```powershell
$env:DATABASE_URL="mysql://user:pass@localhost:3306/mydb"
$env:DATABASE_QUERY="SELECT id, name, price FROM products WHERE active = 1"
$env:OUTPUT_FILE="C:\data\active_products.csv"

gold_digger
```

## Output Format Control

### Force Specific Format

```bash
# Force CSV format regardless of file extension
gold_digger \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT * FROM users" \
  --output data.txt \
  --format csv
```

### Format Comparison

```bash
# CSV output (RFC 4180 compliant)
gold_digger --query "SELECT id, name FROM users LIMIT 5" --output users.csv

# JSON output with type inference
gold_digger --query "SELECT id, name FROM users LIMIT 5" --output users.json

# TSV output (tab-separated)
gold_digger --query "SELECT id, name FROM users LIMIT 5" --output users.tsv
```

## Error Handling and Debugging

### Handle Empty Results

```bash
# Exit with code 0 even if no results (default exits with code 1)
# The --allow-empty flag changes the command's behavior by permitting empty result sets
# and creating an empty output file instead of exiting with error code 1
gold_digger \
  --allow-empty \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT * FROM users WHERE id = 999999" \
  --output empty_result.json
```

### Verbose Logging

```bash
# Enable verbose output for debugging
gold_digger -v \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT COUNT(*) as total FROM large_table" \
  --output count.json
```

### Configuration Debugging

```bash
# Check resolved configuration (credentials redacted)
gold_digger \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT 1 as test" \
  --output test.json \
  --dump-config
```

## Data Type Handling

### Automatic Type Conversion

Gold Digger safely handles all MySQL data types without requiring explicit casting:

```bash
# All data types handled automatically
gold_digger \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT id, name, price, created_at, is_active, description
           FROM products" \
  --output products.json
```

### NULL Value Handling

```bash
# NULL values are handled safely
gold_digger \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT id, name, COALESCE(description, 'No description') as description
           FROM products" \
  --output products_with_defaults.csv
```

### Special Values

```bash
# Handles NaN, Infinity, and other special values
gold_digger \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT id, name,
           CASE WHEN price = 0 THEN 'NaN' ELSE price END as price
           FROM products" \
  --output products_special.json
```

## Automation and Scripting

### Bash Script Example

```bash
#!/bin/bash
set -e

DB_URL="mysql://user:pass@localhost:3306/mydb"
OUTPUT_DIR="/data/exports"
DATE=$(date +%Y%m%d)

# Export users
gold_digger \
  --db-url "$DB_URL" \
  --query "SELECT * FROM users WHERE active = 1" \
  --output "$OUTPUT_DIR/users_$DATE.csv"

# Export orders
gold_digger \
  --db-url "$DB_URL" \
  --query "SELECT * FROM orders WHERE DATE(created_at) = CURDATE()" \
  --output "$OUTPUT_DIR/daily_orders_$DATE.json" \
  --pretty

echo "Export completed successfully"
```

### Error Handling in Scripts

```bash
#!/bin/bash

DB_URL="mysql://user:pass@localhost:3306/mydb"
QUERY="SELECT COUNT(*) as count FROM users"
OUTPUT="user_count.json"

if gold_digger --db-url "$DB_URL" --query "$QUERY" --output "$OUTPUT"; then
    echo "Export successful"
    cat "$OUTPUT"
else
    case $? in
        1) echo "No results found" ;;
        2) echo "Configuration error" ;;
        3) echo "Database connection failed" ;;
        4) echo "Query execution failed" ;;
        5) echo "File I/O error" ;;
        *) echo "Unknown error" ;;
    esac
    exit 1
fi
```

## Performance Optimization

### Large Dataset Export

```bash
# For large datasets, use LIMIT and OFFSET for pagination
gold_digger \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT * FROM large_table ORDER BY id LIMIT 10000 OFFSET 0" \
  --output batch_1.csv

gold_digger \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT * FROM large_table ORDER BY id LIMIT 10000 OFFSET 10000" \
  --output batch_2.csv
```

### Optimized Queries

```bash
# Use indexes and specific columns for better performance
gold_digger \
  --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT id, name, email FROM users
           WHERE created_at >= '2023-01-01'
           AND status = 'active'
           ORDER BY id" \
  --output recent_active_users.json
```

## TLS/SSL Connections

### Secure Connection (Default)

```bash
# Uses platform certificate store for validation
gold_digger \
  --db-url "mysql://user:pass@secure-db.example.com:3306/mydb" \
  --query "SELECT id, name FROM users LIMIT 10" \
  --output secure_users.json
```

### Custom CA Certificate

```bash
# Use custom CA certificate for internal infrastructure
gold_digger \
  --db-url "mysql://user:pass@internal-db.company.com:3306/mydb" \
  --tls-ca-file /etc/ssl/certs/company-ca.pem \
  --query "SELECT * FROM sensitive_data" \
  --output internal_data.csv
```

### Development Environment

```bash
# Skip hostname verification for development servers
gold_digger \
  --db-url "mysql://dev:devpass@192.168.1.100:3306/dev_db" \
  --insecure-skip-hostname-verify \
  --query "SELECT * FROM test_data" \
  --output dev_data.json
```

### Testing Environment (DANGEROUS)

```bash
# Accept invalid certificates for testing only
gold_digger \
  --db-url "mysql://test:test@test-db:3306/test" \
  --allow-invalid-certificate \
  --query "SELECT COUNT(*) as total FROM test_table" \
  --output test_count.json
```

**⚠️ Security Warning**: Never use `--allow-invalid-certificate` in production environments.

## Shell Completion

### Setup Completion

```bash
# Bash
gold_digger completion bash > ~/.bash_completion.d/gold_digger
source ~/.bash_completion.d/gold_digger

# Zsh
gold_digger completion zsh > ~/.zsh/completions/_gold_digger

# Fish
gold_digger completion fish > ~/.config/fish/completions/gold_digger.fish

# PowerShell
gold_digger completion powershell > gold_digger.ps1
```

### Using Completion

After setup, you can use tab completion:

```bash
gold_digger --<TAB>        # Shows available flags
gold_digger --format <TAB> # Shows format options (csv, json, tsv)
gold_digger completion <TAB> # Shows shell options
```
