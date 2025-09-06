# Implementation Plan

- [x] 1. Basic project structure and core functionality

  - Basic Cargo.toml with mysql, anyhow, csv, serde_json dependencies ✓
  - Module structure with csv.rs, json.rs, tab.rs format modules ✓
  - Environment variable-based configuration (OUTPUT_FILE, DATABASE_URL, DATABASE_QUERY) ✓
  - Basic format detection by file extension ✓
  - _Requirements: Basic functionality exists but needs CLI enhancement_

- [x] 2. Add CLI interface and argument parsing

  - Add clap v4 dependency to Cargo.toml ✓
  - Create CLI structure with clap derive macros for all arguments ✓
  - Implement Commands enum for subcommands (completion) ✓
  - Add argument validation and mutual exclusion rules (--query vs --query-file) ✓
  - Maintain backward compatibility with environment variables ✓
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 8.1, 8.2, 8.3, 8.4_

- [x] 3. Implement proper error handling and exit codes

  - Define exit code constants and mapping functions ✓
  - Implement standardized exit codes (0-5) ✓
  - Add exit_code() method returning proper codes for different error types ✓
  - Implement credential redaction for error messages ✓
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7_

- [x] 4. Add configuration resolution system

  - Create configuration resolution functions with precedence: CLI flags > environment variables ✓
  - Add validation for required fields (database_url, query, output_path) ✓
  - Implement --dump-config functionality with credential redaction ✓
  - Add OutputFormat enum with proper format detection and override ✓
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 5. Enhance database connectivity

  - Add TLS/SSL configuration via SslOpts and OptsBuilder ✓
  - Implement proper connection error handling with standardized exit codes ✓
  - Add connection validation and timeout handling ✓
  - _Requirements: 5.1, 5.2, 5.3_

- [x] 6. Fix JSON output determinism

  - Replace HashMap with BTreeMap in json.rs for deterministic key ordering ✓
  - Add --pretty flag support for formatted JSON output ✓
  - Ensure consistent JSON structure: {"data": [...]} ✓
  - _Requirements: 3.5_

- [x] 7. Add shell completion support

  - Add clap_complete dependency ✓
  - Create completion subcommand handler for bash, zsh, fish, PowerShell ✓
  - Generate and output completion scripts ✓
  - _Requirements: 8.1, 8.2, 8.3, 8.4_

- [x] 8. Add operational features

  - Implement --allow-empty flag logic for empty result sets ✓
  - Add --version flag with build information ✓
  - Create comprehensive --help output with examples ✓
  - Add proper file I/O error handling and directory creation ✓
  - _Requirements: 4.7, 9.1, 9.2_

- [x] 9. Add comprehensive TLS support and testing

  - Implement TlsConfig struct with builder pattern for SSL configuration ✓
  - Add create_tls_connection function with enhanced error handling ✓
  - Implement credential redaction in URL logging via redact_url function ✓
  - Add comprehensive TLS integration tests with testcontainers ✓
  - Create TLS error types with helpful user guidance ✓
  - Add dependency validation tests for SSL feature flags ✓
  - _Requirements: 5.1, 5.2, 5.3, 7.3_

- [ ] 10. Fix critical type safety issues in rows_to_strings

  - Replace unsafe MySQL value access in rows_to_strings function with safe iteration
  - Implement safe NULL value handling using match patterns instead of direct indexing
  - Add comprehensive type conversion for all MySQL Value types (Int, UInt, Float, Date, Time,
    Bytes)
  - Create separate JSON value conversion logic to preserve type information
  - Add proper error handling for type conversion failures with meaningful messages
  - _Requirements: 10.1, 10.2, 10.3, 10.4_

- [ ] 11. Implement streaming query execution

  - Create QueryExecutor struct with execute_streaming() method using mysql::query_iter
  - Implement RowStream iterator with proper column metadata handling
  - Add row count tracking and periodic progress logging in RowStream
  - Update FormatWriter trait implementations to work with streaming
    `Iterator<Item = Result<Vec<String>>>`
  - Add memory usage validation to ensure O(row_width) scaling not O(row_count)
  - Implement proper query error handling with structured logging and meaningful error messages
  - _Requirements: 6.1, 6.2, 6.3_

- [ ] 12. Implement structured logging with tracing

  - Add tracing and tracing-subscriber dependencies to Cargo.toml
  - Create LoggingConfig struct with init_tracing() method for verbosity-based configuration
  - Replace all println!/eprintln! with structured logging using tracing macros (info!, debug!,
    warn!, error!)
  - Add #[instrument] attributes to key functions (connect_to_database, execute_query)
  - Implement --verbose flag with structured logging levels (warn=0, info=1, debug=2, trace=3+)
  - Add --quiet flag for suppressing all output except errors
  - Integrate existing redact_url functionality with structured logging
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ] 13. Expand testing coverage

  - Add development dependencies (criterion, insta, assert_cmd) to Cargo.toml
  - Create unit tests for safe type conversion with comprehensive NULL handling and edge cases
  - Add unit tests for all FormatWriter implementations (CSV, JSON, TSV) with streaming support
  - Implement end-to-end CLI tests using assert_cmd for all exit codes and error conditions
  - Add performance benchmarks for streaming memory usage validation (O(row_width) not O(row_count))
  - Create snapshot tests using insta for deterministic output format validation
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 6.1, 6.2, 6.3, 7.2, 7.3_

- [ ] 14. Final integration and validation

  - Wire together all components in main execution pipeline with proper error propagation
  - Integrate LoggingConfig initialization in main() based on CLI flags
  - Connect QueryExecutor streaming with FormatWriter implementations
  - Validate all exit codes (0-5) and error messages work correctly across all scenarios
  - Test complete workflows covering all user stories with structured logging
  - Validate streaming memory usage stays O(row_width) with large datasets
  - Optimize startup time to stay under 250ms target
  - Verify credential redaction works in all error paths and log outputs
  - _Requirements: All requirements 1.1-10.4_
