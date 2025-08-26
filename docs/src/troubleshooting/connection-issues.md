# Connection Problems

Troubleshooting database connection issues with Gold Digger's enhanced error handling.

## Gold Digger Connection Error Codes

Gold Digger provides structured exit codes for different connection failures:

- **Exit Code 2**: Configuration errors (invalid URL format, missing parameters)
- **Exit Code 3**: Connection/authentication failures (network, credentials, TLS)

## Common Connection Errors

### Connection Refused (Exit Code 3)

**Error Message:**

```text
Database connection failed: Connection refused. Check server availability and network connectivity
```

**Causes & Solutions:**

- **Database server not running**: Start MySQL/MariaDB service
- **Wrong port**: Verify port number (default: 3306)
- **Firewall blocking**: Check firewall rules on both client and server
- **Network issues**: Test basic network connectivity

**Diagnostic Steps:**

```bash
# Test network connectivity
telnet hostname 3306

# Check if service is running
systemctl status mysql  # Linux
brew services list | grep mysql  # macOS
```

### Access Denied (Exit Code 3)

**Error Message:**

```text
Database authentication failed: Access denied for user 'username'@'host'. Check username and password
```

**Causes & Solutions:**

- **Invalid credentials**: Verify username and password
- **Insufficient permissions**: Grant appropriate database permissions
- **Host restrictions**: Check MySQL user host permissions
- **Account locked/expired**: Verify account status

**Diagnostic Steps:**

```bash
# Test credentials manually
mysql -h hostname -P port -u username -p database

# Check user permissions
SHOW GRANTS FOR 'username'@'host';
```

### Unknown Database (Exit Code 3)

**Error Message:**

```text
Database connection failed: Unknown database 'dbname'
```

**Causes & Solutions:**

- **Database doesn't exist**: Create database or verify name
- **Typo in database name**: Check spelling and case sensitivity
- **No access to database**: Grant permissions to the database

**Diagnostic Steps:**

```bash
# List available databases
SHOW DATABASES;

# Create database if needed
CREATE DATABASE dbname;
```

### Invalid URL Format (Exit Code 2)

**Error Message:**

```text
Invalid database URL format: Invalid URL scheme. URL: ***REDACTED***
```

**Causes & Solutions:**

- **Wrong URL scheme**: Use `mysql://` not `http://` or others
- **Missing components**: Ensure format is `mysql://user:pass@host:port/db`
- **Special characters**: URL-encode special characters in passwords

**Correct Format:**

```text
mysql://username:password@hostname:port/database
```

## TLS/SSL Connection Issues

### TLS Connection Issues (Exit Code 3)

**Error Message:**

```text
TLS handshake failed: protocol version mismatch
```

**Solutions:**

```bash
# TLS support is always available - check server configuration
# Verify server supports TLS 1.2 or higher
# Check certificate validity and chain
```

### Certificate Validation Failed (Exit Code 3)

**Error Message:**

```text
Certificate validation failed: unable to get local issuer certificate.
Check certificate chain, expiration, and CA configuration
```

**Causes & Solutions:**

- **Missing CA certificates**: Install system CA certificates
- **Self-signed certificates**: Add CA to system trust store
- **Expired certificates**: Renew server certificates
- **Certificate chain issues**: Verify complete certificate chain
- **Testing environments**: Use `ssl-mode=preferred` or `ssl-mode=disabled` in connection URL (not recommended for production)

### TLS Handshake Failed (Exit Code 3)

**Error Message:**

```text
TLS handshake failed: protocol version mismatch.
Check server TLS configuration and certificate validity
```

**Causes & Solutions:**

- **TLS version mismatch**: Ensure server supports TLS 1.2+
- **Cipher suite incompatibility**: Update server TLS configuration
- **Certificate issues**: Verify certificate validity and chain

### Unsupported TLS Version (Exit Code 3)

**Error Message:**

```text
Unsupported TLS version: 1.0. Only TLS 1.2 and 1.3 are supported
```

**Solution:**

- Upgrade database server to support TLS 1.2 or 1.3
- Update server TLS configuration

## Network Troubleshooting

### Firewall Issues

**Symptoms:**

- Connection timeouts
- "Connection refused" errors
- Intermittent connectivity

**Solutions:**

```bash
# Check if port is open (Linux/macOS)
nmap -p 3306 hostname

# Test connectivity
telnet hostname 3306

# Check local firewall (Linux)
sudo ufw status
sudo iptables -L

# Check local firewall (macOS)
sudo pfctl -sr
```

### DNS Resolution Issues

**Symptoms:**

- "Host not found" errors
- Connection works with IP but not hostname

**Solutions:**

```bash
# Test DNS resolution
nslookup hostname
dig hostname

# Try IP address directly
gold_digger --db-url "mysql://user:pass@192.168.1.100:3306/db" --query "SELECT 1" --output test.json
```

## Debugging Connection Issues

### Enable Verbose Logging

```bash
gold_digger -v \
  --db-url "mysql://user:pass@host:3306/db" \
  --query "SELECT 1" \
  --output debug.json
```

**Verbose Output Example:**

```text
Connecting to database...
Database connection failed: Connection refused
```

### Configuration Debugging

```bash
# Check resolved configuration (credentials redacted)
gold_digger \
  --db-url "mysql://user:pass@host:3306/db" \
  --query "SELECT 1" \
  --output test.json \
  --dump-config
```

### Test with Minimal Query

```bash
# Use simple test query
gold_digger \
  --db-url "mysql://user:pass@host:3306/db" \
  --query "SELECT 1 as test" \
  --output connection_test.json
```

## Advanced Diagnostics

### MySQL Error Code Mapping

Gold Digger maps specific MySQL error codes to contextual messages:

| MySQL Error              | Code | Gold Digger Message                       |
| ------------------------ | ---- | ----------------------------------------- |
| ER_ACCESS_DENIED_ERROR   | 1045 | Access denied - invalid credentials       |
| ER_DBACCESS_DENIED_ERROR | 1044 | Access denied to database                 |
| ER_BAD_DB_ERROR          | 1049 | Unknown database                          |
| CR_CONNECTION_ERROR      | 2002 | Connection failed - server not reachable  |
| CR_CONN_HOST_ERROR       | 2003 | Connection failed - server not responding |
| CR_SERVER_GONE_ERROR     | 2006 | Connection lost - server has gone away    |

### Connection Pool Issues

**Symptoms:**

- Intermittent connection failures
- "Too many connections" errors

**Solutions:**

- Check MySQL `max_connections` setting
- Monitor active connection count
- Verify connection pool configuration

### Performance-Related Connection Issues

**Symptoms:**

- Slow connection establishment
- Timeouts on large queries

**Solutions:**

```bash
# Test with smaller query first
gold_digger \
  --db-url "mysql://user:pass@host:3306/db" \
  --query "SELECT COUNT(*) FROM table LIMIT 1" \
  --output count_test.json

# Check server performance
SHOW PROCESSLIST;
SHOW STATUS LIKE 'Threads_connected';
```

## Getting Help

When reporting connection issues, include:

1. **Complete error message** (credentials will be automatically redacted)
2. **Gold Digger version**: `gold_digger --version`
3. **Database server version**: `SELECT VERSION();`
4. **Connection string format** (without credentials)
5. **Network environment** (local, remote, cloud, etc.)
6. **TLS requirements** and certificate setup

**Example Issue Report:**

```text
Gold Digger Version: 0.2.6
Database: MySQL 8.0.35
Error: Certificate validation failed: self signed certificate
Connection: mysql://user:***@remote-db.example.com:3306/production
Environment: Connecting from local machine to cloud database
TLS: Required, using self-signed certificates
```
