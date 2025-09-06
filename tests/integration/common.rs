//! Common test utilities for Gold Digger integration tests
//!
//! This module provides shared utilities for CLI execution, output parsing,
//! and test data management across all integration tests.

#![allow(dead_code)]

use anyhow::{Context, Result};
use assert_cmd::Command;
use predicates::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Output;
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;

use super::{GoldDiggerResult, OutputFormat, TestCase};

/// Enhanced CLI execution utilities for Gold Digger using assert_cmd and predicates
///
/// This struct provides a robust CLI testing framework that replaces the previous
/// bespoke implementation with industry-standard tools:
///
/// - Uses `assert_cmd::Command::cargo_bin("gold_digger")` for reliable binary execution
/// - Leverages `assert_cmd`'s `.assert()` method with `predicates` for robust validation
/// - Implements timeout handling using `assert_cmd`'s built-in timeout functionality
/// - Supports `insta` snapshots for CLI output verification and regression testing
/// - Provides helper functions for common test scenarios (TLS, non-TLS, different formats)
///
/// # Example Usage
///
/// ```rust
/// use gold_digger_tests::integration::common::GoldDiggerCli;
/// use std::time::Duration;
///
/// // Create CLI executor with default timeout
/// let cli = GoldDiggerCli::new();
///
/// // Create CLI executor with custom timeout
/// let cli = GoldDiggerCli::with_timeout(Duration::from_secs(60));
///
/// // Execute with test case
/// let result = cli.execute(&test_case, &db_url, &output_path)?;
///
/// // Execute with assertions and predicates
/// let result = cli.execute_with_assertions(&test_case, &db_url, &output_path)?;
///
/// // Execute with output validation
/// let result = cli.execute_with_output_validation(
///     &test_case, &db_url, &output_path,
///     Some("Processing"), Some("connection")
/// )?;
///
/// // Execute with snapshot testing
/// let result = cli.execute_with_snapshot(&test_case, &db_url, &output_path)?;
/// ```
///
/// # Helper Functions
///
/// The struct provides helper functions for common test scenarios:
///
/// - `create_tls_test_case()` - TLS database connection tests
/// - `create_non_tls_test_case()` - Non-TLS database connection tests
/// - `create_csv_test_case()` - CSV format testing
/// - `create_json_test_case()` - JSON format testing
/// - `create_tsv_test_case()` - TSV format testing
/// - `create_error_test_case()` - Error scenario testing
///
/// # Predicate Utilities
///
/// Use `CliPredicates` for common validation patterns:
///
/// - `success_output_predicate()` - Validate successful execution output
/// - `error_contains_predicate()` - Check for specific error messages
/// - `connection_error_predicate()` - Validate database connection errors
/// - `sql_error_predicate()` - Validate SQL syntax errors
/// - `io_error_predicate()` - Validate file I/O errors
/// - `no_credentials_predicate()` - Ensure no credentials are leaked
pub struct GoldDiggerCli {
    /// Default timeout for command execution
    default_timeout: Duration,
}

impl GoldDiggerCli {
    /// Create a new CLI executor with default timeout
    pub fn new() -> Self {
        Self {
            default_timeout: Duration::from_secs(30), // Default 30 second timeout
        }
    }

    /// Create a new CLI executor with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            default_timeout: timeout,
        }
    }

    /// Execute Gold Digger with the given test case using assert_cmd and predicates
    pub fn execute(&self, test_case: &TestCase, db_url: &str, output_path: &Path) -> Result<GoldDiggerResult> {
        let start_time = Instant::now();

        // Build command using assert_cmd
        let mut cmd = Command::cargo_bin("gold_digger")?;

        // Set database URL (never log the actual URL for security)
        cmd.arg("--db-url").arg(db_url);

        // Set query
        cmd.arg("--query").arg(&test_case.query);

        // Set output file
        cmd.arg("--output").arg(output_path);

        // Add additional CLI arguments
        for arg in &test_case.cli_args {
            cmd.arg(arg);
        }

        // Set environment variables
        for (key, value) in &test_case.env_vars {
            cmd.env(key, value);
        }

        // Execute with timeout using process_control
        let output = self
            .execute_with_timeout(cmd, self.default_timeout)
            .with_context(|| format!("Failed to execute Gold Digger for test case: {}", test_case.name))?;

        let _execution_time = start_time.elapsed();

        // Use predicates for exit code validation
        let actual_exit_code = output.status.code().unwrap_or(-1);
        let exit_code_predicate = predicate::eq(test_case.expected_exit_code);

        if !exit_code_predicate.eval(&actual_exit_code) {
            return Err(anyhow::anyhow!(
                "Gold Digger exited with code {} (expected {}). Stderr: {}",
                actual_exit_code,
                test_case.expected_exit_code,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // If successful execution expected, parse output
        if test_case.expected_exit_code == 0 {
            self.parse_output_file(output_path, &test_case.expected_format)
        } else {
            // For error cases, return empty result
            Ok(GoldDiggerResult {
                row_count: 0,
                output_size: 0,
            })
        }
    }

    /// Execute Gold Digger with assert_cmd assertions and predicates
    pub fn execute_with_assertions(
        &self,
        test_case: &TestCase,
        db_url: &str,
        output_path: &Path,
    ) -> Result<GoldDiggerResult> {
        let start_time = Instant::now();

        // Build command using assert_cmd
        let mut cmd = Command::cargo_bin("gold_digger")?;

        // Set database URL (never log the actual URL for security)
        cmd.arg("--db-url").arg(db_url);

        // Set query
        cmd.arg("--query").arg(&test_case.query);

        // Set output file
        cmd.arg("--output").arg(output_path);

        // Add additional CLI arguments
        for arg in &test_case.cli_args {
            cmd.arg(arg);
        }

        // Set environment variables
        for (key, value) in &test_case.env_vars {
            cmd.env(key, value);
        }

        // Use assert_cmd's assertion API with predicates
        let assert = cmd.assert();

        // Assert exit code using predicates
        let assert = assert.code(predicate::eq(test_case.expected_exit_code));

        // Additional assertions based on expected behavior
        let assert = if test_case.expected_exit_code == 0 {
            // For successful cases, expect no error output
            assert.stderr(
                predicate::str::is_empty()
                    .or(predicate::str::contains("Processing").or(predicate::str::contains("rows"))),
            )
        } else {
            // For error cases, expect error message on stderr
            assert.stderr(predicate::str::is_empty().not())
        };

        // Execute the assertion
        let _output = assert.get_output();
        let _execution_time = start_time.elapsed();

        // If successful execution expected, parse output
        if test_case.expected_exit_code == 0 {
            self.parse_output_file(output_path, &test_case.expected_format)
        } else {
            // For error cases, return empty result
            Ok(GoldDiggerResult {
                row_count: 0,
                output_size: 0,
            })
        }
    }

    /// Execute command with timeout using assert_cmd's built-in timeout
    fn execute_with_timeout(&self, mut cmd: Command, timeout: Duration) -> Result<Output> {
        // Use assert_cmd's built-in timeout functionality
        let output = cmd
            .timeout(timeout)
            .output()
            .context("Failed to execute Gold Digger with timeout")?;

        Ok(output)
    }

    /// Parse output file and calculate metrics
    fn parse_output_file(&self, output_path: &Path, format: &OutputFormat) -> Result<GoldDiggerResult> {
        let output_content = fs::read_to_string(output_path)
            .with_context(|| format!("Failed to read output file: {}", output_path.display()))?;

        let output_size = output_content.len() as u64;

        // Calculate row count based on format
        let row_count = match format {
            OutputFormat::Csv => {
                use csv::ReaderBuilder;
                let mut reader = ReaderBuilder::new()
                    .has_headers(true)
                    .from_reader(output_content.as_bytes());
                reader.records().count()
            },
            OutputFormat::Json => {
                let json: serde_json::Value =
                    serde_json::from_str(&output_content).with_context(|| "Failed to parse JSON output")?;
                if let Some(data) = json.get("data") {
                    if let Some(array) = data.as_array() {
                        array.len()
                    } else {
                        0
                    }
                } else {
                    0
                }
            },
            OutputFormat::Tsv => {
                use csv::ReaderBuilder;
                let mut reader = ReaderBuilder::new()
                    .has_headers(true)
                    .delimiter(b'\t')
                    .from_reader(output_content.as_bytes());
                reader.records().count()
            },
        };

        Ok(GoldDiggerResult { row_count, output_size })
    }

    /// Execute Gold Digger and capture raw output for error testing with predicates
    pub fn execute_raw(&self, test_case: &TestCase, db_url: &str, output_path: &Path) -> Result<Output> {
        let mut cmd = Command::cargo_bin("gold_digger")?;

        cmd.arg("--db-url")
            .arg(db_url)
            .arg("--query")
            .arg(&test_case.query)
            .arg("--output")
            .arg(output_path);

        // Add additional CLI arguments
        for arg in &test_case.cli_args {
            cmd.arg(arg);
        }

        // Set environment variables
        for (key, value) in &test_case.env_vars {
            cmd.env(key, value);
        }

        // Execute with timeout
        self.execute_with_timeout(cmd, self.default_timeout)
            .with_context(|| format!("Failed to execute Gold Digger for test case: {}", test_case.name))
    }

    /// Execute Gold Digger with predicates for stdout/stderr validation
    pub fn execute_with_output_validation(
        &self,
        test_case: &TestCase,
        db_url: &str,
        output_path: &Path,
        stdout_contains: Option<&str>,
        stderr_contains: Option<&str>,
    ) -> Result<GoldDiggerResult> {
        let mut cmd = Command::cargo_bin("gold_digger")?;

        // Set database URL (never log the actual URL for security)
        cmd.arg("--db-url").arg(db_url);
        cmd.arg("--query").arg(&test_case.query);
        cmd.arg("--output").arg(output_path);

        // Add additional CLI arguments
        for arg in &test_case.cli_args {
            cmd.arg(arg);
        }

        // Set environment variables
        for (key, value) in &test_case.env_vars {
            cmd.env(key, value);
        }

        // Use assert_cmd's assertion API with predicates
        let mut assert = cmd.assert().code(predicate::eq(test_case.expected_exit_code));

        // Apply stdout predicate if provided
        if let Some(stdout_text) = stdout_contains {
            assert = assert.stdout(predicate::str::contains(stdout_text));
        }

        // Apply stderr predicate if provided
        if let Some(stderr_text) = stderr_contains {
            assert = assert.stderr(predicate::str::contains(stderr_text));
        }

        // Execute the assertion
        let _output = assert.get_output();

        // If successful execution expected, parse output
        if test_case.expected_exit_code == 0 {
            self.parse_output_file(output_path, &test_case.expected_format)
        } else {
            // For error cases, return empty result
            Ok(GoldDiggerResult {
                row_count: 0,
                output_size: 0,
            })
        }
    }
}

impl Default for GoldDiggerCli {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for common test scenarios
impl GoldDiggerCli {
    /// Create a test case for TLS database connection
    pub fn create_tls_test_case(query: &str, format: OutputFormat) -> TestCase {
        TestCase {
            name: format!("tls_test_{}", format.extension()),
            query: query.to_string(),
            expected_format: format,
            expected_exit_code: 0,
            cli_args: vec!["--verbose".to_string()], // Enable verbose for TLS debugging
            env_vars: HashMap::new(),
            validation_rules: vec![],
        }
    }

    /// Create a test case for non-TLS database connection
    pub fn create_non_tls_test_case(query: &str, format: OutputFormat) -> TestCase {
        TestCase {
            name: format!("non_tls_test_{}", format.extension()),
            query: query.to_string(),
            expected_format: format,
            expected_exit_code: 0,
            cli_args: vec![],
            env_vars: HashMap::new(),
            validation_rules: vec![],
        }
    }

    /// Create a test case for CSV format testing
    pub fn create_csv_test_case(query: &str) -> TestCase {
        TestCase {
            name: "csv_format_test".to_string(),
            query: query.to_string(),
            expected_format: OutputFormat::Csv,
            expected_exit_code: 0,
            cli_args: vec!["--format".to_string(), "csv".to_string()],
            env_vars: HashMap::new(),
            validation_rules: vec![],
        }
    }

    /// Create a test case for JSON format testing
    pub fn create_json_test_case(query: &str) -> TestCase {
        TestCase {
            name: "json_format_test".to_string(),
            query: query.to_string(),
            expected_format: OutputFormat::Json,
            expected_exit_code: 0,
            cli_args: vec!["--format".to_string(), "json".to_string()],
            env_vars: HashMap::new(),
            validation_rules: vec![],
        }
    }

    /// Create a test case for TSV format testing
    pub fn create_tsv_test_case(query: &str) -> TestCase {
        TestCase {
            name: "tsv_format_test".to_string(),
            query: query.to_string(),
            expected_format: OutputFormat::Tsv,
            expected_exit_code: 0,
            cli_args: vec!["--format".to_string(), "tsv".to_string()],
            env_vars: HashMap::new(),
            validation_rules: vec![],
        }
    }

    /// Create a test case for error scenario testing
    pub fn create_error_test_case(query: &str, expected_exit_code: i32) -> TestCase {
        TestCase {
            name: format!("error_test_exit_{}", expected_exit_code),
            query: query.to_string(),
            expected_format: OutputFormat::Csv, // Format doesn't matter for error cases
            expected_exit_code,
            cli_args: vec![],
            env_vars: HashMap::new(),
            validation_rules: vec![],
        }
    }

    /// Execute test with insta snapshot testing for CLI output
    pub fn execute_with_snapshot(
        &self,
        test_case: &TestCase,
        db_url: &str,
        output_path: &Path,
    ) -> Result<GoldDiggerResult> {
        let mut cmd = Command::cargo_bin("gold_digger")?;

        // Set database URL (never log the actual URL for security)
        cmd.arg("--db-url").arg(db_url);
        cmd.arg("--query").arg(&test_case.query);
        cmd.arg("--output").arg(output_path);

        // Add additional CLI arguments
        for arg in &test_case.cli_args {
            cmd.arg(arg);
        }

        // Set environment variables
        for (key, value) in &test_case.env_vars {
            cmd.env(key, value);
        }

        // Execute with timeout
        let output = self
            .execute_with_timeout(cmd, self.default_timeout)
            .with_context(|| format!("Failed to execute Gold Digger for test case: {}", test_case.name))?;

        // Validate exit code
        let actual_exit_code = output.status.code().unwrap_or(-1);
        if actual_exit_code != test_case.expected_exit_code {
            return Err(anyhow::anyhow!(
                "Gold Digger exited with code {} (expected {}). Stderr: {}",
                actual_exit_code,
                test_case.expected_exit_code,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Create snapshot of CLI output (with sensitive data redacted)
        let stdout_content = String::from_utf8_lossy(&output.stdout);
        let stderr_content = String::from_utf8_lossy(&output.stderr);

        // Redact sensitive information before snapshot
        let redacted_stdout = self.redact_sensitive_output(&stdout_content);
        let redacted_stderr = self.redact_sensitive_output(&stderr_content);

        // Create snapshot using insta
        let snapshot_name = format!("{}__stdout", test_case.name);
        insta::assert_snapshot!(snapshot_name, redacted_stdout);

        if !redacted_stderr.is_empty() {
            let stderr_snapshot_name = format!("{}__stderr", test_case.name);
            insta::assert_snapshot!(stderr_snapshot_name, redacted_stderr);
        }

        // If successful execution expected, parse output and create file snapshot
        if test_case.expected_exit_code == 0 {
            let result = self.parse_output_file(output_path, &test_case.expected_format)?;

            // Create snapshot of output file content
            let file_content = fs::read_to_string(output_path)
                .with_context(|| format!("Failed to read output file: {}", output_path.display()))?;

            let file_snapshot_name = format!("{}__output_file", test_case.name);
            insta::assert_snapshot!(file_snapshot_name, file_content);

            Ok(result)
        } else {
            // For error cases, return empty result
            Ok(GoldDiggerResult {
                row_count: 0,
                output_size: 0,
            })
        }
    }

    /// Redact sensitive information from output for safe logging/snapshots
    fn redact_sensitive_output(&self, output: &str) -> String {
        let mut redacted = output.to_string();

        // Redact mysql:// URLs
        if let Ok(re) = regex::Regex::new(r"mysql://[^:]+:[^@]+@[^/]+/[^\s]+") {
            redacted = re.replace_all(&redacted, "mysql://***:***@***/***").to_string();
        }

        // Redact DATABASE_URL references
        redacted = redacted.replace("DATABASE_URL", "***DATABASE_URL***");

        // Redact any potential password patterns
        if let Ok(re) = regex::Regex::new(r"password[=:]\s*[^\s]+") {
            redacted = re.replace_all(&redacted, "password=***").to_string();
        }

        redacted
    }
}

/// Predicate utilities for common CLI testing scenarios
pub struct CliPredicates;

impl CliPredicates {
    /// Check if output indicates successful execution
    pub fn is_success_output(output: &str) -> bool {
        output.is_empty() || output.contains("Processing") || output.contains("rows") || output.contains("Connecting")
    }

    /// Check if output contains error message
    pub fn contains_error(output: &str, message: &str) -> bool {
        output.contains(message)
    }

    /// Check if output contains database connection error
    pub fn is_connection_error(output: &str) -> bool {
        output.contains("connection") || output.contains("Connection") || output.contains("connect")
    }

    /// Check if output contains SQL syntax error
    pub fn is_sql_error(output: &str) -> bool {
        output.contains("SQL") || output.contains("syntax") || output.contains("query")
    }

    /// Check if output contains file I/O error
    pub fn is_io_error(output: &str) -> bool {
        output.contains("file") || output.contains("permission") || output.contains("I/O")
    }

    /// Check if output contains no credentials (should NOT contain sensitive data)
    pub fn has_no_credentials(output: &str) -> bool {
        !output.contains("mysql://") && !output.contains("password") && !output.contains("DATABASE_URL")
    }

    /// Create a predicate for successful execution output
    pub fn success_output_predicate() -> impl Predicate<str> {
        predicate::str::is_empty()
            .or(predicate::str::contains("Processing"))
            .or(predicate::str::contains("rows"))
            .or(predicate::str::contains("Connecting"))
    }

    /// Create a predicate for error output containing specific message
    pub fn error_contains_predicate(message: &str) -> impl Predicate<str> {
        predicate::str::contains(message.to_string())
    }

    /// Create a predicate for database connection error
    pub fn connection_error_predicate() -> impl Predicate<str> {
        predicate::str::contains("connection")
            .or(predicate::str::contains("Connection"))
            .or(predicate::str::contains("connect"))
    }

    /// Create a predicate for SQL syntax error
    pub fn sql_error_predicate() -> impl Predicate<str> {
        predicate::str::contains("SQL")
            .or(predicate::str::contains("syntax"))
            .or(predicate::str::contains("query"))
    }

    /// Create a predicate for file I/O error
    pub fn io_error_predicate() -> impl Predicate<str> {
        predicate::str::contains("file")
            .or(predicate::str::contains("permission"))
            .or(predicate::str::contains("I/O"))
    }

    /// Create a predicate for credential redaction (should NOT contain sensitive data)
    pub fn no_credentials_predicate() -> impl Predicate<str> {
        predicate::str::contains("mysql://")
            .not()
            .and(predicate::str::contains("password").not())
            .and(predicate::str::contains("DATABASE_URL").not())
    }
}

/// Output parsing utilities
pub struct OutputParser;

impl OutputParser {
    /// Parse CSV output and validate structure
    pub fn parse_csv(content: &str) -> Result<CsvParseResult> {
        use csv::ReaderBuilder;

        let mut reader = ReaderBuilder::new().has_headers(true).from_reader(content.as_bytes());

        let headers = reader.headers()?.clone();
        let column_count = headers.len();

        let mut rows = Vec::new();
        for result in reader.records() {
            let record = result?;
            rows.push(record.iter().map(|s| s.to_string()).collect::<Vec<String>>());
        }

        let row_count = rows.len();
        Ok(CsvParseResult {
            headers: headers.iter().map(|s| s.to_string()).collect(),
            rows,
            column_count,
            row_count,
        })
    }

    /// Parse JSON output and validate structure
    pub fn parse_json(content: &str) -> Result<JsonParseResult> {
        let json: serde_json::Value = serde_json::from_str(content)?;

        // Validate expected structure: {"data": [...]}
        let data = json
            .get("data")
            .ok_or_else(|| anyhow::anyhow!("JSON output missing 'data' field"))?;

        let array = data
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("JSON 'data' field is not an array"))?;

        let row_count = array.len();
        let column_count = if let Some(first_row) = array.first() {
            if let Some(obj) = first_row.as_object() {
                obj.len()
            } else {
                0
            }
        } else {
            0
        };

        Ok(JsonParseResult {
            data: array.clone(),
            row_count,
            column_count,
        })
    }

    /// Parse TSV output and validate structure
    pub fn parse_tsv(content: &str) -> Result<TsvParseResult> {
        use csv::ReaderBuilder;

        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .delimiter(b'\t')
            .from_reader(content.as_bytes());

        let headers = reader.headers()?.clone();
        let column_count = headers.len();

        let mut rows = Vec::new();
        for result in reader.records() {
            let record = result?;
            rows.push(record.iter().map(|s| s.to_string()).collect::<Vec<String>>());
        }

        let row_count = rows.len();
        Ok(TsvParseResult {
            headers: headers.iter().map(|s| s.to_string()).collect(),
            rows,
            column_count,
            row_count,
        })
    }
}

/// CSV parsing result
#[derive(Debug, Clone)]
pub struct CsvParseResult {
    /// Column headers
    pub headers: Vec<String>,
    /// Data rows
    pub rows: Vec<Vec<String>>,
    /// Number of columns
    pub column_count: usize,
    /// Number of rows
    pub row_count: usize,
}

/// JSON parsing result
#[derive(Debug, Clone)]
pub struct JsonParseResult {
    /// JSON data array
    pub data: Vec<serde_json::Value>,
    /// Number of rows
    pub row_count: usize,
    /// Number of columns
    pub column_count: usize,
}

/// TSV parsing result
#[derive(Debug, Clone)]
pub struct TsvParseResult {
    /// Column headers
    pub headers: Vec<String>,
    /// Data rows
    pub rows: Vec<Vec<String>>,
    /// Number of columns
    pub column_count: usize,
    /// Number of rows
    pub row_count: usize,
}

/// Test data utilities
pub struct TestDataUtils;

impl TestDataUtils {
    /// Create a temporary output file with the appropriate extension
    pub fn create_temp_output_file(format: &OutputFormat) -> Result<NamedTempFile> {
        let temp_file = tempfile::Builder::new()
            .suffix(&format!(".{}", format.extension()))
            .tempfile()?;
        Ok(temp_file)
    }

    /// Read file content as string
    pub fn read_file_content(path: &Path) -> Result<String> {
        fs::read_to_string(path).with_context(|| format!("Failed to read file: {}", path.display()))
    }

    /// Get file size in bytes
    pub fn get_file_size(path: &Path) -> Result<u64> {
        let metadata =
            fs::metadata(path).with_context(|| format!("Failed to get metadata for file: {}", path.display()))?;
        Ok(metadata.len())
    }
}

/// Environment utilities for test isolation
pub struct TestEnvironment {
    /// Original environment variables (for restoration)
    original_env: HashMap<String, Option<String>>,
}

impl TestEnvironment {
    /// Create a new test environment
    pub fn new() -> Self {
        Self {
            original_env: HashMap::new(),
        }
    }

    /// Set an environment variable for the test
    pub fn set_var(&mut self, key: &str, value: &str) {
        // Store original value for restoration
        self.original_env.insert(key.to_string(), std::env::var(key).ok());

        // Set new value - std::env::set_var is safe in single-threaded tests
        unsafe {
            std::env::set_var(key, value);
        }
    }

    /// Remove an environment variable for the test
    pub fn remove_var(&mut self, key: &str) {
        // Store original value for restoration
        self.original_env.insert(key.to_string(), std::env::var(key).ok());

        // Remove variable - std::env::remove_var is safe in single-threaded tests
        unsafe {
            std::env::remove_var(key);
        }
    }
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TestEnvironment {
    /// Restore original environment variables when dropped
    fn drop(&mut self) {
        for (key, original_value) in &self.original_env {
            match original_value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

/// Assertion utilities for integration tests
pub struct TestAssertions;

impl TestAssertions {
    /// Assert that a file exists and is not empty
    pub fn assert_file_exists_and_not_empty(path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(anyhow::anyhow!("File does not exist: {}", path.display()));
        }

        let size = TestDataUtils::get_file_size(path)?;
        if size == 0 {
            return Err(anyhow::anyhow!("File is empty: {}", path.display()));
        }

        Ok(())
    }

    /// Assert that output contains expected number of rows
    pub fn assert_row_count(actual: usize, expected: usize) -> Result<()> {
        if actual != expected {
            return Err(anyhow::anyhow!("Row count mismatch: expected {}, got {}", expected, actual));
        }
        Ok(())
    }

    /// Assert that output contains expected number of columns
    pub fn assert_column_count(actual: usize, expected: usize) -> Result<()> {
        if actual != expected {
            return Err(anyhow::anyhow!("Column count mismatch: expected {}, got {}", expected, actual));
        }
        Ok(())
    }
}
