# Development Setup

Complete guide for setting up your development environment to contribute to Gold Digger.

## Prerequisites

### Required Software

- **[Rust](https://rustup.rs/)** (latest stable via rustup)
- **[Git](https://git-scm.com/)** for version control
- **MySQL or MariaDB** for integration testing
- **[just](https://github.com/casey/just)** task runner (recommended)

### Platform-Specific Requirements

**macOS:**

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Linux (Ubuntu/Debian):**

```bash
# Install build dependencies
sudo apt update
sudo apt install build-essential pkg-config libssl-dev git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Windows:**

```powershell
# Install Visual Studio Build Tools
# Download from: https://visualstudio.microsoft.com/downloads/

# Install Rust
# Download from: https://rustup.rs/
```

## Initial Setup

### 1. Clone Repository

```bash
git clone https://github.com/EvilBit-Labs/gold_digger.git
cd gold_digger
```

### 2. Install Development Tools

```bash
# Use the justfile for automated setup
just setup

# Or install manually:
rustup component add rustfmt clippy
cargo install cargo-nextest --locked
cargo install cargo-llvm-cov --locked
cargo install cargo-audit --locked
cargo install cargo-deny --locked
```

### 3. Set Up Pre-commit Hooks (Recommended)

Gold Digger uses comprehensive pre-commit hooks for code quality:

```bash
# Install pre-commit
pip install pre-commit

# Install hooks for this repository
pre-commit install

# Test hooks on all files (optional)
pre-commit run --all-files
```

**Pre-commit Hook Coverage:**

- **Rust**: Formatting (`cargo fmt`), linting (`cargo clippy`), security audit (`cargo audit`)
- **YAML/JSON**: Formatting with Prettier
- **Markdown**: Formatting (`mdformat`) with GitHub Flavored Markdown support
- **Shell Scripts**: Validation with ShellCheck
- **GitHub Actions**: Workflow validation with actionlint
- **Commit Messages**: Conventional commit format validation
- **Documentation**: Link checking and build validation

### 3. Install Documentation Tools

```bash
# Install mdBook and plugins for documentation
just docs-install

# Or install manually:
cargo install mdbook mdbook-admonish mdbook-mermaid mdbook-linkcheck mdbook-toc mdbook-open-on-gh mdbook-tabs mdbook-i18n-helpers
```

### 4. Verify Installation

```bash
# Build the project
cargo build

# Run tests
cargo test

# Check code quality
just ci-check

# Test pre-commit hooks (if installed)
pre-commit run --all-files
```

## Development Tools

### Essential Tools

| Tool             | Purpose                                 | Installation                   |
| ---------------- | --------------------------------------- | ------------------------------ |
| `cargo-nextest`  | Fast parallel test runner               | `cargo install cargo-nextest`  |
| `cargo-llvm-cov` | Code coverage analysis (cross-platform) | `cargo install cargo-llvm-cov` |
| `cargo-audit`    | Security vulnerability scanning         | `cargo install cargo-audit`    |
| `cargo-deny`     | License and security policy enforcement | `cargo install cargo-deny`     |
| `just`           | Task runner (like make)                 | `cargo install just`           |

### Optional Tools

| Tool             | Purpose                         | Installation                                        |
| ---------------- | ------------------------------- | --------------------------------------------------- |
| `cargo-watch`    | Auto-rebuild on file changes    | `cargo install cargo-watch`                         |
| `cargo-outdated` | Check for outdated dependencies | `cargo install cargo-outdated`                      |
| `act`            | Run GitHub Actions locally      | [Installation guide](https://github.com/nektos/act) |

## Project Structure

### Source Code Organization

```
src/
├── main.rs     # CLI entry point, env handling, format dispatch
├── lib.rs      # Public API, shared utilities (rows_to_strings)
├── cli.rs      # Clap CLI definitions and configuration
├── csv.rs      # CSV output format (RFC4180, QuoteStyle::Necessary)
├── json.rs     # JSON output format ({"data": [...]} with BTreeMap)
├── tab.rs      # TSV output format (QuoteStyle::Necessary)
├── tls.rs      # TLS/SSL configuration utilities
└── exit.rs     # Exit code definitions and utilities
```

### Configuration Files

```
├── Cargo.toml              # Package configuration and dependencies
├── Cargo.lock              # Dependency lock file
├── justfile                # Task runner recipes
├── rustfmt.toml            # Code formatting configuration
├── deny.toml               # Security and license policy
├── rust-toolchain.toml     # Rust version specification
├── .pre-commit-config.yaml # Pre-commit hooks
└── .editorconfig           # Editor configuration
```

### Documentation Structure

```
docs/
├── book.toml               # mdBook configuration
├── src/                    # Documentation source
│   ├── SUMMARY.md         # Table of contents
│   ├── introduction.md    # Landing page
│   ├── installation/      # Installation guides
│   ├── usage/             # Usage documentation
│   ├── security/          # Security considerations
│   ├── development/       # Developer guides
│   └── troubleshooting/   # Common issues
└── book/                  # Generated output (gitignored)
```

## Development Workflow

### 1. Code Quality Checks

```bash
# Format code (includes pre-commit hooks)
just format

# Check formatting
just fmt-check

# Run linter
just lint

# Run all quality checks
just ci-check

# Run pre-commit hooks manually
pre-commit run --all-files
```

### 2. Security Scanning

```bash
# Run security audit
just audit

# Check licenses and security policies
just deny

# Comprehensive security scanning (audit + deny + grype)
just security

# Generate Software Bill of Materials (SBOM)
just sbom

# Cargo-deny configuration validation
just deny
```

### 3. Cargo-deny Configuration

Gold Digger uses a dual cargo-deny configuration to balance local development flexibility with CI security enforcement:

#### Local Development (Tolerant)

- **File**: `deny.toml` (default)
- **Yanked crates**: `warn` (shows warnings but doesn't fail)
- **Purpose**: Allows local development to continue even with yanked dependencies

#### CI Environment (Strict)

- **File**: `deny.ci.toml`
- **Yanked crates**: `error` (fails pipeline on yanked crates)
- **Purpose**: Enforces strict security policies in CI

#### Usage

```bash
# Local development (tolerant)
just deny

# CI enforcement (strict)
just deny-ci

# CI workflow automatically uses strict configuration
# See .github/workflows/security.yml
```

#### Configuration Files

- `deny.toml` - Local development configuration with tolerant settings
- `deny.ci.toml` - CI-specific configuration with strict enforcement
- Both files maintain the same license and security policies, differing only in yanked crate handling

````

### 3. Testing

```bash
# Run tests (standard)
just test

# Run tests (fast parallel)
just test-nextest

# Run with coverage
just coverage

# Run specific test
cargo test test_name
````

### 4. Building

```bash
# Debug build
just build

# Release build
just build-release



# Build all variants
just build-all
```

### 4. Documentation

```bash
# Build documentation
just docs-build

# Serve documentation locally
just docs-serve

# Check documentation links
just docs-check

# Generate rustdoc only
just docs
```

## Feature Development

### Feature Flag System

Gold Digger uses Cargo features for conditional compilation:

```toml
# Default features
default = ["json", "csv", "additional_mysql_types", "verbose"]

# Individual features
json = []  # Enable JSON output format
csv = []   # Enable CSV output format
additional_mysql_types = ["mysql_common?/bigdecimal", ...]
verbose = []  # Enable verbose logging

# TLS is always enabled with rustls - no feature flags needed
```

### Testing Feature Combinations

```bash
# Test default features
cargo test

# Test minimal features
cargo test --no-default-features --features "csv json"

# Test with all features
cargo test --release

# Test without optional features
cargo test --no-default-features
```

## Database Setup for Testing

### Local MySQL/MariaDB

```bash
# Install MySQL (macOS)
brew install mysql
brew services start mysql

# Install MariaDB (Ubuntu)
sudo apt install mariadb-server
sudo systemctl start mariadb

# Create test database
mysql -u root -p
CREATE DATABASE gold_digger_test;
CREATE USER 'test_user'@'localhost' IDENTIFIED BY 'test_password';
GRANT ALL PRIVILEGES ON gold_digger_test.* TO 'test_user'@'localhost';
```

### Docker Setup

```bash
# Start MySQL container
docker run --name gold-digger-mysql \
  -e MYSQL_ROOT_PASSWORD=rootpass \
  -e MYSQL_DATABASE=gold_digger_test \
  -e MYSQL_USER=test_user \
  -e MYSQL_PASSWORD=test_password \
  -p 3306:3306 \
  -d mysql:8.0

# Test connection
export DATABASE_URL="mysql://test_user:test_password@localhost:3306/gold_digger_test"
export DATABASE_QUERY="SELECT 1 as test"
export OUTPUT_FILE="test.json"
cargo run
```

## Code Style Guidelines

### Rust Style

- **Formatting**: Use `rustfmt` with 100-character line limit
- **Linting**: Zero tolerance for clippy warnings
- **Error Handling**: Use `anyhow::Result<T>` for fallible functions
- **Documentation**: Document all public APIs with `///` comments

### Module Organization

```rust
// Standard library imports
use std::{env, fs::File};

// External crate imports
use anyhow::Result;
use mysql::Pool;

// Local module imports
use gold_digger::rows_to_strings;
```

### Safe Patterns

```rust
// ✅ Safe database value conversion
match database_value {
    mysql::Value::NULL => "".to_string(),
    val => from_value_opt::<String>(val)
        .unwrap_or_else(|_| format!("{:?}", val))
}

// ✅ Feature-gated compilation
#[cfg(feature = "verbose")]
eprintln!("Debug information");

// ✅ Error propagation
fn process_data() -> anyhow::Result<()> {
    let data = fetch_data()?;
    transform_data(data)?;
    Ok(())
}
```

## Debugging

### Environment Variables

```bash
# Enable Rust backtrace
export RUST_BACKTRACE=1

# Enable verbose logging
export RUST_LOG=debug

# Test with safe example
just run-safe
```

### Common Issues

**Build failures:**

- Check Rust version: `rustc --version`
- Update toolchain: `rustup update`
- Clean build: `cargo clean && cargo build`

**Test failures:**

- Check database connection
- Verify environment variables
- Run single-threaded: `cargo test -- --test-threads=1`

**Clippy warnings:**

- Fix automatically: `just fix`
- Check specific lint: `cargo clippy -- -W clippy::lint_name`

## Contributing Guidelines

### Before Submitting

1. **Run quality checks**: `just ci-check`
2. **Add tests** for new functionality
3. **Update documentation** if needed
4. **Follow commit conventions** (Conventional Commits)
5. **Test feature combinations** if adding features
6. **Ensure pre-commit hooks pass**: `pre-commit run --all-files`

### Pull Request Process

1. Fork the repository
2. Create feature branch: `git checkout -b feature/description`
3. Make changes with tests
4. Run `just ci-check`
5. Commit with conventional format
6. Push and create pull request

### Commit Message Format

```
type(scope): description

feat(csv): add support for custom delimiters
fix(json): handle null values in nested objects
docs(api): update configuration examples
test(integration): add TLS connection tests
```

## Local GitHub Actions Testing

### Setup act

```bash
# Install act
just act-setup

# Run CI workflow locally (dry-run)
just act-ci-dry

# Run full CI workflow
just act-ci
```

### Environment Configuration

Create a `.env.local` file in the project root to store tokens and secrets for local testing:

```bash
# .env.local
GITHUB_TOKEN=ghp_your_github_token_here
CODECOV_TOKEN=your_codecov_token_here
DATABASE_URL=mysql://user:pass@localhost:3306/testdb
```

**Important**: Never commit `.env.local` to version control. It's already added to `.gitignore`.

### Running Workflows with Environment Files

```bash
# Test CI workflow with environment file
act --env-file .env.local

# Test specific job
act -j validate --env-file .env.local

# Test with secrets
act --env-file .env.local -s GITHUB_TOKEN=ghp_xxx -s CODECOV_TOKEN=xxx

# Dry run (simulation only)
act --dryrun --env-file .env.local

# Test release workflow
act workflow_dispatch --env-file .env.local -s GITHUB_TOKEN=ghp_xxx --input tag=v0.test.1
```

### Workflow Testing

```bash
# Test specific job
just act-job validate

# Test release workflow
just act-release-dry v1.0.0

# Test cargo-dist workflow
just dist-plan

# Build cargo-dist artifacts locally
just dist-build

# Clean up act containers
just act-clean
```

## Performance Profiling

### Benchmarking

```bash
# Run benchmarks (when available)
just bench

# Profile release build
just profile
```

### Memory Analysis

```bash
# Build with debug info
cargo build --release --profile release-with-debug

# Use valgrind (Linux)
valgrind --tool=massif target/release/gold_digger

# Use Instruments (macOS)
instruments -t "Time Profiler" target/release/gold_digger
```

## Getting Help

### Resources

- **Documentation**: This guide and API docs
- **Issues**: [GitHub Issues](https://github.com/EvilBit-Labs/gold_digger/issues)
- **Discussions**: [GitHub Discussions](https://github.com/EvilBit-Labs/gold_digger/discussions)

### Common Commands Reference

```bash
# Quick development check
just check

# Full CI reproduction
just ci-check

# Security scanning
just security

# Generate SBOM
just sbom

# Coverage analysis
just cover

# Release preparation
just release-check

# Show all available commands
just help
```
