# Dependency Failure Troubleshooting

This guide provides comprehensive solutions for dependency conflicts, version issues, and feature flag problems in the Gold Digger project.

## Quick Reference

| Failure Type         | Common Cause                     | Quick Fix                         |
| -------------------- | -------------------------------- | --------------------------------- |
| Version Conflicts    | Incompatible dependency versions | `cargo update`, pin versions      |
| Feature Conflicts    | TLS backend conflicts            | `just validate-deps`              |
| Missing Dependencies | Crate not found or unavailable   | Add to Cargo.toml, check spelling |
| Platform Issues      | OS-specific dependency problems  | Install platform libraries        |
| Build Failures       | Dependency compilation errors    | Update deps, check features       |

## Version Conflicts

### Incompatible Dependency Versions

**Error Pattern:**

```
error: failed to select a version for `some-crate`
    required by package `gold_digger v0.2.5`
    versions that meet the requirements `^1.0.0` are: 1.2.0, 1.1.0, 1.0.0

    the package `other-crate` depends on `some-crate`, with features: `feature1` but `some-crate` does not have these features.
```

**Solutions:**

1. **Update Dependencies:**

   ```bash
   # Update all dependencies to latest compatible versions
   cargo update

   # Update specific dependency
   cargo update -p some-crate

   # Check for available updates
   cargo outdated
   ```

2. **Pin Specific Versions:**

   ```toml
   # In Cargo.toml
   [dependencies]
   some-crate = "~1.2"         # Compatible patch versions (use =1.2.0 only for security/compatibility exceptions)
   other-crate = ">=1.0, <2.0" # Version range
   ```

3. **Resolve Conflicts Manually:**

   ```bash
   # Find conflicting dependencies
   cargo tree --duplicates

   # Show dependency path
   cargo tree -i conflicting-crate

   # Analyze dependency graph
   cargo tree --format "{p} -> {d}"
   ```

### Cargo.lock Inconsistencies

**Error Pattern:**

```
error: the lock file needs to be updated but --locked was passed to prevent this
error: lock file is out of date
```

**Solutions:**

1. **Update Lock File:**

   ```bash
   # Remove and regenerate lock file
   rm Cargo.lock
   cargo build

   # Or update existing lock file
   cargo update
   ```

2. **Resolve Lock File Conflicts:**

   ```bash
   # During merge conflicts in Cargo.lock
   git checkout --theirs Cargo.lock
   cargo update

   # Or regenerate completely
   rm Cargo.lock
   cargo generate-lockfile
   ```

## Feature Flag Conflicts

### TLS Backend Conflicts

**Error Pattern:**

```
error: TLS connection failed: certificate validation error
```

**Solutions:**

1. **Validate TLS Dependencies:**

   ```bash
   # Check TLS dependency conflicts
   just validate-deps

   # Show TLS-related dependencies (always included)
   cargo tree --format "{p} {f}" | grep -E "rustls"
   ```

2. **Build with TLS Support:**

   ```bash
   # Standard build with TLS support (always included)
   cargo build --release

   # Minimal build (TLS still available)
   cargo build --no-default-features --features "json csv additional_mysql_types verbose"
   ```

3. **Verify TLS Dependencies:**

   ```toml
   # In Cargo.toml - rustls is always available
   [dependencies]
   mysql = { version = "26.0.1", features = [
     "rustls-tls",
   ], default-features = false }
   rustls = "0.23.31"
   rustls-native-certs = "0.8.1"
   rustls-pemfile = "2.2.0"

   # TLS is built-in, no feature toggling required
   ```

### Missing Feature Dependencies

**Error Pattern:**

```
error: feature `feature_name` not found in package `crate_name`
error: Package does not have feature `missing_feature`
```

**Solutions:**

1. **Check Available Features:**

   ```bash
   # List features in current project
   grep -A 20 '\[features\]' Cargo.toml

   # Check dependency features
   cargo info dependency_name
   ```

2. **Add Missing Features:**

   ```toml
   # In Cargo.toml - rustls configuration (always available)
   [dependencies]
   mysql = { version = "26.0.1", default-features = false, features = [
     "rustls-tls",
   ] }
   rustls = "0.23.31"
   rustls-native-certs = "0.8.1"
   rustls-pemfile = "2.2.0"
   serde = { version = "1.0", features = ["derive"] }
   ```

   **Note:** Gold Digger uses rustls exclusively for TLS support. TLS is always available and does not require feature toggling.

3. **TLS Configuration:**

   ```rust
   // In source code - TLS is always available
   use mysql::SslOpts;

   #[cfg(feature = "json")]
   use serde_json::Value;
   ```

## Missing Dependencies

### Crate Not Found

**Error Pattern:**

```
error: no matching package named `missing-crate` found
error: failed to get `some-crate` as a dependency of package `gold_digger`
```

**Solutions:**

1. **Add Missing Dependencies:**

   ```bash
   # Add dependency using cargo add
   cargo add missing-crate

   # Add with specific version
   cargo add missing-crate@1.0.0

   # Add with features
   cargo add missing-crate --features feature1,feature2
   ```

2. **Manual Addition:**

   ```toml
   # In Cargo.toml
   [dependencies]
   missing-crate = "1.0.0"

   # With features
   another-crate = { version = "2.0", features = ["feature1"] }

   # Optional dependency
   optional-crate = { version = "1.0", optional = true }
   ```

3. **Check Crate Availability:**

   ```bash
   # Search for crate
   cargo search crate-name

   # Check crate information
   cargo info crate-name

   # Verify crate exists on crates.io
   curl -s https://crates.io/api/v1/crates/crate-name
   ```

### Registry Issues

**Error Pattern:**

```
error: failed to get `crate` as a dependency
error: unable to get packages from source
```

**Solutions:**

1. **Update Registry Index:**

   ```bash
   # Update crates.io index
   cargo update

   # Clear registry cache
   rm -rf ~/.cargo/registry/cache
   rm -rf ~/.cargo/registry/src
   ```

2. **Check Network Connectivity:**

   ```bash
   # Test crates.io connectivity
   curl -I https://crates.io/

   # Check proxy settings
   echo $HTTP_PROXY
   echo $HTTPS_PROXY
   ```

3. **Alternative Registry:**

   Alternative registries must be declared in `.cargo/config.toml` before use:

   ```toml
   # In .cargo/config.toml
   [registries.alternative-registry]
   index = "https://alternative-registry.example.com/git/index"

   # In Cargo.toml, use alternative registry
   [dependencies]
   some-crate = { version = "1.0", registry = "alternative-registry" }
   ```

## Platform-Specific Dependency Issues

### Windows Dependencies

**Common Issues:**

- MSVC vs GNU toolchain conflicts
- Windows-specific system libraries
- Missing Visual Studio Build Tools

**Solutions:**

1. **Gold Digger uses rustls exclusively:**

   ```toml
   # In Cargo.toml - rustls is always available
   [dependencies]
   mysql = { version = "26.0.1", features = [
     "rustls-tls",
   ], default-features = false }
   rustls = "0.23.31"
   rustls-native-certs = "0.8.1"
   ```

   **Note:** Gold Digger uses rustls exclusively. No OpenSSL dependencies are required.

2. **Install Windows Build Tools:**

   ```bash
   # Install Visual Studio Build Tools
   # Download from: https://visualstudio.microsoft.com/downloads/

   # No additional TLS libraries needed - rustls is pure Rust
   ```

3. **Configure Windows-Specific Dependencies:**

   ```toml
   # In Cargo.toml
   [target.'cfg(windows)'.dependencies]
   winapi = { version = "0.3", features = ["winuser"] }
   ```

### macOS Dependencies

**Common Issues:**

- Homebrew library linking
- Apple Silicon vs Intel differences
- System framework dependencies

**Solutions:**

1. **Install Homebrew Dependencies:**

   ```bash
   # Install OpenSSL via Homebrew
   brew install openssl
   export OPENSSL_DIR=$(brew --prefix openssl)

   # Install pkg-config
   brew install pkg-config
   ```

2. **Configure macOS-Specific Dependencies:**

   ```toml
   # In Cargo.toml
   [target.'cfg(target_os = "macos")'.dependencies]
   core-foundation = "0.9"
   security-framework = "2.0"
   ```

3. **Handle Apple Silicon:**

   ```bash
   # Build for specific architecture
   cargo build --target aarch64-apple-darwin
   cargo build --target x86_64-apple-darwin

   # Universal binary
   cargo build --target universal2-apple-darwin
   ```

### Linux Dependencies

**Common Issues:**

- Missing system libraries
- Distribution-specific packages
- Library version conflicts

**Solutions:**

1. **Install System Dependencies:**

   ```bash
   # Ubuntu/Debian
   sudo apt-get install build-essential pkg-config libssl-dev

   # CentOS/RHEL
   sudo yum groupinstall "Development Tools"
   sudo yum install openssl-devel pkg-config

   # Fedora
   sudo dnf groupinstall "Development Tools"
   sudo dnf install openssl-devel pkg-config
   ```

2. **Configure Linux-Specific Dependencies:**

   ```toml
   # In Cargo.toml
   [target.'cfg(target_os = "linux")'.dependencies]
   libc = "0.2"
   ```

## Dependency Tree Analysis

### Understanding Dependencies

**Useful commands for dependency analysis:**

1. **Dependency Tree:**

   ```bash
   # Show full dependency tree
   cargo tree

   # Show dependencies of specific crate
   cargo tree -p gold_digger

   # Show reverse dependencies
   cargo tree -i mysql

   # Show duplicates
   cargo tree --duplicates
   ```

2. **Dependency Information:**

   ```bash
   # Show dependency features
   cargo tree --format "{p} {f}"

   # Show dependency licenses
   cargo tree --format "{p} {l}"

   # Limit depth
   cargo tree --depth 2
   ```

3. **Dependency Graph:**

   ```bash
   # Generate dependency graph (requires graphviz)
   cargo deps | dot -Tpng > deps.png

   # Or use cargo-depgraph
   cargo install cargo-depgraph
   cargo depgraph | dot -Tpng > graph.png
   ```

### Analyzing Conflicts

**Finding and resolving dependency conflicts:**

1. **Identify Conflicts:**

   ```bash
   # Find version conflicts
   cargo tree --duplicates

   # Show conflicting versions
   cargo tree -d -f "{p} {r}"

   # Find dependency paths
   cargo tree -i conflicting-crate
   ```

2. **Resolve Conflicts:**

   ```toml
   # In Cargo.toml, use dependency resolution
   [patch.crates-io]
   conflicting-crate = { version = "1.2.0" }

   # Or use specific versions
   [dependencies]
   crate-a = { version = "1.0", default-features = false }
   crate-b = { version = "2.0", features = ["minimal"] }
   ```

## Build Dependencies vs Runtime Dependencies

### Development Dependencies

**Managing dev-dependencies:**

```toml
# In Cargo.toml
[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.0"
tempfile = "3.0"
criterion = "0.5"

# These don't affect production builds
```

### Build Dependencies

**Managing build-dependencies:**

```toml
# In Cargo.toml
[build-dependencies]
cc = "1.0"
pkg-config = "0.3"

# Used only during build process
```

### Optional Dependencies

**Managing optional dependencies:**

```toml
# In Cargo.toml
[dependencies]
serde = { version = "1.0", optional = true }
tokio = { version = "1.0", optional = true }

[features]
default = []
async = ["tokio"]
serialization = ["serde"]
```

## Dependency Security and Maintenance

### Prerequisites

Before using the cargo-based tools in this section, install the required tools:

```bash
# Install required cargo tools
cargo install cargo-audit
cargo install cargo-outdated
cargo install cargo-deny
cargo install cargo-license
cargo install cargo-machete

# Optional tools for dependency analysis
cargo install cargo-deps
cargo install depgraph
```

**Initialize cargo-deny configuration:**

```bash
# Create and customize deny.toml configuration
cargo deny init
```

Edit the generated `deny.toml` to customize license policies, vulnerability checks, and dependency restrictions for your project.

### Security Considerations

1. **Audit Dependencies:**

   ```bash
   # Check for security vulnerabilities
   cargo audit

   # Update advisory database
   cargo audit --db ~/.cargo/advisory-db
   ```

2. **License Compliance:**

   ```bash
   # Check licenses (requires deny.toml configuration)
   cargo deny check licenses

   # Show all licenses
   cargo license
   ```

### Maintenance Strategies

1. **Regular Updates:**

   ```bash
   # Check for outdated dependencies
   cargo outdated

   # Update dependencies
   cargo update

   # Update specific dependency
   cargo update -p dependency-name
   ```

2. **Dependency Minimization:**

   ```bash
   # Use minimal features
   cargo build --no-default-features --features "minimal,required"

   # Check unused dependencies (optional tool)
   cargo machete
   ```

## Troubleshooting Specific Dependencies

### MySQL Dependencies

**Common mysql crate issues:**

1. **TLS Configuration:**

   ```toml
   # Gold Digger uses rustls exclusively
   [dependencies]
   mysql = { version = "26.0.1", features = [
     "rustls-tls",
   ], default-features = false }
   rustls = "0.23.31"
   rustls-native-certs = "0.8.1"
   rustls-pemfile = "2.2.0"
   ```

   **Note:** Gold Digger no longer supports native-tls. All builds use rustls for TLS connections.

2. **Feature Conflicts:**

   ```bash
   # Validate MySQL TLS dependencies
   just validate-deps

   # Check MySQL features
   cargo tree -p mysql --format "{p} {f}"
   ```

### Serde Dependencies

**Common serde issues:**

1. **Derive Feature:**

   ```toml
   # Ensure derive feature is enabled
   [dependencies]
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   ```

2. **Version Compatibility:**

   ```bash
   # Check serde ecosystem versions
   cargo tree | grep serde
   ```

### Clap Dependencies

**Common clap issues:**

1. **Version Migration:**

   ```toml
   # Clap 4.x configuration
   [dependencies]
   clap = { version = "4.0", features = ["derive", "env"] }
   ```

2. **Feature Configuration:**

   ```rust
   // Update derive syntax for clap 4.x
   use clap::Parser;

   #[derive(Parser)]
   #[command(author, version, about)]
   struct Cli {
       // fields
   }
   ```

## Prevention Strategies

### Dependency Management Best Practices

1. **Version Pinning:**

   ```toml
   # Use flexible version constraints for normal dependencies
   [dependencies]
   mysql = "~24.0"    # Compatible patch versions (use =24.0.0 only for security/compatibility exceptions)
   serde = "~1.0.150" # Compatible patch versions
   ```

2. **Feature Minimization:**

   ```toml
   # Use minimal features
   [dependencies]
   tokio = { version = "1.0", features = ["rt-multi-thread", "net"] }
   serde = { version = "1.0", features = ["derive"], default-features = false }
   ```

3. **Regular Maintenance:**

   ```bash
   # Weekly dependency check
   cargo outdated
   cargo audit

   # Monthly updates
   cargo update
   just validate-deps
   ```

### Development Workflow

1. **Before Adding Dependencies:**

   ```bash
   # Research alternatives
   cargo search crate-name

   # Check crate quality
   # - Recent updates
   # - Good documentation
   # - Active maintenance
   # - Security history
   ```

2. **After Adding Dependencies:**

   ```bash
   # Validate build
   just build-all

   # Check for conflicts
   just validate-deps

   # Run tests
   just test
   ```

## Getting Help

### Useful Commands

```bash
# Dependency analysis
cargo tree                    # Show dependency tree
cargo tree --duplicates      # Find version conflicts
cargo tree -i crate-name     # Reverse dependencies
just validate-deps           # Check TLS conflicts

# Dependency information
cargo info crate-name        # Crate information
cargo search keyword         # Search crates
cargo outdated              # Check for updates

# Troubleshooting
cargo update                 # Update dependencies
cargo clean                  # Clean build artifacts
rm Cargo.lock && cargo build # Regenerate lock file
```

### Resources

- [Cargo Book - Dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html)
- [Cargo Book - Features](https://doc.rust-lang.org/cargo/reference/features.html)
- [Crates.io](https://crates.io/) - Official Rust package registry
- [Lib.rs](https://lib.rs/) - Alternative crate discovery
- [Cargo Audit](https://github.com/RustSec/rustsec/tree/main/cargo-audit) - Security auditing

### Emergency Dependency Resolution

**When builds are completely broken:**

1. **Reset to Known Good State:**

   ```bash
   # Restore from backup
   git checkout HEAD~1 -- Cargo.toml Cargo.lock

   # Or reset to last working commit
   git reset --hard HEAD~1
   ```

2. **Minimal Dependency Set:**

   ```bash
   # Build with minimal features
   cargo build --no-default-features

   # Add features incrementally
   cargo build --no-default-features --features json
   cargo build --no-default-features --features json,csv
   ```

3. **Dependency Bisection:**

   ```bash
   # Remove half of dependencies
   # Test build
   # Repeat until problem is isolated
   ```
