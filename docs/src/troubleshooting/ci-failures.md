# CI Failure Troubleshooting Guide

This guide provides detailed troubleshooting steps for common CI/CD pipeline failures in the Gold
Digger project.

## Quick Reference

| Failure Type | Common Cause                     | Quick Fix                        |
| ------------ | -------------------------------- | -------------------------------- |
| Build        | Compilation errors, missing deps | `just build-all`, check features |
| Test         | Test logic, environment setup    | `just test-nextest`, check logs  |
| Security     | Vulnerabilities, license issues  | `just security`, update deps     |
| Format       | Code style, linting warnings     | `just format && just fix`        |
| Dependency   | Version conflicts, feature flags | `just validate-deps`             |

## Build Failures

### Rust Compilation Errors

**Symptoms:**

- `error[E0xxx]` messages during compilation
- Missing trait implementations
- Type mismatch errors

**Diagnosis:**

```bash
# Reproduce locally
just build-all

# Check TLS and non-TLS builds
cargo build --release  # With TLS
cargo build --no-default-features --features "json csv"  # Without TLS
```

**Common Solutions:**

1. **Missing Dependencies:**

   ```bash
   # Update Cargo.lock
   cargo update

   # Check dependency tree
   cargo tree
   ```

2. **Feature Flag Issues:**

   ```bash
   # Validate TLS dependencies
   just validate-deps

   # Test all feature combinations
   just build-all
   ```

3. **Platform-Specific Issues:**

   ```bash
   # Windows build issues
   # Install Visual Studio Build Tools

   # macOS linking issues
   # Install Xcode command line tools
   xcode-select --install
   ```

### Missing Feature Dependencies

**Symptoms:**

- `feature 'xxx' not found` errors
- Conditional compilation failures

**Solutions:**

```bash
# Check available features
grep -A 20 '\[features\]' Cargo.toml

# Validate feature combinations
just features

# Test minimal build
just build-minimal
```

## Test Failures

### Unit Test Failures

**Symptoms:**

- Assertion failures in tests
- Panic messages during test execution
- Test timeouts

**Diagnosis:**

```bash
# Run tests with detailed output
cargo test -- --nocapture

# Use nextest for better reporting
just test-nextest

# Run specific test
cargo test test_name -- --exact --nocapture
```

**Common Solutions:**

1. **Database Connection Issues:**

   ```bash
   # Check if tests require database
   grep -r "DATABASE_URL" tests/

   # Option 1: Use local MySQL instance
   export DATABASE_URL="mysql://root@localhost:3306/mysql"

   # Option 2: Use Testcontainers for automated MySQL setup
   # This automatically spins up a MySQL container for tests
   cargo test --features testcontainers

   # Option 3: Manual MySQL container setup
   docker run --name test-mysql -e MYSQL_ROOT_PASSWORD=testpass -p 3306:3306 -d mysql:8.0
   export DATABASE_URL="mysql://root:testpass@localhost:3306/mysql"
   ```

2. **Platform-Specific Test Behavior:**

   ```bash
   # Check for platform-specific code
   grep -r "cfg(target_os" src/ tests/

   # Run tests on specific platform
   cargo test --target x86_64-pc-windows-msvc
   ```

### Integration Test Issues

**Symptoms:**

- External service connection failures
- File system permission errors
- Environment setup problems

**Solutions:**

```bash
# Check integration test setup
ls -la tests/

# Run integration tests separately
cargo test --test integration_test_name

# Check test dependencies
grep -r "testcontainers\|assert_cmd" tests/
```

## Security Failures

### Vulnerability Scanning

**Symptoms:**

- High/critical vulnerabilities detected
- Security advisory warnings
- SARIF upload failures

**Diagnosis:**

```bash
# Run comprehensive security scan
just security

# Check specific vulnerabilities
cargo audit

# Inspect dependency tree
cargo tree | grep vulnerable_crate
```

**Common Solutions:**

1. **Update Dependencies:**

   ```bash
   # Update to latest patch versions
   cargo update

   # Update specific crate
   cargo update -p crate_name
   ```

2. **Exclude Vulnerable Versions:**

   ```toml
   # In Cargo.toml
   [dependencies]
   vulnerable_crate = { version = ">=1.2.3", features = ["secure"] }
   ```

3. **Alternative Dependencies:**

   ```bash
   # Find alternative crates
   cargo search alternative_crate

   # Check crate security status
   cargo audit --db /path/to/advisory-db
   ```

### License Compliance

**Symptoms:**

- `cargo-deny` failures
- License compatibility issues
- Unknown license warnings

**Solutions:**

```bash
# Check license configuration
cat deny.toml

# Update license database
cargo deny fetch

# Check specific license
cargo deny check licenses
```

## Format and Linting Failures

### Code Formatting Issues

**Symptoms:**

- `cargo fmt --check` failures
- Inconsistent code style
- Line length violations

**Solutions:**

```bash
# Auto-format code
just format

# Check formatting
just fmt-check

# Format specific file
cargo fmt -- src/main.rs
```

### Clippy Warnings

**Symptoms:**

- Clippy lint warnings (zero-tolerance policy)
- Performance or correctness suggestions
- Style violations

**Solutions:**

```bash
# Auto-fix clippy issues
just fix

# Run clippy with explanations
cargo clippy -- -D warnings --verbose

# Disable specific lint (use sparingly)
#[allow(clippy::lint_name)]
```

## Dependency Failures

### Version Conflicts

**Symptoms:**

- Dependency resolution failures
- Conflicting version requirements
- Feature flag conflicts

**Diagnosis:**

```bash
# Check dependency conflicts
cargo tree --duplicates

# Validate TLS dependencies
just validate-deps

# Check feature combinations
cargo check --no-default-features --features "feature1,feature2"
```

**Solutions:**

1. **Resolve Version Conflicts:**

   ```toml
   # In Cargo.toml, use specific versions
   [dependencies]
   conflicting_crate = "=1.2.3"
   ```

2. **Feature Flag Issues:**

   ```bash
   # Check TLS backend conflicts
   just validate-deps

   # Test TLS and non-TLS builds
   cargo build --release  # With TLS
   cargo build --no-default-features --features "json csv"  # Without TLS
   ```

### Platform-Specific Dependencies

**Symptoms:**

- Windows-specific build failures
- macOS linking errors
- Linux distribution issues

**Solutions:**

1. **Windows Issues:**

   ```bash
   # Install Visual Studio Build Tools
   # No additional TLS dependencies needed - rustls is pure Rust
   ```

2. **macOS Issues:**

   ```bash
   # Install Xcode command line tools
   xcode-select --install

   # Update Homebrew packages
   brew update && brew upgrade
   ```

3. **Linux Issues:**

   ```bash
   # Install development packages
   sudo apt-get install build-essential pkg-config libssl-dev

   # For CentOS/RHEL
   sudo yum groupinstall "Development Tools"
   sudo yum install pkg-config
   ```

## Environment-Specific Issues

### GitHub Actions Environment

**Symptoms:**

- CI passes locally but fails in GitHub Actions
- Environment variable issues
- Permission problems

**Solutions:**

1. **Reproduce CI Environment:**

   ```bash
   # Use act to simulate GitHub Actions
   just act-ci-dry

   # Run full CI locally
   just ci-check
   ```

2. **Environment Variables:**

   ```bash
   # Check required environment variables
   grep -r "env::" .github/workflows/

   # Verify secrets are set in repository settings
   ```

3. **Permission Issues:**

   ```yaml
   # In workflow file, ensure proper permissions
   permissions:
     contents: read
     security-events: write
   ```

### Cross-Platform Issues

**Symptoms:**

- Tests pass on one platform but fail on others
- Platform-specific compilation errors
- File path or line ending issues

**Solutions:**

1. **Path Handling:**

   ```rust
   // Use std::path::Path for cross-platform paths
   use std::path::Path;
   let path = Path::new("src").join("main.rs");
   ```

2. **Line Endings:**

   ```bash
   # Configure git for consistent line endings
   git config core.autocrlf input
   ```

3. **Platform-Specific Code:**

   ```rust
   #[cfg(target_os = "windows")]
   fn windows_specific() { /* ... */
   }

   #[cfg(unix)]
   fn unix_specific() { /* ... */
   }
   ```

## Debug Artifact Analysis

When CI failures occur, debug artifacts are automatically collected. Here's how to analyze them:

### System Information

- Check `system-info.txt` for environment details
- Look for resource constraints (memory, disk space)
- Verify OS and architecture compatibility

### Rust Environment

- Review `rust-info.txt` for toolchain issues
- Check dependency tree for conflicts
- Verify feature flag configurations

### Build Information

- Examine `build-info.txt` for compilation details
- Look for missing build artifacts
- Check build log patterns

### Test Results

- Analyze `test-info.txt` for test execution details
- Review coverage information if available
- Check for test-specific environment issues

## Additional Troubleshooting Guides

For detailed guidance on specific failure types, see:

- **[Build Failures](build-failures.md)** - Comprehensive guide for Rust compilation and build
  issues
- **[Test Failures](test-failures.md)** - Detailed solutions for unit and integration test problems
- **[Security Failures](security-failures.md)** - Vulnerability scanning and license compliance
  issues
- **[Format Failures](format-failures.md)** - Code formatting and linting problem resolution
- **[Dependency Failures](dependency-failures.md)** - Version conflicts and feature flag issues

## Getting Additional Help

### Local Reproduction

```bash
# Full CI reproduction
just ci-check

# Specific failure reproduction
just lint          # For format/lint issues
just test-nextest  # For test failures
just security      # For security issues
just build-all     # For build issues
```

### GitHub Actions Simulation

```bash
# Set up act for local testing
just act-setup

# Run CI workflow simulation
just act-ci-dry

# Test specific job
just act-job validate
```

### Creating Issues

When creating a GitHub issue for CI failures:

1. **Include Error Category:** Specify build/test/security/format/dependency
2. **Provide Context:** Include the specific error messages
3. **Environment Details:** OS, Rust version, feature flags used
4. **Reproduction Steps:** Commands that reproduce the issue locally
5. **Debug Artifacts:** Reference the uploaded debug artifacts from the failed run

### Useful Commands Reference

```bash
# Development workflow
just setup          # Set up development environment
just ci-check       # Run all CI checks locally
just help           # Show all available commands

# Debugging
just validate-deps  # Check dependency conflicts
just features       # Show feature combinations
just status         # Show project status

# Local CI testing
just act-setup      # Set up GitHub Actions simulation
just act-ci-dry     # Simulate CI workflow
just act-clean      # Clean up act artifacts
```

## Prevention Strategies

### Pre-commit Hooks

```bash
# Install pre-commit hooks
pre-commit install

# Run all hooks
pre-commit run --all-files
```

### Local CI Validation

```bash
# Before pushing changes
just ci-check

# Test all feature combinations
just build-all

# Run comprehensive security scan
just security
```

### Dependency Management

```bash
# Regular dependency updates
cargo update

# Security audits
just audit

# License compliance checks
just deny
```
