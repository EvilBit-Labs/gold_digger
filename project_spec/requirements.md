# Requirements Document: Gold Digger

**Version**: 1.0
**Status**: Draft
**Author(s)**: EvilBit-Labs
**Date Created**: 2025-01-15
**Last Modified**: 2025-01-15

---

## Table of Contents

[TOC]

---

## Introduction & Scope

### Project Overview

- **Project Description and Purpose**
  Gold Digger is a Rust-based command-line tool that executes MySQL and MariaDB database queries and exports results to structured data files. The tool is designed for headless operation, making it ideal for automated database reporting, monitoring, and data extraction workflows.

- **Project Goals and Objectives**

  - Provide an offline-first, zero-dependency binary for reliable database query automation
  - Support deterministic output formats (CSV, JSON, TSV) for downstream processing
  - Enable easy integration with development workflows, CI/CD pipelines, and automation systems
  - Design as a composable component for larger data processing pipelines and toolsets
  - Maintain minimal operational friction with environment variable configuration
  - Ensure cross-platform compatibility for diverse development and deployment environments

- **Target Audience and Stakeholders**

  - **Developers**: Database query automation for development workflows and testing
  - **Database Administrators (DBAs)**: Automated reporting and data extraction
  - **Automation Engineers**: CI/CD pipeline data validation and automated data processing
  - **Data Engineers**: ETL pipeline integration and data processing workflows
  - **Machine Learning Engineers**: Data extraction for ML training pipelines and model validation
  - **System Administrators**: Monitoring queries and health checks
  - **Security Analysts**: Data collection for compliance and audit workflows
  - **Container Engineers**: Docker-based database query automation with environment variable configuration
  - **Pipeline Architects**: Integration into complex data processing and automation systems

- **Project Boundaries and Limitations**
  Gold Digger will *not* provide:

  - Interactive database client functionality
  - Query composition or SQL generation features
  - Schema migration or database administration tools
  - ORM or database abstraction capabilities
  - Server or daemon mode operation
  - Support for non-MySQL database engines

### Scope Definition

- **In-scope Features and Functionality**

  - MySQL/MariaDB query execution with connection pooling
  - CSV, JSON, and tab-delimited (TSV) output formats
  - Environment variable and CLI flag configuration for automation workflows
  - TLS/SSL connection support via MySQL native client
  - Cross-platform binary distribution (macOS, Windows, Linux)
  - Streaming output for large result sets
  - Comprehensive error handling with meaningful exit codes
  - Shell completion generation for major shells
  - Pipeline-friendly design with standardized exit codes and output formats

- **Out-of-scope Items**

  - Interactive terminal user interface (TUI)
  - Multi-database engine support (PostgreSQL, SQLite, etc.)
  - Query planning, optimization, or performance analysis
  - Database schema introspection or documentation generation
  - Long-running server or daemon processes
  - Built-in query scheduling or cron functionality
  - Real-time data streaming or change data capture
  - Complex data transformation or ETL capabilities
  - Orchestration or workflow management (designed to be orchestrated by external tools)

- **Success Criteria and Acceptance Criteria**

  - **Functional**: Tool successfully executes queries and produces correct output formats
  - **Performance**: Startup time under 250ms; memory usage scales with row width, not row count
  - **Security**: No telemetry; credentials never logged; all CI security gates pass
  - **Compliance**: FOSSA license scanning passes; SBOM and provenance attached to releases
  - **Cross-platform**: CI passes on Ubuntu, Windows, and macOS
  - **Documentation**: Complete installation, usage, and verification guides
  - **Integration**: Successfully integrates with common data processing tools and pipeline frameworks

- **Timeline and Milestones**

  - **v0.3.0** — CLI and Config Parity
  - **v0.4.0** — Streaming and Pipeline Compliance
  - **v1.0.0** — GA Hardening

### Context and Background

- **Business Context and Justification**
  Database query automation is a critical need across development, data engineering, and various automation workflows. Existing solutions often require heavy runtime dependencies, complex configuration, or lack offline-first design principles essential for lightweight, self-contained deployments. This tool is designed to be a reliable, composable component that can be easily integrated into larger data processing pipelines, ETL workflows, and automation systems.

- **Previous Work and Dependencies**
  Current implementation (v0.2.5) provides basic functionality with:

  - Environment variable configuration (DATABASE_URL, DATABASE_QUERY, OUTPUT_FILE)
  - Modular output format writers (CSV, JSON, TSV)
  - Rust-based implementation using mysql crate with minimal features
  - GitHub Actions CI/CD with semantic versioning

- **Assumptions and Constraints**

  - Developers and automation engineers provide trusted SQL queries (no query sanitization)
  - Database connectivity and credentials managed externally
  - Results fit within reasonable memory constraints or use streaming
  - No telemetry or external service dependencies in runtime artifacts

- **Risk Assessment Overview**

  - **SQL Execution Risk**: Developer-provided queries trusted; no injection protection needed
  - **Memory Usage Risk**: Large result sets require streaming implementation
  - **Credential Exposure Risk**: Database URLs must never be logged or displayed
  - **Dependency Risk**: Minimize external crate dependencies for supply chain security

---

## Functional Requirements

### Core Features

- **F001**: Accept database URL via environment variable DATABASE_URL and CLI flag --db-url (flag overrides environment)

- **F002**: Accept SQL query via environment variable DATABASE_QUERY or CLI flags --query/--query-file (mutually exclusive; flag overrides environment)

- **F003**: Determine output format by OUTPUT_FILE extension; support .csv, .json, .tsv file extensions; allow --format flag override

- **F004**: Write header row with column names; preserve column order as returned from database

- **F005**: Provide standardized exit codes: 0 (success), 1 (no rows found), 2 (usage/config error), 3 (connection/auth error), 4 (query execution error), 5 (file I/O error)

- **F006**: Support TLS/SSL connections via MySQL native-tls features; TLS/SSL must be configurable programmatically via the crate's native-tls features using `SslOpts` and `OptsBuilder::ssl_opts()`; URL-based ssl-mode parameters are not supported by the chosen mysql crate (see [mysql crate SSL documentation](https://docs.rs/mysql/26.0.1/mysql/struct.SslOpts.html) for programmatic TLS setup examples)

- **F007**: Implement streaming export mode for large result sets to avoid loading all rows into memory simultaneously

- **F008**: Provide structured logging via --verbose flag using tracing crate; never log credentials; redact connection URLs by default

- **F009**: Generate shell completion scripts for common shells (bash, zsh, fish) via clap_complete

- **F010**: Produce deterministic JSON output with optional pretty printing (--pretty flag; disabled by default for consistency)

- **F011**: Implement configuration precedence: CLI flags > environment variables > defaults; document precedence clearly

- **F012**: Provide --version, --help, and machine-readable --dump-config (JSON format) for operational auditability

- **F013**: Return nonzero exit code on empty result set (current behavior); add --allow-empty flag to return success on empty results

- **F014**: Support additional MySQL data types via existing feature flags; ensure safe string conversion and null value handling

### User Stories and Use Cases

#### Use Case 1: DBA Scheduled Export

**Scenario**: Database administrator needs daily JSON export with TLS for compliance reporting.

```bash
# Environment file: /opt/reports/db.env
DATABASE_URL="mysql://user:pass@db.internal:3306/audit"
DATABASE_QUERY="SELECT user_id, action, timestamp FROM audit_log WHERE DATE(timestamp) = CURDATE()"
```

```ini
# Systemd service unit: /etc/systemd/system/daily-audit.service
[Unit]
Description=Daily audit log export
After=network.target

[Service]
Type=oneshot
EnvironmentFile=/opt/reports/db.env
ExecStart=/bin/sh -lc 'gold_digger --verbose --output "/var/reports/daily-audit-$(date +%Y%m%d).json"'
User=reports
Group=reports
```

```ini
# Systemd timer unit: /etc/systemd/system/daily-audit.timer
[Unit]
Description=Run daily audit export at 2 AM
Requires=daily-audit.service

[Timer]
OnCalendar=*-*-* 02:00:00
Persistent=true

[Install]
WantedBy=timers.target
```

#### Use Case 2: SOC Analyst Ad-hoc Query

**Scenario**: Security analyst on airgapped workstation needs CSV output for SIEM ingestion.

```bash
# Query file: suspicious_logins.sql
SELECT
    ip_address,
    username,
    failed_attempts,
    last_attempt
FROM login_failures
WHERE failed_attempts > 5
    AND last_attempt > DATE_SUB(NOW(), INTERVAL 1 HOUR);

# Execution
gold_digger \
    --db-url "mysql://readonly:secret@security-db:3306/logs" \
    --query-file suspicious_logins.sql \
    --output /tmp/suspicious_logins.csv \
    --format csv
```

#### Use Case 3: Data Pipeline Extraction

**Scenario**: Automated data extraction for various downstream processing workflows.

```bash
# Data pipeline extraction
export DATABASE_URL="mysql://data_user:${DATA_DB_PASS}@${DB_HOST}/analytics"
export DATABASE_QUERY="SELECT user_id, action, timestamp FROM user_events WHERE DATE(timestamp) = CURDATE()"

if gold_digger --output /tmp/daily_events.json --allow-empty; then
    echo "Daily events extracted successfully"
    # Continue with data processing pipeline...
else
    echo "Data extraction failed"
    exit 1
fi
```

#### Use Case 4: Development Workflow Automation

**Scenario**: Developer automating database queries for testing and development workflows.

```bash
# Development environment setup
export DATABASE_URL="mysql://dev:dev_pass@localhost:3306/dev_db"
export DATABASE_QUERY="SELECT COUNT(*) as user_count FROM users WHERE created_at > DATE_SUB(NOW(), INTERVAL 1 DAY)"

gold_digger --output /tmp/daily_users.csv --verbose
echo "Daily user count: $(tail -n 1 /tmp/daily_users.csv | cut -d',' -f2)"
```

#### Use Case 5: Large-Scale Data Pipeline Integration

**Scenario**: Integration into complex data processing pipelines and ETL workflows.

```bash
# Part of a larger data pipeline
export DATABASE_URL="mysql://pipeline:${PIPELINE_PASS}@${DB_HOST}/warehouse"
export DATABASE_QUERY="SELECT * FROM raw_events WHERE processed = 0 LIMIT 10000"

# Extract data for processing
if gold_digger --output /tmp/batch_events.json; then
    # Process with other tools in the pipeline
    jq -r '.data[] | select(.event_type == "purchase")' /tmp/batch_events.json | \
    process_events.py --input - --output /tmp/processed_events.csv

    # Update processing status
    update_processing_status.py --batch-id $(date +%Y%m%d_%H%M%S)
else
    echo "Data extraction failed"
    exit 1
fi
```

#### Use Case 6: Containerized Database Automation

**Scenario**: Docker-based deployment with environment variable configuration for easy container orchestration.

```bash
# CI pipeline step
export DATABASE_URL="mysql://monitor:${DB_PASS}@${DB_HOST}/health"
export DATABASE_QUERY="SELECT service, status FROM service_health WHERE status != 'healthy'"

if gold_digger --output /tmp/health.json --allow-empty; then
    UNHEALTHY=$(jq '.data | length' /tmp/health.json)
    if [ "$UNHEALTHY" -gt 0 ]; then
        echo "$UNHEALTHY unhealthy services found"
        exit 1
    fi
    echo "All services healthy"
else
    echo "Health check query failed"
    exit 1
fi
```

#### Use Case 7: Containerized Database Automation

**Scenario**: Docker-based deployment with environment variable configuration for easy container orchestration.

```bash
# Docker run with shell expansion for dynamic filenames
docker run --rm \
  -e DATABASE_URL="mysql://user:pass@db-host:3306/production" \
  -e DATABASE_QUERY="SELECT user_id, action, timestamp FROM audit_log WHERE DATE(timestamp) = CURDATE()" \
  -v $(pwd)/output:/app/output \
  gold-digger:latest \
  /bin/sh -lc "gold_digger --output \"/app/output/daily-audit-\$(date +%Y%m%d).json\""

# Docker Compose with .env file (static configuration)
# docker-compose.yml
version: '3.8'
services:
  gold-digger:
    image: gold-digger:latest
    env_file: .env
    volumes:
      - ./output:/app/output
    restart: "no"
    command: ["/bin/sh", "-lc", "gold_digger --output \"/app/output/daily-audit-$(date +%Y%m%d).json\""]

# .env file (no shell substitution - use static paths or generate in command)
DATABASE_URL=mysql://user:pass@db-host:3306/production
DATABASE_QUERY=SELECT user_id, action, timestamp FROM audit_log WHERE DATE(timestamp) = CURDATE()

# Note: Shell substitution $(date ...) is performed by the shell and works in shell-evaluated commands
# but not in Docker Compose .env files or environment files that are not processed by a shell.
# Use /bin/sh -lc wrapper for command evaluation or implement --output-template flag in the tool.
# Example: gold_digger --output-template "/app/output/daily-audit-{date}.json"
```

### Feature Priority Matrix

| Priority   | Features                     | Rationale                                                                    |
| ---------- | ---------------------------- | ---------------------------------------------------------------------------- |
| **High**   | F001-F006, F011, F012        | Core operation, security, config precedence, help/version                    |
| **Medium** | F007, F008, F009, F010, F013 | Streaming, logging, shell completion, JSON formatting, empty result handling |
| **Low**    | Future enhancements          | NDJSON output, Parquet format, query timing statistics                       |

### Performance Requirements

- **CLI Startup**: Tool initialization and argument parsing under 250ms on typical development hardware
- **File Size Limits**: Verified operation with output files up to 2GB (document OS filesystem constraints)
- **Time-to-Completion**: Tool overhead less than 5% of total query runtime when using streaming writers
- **Memory Usage**: O(row_width) steady-state memory when streaming enabled; avoid `Vec<Row>` accumulation for large result sets

---

## User Interface Requirements

### CLI Framework

- **Framework**: clap v4 with derive macros for type-safe argument parsing
- **Completion**: clap_complete for shell completion generation
- **Help Generation**: Automatic help text generation with examples and flag descriptions

### Command Line Interface

```text
gold_digger [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -o, --output <FILE>           Output file path (overrides OUTPUT_FILE env)
    --db-url <URL>                Database connection URL (overrides DATABASE_URL env)
    --query <SQL>                  SQL query string (mutually exclusive with --query-file)
    --query-file <FILE>           File containing SQL query (mutually exclusive with --query)
    --format <FORMAT>             Output format override [csv|json|tsv]
    -v, --verbose                 Enable verbose logging (repeatable for levels)
    --quiet                       Suppress all output except errors
    --pretty                      Pretty-print JSON output (ignored for other formats)
    --allow-empty                 Exit successfully on empty result sets
    --version                     Print version information
    --dump-config                 Print current configuration as JSON
    -h, --help                    Print help information

SUBCOMMANDS:
    completion                    Generate shell completion scripts
```

### Output Formats

- **CSV**: RFC 4180 compliant with QuoteStyle::Necessary; headers in first row
- **JSON**: Structured as `{"data": [{"col": "val", ...}]}` with deterministic key ordering
- **TSV**: Tab-delimited with minimal quoting (QuoteStyle::Necessary)

### Accessibility and Terminal Compatibility

- **Color Support**: No ANSI colors by default; respects TERM=dumb environment
- **Error Messages**: Clear, actionable error messages without sensitive information exposure
- **Path Handling**: Cross-platform compatible file path processing
- **Character Encoding**: UTF-8 input/output with proper error handling

---

## Technical Specifications

### Language and Runtime

- **Language**: Rust 2021 edition
- **MSRV**: Minimum Supported Rust Version 1.74+ (documented and CI-enforced)
- **Target Platforms**: x86_64 and aarch64 for Linux, macOS, Windows

### Core Libraries and Tooling

#### Production Dependencies

```toml
mysql = { version = "26.0.1", features = [
  "minimal",
  "native-tls",
], default-features = false }
clap = { version = "4", features = ["derive", "env"] }
clap_complete = "4"
csv = "1.3"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
serde_json = "1"
```

#### Development Dependencies

```toml
criterion = { version = "0.5", features = ["html_reports"] }
insta = "1"
rstest = "0.18"
assert_cmd = "2"
tempfile = "3"
proptest = "1"                                               # optional
```

### Build Tools and Automation

#### Justfile Recipes

```justfile
# Development setup
setup:           # Install development dependencies and tools
install-tools:   # Install Rust toolchain components

# Quality gates (CI parity)
fmt:            # Format code with cargo fmt
lint:           # Run clippy with -D warnings
test:           # Run all tests with nextest
cover:          # Generate coverage reports with tarpaulin
security:       # Run security analysis (audit, deny)

# Build and release
build:          # Build release binaries
release-dry:    # Simulate release process
sbom:           # Generate SBOM for inspection

# Documentation
docs:           # Serve documentation locally
docs-build:     # Build documentation for verification
```

### CI/CD & Testing

#### GitHub Actions Workflows

- **ci.yml**: PR and push checks (lint, test, security, coverage)
- **dist-workspace.toml** or **Cargo.toml [workspace.metadata.dist]**: Configuration for cargo-dist (artifact/distribution settings, workspace level). This repository uses one of these locations for cargo-dist configuration.
- **release.yml**: Release artifact construction and publishing

#### Quality Gates

- **Linting**: `cargo clippy -- -D warnings` (fail on any warnings)
- **Formatting**: `cargo fmt --check` (enforce consistent style)
- **Testing**: `cargo nextest run` with cross-platform matrix
- **Coverage**: `cargo tarpaulin` with Codecov upload
- **Security**: CodeQL, Syft SBOM, Grype vulnerability scanning
- **License**: FOSSA scanning with PR enforcement

### Packaging and Distribution

#### cargo-dist Configuration

```toml
[workspace.metadata.dist]
cargo-dist-version = "0.8.0"
ci = ["github"]
installers = ["shell", "powershell"]
targets = [
  "x86_64-unknown-linux-gnu",
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-pc-windows-msvc",
]
```

#### Release Artifacts

- Cross-platform archives with embedded checksums
- SHA256SUMS.txt signed with Cosign keyless OIDC
- SLSA Level 3 provenance attestations
- SPDX JSON SBOM via Syft
- Optional Homebrew tap formula

---

## Security Requirements

### Code Security

- **Secret Management**: No hardcoded credentials; DATABASE_URL never logged or displayed
- **Input Validation**: File path validation; SQL injection not applicable (trusted operator input)
- **Error Handling**: Error messages avoid leaking sensitive connection details
- **Memory Safety**: Leverage Rust's memory safety guarantees; avoid unsafe code blocks

### Operational Security

- **Offline-First Design**: No telemetry, analytics, or external service calls in runtime
- **Self-Contained Binaries**: Static linking where possible; minimal external dependencies
- **Credential Handling**: Support secure credential passing via environment variables
- **Audit Trail**: Machine-readable configuration dump for operational auditability

### Output Security

- **Data Integrity**: Deterministic output formats prevent credential leakage
- **Schema Documentation**: Well-defined JSON schema prevents information disclosure
- **File Permissions**: Respect system umask; no automatic permission elevation

### Supply Chain Security

- **SBOM Generation**: Syft-generated SPDX JSON for all releases
- **Vulnerability Scanning**: Grype scanning of dependencies and build artifacts
- **License Compliance**: FOSSA enforcement preventing non-approved license introduction
- **Cryptographic Signatures**: Cosign keyless OIDC signing of all release artifacts
- **Provenance**: SLSA Level 3 attestations via slsa-github-generator

### Advisory Tools (Non-blocking)

- **CodeRabbit.ai**: AI-powered code review and security analysis
- **OSSF Scorecard**: Repository security posture assessment
- **Snyk**: Dependency vulnerability analysis

---

## System Architecture

### System Components

1. **CLI Interface**: Argument parsing and validation using clap
2. **Configuration Resolver**: Precedence handling (flags > env > defaults)
3. **Database Connector**: MySQL connection management with pooling
4. **Query Executor**: SQL execution with result streaming capability
5. **Row Transformer**: MySQL Row to String vector conversion
6. **Format Writers**: Modular CSV/JSON/TSV output generation
7. **Logging System**: Structured logging with credential redaction

### Data Flow Diagrams

```mermaid
graph LR
    CLI[CLI Args/Env] --> Config[Config Resolver]
    Config --> DB[(MySQL/MariaDB)]
    DB -->|Rows Stream| Transform[Row Transformer]
    Transform --> Writer[Format Writer]
    Writer -->|CSV/JSON/TSV| File[(Output File)]

    Config --> Logger[Tracing Logger]
    Logger --> Stderr[Stderr Output]
```

### Data Processing Flow

1. **Configuration Resolution**: CLI flags override environment variables override defaults
2. **Database Connection**: Establish connection with TLS support and connection pooling
3. **Query Execution**: Execute SQL with streaming result processing
4. **Row Transformation**: Convert MySQL Row types to String representations
5. **Format Writing**: Apply format-specific serialization (CSV/JSON/TSV)
6. **File Output**: Write structured data to specified output file
7. **Error Handling**: Provide meaningful exit codes and error messages

### Deployment Strategy

- **Binary Distribution**: GitHub Releases with cross-platform archives
- **Package Management**: Optional Homebrew tap for macOS users
- **Container Support**: Provide minimal container image for containerized deployments with environment variable configuration
- **Verification**: Complete offline verification workflow with checksums and signatures

### Error Handling Architecture

```rust
#[derive(Debug, thiserror::Error)]
pub enum GoldDiggerError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Database connection failed")]
    Connection(#[source] mysql::Error),
    #[error("Query execution failed")]
    Query(#[source] mysql::Error),
    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Format serialization error")]
    Format(#[source] Box<dyn std::error::Error>),
}
```

---

## Development and Automation Standards

| Principle                | Implementation                                                                     |
| ------------------------ | ---------------------------------------------------------------------------------- |
| **Offline-first**        | No telemetry; self-contained binaries; development environment ready               |
| **Developer-focused**    | CLI-first interface; minimal flags; clear exit codes; comprehensive error messages |
| **Transparent outputs**  | Documented CSV/JSON/TSV schemas; deterministic formatting                          |
| **Ethical distribution** | No tracking; complete SBOM/signatures/provenance attached                          |
| **Sustainable design**   | Modular architecture; comprehensive tests; maintainable codebase                   |

### Pipeline Compliance

- **Conventional Commits**: Semantic versioning via cargo-dist
- **Security Gates**: CodeQL, Syft, Grype, FOSSA as merge blockers
- **Cross-platform**: CI matrix testing on Ubuntu, Windows, macOS
- **Supply Chain**: SLSA L3 provenance; Cosign keyless signing
- **Quality**: cargo clippy -D warnings; comprehensive test coverage

### Testing Strategy Alignment

- **Unit Tests**: Core functionality (row transformation, format writers)
- **Integration Tests**: Database connectivity with testcontainers
- **CLI Tests**: End-to-end binary testing with assert_cmd
- **Property Tests**: Data transformation correctness with proptest
- **Snapshot Tests**: Output format validation with insta
- **Benchmark Tests**: Performance regression detection with criterion
- **Format Tests**: Output format validation for various downstream processing workflows

---

## Document Metadata

| Field               | Value        |
| ------------------- | ------------ |
| **Version**         | 1.0          |
| **Created Date**    | 2025-01-15   |
| **Last Modified**   | 2025-08-16   |
| **Author(s)**       | EvilBit-Labs |
| **Approval Status** | Draft        |

---

## Glossary & References

### Glossary

- **CLI**: Command Line Interface
- **SBOM**: Software Bill of Materials
- **SLSA**: Supply-chain Levels for Software Artifacts
- **OIDC**: OpenID Connect
- **TSV**: Tab-Separated Values
- **JSON**: JavaScript Object Notation
- **CSV**: Comma-Separated Values
- **TLS/SSL**: Transport Layer Security/Secure Sockets Layer

### References

- **MySQL Rust Crate**: <https://docs.rs/mysql/>
- **Clap Argument Parser**: <https://docs.rs/clap/>
- **Conventional Commits**: <https://conventionalcommits.org/>
- **SLSA Framework**: <https://slsa.dev/>
- **Cosign**: <https://docs.sigstore.dev/cosign/overview/>

---

## Guidance Notes

- **Version Discrepancy**: Current codebase shows v0.2.5 in Cargo.toml but CHANGELOG.md references v0.2.6. Recommend resolving this discrepancy in next release.
- **License Retention**: Maintain MIT license for open source compatibility.
- **Single Maintainer**: Configure branch protection rules appropriate for single-maintainer workflow.
- **Milestone Naming**: Use version numbers (v0.3.0, v0.4.0, v1.0.0) with descriptive context in milestone descriptions.
- **Code Review**: Enable CodeRabbit.ai for automated PR reviews; disable GitHub Copilot automatic reviews per user preference.
- **Testing Framework**: Emphasize criterion for benchmarking and insta for snapshot testing per user preference.
- **General Purpose**: Ensure output formats are compatible with common data processing tools and workflows.
