# TLS/SSL Configuration

Configure secure connections to your MySQL/MariaDB database with Gold Digger's flexible TLS security
controls.

## TLS Implementation

Gold Digger uses rustls for secure database connections with platform certificate store integration:

- **Pure Rust TLS**: Uses rustls for consistent cross-platform behavior
- **Platform Integration**: Automatically uses system certificate stores (Windows, macOS, Linux)
- **Enhanced Security Controls**: Four distinct TLS security modes via CLI flags
- **Smart Error Messages**: Provides specific guidance when certificate validation fails

## TLS Security Modes

Gold Digger provides four TLS security modes via mutually exclusive CLI flags:

### 1. Platform Trust (Default)

Uses your system's certificate store with full validation:

```bash
# Default behavior - uses platform certificate store
gold_digger \
  --db-url "mysql://user:pass@prod.db:3306/mydb" \
  --query "SELECT * FROM users" \
  --output users.json
```

### 2. Custom CA Trust

Use a custom CA certificate file for trust anchor pinning:

```bash
# Use custom CA certificate
gold_digger \
  --db-url "mysql://user:pass@internal.db:3306/mydb" \
  --tls-ca-file /etc/ssl/certs/internal-ca.pem \
  --query "SELECT * FROM sensitive_data" \
  --output data.json
```

### 3. Skip Hostname Verification

Skip hostname verification while keeping other security checks:

```bash
# Skip hostname verification for development
gold_digger \
  --db-url "mysql://user:pass@192.168.1.100:3306/mydb" \
  --insecure-skip-hostname-verify \
  --query "SELECT * FROM test_data" \
  --output test.json
```

**⚠️ Security Warning**: Displays warning about man-in-the-middle attack vulnerability.

### 4. Accept Invalid Certificates

Disable all certificate validation (DANGEROUS - testing only):

```bash
# Accept any certificate for testing (DANGEROUS)
gold_digger \
  --db-url "mysql://user:pass@test.db:3306/mydb" \
  --allow-invalid-certificate \
  --query "SELECT * FROM test_table" \
  --output test.csv
```

**🚨 Security Warning**: Displays prominent warning about insecure connection.

## TLS Error Handling

Gold Digger provides intelligent error messages with specific CLI flag suggestions:

### Certificate Validation Failures

```text
Error: Certificate validation failed: certificate has expired
Suggestion: Use --allow-invalid-certificate for testing environments
```

**Solutions by error type:**

- **Expired certificates**: Use `--allow-invalid-certificate` (testing only)
- **Self-signed certificates**: Use `--allow-invalid-certificate` or `--tls-ca-file` with custom CA
- **Internal CA certificates**: Use `--tls-ca-file /path/to/internal-ca.pem`

### Hostname Verification Failures

```text
Error: Hostname verification failed for 192.168.1.100: certificate is for db.company.com
Suggestion: Use --insecure-skip-hostname-verify to bypass hostname checks
```

**Common causes:**

- Connecting to servers by IP address
- Certificates with mismatched hostnames
- Development environments with generic certificates

### Custom CA File Issues

```text
Error: CA certificate file not found: /path/to/ca.pem
Solution: Ensure the file exists and is readable

Error: Invalid CA certificate format in /path/to/ca.pem: not valid PEM
Solution: Ensure the file contains valid PEM-encoded certificates
```

### Mutually Exclusive Flag Errors

```text
Error: Mutually exclusive TLS flags provided: --tls-ca-file, --insecure-skip-hostname-verify
Solution: Use only one TLS security option at a time
```

## Security Features

### Credential Protection

Gold Digger automatically protects sensitive information:

```bash
# Credentials are automatically redacted in error messages
gold_digger \
  --db-url "mysql://user:secret@host:3306/db" \
  --query "SELECT 1" \
  --output test.json \
  --dump-config

# Output shows:
# "database_url": "***REDACTED***"
```

### URL Redaction

All database URLs are sanitized in logs and error output:

```text
# Before redaction:
mysql://admin:supersecret@db.example.com:3306/production

# After redaction:
mysql://***REDACTED***:***REDACTED***@db.example.com:3306/production
```

### Error Message Sanitization

TLS error messages are scrubbed of sensitive information:

- Passwords and tokens are replaced with `***REDACTED***`
- Connection strings are sanitized
- Query content with credentials is masked

## Build Configuration

```bash
# Standard build
cargo build --release

# Minimal build
cargo build --release --no-default-features --features "json csv"
```

## Production Recommendations

### Security Best Practices

1. **Always use TLS** for production databases
2. **Verify certificates** - don't skip validation
3. **Use strong passwords** and rotate regularly
4. **Monitor TLS versions** - ensure TLS 1.2+ only
5. **Keep certificates updated** - monitor expiration

### Connection Security

```bash
# ✅ Secure connection example
gold_digger \
  --db-url "mysql://app_user:strong_password@secure-db.example.com:3306/production" \
  --query "SELECT COUNT(*) FROM users" \
  --output user_count.json

# ❌ Avoid insecure connections in production
# mysql://user:pass@insecure-db.example.com:3306/db (no TLS)
```

### Certificate Management

- Use certificates from trusted CAs
- Implement certificate rotation procedures
- Monitor certificate expiration dates
- Test certificate changes in staging first

## Troubleshooting Guide

### Common TLS Issues

#### Issue: TLS connection failed

```text
TLS connection failed: connection timed out
```

**Solutions**:

1. Check network connectivity
2. Verify server is running
3. Confirm firewall allows TLS traffic
4. Verify TLS configuration:
   - Check server certificate validity and chain integrity
   - Confirm server is listening on the TLS port (typically 3306)
   - Validate TLS cipher/protocol compatibility between client and server
   - Test using `openssl s_client -connect host:3306` or `curl --tlsv1.2` to capture handshake
     details
   - Review server logs for TLS-related errors or connection rejections
   - Enable verbose TLS client debug output to diagnose timeouts and handshake failures

#### Issue: Certificate validation failed

```text
Certificate validation failed: self signed certificate in certificate chain
```

**Solutions**:

1. Install proper CA certificates
2. Use certificates from trusted CA
3. For testing only: use `--allow-invalid-certificate` flag
4. For custom CAs: use `--tls-ca-file /path/to/ca.pem`

### Debugging TLS Issues

Enable verbose logging for TLS troubleshooting:

```bash
gold_digger -v \
  --db-url "mysql://user:pass@host:3306/db" \
  --query "SELECT 1" \
  --output debug.json

# Example verbose output
Using platform certificate store for TLS validation
TLS connection established: TLS 1.3, cipher: TLS_AES_256_GCM_SHA384
```

### Security Warnings

Gold Digger displays security warnings for insecure modes:

```bash
# Hostname verification disabled
WARNING: Hostname verification disabled. Connection is vulnerable to man-in-the-middle attacks.

# Certificate validation disabled
WARNING: Certificate validation disabled. Connection is NOT secure.
This should ONLY be used for testing. Never use in production.
```

### Exit Codes for TLS Errors

TLS-related errors map to specific exit codes:

- **Exit 2**: TLS configuration errors (mutually exclusive flags, invalid CA files)
- **Exit 3**: TLS connection failures (handshake, validation, network issues)

## Deployment Recommendations

### Production Environments

Use default platform trust mode:

```bash
gold_digger --db-url "mysql://app:secure@prod.db:3306/app" \
  --query "SELECT * FROM orders" --output orders.json
```

### Internal Infrastructure

Use custom CA trust mode:

```bash
gold_digger --db-url "mysql://service:token@internal.db:3306/data" \
  --tls-ca-file /etc/pki/internal-ca.pem \
  --query "SELECT * FROM metrics" --output metrics.csv
```

### Development Environments

Use hostname skip for development servers:

```bash
gold_digger --db-url "mysql://dev:devpass@192.168.1.100:3306/dev" \
  --insecure-skip-hostname-verify \
  --query "SELECT * FROM test_data" --output dev_data.json
```

### Testing Environments Only

Accept invalid certificates for testing:

```bash
gold_digger --db-url "mysql://test:test@localhost:3306/test" \
  --allow-invalid-certificate \
  --query "SELECT COUNT(*) FROM test_table" --output count.json
```

**⚠️ Security Warning**: Never use `--allow-invalid-certificate` in production.
