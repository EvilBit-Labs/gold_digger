# Gold Digger

Gold Digger is a Rust-based MySQL/MariaDB query tool that exports results to structured data files
(CSV, JSON, TSV). Designed for headless operation and automation workflows, it provides CLI-first
configuration with environment variable fallbacks.

[![CI](https://github.com/EvilBit-Labs/gold_digger/actions/workflows/ci.yml/badge.svg)](https://github.com/EvilBit-Labs/gold_digger/actions/workflows/ci.yml)
[![CodeQL](https://github.com/EvilBit-Labs/gold_digger/actions/workflows/codeql.yml/badge.svg)](https://github.com/EvilBit-Labs/gold_digger/actions/workflows/codeql.yml)
[![Security](https://github.com/EvilBit-Labs/gold_digger/actions/workflows/security.yml/badge.svg)](https://github.com/EvilBit-Labs/gold_digger/actions/workflows/security.yml)
[![codecov](https://codecov.io/github/EvilBit-Labs/gold_digger/graph/badge.svg)](https://codecov.io/github/EvilBit-Labs/gold_digger)
[![GitHub](https://img.shields.io/github/license/EvilBit-Labs/gold_digger)](https://github.com/EvilBit-Labs/gold_digger/blob/main/LICENSE)
[![GitHub issues](https://img.shields.io/github/issues/EvilBit-Labs/gold_digger)](https://github.com/EvilBit-Labs/gold_digger/issues)
[![GitHub Repo stars](https://img.shields.io/github/stars/EvilBit-Labs/gold_digger?style=social)](https://github.com/EvilBit-Labs/gold_digger/stargazers)
[![Maintenance](https://img.shields.io/maintenance/yes/EvilBit-Labs/gold_digger)](https://github.com/EvilBit-Labs/gold_digger/graphs/commit-activity)

## Features

- **CLI-first design** with environment variable fallbacks and comprehensive command-line interface
- **Multiple output formats**: CSV (RFC 4180), JSON with pretty-printing, TSV
- **Safe type handling**: Graceful NULL and type conversion without panics, with intelligent JSON
  type inference
- **Secure TLS support**: Built-in rustls implementation with detailed error handling
- **Comprehensive error handling**: Structured exit codes, intelligent error categorization, and
  actionable error messages
- **Shell completion**: Support for Bash, Zsh, Fish, and PowerShell with easy generation
- **Configuration debugging**: JSON config dump with automatic credential redaction
- **Query flexibility**: Support for inline queries or external query files
- **Verbose logging**: Multi-level verbose output with security-aware credential redaction
- **Empty result handling**: Configurable behavior for queries returning no data
- **Cross-platform**: Linux, macOS, and Windows support with consistent behavior

### Why "Gold Digger"?

The name "Gold Digger" refers to the tool's ability to extract valuable data from databases - just
as gold miners dig through earth to find precious metal, this tool digs through database tables to
extract valuable information and insights. It's designed to help you discover the "golden" data
hidden within your database systems.

## Installation

### Pre-built Binaries (Recommended)

Download pre-built binaries from the
[GitHub Releases](https://github.com/EvilBit-Labs/gold_digger/releases) page, which include:

- **Cross-platform binaries** for Linux (x86_64, ARM64), macOS (Intel, Apple Silicon), and Windows
- **Automated installers** for easy setup
- **Signed artifacts** with Cosign for supply chain security
- **Complete SBOMs** (Software Bill of Materials) for security auditing

#### Quick Install Scripts

```bash
# Shell installer (Linux/macOS)
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/EvilBit-Labs/gold_digger/releases/latest/download/gold_digger-installer.sh | sh

# PowerShell installer (Windows)
powershell -c "irm https://github.com/EvilBit-Labs/gold_digger/releases/latest/download/gold_digger-installer.ps1 | iex"
```

#### Package Managers

```bash
# Homebrew (macOS/Linux)
brew install EvilBit-Labs/tap/gold-digger

# MSI installer (Windows)
# Download from releases page: gold_digger-x86_64-pc-windows-msvc.msi
# Note: The MSI installer does not include license dialogs. The MIT license is available in the LICENSE file and project documentation.
```

### Build from Source

To build and install Gold Digger from source:

```bash
git clone git@github.com:EvilBit-Labs/gold_digger.git
cd gold_digger
cargo install --path .
```

### Build Options

```bash
# Standard build
cargo build --release

# Minimal build (fewer features)
cargo build --release --no-default-features --features "json csv"
```

### TLS Support

Gold Digger includes built-in secure database connections with rustls implementation:

- **Pure Rust TLS**: Consistent cross-platform behavior using rustls with platform certificate store
  integration
- **Platform Certificate Store**: Automatic integration with system certificate stores
  (Windows/macOS/Linux)
- **Flexible Security Controls**: Four distinct TLS validation modes via mutually exclusive CLI
  flags
- **Enhanced Error Messages**: Intelligent error classification with specific CLI flag suggestions
- **Security Warnings**: Prominent warnings for insecure TLS modes with clear guidance

## Usage

Gold Digger supports CLI-first configuration with environment variable fallbacks. CLI flags take
precedence over environment variables.

### CLI Usage

```bash
# Basic usage with CLI flags
gold_digger --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT id, name FROM users LIMIT 10" \
  --output /tmp/results.json

# Pretty-print JSON output
gold_digger --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT id, name FROM users LIMIT 10" \
  --output /tmp/results.json --pretty

# Use query file instead of inline query
gold_digger --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query-file query.sql --output /tmp/results.csv

# Force output format regardless of file extension
gold_digger --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT id, name FROM users LIMIT 10" \
  --output /tmp/results --format csv

# Verbose logging
gold_digger -v --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT COUNT(*) as total FROM users" --output stats.json

# Exit successfully on empty result sets
gold_digger --allow-empty --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT * FROM users WHERE id = 999999" --output empty.json

# Generate shell completions
gold_digger completion bash > ~/.bash_completion.d/gold_digger
gold_digger completion zsh > ~/.zsh/completions/_gold_digger
gold_digger completion fish > ~/.config/fish/completions/gold_digger.fish
gold_digger completion powershell > $PROFILE

# Debug configuration (credentials redacted)
gold_digger --db-url "mysql://user:pass@localhost:3306/mydb" \
  --query "SELECT 1" --output test.json --dump-config

# TLS with custom CA certificate
gold_digger --db-url "mysql://user:pass@internal.db:3306/mydb" \
  --tls-ca-file /path/to/internal-ca.pem \
  --query "SELECT id FROM users LIMIT 5" --output results.json

# Skip hostname verification for development
gold_digger --db-url "mysql://user:pass@dev.db:3306/mydb" \
  --insecure-skip-hostname-verify \
  --query "SELECT COUNT(*) FROM logs" --output count.json

# Accept invalid certificates for testing (DANGEROUS)
gold_digger --db-url "mysql://user:pass@test.db:3306/mydb" \
  --allow-invalid-certificate \
  --query "SELECT * FROM test_data" --output test.csv
```

### CLI Options

| Flag                              | Short | Environment Variable | Description                                             |
| --------------------------------- | ----- | -------------------- | ------------------------------------------------------- |
| `--db-url <URL>`                  | -     | `DATABASE_URL`       | Database connection string                              |
| `--query <SQL>`                   | `-q`  | `DATABASE_QUERY`     | SQL query to execute                                    |
| `--query-file <FILE>`             | -     | -                    | Read SQL from file (mutually exclusive with `--query`)  |
| `--output <FILE>`                 | `-o`  | `OUTPUT_FILE`        | Output file path                                        |
| `--format <FORMAT>`               | -     | -                    | Force output format: `csv`, `json`, or `tsv`            |
| `--pretty`                        | -     | -                    | Pretty-print JSON output                                |
| `--verbose`                       | `-v`  | -                    | Enable verbose logging (repeatable: `-v`, `-vv`)        |
| `--quiet`                         | -     | -                    | Suppress non-error output                               |
| `--allow-empty`                   | -     | -                    | Exit with code 0 even if no results                     |
| `--dump-config`                   | -     | -                    | Print current configuration as JSON                     |
| `--tls-ca-file <FILE>`            | -     | -                    | Use custom CA certificate file for trust anchor pinning |
| `--insecure-skip-hostname-verify` | -     | -                    | Skip hostname verification (keeps chain validation)     |
| `--allow-invalid-certificate`     | -     | -                    | Disable certificate validation entirely (DANGEROUS)     |

**Note**: TLS flags are mutually exclusive - use only one at a time.

### Subcommands

| Command              | Description                       |
| -------------------- | --------------------------------- |
| `completion <SHELL>` | Generate shell completion scripts |

#### Completion Shells

Supported shells for completion generation:

- `bash` - Bash shell completion
- `zsh` - Zsh shell completion
- `fish` - Fish shell completion
- `powershell` - PowerShell completion

### Environment Variables (Fallback)

When CLI flags are not provided, Gold Digger falls back to environment variables:

- `DATABASE_URL`: MySQL/MariaDB connection URL in standard format:
  `mysql://username:password@host:port/database`
- `DATABASE_QUERY`: SQL query to execute
- `OUTPUT_FILE`: Path to output file. Extension determines format:
  - `.csv` → CSV output with RFC 4180 formatting
  - `.json` → JSON output with `{"data": [...]}` structure
  - `.txt` or any other extension → TSV (tab-separated values)

### Example Usage

```bash
# Linux/macOS
OUTPUT_FILE=/tmp/results.json \
DATABASE_URL="mysql://user:pass@localhost:3306/mydb" \
DATABASE_QUERY="SELECT id, name FROM users LIMIT 10" \
gold_digger

# Windows PowerShell
$env:OUTPUT_FILE="C:\temp\results.json"
$env:DATABASE_URL="mysql://user:pass@localhost:3306/mydb"
$env:DATABASE_QUERY="SELECT id, name FROM users LIMIT 10"
gold_digger

# Using justfile for development
just run /tmp/out.json "mysql://user:pass@host:3306/db" "SELECT 1 as test"
```

### Exit Codes

Gold Digger uses structured exit codes for better automation and error handling:

- **0**: Success with results (or empty with `--allow-empty`)
- **1**: Success but no rows returned (use `--allow-empty` to get exit code 0)
- **2**: Configuration error (missing/invalid parameters, mutually exclusive flags, TLS
  configuration issues)
- **3**: Database connection/authentication failure (access denied, connection refused, TLS
  handshake failures)
- **4**: Query execution failure (SQL syntax errors, type conversion errors, database-level errors)
- **5**: File I/O operation failure (cannot read query file, cannot write output file, permission
  errors)

The exit code mapping includes intelligent error detection based on error message patterns,
providing consistent behavior across different failure scenarios.

## Testing

Gold Digger includes comprehensive test suites to ensure reliability and correctness across multiple
database systems and configurations:

### Unit Tests

Run the standard test suite (no external dependencies):

```bash
# Run all unit tests
cargo test

# Run tests with nextest (faster parallel execution)
cargo nextest run

# Run tests excluding Docker-dependent tests
just test-no-docker
```

### Integration Tests

Gold Digger features a comprehensive integration testing framework that validates functionality
against real MySQL and MariaDB databases:

```bash
# Run integration tests (requires Docker)
cargo test --features integration_tests -- --ignored

# Run all tests including integration tests
cargo test --features integration_tests -- --include-ignored

# Using justfile commands
just test-integration  # Run only integration tests
just test-all         # Run all tests including integration tests
```

#### Integration Test Coverage

The integration testing framework validates:

- **Database Compatibility**: Both MySQL (8.0+) and MariaDB (10.11+) support
- **TLS/Non-TLS Connections**: Secure and standard connection modes
- **Data Type Handling**: All MySQL/MariaDB data types including NULL values
- **Output Format Validation**: CSV (RFC4180), JSON, and TSV format compliance
- **Error Scenarios**: Connection failures, SQL errors, and file I/O issues
- **CLI Integration**: Command-line flag precedence and configuration validation
- **Performance Testing**: Large result sets and memory usage validation
- **Security Features**: Credential redaction and TLS certificate validation

#### Test Database Systems

Integration tests use testcontainers to provide isolated database environments:

- **MySQL**: Versions 8.0, 8.1 with both TLS and non-TLS configurations
- **MariaDB**: Version 10.11+ with SSL certificate management
- **Test Data**: Comprehensive schema covering all MySQL data types
- **Fixtures**: Pre-seeded test data including edge cases and Unicode content

### Test Requirements

**Integration Tests:**

- Docker installed and running
- MySQL and MariaDB testcontainers modules (automatically pulled)
- `integration_tests` feature enabled
- Tests are marked with `#[ignore]` by default for CI efficiency

**Unit Tests:**

- No external dependencies required
- Run in CI environments
- Cover configuration, error handling, and format validation

### Test Categories

1. **Unit Tests**: Fast tests without external dependencies
2. **TLS Integration Tests**: Certificate validation and secure connections
3. **Database Integration Tests**: Real database query execution and data validation
4. **CLI Integration Tests**: Command-line interface and configuration testing
5. **Performance Tests**: Large dataset handling and memory usage validation
6. **Cross-Platform Tests**: Consistent behavior across Linux, macOS, and Windows

### Test Coverage

```bash
# Generate coverage report
cargo llvm-cov --html

# Generate coverage for CI
cargo llvm-cov --lcov --output-path lcov.info

# Coverage with integration tests
cargo llvm-cov --features integration_tests --html
```

## Security & Quality Assurance

Gold Digger maintains high security and quality standards for all releases:

### Release Security

- **Signed Artifacts:** All release binaries are cryptographically signed using GitHub attestation
- **Supply Chain Security:** Automated security scanning of all dependencies
- **Software Bill of Materials (SBOM):** Complete dependency information in CycloneDX format
  included with each release
- **Cross-Platform Distribution:** 6 target platforms (ARM64 & x86_64 for macOS/Linux/Windows) via
  cargo-dist

### Quality Standards

- **Cross-Platform Testing:** All releases tested on Linux, macOS, and Windows
- **Code Coverage:** Comprehensive test coverage tracked and maintained
- **Static Analysis:** Automated security analysis with CodeQL
- **Zero-Warning Policy:** All code passes strict linting standards

## Authors

Gold Digger is authored by [@unclesp1d3r](https://www.github.com/EvilBit-Labs)

## Contributing and Feedback

We welcome your feedback and suggestions for Gold Digger! If you have any ideas for new features,
encounter any bugs or issues, or have any other comments, please reach out to us by creating an
issue on our [GitHub repository](https://github.com/EvilBit-Labs/gold_digger/issues).

If you're interested in contributing to Gold Digger, we encourage you to submit a pull request.
Please see our `CONTRIBUTING.md` for more information on how to get started.

Our team is committed to providing a welcoming and inclusive environment for all contributors.
Please adhere to our `CODE_OF_CONDUCT.md` when contributing to the project.

Thank you for your interest in Gold Digger, and we look forward to hearing from you!

## License

[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2Funclesp1d3r%2Fgold_digger.svg?type=large)](https://app.fossa.com/projects/git%2Bgithub.com%2Funclesp1d3r%2Fgold_digger?ref=badge_large)
