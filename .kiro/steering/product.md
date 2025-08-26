---
inclusion: always
---

# Gold Digger Product Requirements

Gold Digger is a MySQL/MariaDB query tool that exports structured data to CSV/JSON/TSV formats. Core principles: type safety, security, and CLI-first design for automation workflows.

## Product Identity

**Purpose**: Headless database query automation for CI/CD pipelines and data export workflows\
**Design Philosophy**: Offline-first, environment-driven, structured output\
**Target Users**: DevOps engineers, data analysts, automation scripts

## Critical Safety Requirements

### Security (NON-NEGOTIABLE)

- **NEVER** log credentials or connection strings - implement automatic redaction
- **NEVER** make external network calls at runtime (offline-first design)
- **ALWAYS** validate and sanitize all user inputs

## Product Architecture

### Configuration Model

- **CLI-first**: Flags override environment variables
- **Environment fallback**: Support automation workflows
- **No config files**: Simplifies deployment and security

### Output Format Strategy

- **CSV**: Standard business format (RFC4180 compliant)
- **JSON**: API integration format with deterministic field ordering
- **TSV**: Fallback format for simple parsing
- **Auto-detection**: File extension determines format (.csv/.json/fallback to TSV)
- **Format override**: `--format` flag for explicit control

### NULL Value Handling Policy

- **CSV/TSV**: NULL → empty string (standard business practice)
- **JSON**: NULL → `null` (JSON standard compliance)

## User Interface Design

### Required Inputs (CLI flag OR environment variable)

- **Database connection**: `--db-url` / `DATABASE_URL`
- **SQL query**: `--query` OR `--query-file` / `DATABASE_QUERY`
- **Output destination**: `--output` / `OUTPUT_FILE`

### User Experience Rules

- **Mutually exclusive options**: Prevent conflicting configurations
  - Query source: `--query` vs `--query-file`
  - Output verbosity: `--verbose` vs `--quiet`
- **Clear error messages**: Specific exit codes for different failure types
- **Automation-friendly**: Predictable behavior for scripting

### Exit Code Contract

- **0**: Successful execution with data (or empty results with `--allow-empty`)
- **1**: Successful execution but no data returned
- **2**: User configuration error (missing params, conflicts)
- **3**: Database connectivity/authentication failure
- **4**: Query execution failure (SQL errors, type conversion)
- **5**: File system operation failure

## Quality Standards

### Reliability Requirements

- **Zero tolerance for runtime panics** - all database operations must be safe
- **Deterministic output** - same query produces identical results across runs
- **Cross-platform compatibility** - consistent behavior on Windows/macOS/Linux

### Performance Expectations

- **Memory usage**: Currently O(row_count × row_width) - loads all results into memory
- **Startup time**: Target under 250ms for CLI responsiveness
- **Connection model**: Single database connection per execution (simple and reliable)

### Feature Completeness

- **Core formats**: CSV, JSON, TSV support with proper standards compliance
- **Configuration flexibility**: CLI flags with environment variable fallbacks
- **Error handling**: Comprehensive exit codes for automation integration
- **Security**: Credential protection and offline-first operation
