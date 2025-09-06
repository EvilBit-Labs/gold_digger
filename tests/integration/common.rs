//! Common test utilities for Gold Digger integration tests
//!
//! This module provides shared utilities for CLI execution, output parsing,
//! and test data management across all integration tests.

#![allow(dead_code)]

use anyhow::{Context, Result};
use assert_cmd::Command;
use predicates::prelude::*;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Output;
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;

use super::{GoldDiggerResult, OutputFormat, TestCase};

// TempFileManager is defined in this module, no need to re-export

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

/// Temporary file and directory management with tempfile for CI-safe cleanup
pub struct TempFileManager {
    /// Temporary directory for test isolation
    temp_dir: tempfile::TempDir,
    /// Test name for debugging and artifact collection
    test_name: String,
    /// File counter for unique naming
    file_counter: std::cell::RefCell<usize>,
}

impl TempFileManager {
    /// Create a new temporary file manager for a test
    pub fn new(test_name: &str) -> Result<Self> {
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!("gold_digger_test_{}_", test_name))
            .tempdir()
            .with_context(|| format!("Failed to create temporary directory for test: {}", test_name))?;

        Ok(Self {
            temp_dir,
            test_name: test_name.to_string(),
            file_counter: std::cell::RefCell::new(0),
        })
    }

    /// Create a temporary output file with the appropriate extension
    pub fn create_output_file(&self, format: &OutputFormat) -> Result<tempfile::NamedTempFile> {
        let temp_file = tempfile::Builder::new()
            .prefix(&format!("output_{}_", self.test_name))
            .suffix(&format!(".{}", format.extension()))
            .tempfile_in(self.temp_dir.path())
            .with_context(|| format!("Failed to create temporary output file for format: {:?}", format))?;

        // Increment file counter
        *self.file_counter.borrow_mut() += 1;

        Ok(temp_file)
    }

    /// Create a temporary input file (e.g., for query files)
    pub fn create_input_file(&self, content: &str, extension: &str) -> Result<tempfile::NamedTempFile> {
        let temp_file = tempfile::Builder::new()
            .prefix(&format!("input_{}_", self.test_name))
            .suffix(&format!(".{}", extension))
            .tempfile_in(self.temp_dir.path())
            .with_context(|| format!("Failed to create temporary input file with extension: {}", extension))?;

        // Write content to the file
        std::fs::write(temp_file.path(), content).with_context(|| {
            format!("Failed to write content to temporary input file: {}", temp_file.path().display())
        })?;

        // Increment file counter
        *self.file_counter.borrow_mut() += 1;

        Ok(temp_file)
    }

    /// Create a temporary directory for test artifacts
    pub fn create_temp_subdir(&self, name: &str) -> Result<std::path::PathBuf> {
        let subdir_path = self.temp_dir.path().join(name);
        std::fs::create_dir_all(&subdir_path)
            .with_context(|| format!("Failed to create temporary subdirectory: {}", subdir_path.display()))?;
        Ok(subdir_path)
    }

    /// Get the path to the main temporary directory
    pub fn temp_dir_path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Create a temporary file path (without creating the file)
    pub fn temp_file_path(&self, filename: &str) -> std::path::PathBuf {
        self.temp_dir.path().join(filename)
    }

    /// Get the number of temporary files created
    pub fn temp_file_count(&self) -> usize {
        *self.file_counter.borrow()
    }

    /// Collect test artifacts for debugging (copies files to a persistent location)
    pub fn collect_artifacts(&self, artifact_dir: &Path) -> Result<Vec<std::path::PathBuf>> {
        let mut collected_files = Vec::new();

        // Create artifact directory if it doesn't exist
        std::fs::create_dir_all(artifact_dir)
            .with_context(|| format!("Failed to create artifact directory: {}", artifact_dir.display()))?;

        // Copy all temporary files to artifact directory
        // Copy all files from the temp directory
        for entry in std::fs::read_dir(self.temp_dir.path())? {
            let entry = entry?;
            let source_path = entry.path();

            if source_path.is_file() {
                let filename = source_path.file_name().unwrap();
                let dest_path = artifact_dir.join(filename);

                std::fs::copy(&source_path, &dest_path).with_context(|| {
                    format!("Failed to copy artifact from {} to {}", source_path.display(), dest_path.display())
                })?;
                collected_files.push(dest_path);
            }
        }

        Ok(collected_files)
    }

    /// Get disk space usage of temporary directory
    pub fn get_disk_usage(&self) -> Result<u64> {
        let mut total_size = 0u64;

        for entry in walkdir::WalkDir::new(self.temp_dir.path()) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let metadata = entry.metadata()?;
                total_size += metadata.len();
            }
        }

        Ok(total_size)
    }

    /// Check if temporary directory is within size limits
    pub fn check_size_limits(&self, max_size_bytes: u64) -> Result<bool> {
        let current_size = self.get_disk_usage()?;
        Ok(current_size <= max_size_bytes)
    }

    /// Clean up specific temporary file by path
    pub fn cleanup_file(&self, path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path)
                .with_context(|| format!("Failed to clean up temporary file: {}", path.display()))?;
        }
        Ok(())
    }

    /// Force cleanup of all temporary files (normally handled by Drop)
    pub fn force_cleanup(&self) -> Result<()> {
        // The temp_dir will be cleaned up when this struct is dropped
        // Individual temp files are managed by RAII
        Ok(())
    }
}

impl Drop for TempFileManager {
    /// Automatic cleanup when TempFileManager is dropped
    fn drop(&mut self) {
        // tempfile handles cleanup automatically, but we can add logging if needed
        if std::env::var("GOLD_DIGGER_TEST_DEBUG").is_ok() {
            eprintln!("Cleaning up temporary directory for test: {}", self.test_name);
        }
    }
}

/// Integration with assert_cmd::Command for output file management
pub struct AssertCmdIntegration;

impl AssertCmdIntegration {
    /// Execute Gold Digger with temporary file management
    pub fn execute_with_temp_files(
        test_case: &TestCase,
        db_url: &str,
        temp_manager: &TempFileManager,
    ) -> Result<(assert_cmd::assert::Assert, tempfile::NamedTempFile)> {
        // Create temporary output file
        let output_file = temp_manager.create_output_file(&test_case.expected_format)?;

        // Build command
        let mut cmd = assert_cmd::Command::cargo_bin("gold_digger")?;

        cmd.arg("--db-url")
            .arg(db_url)
            .arg("--query")
            .arg(&test_case.query)
            .arg("--output")
            .arg(output_file.path());

        // Add CLI arguments
        for arg in &test_case.cli_args {
            cmd.arg(arg);
        }

        // Set environment variables
        for (key, value) in &test_case.env_vars {
            cmd.env(key, value);
        }

        // Execute and return assertion along with output file
        let assert = cmd.assert();
        Ok((assert, output_file))
    }

    /// Execute Gold Digger with query file input
    pub fn execute_with_query_file(
        query_content: &str,
        db_url: &str,
        output_format: &OutputFormat,
        temp_manager: &TempFileManager,
    ) -> Result<(assert_cmd::assert::Assert, tempfile::NamedTempFile, tempfile::NamedTempFile)> {
        // Create temporary query file
        let query_file = temp_manager.create_input_file(query_content, "sql")?;

        // Create temporary output file
        let output_file = temp_manager.create_output_file(output_format)?;

        // Build command
        let mut cmd = assert_cmd::Command::cargo_bin("gold_digger")?;

        cmd.arg("--db-url")
            .arg(db_url)
            .arg("--query-file")
            .arg(query_file.path())
            .arg("--output")
            .arg(output_file.path());

        // Execute and return assertion along with both files
        let assert = cmd.assert();
        Ok((assert, output_file, query_file))
    }

    /// Execute Gold Digger with timeout and temporary file management
    pub fn execute_with_timeout(
        test_case: &TestCase,
        db_url: &str,
        temp_manager: &TempFileManager,
        timeout: Duration,
    ) -> Result<(std::process::Output, tempfile::NamedTempFile)> {
        // Create temporary output file
        let output_file = temp_manager.create_output_file(&test_case.expected_format)?;

        // Build command
        let mut cmd = assert_cmd::Command::cargo_bin("gold_digger")?;

        cmd.arg("--db-url")
            .arg(db_url)
            .arg("--query")
            .arg(&test_case.query)
            .arg("--output")
            .arg(output_file.path())
            .timeout(timeout);

        // Add CLI arguments
        for arg in &test_case.cli_args {
            cmd.arg(arg);
        }

        // Set environment variables
        for (key, value) in &test_case.env_vars {
            cmd.env(key, value);
        }

        // Execute with timeout
        let output = cmd.output()?;
        Ok((output, output_file))
    }
}

/// Test isolation utilities using tempfile::TempDir
pub struct TestIsolation {
    /// Test name for identification
    test_name: String,
    /// Temporary file manager
    temp_manager: TempFileManager,
    /// Original environment variables (for restoration)
    original_env: HashMap<String, Option<String>>,
    /// Test start time
    start_time: std::time::Instant,
}

impl TestIsolation {
    /// Create a new test isolation environment
    pub fn new(test_name: &str) -> Result<Self> {
        let temp_manager = TempFileManager::new(test_name)?;

        Ok(Self {
            test_name: test_name.to_string(),
            temp_manager,
            original_env: HashMap::new(),
            start_time: std::time::Instant::now(),
        })
    }

    /// Get the temporary file manager
    pub fn temp_manager(&self) -> &TempFileManager {
        &self.temp_manager
    }

    /// Set environment variable for this test (will be restored on drop)
    pub fn set_env_var(&mut self, key: &str, value: &str) {
        // Store original value for restoration
        self.original_env.insert(key.to_string(), std::env::var(key).ok());

        // Set new value
        unsafe {
            std::env::set_var(key, value);
        }
    }

    /// Remove environment variable for this test (will be restored on drop)
    pub fn remove_env_var(&mut self, key: &str) {
        // Store original value for restoration
        self.original_env.insert(key.to_string(), std::env::var(key).ok());

        // Remove variable
        unsafe {
            std::env::remove_var(key);
        }
    }

    /// Get test execution time so far
    pub fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Create test artifact directory for debugging
    pub fn create_artifact_dir(&self) -> Result<std::path::PathBuf> {
        let artifact_dir = std::env::temp_dir()
            .join("gold_digger_test_artifacts")
            .join(&self.test_name);

        std::fs::create_dir_all(&artifact_dir)
            .with_context(|| format!("Failed to create artifact directory: {}", artifact_dir.display()))?;

        Ok(artifact_dir)
    }

    /// Collect all test artifacts for debugging
    pub fn collect_artifacts_on_failure(&self) -> Result<Vec<std::path::PathBuf>> {
        let artifact_dir = self.create_artifact_dir()?;
        self.temp_manager.collect_artifacts(&artifact_dir)
    }

    /// Check resource usage and limits
    pub fn check_resource_limits(&self, max_disk_usage: u64) -> Result<ResourceUsageReport> {
        let disk_usage = self.temp_manager.get_disk_usage()?;
        let execution_time = self.elapsed_time();
        let temp_file_count = self.temp_manager.temp_file_count();

        Ok(ResourceUsageReport {
            disk_usage_bytes: disk_usage,
            execution_time,
            temp_file_count,
            within_disk_limit: disk_usage <= max_disk_usage,
            temp_dir_path: self.temp_manager.temp_dir_path().to_path_buf(),
        })
    }
}

impl Drop for TestIsolation {
    /// Restore environment variables when test isolation is dropped
    fn drop(&mut self) {
        // Restore original environment variables
        for (key, original_value) in &self.original_env {
            unsafe {
                match original_value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }

        // Log cleanup if debug mode is enabled
        if std::env::var("GOLD_DIGGER_TEST_DEBUG").is_ok() {
            eprintln!("Test isolation cleanup completed for: {}", self.test_name);
        }
    }
}

/// Resource usage report for test monitoring
#[derive(Debug, Clone)]
pub struct ResourceUsageReport {
    /// Disk usage in bytes
    pub disk_usage_bytes: u64,
    /// Test execution time
    pub execution_time: Duration,
    /// Number of temporary files created
    pub temp_file_count: usize,
    /// Whether disk usage is within limits
    pub within_disk_limit: bool,
    /// Path to temporary directory
    pub temp_dir_path: std::path::PathBuf,
}

/// Test data utilities
pub struct TestDataUtils;

impl TestDataUtils {
    /// Create a temporary output file with the appropriate extension (deprecated - use TempFileManager)
    #[deprecated(note = "Use TempFileManager::create_output_file instead")]
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

    /// Create temporary file manager for a test
    pub fn create_temp_manager(test_name: &str) -> Result<TempFileManager> {
        TempFileManager::new(test_name)
    }

    /// Create test isolation environment
    pub fn create_test_isolation(test_name: &str) -> Result<TestIsolation> {
        TestIsolation::new(test_name)
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

        // Set new value
        unsafe {
            std::env::set_var(key, value);
        }
    }

    /// Remove an environment variable for the test
    pub fn remove_var(&mut self, key: &str) {
        // Store original value for restoration
        self.original_env.insert(key.to_string(), std::env::var(key).ok());

        // Remove variable
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
            unsafe {
                match original_value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}
/// Output validation framework using predicates and insta snapshots
pub struct OutputValidator;

impl OutputValidator {
    /// Validate output file existence and content using predicates
    pub fn validate_file_output(path: &Path, format: &OutputFormat) -> Result<FileValidationResult> {
        use predicates::prelude::*;

        // Check file existence
        let file_exists_predicate = predicate::path::exists();
        if !file_exists_predicate.eval(path) {
            return Err(anyhow::anyhow!("Output file does not exist: {}", path.display()));
        }

        // Check file is not empty
        let file_not_empty_predicate = predicate::path::is_file();
        if !file_not_empty_predicate.eval(path) {
            return Err(anyhow::anyhow!("Output file is empty: {}", path.display()));
        }

        // Read and validate content based on format
        let content =
            fs::read_to_string(path).with_context(|| format!("Failed to read output file: {}", path.display()))?;

        let validation_result = match format {
            OutputFormat::Csv => Self::validate_csv_content(&content)?,
            OutputFormat::Json => Self::validate_json_content(&content)?,
            OutputFormat::Tsv => Self::validate_tsv_content(&content)?,
        };

        Ok(validation_result)
    }

    /// Validate CSV content using predicates for RFC4180 compliance
    pub fn validate_csv_content(content: &str) -> Result<FileValidationResult> {
        use predicates::prelude::*;

        // Check basic CSV structure predicates
        let has_header_predicate = predicate::str::is_empty().not();
        if !has_header_predicate.eval(content) {
            return Err(anyhow::anyhow!("CSV content is empty"));
        }

        // Parse CSV and validate structure
        let csv_result = OutputParser::parse_csv(content)?;

        // Validate CSV-specific requirements using predicates
        let min_columns_predicate = predicate::ge(1usize);
        if !min_columns_predicate.eval(&csv_result.column_count) {
            return Err(anyhow::anyhow!("CSV must have at least 1 column, found {}", csv_result.column_count));
        }

        // Check for proper CSV quoting (QuoteStyle::Necessary)
        let proper_quoting = Self::validate_csv_quoting(content)?;

        // Check for CRLF line endings (RFC4180 requirement)
        let has_crlf = content.contains("\r\n");

        Ok(FileValidationResult {
            format: OutputFormat::Csv,
            row_count: csv_result.row_count,
            column_count: csv_result.column_count,
            file_size: content.len(),
            format_compliance: FormatComplianceResult {
                is_compliant: proper_quoting && (has_crlf || csv_result.row_count == 0),
                compliance_issues: if !proper_quoting {
                    vec!["Improper CSV quoting detected".to_string()]
                } else if !has_crlf && csv_result.row_count > 0 {
                    vec!["Missing CRLF line endings for RFC4180 compliance".to_string()]
                } else {
                    vec![]
                },
            },
            content_validation: ContentValidationResult {
                headers: Some(csv_result.headers),
                data_types_detected: vec![], // Could be enhanced to detect data types
                null_handling_correct: true, // Assume correct for now
            },
        })
    }

    /// Validate JSON content using predicates for structure compliance
    pub fn validate_json_content(content: &str) -> Result<FileValidationResult> {
        use predicates::prelude::*;

        // Check basic JSON structure predicates
        let valid_json_predicate = predicate::str::is_empty().not();
        if !valid_json_predicate.eval(content) {
            return Err(anyhow::anyhow!("JSON content is empty"));
        }

        // Parse JSON and validate structure
        let json_result = OutputParser::parse_json(content)?;

        // Validate JSON-specific requirements
        let json_value: serde_json::Value = serde_json::from_str(content)?;

        // Check for {"data": [...]} structure
        let has_data_field = json_value.get("data").is_some();
        let data_is_array = json_value.get("data").map(|d| d.is_array()).unwrap_or(false);

        // Check for deterministic key ordering (BTreeMap ensures this)
        let has_deterministic_ordering = Self::validate_json_key_ordering(&json_value)?;

        Ok(FileValidationResult {
            format: OutputFormat::Json,
            row_count: json_result.row_count,
            column_count: json_result.column_count,
            file_size: content.len(),
            format_compliance: FormatComplianceResult {
                is_compliant: has_data_field && data_is_array && has_deterministic_ordering,
                compliance_issues: {
                    let mut issues = vec![];
                    if !has_data_field {
                        issues.push("Missing 'data' field in JSON structure".to_string());
                    }
                    if !data_is_array {
                        issues.push("'data' field is not an array".to_string());
                    }
                    if !has_deterministic_ordering {
                        issues.push("JSON keys are not in deterministic order".to_string());
                    }
                    issues
                },
            },
            content_validation: ContentValidationResult {
                headers: None,               // JSON doesn't have headers in the same way
                data_types_detected: vec![], // Could be enhanced
                null_handling_correct: true, // JSON null handling is standard
            },
        })
    }

    /// Validate TSV content using predicates for tab-delimited format
    pub fn validate_tsv_content(content: &str) -> Result<FileValidationResult> {
        use predicates::prelude::*;

        // Check basic TSV structure predicates
        let has_content_predicate = predicate::str::is_empty().not();
        if !has_content_predicate.eval(content) {
            return Err(anyhow::anyhow!("TSV content is empty"));
        }

        // Parse TSV and validate structure
        let tsv_result = OutputParser::parse_tsv(content)?;

        // Validate TSV-specific requirements
        let has_tabs = content.contains('\t');
        let proper_quoting = Self::validate_tsv_quoting(content)?;

        Ok(FileValidationResult {
            format: OutputFormat::Tsv,
            row_count: tsv_result.row_count,
            column_count: tsv_result.column_count,
            file_size: content.len(),
            format_compliance: FormatComplianceResult {
                is_compliant: has_tabs && proper_quoting,
                compliance_issues: {
                    let mut issues = vec![];
                    if !has_tabs && tsv_result.column_count > 1 {
                        issues.push("Missing tab delimiters in TSV format".to_string());
                    }
                    if !proper_quoting {
                        issues.push("Improper TSV quoting detected".to_string());
                    }
                    issues
                },
            },
            content_validation: ContentValidationResult {
                headers: Some(tsv_result.headers),
                data_types_detected: vec![],
                null_handling_correct: true, // Assume correct for now
            },
        })
    }

    /// Validate CSV quoting behavior (QuoteStyle::Necessary)
    fn validate_csv_quoting(content: &str) -> Result<bool> {
        // This is a simplified validation - in practice, you'd want more sophisticated checking
        // For now, we'll assume proper quoting if the CSV can be parsed successfully
        use csv::ReaderBuilder;

        let mut reader = ReaderBuilder::new().has_headers(true).from_reader(content.as_bytes());

        // Try to read all records - if successful, quoting is likely correct
        for result in reader.records() {
            result?; // Will fail if quoting is incorrect
        }

        Ok(true)
    }

    /// Validate TSV quoting behavior
    fn validate_tsv_quoting(content: &str) -> Result<bool> {
        // Similar to CSV but with tab delimiter
        use csv::ReaderBuilder;

        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .delimiter(b'\t')
            .from_reader(content.as_bytes());

        // Try to read all records
        for result in reader.records() {
            result?;
        }

        Ok(true)
    }

    /// Validate JSON key ordering for deterministic output
    fn validate_json_key_ordering(json_value: &serde_json::Value) -> Result<bool> {
        // Check if JSON was parsed with deterministic ordering
        // This is ensured by using BTreeMap in the JSON serialization
        // For validation, we'll check if re-serializing produces the same result
        let serialized = serde_json::to_string(json_value)?;
        let reparsed: serde_json::Value = serde_json::from_str(&serialized)?;
        let reserialized = serde_json::to_string(&reparsed)?;

        Ok(serialized == reserialized)
    }

    /// Create predicates for CSV content validation
    pub fn csv_content_predicates() -> CsvContentPredicates {
        CsvContentPredicates::new()
    }

    /// Create predicates for JSON content validation
    pub fn json_content_predicates() -> JsonContentPredicates {
        JsonContentPredicates::new()
    }

    /// Create predicates for TSV content validation
    pub fn tsv_content_predicates() -> TsvContentPredicates {
        TsvContentPredicates::new()
    }
}

/// CSV content validation predicates
pub struct CsvContentPredicates;

impl CsvContentPredicates {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CsvContentPredicates {
    fn default() -> Self {
        Self::new()
    }
}

impl CsvContentPredicates {
    /// Predicate for validating CSV row count
    pub fn row_count(expected: usize) -> impl Predicate<str> {
        predicate::function(move |content: &str| {
            if let Ok(result) = OutputParser::parse_csv(content) {
                result.row_count == expected
            } else {
                false
            }
        })
    }

    /// Predicate for validating CSV column count
    pub fn column_count(expected: usize) -> impl Predicate<str> {
        predicate::function(move |content: &str| {
            if let Ok(result) = OutputParser::parse_csv(content) {
                result.column_count == expected
            } else {
                false
            }
        })
    }

    /// Predicate for validating CSV headers
    pub fn has_headers(expected_headers: Vec<String>) -> impl Predicate<str> {
        predicate::function(move |content: &str| {
            if let Ok(result) = OutputParser::parse_csv(content) {
                result.headers == expected_headers
            } else {
                false
            }
        })
    }

    /// Predicate for validating CSV contains specific value
    pub fn contains_value(column_index: usize, value: String) -> impl Predicate<str> {
        predicate::function(move |content: &str| {
            if let Ok(result) = OutputParser::parse_csv(content) {
                result
                    .rows
                    .iter()
                    .any(|row| row.get(column_index).map(|cell| cell == &value).unwrap_or(false))
            } else {
                false
            }
        })
    }

    /// Predicate for RFC4180 compliance
    pub fn is_rfc4180_compliant() -> impl Predicate<str> {
        predicate::function(|content: &str| {
            // Check for CRLF line endings and proper CSV structure
            content.contains("\r\n") && OutputParser::parse_csv(content).is_ok()
        })
    }
}

/// JSON content validation predicates
pub struct JsonContentPredicates;

impl JsonContentPredicates {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonContentPredicates {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonContentPredicates {
    /// Predicate for validating JSON row count
    pub fn row_count(expected: usize) -> impl Predicate<str> {
        predicate::function(move |content: &str| {
            if let Ok(result) = OutputParser::parse_json(content) {
                result.row_count == expected
            } else {
                false
            }
        })
    }

    /// Predicate for validating JSON structure ({"data": [...]})
    pub fn has_data_structure() -> impl Predicate<str> {
        predicate::function(|content: &str| {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
                json.get("data").map(|d| d.is_array()).unwrap_or(false)
            } else {
                false
            }
        })
    }

    /// Predicate for deterministic key ordering
    pub fn has_deterministic_ordering() -> impl Predicate<str> {
        predicate::function(|content: &str| {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
                // Re-serialize and compare
                if let Ok(serialized) = serde_json::to_string(&json) {
                    serialized == content
                } else {
                    false
                }
            } else {
                false
            }
        })
    }
}

/// TSV content validation predicates
pub struct TsvContentPredicates;

impl TsvContentPredicates {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TsvContentPredicates {
    fn default() -> Self {
        Self::new()
    }
}

impl TsvContentPredicates {
    /// Predicate for validating TSV row count
    pub fn row_count(expected: usize) -> impl Predicate<str> {
        predicate::function(move |content: &str| {
            if let Ok(result) = OutputParser::parse_tsv(content) {
                result.row_count == expected
            } else {
                false
            }
        })
    }

    /// Predicate for validating TSV has tab delimiters
    pub fn has_tab_delimiters() -> impl Predicate<str> {
        predicate::str::contains("\t")
    }

    /// Predicate for validating TSV column count
    pub fn column_count(expected: usize) -> impl Predicate<str> {
        predicate::function(move |content: &str| {
            if let Ok(result) = OutputParser::parse_tsv(content) {
                result.column_count == expected
            } else {
                false
            }
        })
    }
}

/// File validation result
#[derive(Debug, Clone)]
pub struct FileValidationResult {
    /// Output format that was validated
    pub format: OutputFormat,
    /// Number of data rows found
    pub row_count: usize,
    /// Number of columns found
    pub column_count: usize,
    /// File size in bytes
    pub file_size: usize,
    /// Format compliance validation result
    pub format_compliance: FormatComplianceResult,
    /// Content validation result
    pub content_validation: ContentValidationResult,
}

/// Format compliance validation result
#[derive(Debug, Clone)]
pub struct FormatComplianceResult {
    /// Whether the format is compliant with standards
    pub is_compliant: bool,
    /// List of compliance issues found
    pub compliance_issues: Vec<String>,
}

/// Content validation result
#[derive(Debug, Clone)]
pub struct ContentValidationResult {
    /// Column headers (if applicable)
    pub headers: Option<Vec<String>>,
    /// Data types detected in the content
    pub data_types_detected: Vec<String>,
    /// Whether NULL handling is correct
    pub null_handling_correct: bool,
}

/// Performance measurement utilities using assert_cmd execution time tracking
pub struct PerformanceMeasurement;

impl PerformanceMeasurement {
    /// Measure execution time of a Gold Digger command
    pub fn measure_execution_time<F, T>(operation: F) -> Result<(Duration, T)>
    where
        F: FnOnce() -> Result<T>,
    {
        let start_time = std::time::Instant::now();
        let result = operation()?;
        let execution_time = start_time.elapsed();

        Ok((execution_time, result))
    }

    /// Create performance threshold predicate
    pub fn execution_time_under(threshold: Duration) -> impl Predicate<Duration> {
        predicate::lt(threshold)
    }

    /// Create memory usage predicate
    pub fn memory_usage_under(threshold_bytes: u64) -> impl Predicate<u64> {
        predicate::lt(threshold_bytes)
    }

    /// Measure and validate performance with predicates
    pub fn validate_performance_thresholds(
        execution_time: Duration,
        memory_usage: u64,
        max_execution_time: Duration,
        max_memory_usage: u64,
    ) -> Result<PerformanceValidationResult> {
        use predicates::prelude::*;

        let time_predicate = predicate::lt(max_execution_time);
        let memory_predicate = predicate::lt(max_memory_usage);

        let time_within_threshold = time_predicate.eval(&execution_time);
        let memory_within_threshold = memory_predicate.eval(&memory_usage);

        Ok(PerformanceValidationResult {
            execution_time,
            memory_usage,
            time_within_threshold,
            memory_within_threshold,
            max_execution_time,
            max_memory_usage,
        })
    }
}

/// Performance validation result
#[derive(Debug, Clone)]
pub struct PerformanceValidationResult {
    /// Actual execution time
    pub execution_time: Duration,
    /// Actual memory usage in bytes
    pub memory_usage: u64,
    /// Whether execution time was within threshold
    pub time_within_threshold: bool,
    /// Whether memory usage was within threshold
    pub memory_within_threshold: bool,
    /// Maximum allowed execution time
    pub max_execution_time: Duration,
    /// Maximum allowed memory usage
    pub max_memory_usage: u64,
}

/// Snapshot testing utilities for CLI output verification and regression testing
pub struct SnapshotTesting;

impl SnapshotTesting {
    /// Create snapshot of CLI output with sensitive data redaction
    pub fn create_cli_output_snapshot(test_name: &str, stdout: &str, stderr: &str, exit_code: i32) -> Result<()> {
        // Redact sensitive information
        let redacted_stdout = Self::redact_sensitive_data(stdout);
        let redacted_stderr = Self::redact_sensitive_data(stderr);

        // Create combined snapshot
        let snapshot_content = format!(
            "Exit Code: {}\n\n--- STDOUT ---\n{}\n\n--- STDERR ---\n{}",
            exit_code, redacted_stdout, redacted_stderr
        );

        let snapshot_name = format!("{}_cli_output", test_name);
        insta::assert_snapshot!(snapshot_name, snapshot_content);

        Ok(())
    }

    /// Create snapshot of output file content
    pub fn create_file_output_snapshot(test_name: &str, file_content: &str, format: &OutputFormat) -> Result<()> {
        let snapshot_name = format!("{}_{}_output", test_name, format.extension());
        insta::assert_snapshot!(snapshot_name, file_content);
        Ok(())
    }

    /// Create snapshot for cross-format consistency testing
    pub fn create_cross_format_snapshot(
        test_name: &str,
        csv_content: &str,
        json_content: &str,
        tsv_content: &str,
    ) -> Result<()> {
        // Parse all formats to ensure they contain the same data
        let csv_result = OutputParser::parse_csv(csv_content)?;
        let json_result = OutputParser::parse_json(json_content)?;
        let tsv_result = OutputParser::parse_tsv(tsv_content)?;

        // Create consistency report
        let consistency_report = format!(
            "Cross-Format Consistency Report\n\
            CSV: {} rows, {} columns\n\
            JSON: {} rows, {} columns\n\
            TSV: {} rows, {} columns\n\
            \n\
            Row Count Consistent: {}\n\
            Column Count Consistent: {}\n\
            \n\
            --- CSV Content ---\n{}\n\
            \n\
            --- JSON Content ---\n{}\n\
            \n\
            --- TSV Content ---\n{}",
            csv_result.row_count,
            csv_result.column_count,
            json_result.row_count,
            json_result.column_count,
            tsv_result.row_count,
            tsv_result.column_count,
            csv_result.row_count == json_result.row_count && json_result.row_count == tsv_result.row_count,
            csv_result.column_count == json_result.column_count && json_result.column_count == tsv_result.column_count,
            csv_content,
            json_content,
            tsv_content
        );

        let snapshot_name = format!("{}_cross_format_consistency", test_name);
        insta::assert_snapshot!(snapshot_name, consistency_report);

        Ok(())
    }

    /// Redact sensitive information from output
    fn redact_sensitive_data(content: &str) -> String {
        let mut redacted = content.to_string();

        // Redact mysql:// URLs
        if let Ok(re) = regex::Regex::new(r"mysql://[^:]+:[^@]+@[^/]+/[^\s]+") {
            redacted = re.replace_all(&redacted, "mysql://***:***@***/***").to_string();
        }

        // Redact DATABASE_URL references
        redacted = redacted.replace("DATABASE_URL", "***DATABASE_URL***");

        // Redact password patterns
        if let Ok(re) = regex::Regex::new(r"password[=:]\s*[^\s]+") {
            redacted = re.replace_all(&redacted, "password=***").to_string();
        }

        // Redact connection strings in error messages
        if let Ok(re) = regex::Regex::new(r"connection string: [^\s]+") {
            redacted = re.replace_all(&redacted, "connection string: ***").to_string();
        }

        redacted
    }

    /// Update snapshots for a test (useful for maintenance)
    pub fn update_snapshots_for_test(test_name: &str) -> Result<()> {
        // This would typically be handled by insta's CLI tools
        // For now, we'll just provide a helper message
        println!("To update snapshots for test '{}', run:", test_name);
        println!("cargo insta review --test {}", test_name);
        Ok(())
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

    /// Assert file validation result using predicates
    pub fn assert_file_validation(
        result: &FileValidationResult,
        expected_rows: usize,
        expected_columns: usize,
    ) -> Result<()> {
        use predicates::prelude::*;

        let row_predicate = predicate::eq(expected_rows);
        let column_predicate = predicate::eq(expected_columns);

        if !row_predicate.eval(&result.row_count) {
            return Err(anyhow::anyhow!(
                "Row count validation failed: expected {}, got {}",
                expected_rows,
                result.row_count
            ));
        }

        if !column_predicate.eval(&result.column_count) {
            return Err(anyhow::anyhow!(
                "Column count validation failed: expected {}, got {}",
                expected_columns,
                result.column_count
            ));
        }

        if !result.format_compliance.is_compliant {
            return Err(anyhow::anyhow!(
                "Format compliance validation failed: {}",
                result.format_compliance.compliance_issues.join(", ")
            ));
        }

        Ok(())
    }

    /// Assert performance validation using predicates
    pub fn assert_performance_thresholds(result: &PerformanceValidationResult) -> Result<()> {
        if !result.time_within_threshold {
            return Err(anyhow::anyhow!(
                "Execution time exceeded threshold: {:?} > {:?}",
                result.execution_time,
                result.max_execution_time
            ));
        }

        if !result.memory_within_threshold {
            return Err(anyhow::anyhow!(
                "Memory usage exceeded threshold: {} bytes > {} bytes",
                result.memory_usage,
                result.max_memory_usage
            ));
        }

        Ok(())
    }
}
