# Requirements Document

## Introduction

Gold Digger is a Rust-based command-line tool that executes MySQL and MariaDB database queries and
exports results to structured data files. The tool is designed for headless operation, making it
ideal for automated database reporting, monitoring, and data extraction workflows.

## Requirements

### Requirement 1

**User Story:** As a developer, I want to configure database connections via CLI flags or
environment variables, so that I can easily integrate the tool into different environments and
automation workflows.

#### Acceptance Criteria

1. WHEN I provide a --db-url flag THEN the system SHALL use that URL for database connection
2. WHEN I set DATABASE_URL environment variable AND no --db-url flag is provided THEN the system
   SHALL use the environment variable
3. WHEN both --db-url flag and DATABASE_URL environment variable are provided THEN the system SHALL
   prioritize the CLI flag
4. IF no database URL is provided THEN the system SHALL exit with code 2 (configuration error)

### Requirement 2

**User Story:** As an automation engineer, I want to specify SQL queries via CLI flags or files, so
that I can execute both inline queries and complex queries stored in files.

#### Acceptance Criteria

1. WHEN I provide a --query flag with SQL text THEN the system SHALL execute that query
2. WHEN I provide a --query-file flag with a file path THEN the system SHALL read and execute the
   SQL from that file
3. WHEN I set DATABASE_QUERY environment variable AND no query flags are provided THEN the system
   SHALL use the environment variable
4. WHEN both --query and --query-file flags are provided THEN the system SHALL exit with code 2
   (mutually exclusive error)
5. IF no query is provided THEN the system SHALL exit with code 2 (configuration error)

### Requirement 3

**User Story:** As a data engineer, I want to export query results in multiple formats (CSV, JSON,
TSV), so that I can integrate with different downstream processing tools.

#### Acceptance Criteria

1. WHEN output file has .csv extension THEN the system SHALL write RFC 4180 compliant CSV with
   headers
2. WHEN output file has .json extension THEN the system SHALL write structured JSON as {"data":
   [{"col": "val"}]}
3. WHEN output file has .tsv extension OR no recognized extension THEN the system SHALL write
   tab-delimited format
4. WHEN --format flag is provided THEN the system SHALL override format detection from file
   extension
5. WHEN --pretty flag is provided with JSON format THEN the system SHALL format JSON with
   indentation

### Requirement 4

**User Story:** As a system administrator, I want standardized exit codes for different error
conditions, so that I can properly handle errors in automation scripts.

#### Acceptance Criteria

1. WHEN query executes successfully with results THEN the system SHALL exit with code 0
2. WHEN query executes successfully but returns no rows THEN the system SHALL exit with code 1
3. WHEN configuration is invalid or missing THEN the system SHALL exit with code 2
4. WHEN database connection or authentication fails THEN the system SHALL exit with code 3
5. WHEN query execution fails THEN the system SHALL exit with code 4
6. WHEN file I/O operations fail THEN the system SHALL exit with code 5
7. WHEN --allow-empty flag is provided AND query returns no rows THEN the system SHALL exit with
   code 0

### Requirement 5

**User Story:** As a security analyst, I want TLS/SSL connection support, so that I can securely
connect to production databases.

#### Acceptance Criteria

1. WHEN TLS connection is configured programmatically THEN the system SHALL use MySQL native-tls
   features via OptsBuilder::ssl_opts()
2. WHEN TLS handshake fails THEN the system SHALL exit with code 3 (connection error)
3. WHEN credentials are processed THEN the system SHALL never log or display DATABASE_URL contents
4. NOTE: URL-based SSL parameters are not supported by the mysql crate; see F006 in
   project_spec/requirements.md for programmatic TLS configuration

### Requirement 6

**User Story:** As a data engineer processing large datasets, I want streaming export mode, so that
I can handle result sets that don't fit in memory.

#### Acceptance Criteria

1. WHEN processing large result sets THEN the system SHALL stream rows without loading all into
   memory
2. WHEN streaming is active THEN memory usage SHALL scale with row width, not row count
3. WHEN streaming fails THEN the system SHALL exit with code 4 (query execution error)

### Requirement 7

**User Story:** As a DevOps engineer, I want structured logging with credential protection, so that
I can debug issues without exposing sensitive information.

#### Acceptance Criteria

1. WHEN --verbose flag is provided THEN the system SHALL output structured logs using tracing crate
2. WHEN logging database operations THEN the system SHALL redact connection URLs by default
3. WHEN credentials appear in any log output THEN the system SHALL never log DATABASE_URL or
   password information
4. WHEN --quiet flag is provided THEN the system SHALL suppress all output except errors

### Requirement 8

**User Story:** As a developer, I want shell completion support, so that I can efficiently use the
tool in interactive environments.

#### Acceptance Criteria

1. WHEN completion subcommand is called with bash THEN the system SHALL generate bash completion
   script
2. WHEN completion subcommand is called with zsh THEN the system SHALL generate zsh completion
   script
3. WHEN completion subcommand is called with fish THEN the system SHALL generate fish completion
   script
4. WHEN completion subcommand is called with PowerShell THEN the system SHALL generate PowerShell
   completion script

### Requirement 9

**User Story:** As an operations engineer, I want configuration introspection capabilities, so that
I can audit and verify tool configuration in production environments.

#### Acceptance Criteria

1. WHEN --version flag is provided THEN the system SHALL display version information and exit
2. WHEN --help flag is provided THEN the system SHALL display usage information and exit
3. WHEN --dump-config flag is provided THEN the system SHALL output current configuration as JSON
4. WHEN configuration precedence is applied THEN CLI flags SHALL override environment variables
   SHALL override defaults

### Requirement 10

**User Story:** As a database administrator, I want support for additional MySQL data types, so that
I can export complex data without type conversion errors.

#### Acceptance Criteria

1. WHEN query results contain NULL values THEN the system SHALL handle them safely without panicking
2. WHEN query results contain non-string types THEN the system SHALL convert them to string
   representation safely
3. WHEN type conversion fails THEN the system SHALL provide meaningful error message and exit with
   code 4
4. WHEN additional MySQL types feature is enabled THEN the system SHALL support DECIMAL, BIGINT, and
   other extended types
