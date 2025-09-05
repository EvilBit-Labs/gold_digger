# Build Failure Troubleshooting

This guide provides comprehensive solutions for Rust compilation and build issues in the Gold Digger
project.

## Quick Reference

| Error Type   | Common Cause                   | Quick Fix                      |
| ------------ | ------------------------------ | ------------------------------ |
| Compilation  | Syntax errors, type mismatches | Check error messages, fix code |
| Dependencies | Missing or conflicting deps    | `cargo update`, check features |
| Platform     | OS-specific build issues       | Install platform tools         |
| Features     | Feature flag conflicts         | `just validate-deps`           |
| Linking      | Library linking problems       | Install system libraries       |

## Rust Compilation Errors

### Syntax and Type Errors

**Error Pattern:**

```
error[E0308]: mismatched types
error[E0425]: cannot find value `variable_name` in this scope
error[E0277]: the trait `TraitName` is not implemented
```

**Solutions:**

1. **Type Mismatch Errors:**

   ```rust
   // Problem: Type conversion issues
   let value: String = some_function(); // Returns &str

   // Solution: Proper type conversion
   let value: String = some_function().to_string();
   ```

2. **Missing Trait Implementations:**

   ```rust
   // Problem: Trait not in scope
   use std::collections::HashMap;
   let mut map = HashMap::new();
   map.insert("key", "value"); // May need trait

   // Solution: Import required traits
   use std::collections::HashMap;
   use std::iter::FromIterator;
   ```

3. **Lifetime Issues:**

   ```rust
   // Problem: Lifetime annotation needed
   fn get_value(data: &str) -> &str {
       &data[0..5] // Lifetime issue
   }

   // Solution: Proper lifetime annotation
   fn get_value(data: &str) -> &str {
       &data[0..data.len().min(5)]
   }
   ```

### Macro and Procedural Macro Errors

**Error Pattern:**

```
error: proc-macro derive panicked
error: cannot find macro `macro_name` in this scope
```

**Solutions:**

1. **Update Procedural Macros:**

   ```bash
   # Update all dependencies
   cargo update

   # Clean and rebuild
   cargo clean && cargo build
   ```

2. **Check Macro Dependencies:**

   ```toml
   # In Cargo.toml, ensure proper versions
   [dependencies]
   serde = { version = "1.0", features = ["derive"] }
   clap = { version = "4.0", features = ["derive"] }
   ```

## Dependency Issues

### Missing Dependencies

**Error Pattern:**

```
error[E0432]: unresolved import `crate_name`
error: failed to resolve dependencies
```

**Solutions:**

1. **Add Missing Dependencies:**

   ```bash
   # Add dependency to Cargo.toml
   cargo add dependency_name

   # Or manually add to Cargo.toml
   [dependencies]
   missing_crate = "1.0"
   ```

2. **Check Feature Flags:**

   ```bash
   # Validate TLS dependencies
   just validate-deps

   # Check specific feature combination
   cargo build --no-default-features --features "json csv"
   ```

### Version Conflicts

**Error Pattern:**

```
error: failed to select a version for `crate_name`
error: conflicting requirements for `dependency`
```

**Solutions:**

1. **Resolve Version Conflicts:**

   ```toml
   # In Cargo.toml, specify exact versions
   [dependencies]
   conflicting_crate = "=1.2.3"

   # Or use version ranges
   another_crate = ">=1.0, <2.0"
   ```

2. **Update Cargo.lock:**

   ```bash
   # Remove lock file and regenerate
   rm Cargo.lock
   cargo build

   # Or update specific dependency
   cargo update -p dependency_name
   ```

3. **Check Dependency Tree:**

   ```bash
   # Find duplicate dependencies
   cargo tree --duplicates

   # Show dependency path
   cargo tree -i dependency_name
   ```

## Platform-Specific Build Issues

### Windows Build Problems

**Common Issues:**

- Visual Studio Build Tools missing
- Long path limitations
- MSVC vs GNU toolchain conflicts

**Solutions:**

1. **Standard Build:**

   ```bash
   # Standard build (TLS always available with rustls)
   cargo build --release

   # Minimal build (TLS still available)
   cargo build --no-default-features --features "json csv"

   # No additional TLS dependencies needed - rustls is pure Rust
   ```

2. **Visual Studio Build Tools:**

   ```bash
   # Install Visual Studio Build Tools
   # Download from: https://visualstudio.microsoft.com/downloads/

   # Or use chocolatey
   choco install visualstudio2022buildtools
   ```

3. **Path Length Issues:**

   ```bash
   # Enable long paths in Windows
   # Run as administrator:
   New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force

   # Or use shorter build paths
   set CARGO_TARGET_DIR=C:\tmp\target
   ```

### macOS Build Problems

**Common Issues:**

- Xcode command line tools missing
- Apple Silicon vs Intel differences
- Homebrew dependency conflicts

**Solutions:**

1. **Xcode Command Line Tools:**

   ```bash
   # Install Xcode command line tools
   xcode-select --install

   # Verify installation
   xcode-select -p
   ```

2. **Build Configuration:**

   ```bash
   # Standard build (TLS always available with rustls)
   cargo build --release

   # Minimal build (TLS still available)
   cargo build --no-default-features --features "json csv"
   ```

   **Note:** Gold Digger uses rustls exclusively - no Homebrew OpenSSL installation required.

3. **Apple Silicon Issues:**

   ```bash
   # Build for specific architecture
   cargo build --target aarch64-apple-darwin
   cargo build --target x86_64-apple-darwin

   # Combine into universal binary using lipo
   lipo -create -output gold_digger_universal \
     target/aarch64-apple-darwin/release/gold_digger \
     target/x86_64-apple-darwin/release/gold_digger

   # Or use cargo-dist to produce universal packages automatically
   cargo dist build
   ```

### Linux Build Problems

**Common Issues:**

- Missing development packages
- Library path issues
- Distribution-specific problems

**Solutions:**

1. **Install Development Packages:**

   ```bash
   # Ubuntu/Debian
   sudo apt-get update
   sudo apt-get install build-essential pkg-config libssl-dev

   # CentOS/RHEL/Fedora
   sudo yum groupinstall "Development Tools"
   sudo yum install pkg-config

   # Or use dnf on newer systems
   sudo dnf groupinstall "Development Tools"
   sudo dnf install pkg-config
   ```

2. **Library Path Issues:**

   ```bash
   # Set library path
   export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH

   # Use pkg-config
   export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig:$PKG_CONFIG_PATH
   ```

## Feature Flag Issues

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

   # Test TLS and non-TLS builds
   cargo build --release  # With TLS
   cargo build --no-default-features --features "json csv"  # Without TLS
   ```

2. **Choose TLS Backend:**

   ```bash
   # Standard build with TLS support (always included)
   cargo build --release

   # Minimal build without TLS
   cargo build --release --no-default-features

   # For no TLS (testing only)
   cargo build --no-default-features --features "json csv additional_mysql_types verbose"
   ```

### Missing Feature Dependencies

**Error Pattern:**

```
error: feature `feature_name` not found
error: conditional compilation cfg predicate not satisfied
```

**Solutions:**

1. **Check Available Features:**

   ```bash
   # List all features
   grep -A 20 '\[features\]' Cargo.toml

   # Test feature combinations
   just build-all
   ```

2. **Add Missing Features:**

   ```toml
   # In Cargo.toml
   [features]
   default = ["json", "csv", "ssl", "additional_mysql_types", "verbose"]
   new_feature = ["dependency/feature"]
   ```

## Cross-Compilation Issues

### Target Architecture Problems

**Error Pattern:**

```
error: linker `cc` not found
error: could not find native static library `library_name`
```

**Solutions:**

1. **Install Cross-Compilation Tools:**

   ```bash
   # Install cross-compilation target
   rustup target add x86_64-pc-windows-gnu
   rustup target add aarch64-unknown-linux-gnu

   # Install cross-compilation toolchain
   sudo apt-get install gcc-mingw-w64-x86-64
   sudo apt-get install gcc-aarch64-linux-gnu
   ```

2. **Configure Cross-Compilation:**

   ```toml
   # In .cargo/config.toml
   [target.x86_64-pc-windows-gnu]
   linker = "x86_64-w64-mingw32-gcc"

   [target.aarch64-unknown-linux-gnu]
   linker = "aarch64-linux-gnu-gcc"
   ```

## Build Performance Issues

### Slow Compilation

**Solutions:**

1. **Optimize Build Settings:**

   ```toml
   # In Cargo.toml
   [profile.dev]
   opt-level = 1 # Some optimization for faster builds

   [profile.release]
   lto = "thin"      # Faster linking
   codegen-units = 1 # Better optimization
   ```

2. **Use Parallel Compilation:**

   ```bash
   # Use all available cores
   cargo build -j $(nproc)

   # Or specify a specific number of jobs
   cargo build -j 4
   ```

   You can persist this setting by adding `build.jobs = N` under the `[build]` table in
   `.cargo/config.toml`.

3. **Enable Incremental Compilation:**

   ```bash
   # Enable incremental compilation (default in dev)
   export CARGO_INCREMENTAL=1

   # Use shared target directory
   export CARGO_TARGET_DIR=/tmp/cargo-target
   ```

### Large Binary Size

**Solutions:**

1. **Optimize for Size:**

   ```toml
   # In Cargo.toml
   [profile.release]
   opt-level = 'z'   # Optimize for size
   lto = true        # Link-time optimization
   codegen-units = 1
   panic = 'abort'   # Remove panic handling code
   strip = true      # Remove debug symbols
   ```

2. **Remove Unused Features:**

   ```bash
   # Build with minimal features
   cargo build --release --no-default-features --features "csv json"

   # Check binary size
   ls -lh target/release/gold_digger
   ```

## Memory and Resource Issues

### Out of Memory During Build

**Error Pattern:**

```
error: could not compile due to previous error
fatal error: 'memory exhausted'
```

**Solutions:**

1. **Reduce Memory Usage:**

   ```bash
   # Reduce parallel jobs
   export CARGO_BUILD_JOBS=1

   # Use less memory for linking
   export RUSTFLAGS="-C link-arg=-Wl,--no-keep-memory"
   ```

2. **Increase Available Memory:**

   ```bash
   # Increase swap space (Linux)
   sudo fallocate -l 2G /swapfile
   sudo chmod 600 /swapfile
   sudo mkswap /swapfile
   sudo swapon /swapfile
   ```

## Debugging Build Issues

### Verbose Build Output

```bash
# Enable verbose output
cargo build --verbose

# Show build commands
cargo build -vv

# Show timing information
cargo build --timings
```

### Build Environment Information

```bash
# Show Rust environment
rustc --version --verbose
cargo --version --verbose

# Show target information
rustc --print target-list
rustc --print cfg

# Show environment variables
env | grep -E '^(CARGO_|RUST_|CC|CXX)'
```

### Clean Build Environment

```bash
# Clean all build artifacts
cargo clean

# Remove Cargo cache
rm -rf ~/.cargo/registry/cache
rm -rf ~/.cargo/git/db

# Reset Rust toolchain
rustup self update
rustup update
```

## Prevention Strategies

### Pre-Build Checks

```bash
# Validate dependencies before building
just validate-deps

# Check for common issues
cargo check

# Run clippy for potential issues
just lint
```

### Build Environment Setup

```bash
# Set up development environment
just setup

# Install additional tools
just install-tools

# Verify environment
just ci-check
```

### Regular Maintenance

```bash
# Update dependencies regularly
cargo update

# Clean build artifacts periodically
cargo clean

# Update Rust toolchain
rustup update
```

## Getting Help

### Useful Commands

```bash
# Build troubleshooting
just build-all         # Test all feature combinations
just validate-deps     # Check dependency conflicts
cargo tree             # Show dependency tree
cargo check            # Quick compilation check

# Environment debugging
rustc --version --verbose
cargo --version --verbose
env | grep CARGO
```

### Resources

- [Rust Compilation Error Index](https://doc.rust-lang.org/error-index.html)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [Rust Cross-Compilation Guide](https://rust-lang.github.io/rustup/cross-compilation.html)
- [Platform Support](https://forge.rust-lang.org/infra/platform-support.html)
