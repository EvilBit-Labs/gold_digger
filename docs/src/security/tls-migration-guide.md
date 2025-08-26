# TLS Migration Guide

This guide helps you migrate from the previous dual TLS implementation to the new simplified rustls-only model.

## What Changed

Gold Digger v0.2.7+ has migrated from a dual TLS implementation to a simplified, rustls-only approach with enhanced security controls.

### Before (v0.2.6 and earlier)

```toml
# Two mutually exclusive TLS features
ssl = ["mysql/native-tls"]        # Platform-native TLS
ssl-rustls = ["mysql/rustls-tls"] # Pure Rust TLS
```

### After (v0.2.7+)

```toml
# TLS dependencies are now always included - no feature flags required
# TLS support is built into all Gold Digger binaries
```

## Migration Steps

### 1. Update Build Commands

**Before:**

```bash
# Native TLS build
cargo build --release --features ssl

# Pure Rust TLS build
cargo build --release --no-default-features --features "json csv ssl-rustls additional_mysql_types verbose"
```

**After:**

```bash
# Standard build with TLS support (always included)
cargo build --release

# No TLS build (insecure connections only)
cargo build --release --no-default-features --features "json csv additional_mysql_types verbose"
```

### 2. Update CI/CD Configurations

Remove any references to SSL feature flags from your CI/CD pipelines:

**Before:**

```yaml
  - name: Build with pure Rust TLS
    run: cargo build --release --no-default-features --features "json csv 
      ssl-rustls additional_mysql_types verbose"
```

**After:**

```yaml
  - name: Build with TLS support (always included)
    run: cargo build --release
```

### 3. Update Documentation

Update any internal documentation that references the old feature-based TLS model:

- Remove mentions of `ssl` and `ssl-rustls` features
- Update build instructions to reflect that TLS is always available
- Note that TLS is now consistently rustls-based across all platforms without requiring feature flags

## New TLS Features

The new rustls-only implementation provides enhanced security controls:

### TLS Validation Modes

```bash
# Platform certificate store (default)
gold_digger --db-url "mysql://user:pass@host:3306/db" --query "SELECT 1" --output result.json

# Custom CA certificate
gold_digger --tls-ca-file /path/to/ca.pem --db-url "mysql://user:pass@host:3306/db" --query "SELECT 1" --output result.json

# Skip hostname verification (development)
gold_digger --insecure-skip-hostname-verify --db-url "mysql://user:pass@192.168.1.100:3306/db" --query "SELECT 1" --output result.json

# Accept invalid certificates (testing only)
gold_digger --allow-invalid-certificate --db-url "mysql://user:pass@test:3306/db" --query "SELECT 1" --output result.json
```

### Enhanced Error Messages

The new implementation provides intelligent error messages with specific CLI flag suggestions:

```text
Error: Certificate validation failed: certificate has expired
Suggestion: Use --allow-invalid-certificate for testing environments

Error: Hostname verification failed for 192.168.1.100: certificate is for db.company.com
Suggestion: Use --insecure-skip-hostname-verify to bypass hostname checks
```

## Benefits of the Migration

### Simplified Configuration

- **Single TLS implementation**: No more confusion between `ssl` and `ssl-rustls` features
- **Consistent behavior**: Same TLS implementation across all platforms
- **Enhanced security**: Granular TLS validation options for different deployment scenarios

### Platform Integration

- **Automatic certificate store usage**: Uses system certificate stores on Windows, macOS, and Linux
- **No native dependencies**: Pure Rust implementation eliminates platform-specific TLS library requirements
- **Better error handling**: Specific guidance for different certificate validation failures

### Security Improvements

- **Security warnings**: Prominent warnings for insecure TLS modes
- **Credential protection**: Automatic redaction of sensitive information in error messages
- **Intelligent error classification**: Specific suggestions for resolving TLS issues

## Troubleshooting

### Build Errors

If you encounter build errors after migration:

1. **Remove old feature references**: Ensure no `ssl-rustls` features remain in your build commands
2. **Clean build cache**: Run `cargo clean` to remove old build artifacts
3. **Update dependencies**: Run `cargo update` to ensure latest dependency versions

### Runtime Issues

If you experience TLS connection issues:

1. **Check certificate validity**: Ensure your database server has valid certificates
2. **Verify hostname matching**: Ensure certificate hostname matches connection hostname
3. **Use appropriate TLS mode**: Choose the right TLS validation mode for your environment

### Getting Help

If you encounter issues during migration:

1. **Check the documentation**: Review the updated TLS configuration guide
2. **Enable verbose logging**: Use `-v` flag for detailed TLS connection information
3. **File an issue**: Report migration issues on the GitHub repository

## Compatibility

### Backward Compatibility

- **Database connections**: Existing database URLs continue to work without changes
- **Configuration**: Environment variables and CLI flags remain the same
- **Output formats**: No changes to CSV, JSON, or TSV output formats

### Breaking Changes

- **Build commands**: Remove all SSL feature flag references (`ssl`, `ssl-rustls`)
- **CI/CD**: Update build scripts to remove feature flag dependencies
- **Documentation**: Update any references to feature-based TLS implementation

## Example Migration

### Before Migration

```bash
# Dockerfile
FROM rust:1.89 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --no-default-features --features "json csv ssl-rustls additional_mysql_types verbose"

# CI pipeline
- name: Build pure Rust TLS
  run: cargo build --release --no-default-features --features "json csv ssl-rustls additional_mysql_types verbose"
```

### After Migration

```bash
# Dockerfile
FROM rust:1.89 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

# CI pipeline
- name: Build with rustls TLS
  run: cargo build --release
```

The migration simplifies your build process while providing enhanced security controls and better cross-platform consistency.
