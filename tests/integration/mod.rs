//! Integration test module for Gold Digger
//!
//! This module provides common utilities, test infrastructure, and shared components
//! for comprehensive integration testing of Gold Digger with real MySQL/MariaDB instances.

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

// Re-export submodules
pub mod common;
pub mod containers;

// Re-export commonly used types and functions
#[allow(unused_imports)]
pub use common::*;
#[allow(unused_imports)]
pub use containers::*;

/// Test database type enumeration for managing different database containers
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum TestDatabase {
    /// MySQL container instance
    MySQL,
    /// MariaDB container instance
    MariaDB,
}

impl TestDatabase {
    /// Get the database type name as a string
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            TestDatabase::MySQL => "mysql",
            TestDatabase::MariaDB => "mariadb",
        }
    }
}

/// Output format enumeration for test validation
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum OutputFormat {
    /// CSV format with RFC4180 compliance
    Csv,
    /// JSON format with deterministic ordering
    Json,
    /// TSV format (tab-separated values)
    Tsv,
}

impl OutputFormat {
    /// Get the file extension for this format
    #[allow(dead_code)]
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Csv => "csv",
            OutputFormat::Json => "json",
            OutputFormat::Tsv => "tsv",
        }
    }
}

/// Test case definition for integration tests
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TestCase {
    /// Test case name for identification
    pub name: String,
    /// SQL query to execute
    pub query: String,
    /// Expected output format
    pub expected_format: OutputFormat,
    /// Expected exit code from Gold Digger
    pub expected_exit_code: i32,
    /// Additional CLI arguments
    pub cli_args: Vec<String>,
    /// Environment variables to set
    pub env_vars: HashMap<String, String>,
    /// Validation rules to apply
    pub validation_rules: Vec<ValidationRule>,
}

impl TestCase {
    /// Create a new test case with default values
    #[allow(dead_code)]
    pub fn new(name: &str, query: &str) -> Self {
        Self {
            name: name.to_string(),
            query: query.to_string(),
            expected_format: OutputFormat::Csv,
            expected_exit_code: 0,
            cli_args: Vec::new(),
            env_vars: HashMap::new(),
            validation_rules: Vec::new(),
        }
    }

    /// Set the expected output format
    #[allow(dead_code)]
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.expected_format = format;
        self
    }

    /// Set the expected exit code
    #[allow(dead_code)]
    pub fn with_exit_code(mut self, code: i32) -> Self {
        self.expected_exit_code = code;
        self
    }

    /// Add a CLI argument
    #[allow(dead_code)]
    pub fn with_arg(mut self, arg: &str) -> Self {
        self.cli_args.push(arg.to_string());
        self
    }

    /// Add an environment variable
    #[allow(dead_code)]
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env_vars.insert(key.to_string(), value.to_string());
        self
    }

    /// Add a validation rule
    #[allow(dead_code)]
    pub fn with_validation(mut self, rule: ValidationRule) -> Self {
        self.validation_rules.push(rule);
        self
    }
}

/// Validation rules for test output verification
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ValidationRule {
    /// Validate exact row count
    RowCount(usize),
    /// Validate exact column count
    ColumnCount(usize),
    /// Validate that a column contains a specific value
    ContainsValue(String, String), // column, value
    /// Validate NULL handling for a specific column
    NullHandling(String), // column name
    /// Validate format compliance
    FormatCompliance(FormatType),
    /// Validate performance threshold
    PerformanceThreshold(Duration),
}

/// Format compliance types for validation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum FormatType {
    /// RFC4180 CSV compliance
    Rfc4180Csv,
    /// JSON structure compliance
    JsonStructure,
    /// TSV format compliance
    TsvFormat,
}

/// Test result structure for validation outcomes
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TestResult {
    /// Test case name
    pub test_name: String,
    /// Test execution status
    pub status: TestStatus,
    /// Execution time
    pub execution_time: Duration,
    /// Output file path (if any)
    pub output_file: Option<PathBuf>,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Validation results
    pub validation_results: Vec<ValidationResult>,
    /// Performance metrics (if measured)
    pub performance_metrics: Option<PerformanceResult>,
}

/// Test execution status
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum TestStatus {
    /// Test passed successfully
    Passed,
    /// Test failed
    Failed,
    /// Test was skipped
    Skipped,
    /// Test encountered an error
    Error,
}

/// Validation result for individual rules
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ValidationResult {
    /// The validation rule that was applied
    pub rule: ValidationRule,
    /// Whether the validation passed
    pub passed: bool,
    /// Validation message
    pub message: String,
    /// Actual value (if applicable)
    pub actual_value: Option<String>,
    /// Expected value (if applicable)
    pub expected_value: Option<String>,
}

/// Performance measurement result
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PerformanceResult {
    /// Query execution time
    pub execution_time: Duration,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Number of rows processed
    pub rows_processed: usize,
    /// Output file size in bytes
    pub output_size: usize,
}

/// Gold Digger execution result
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GoldDiggerResult {
    /// Number of rows in the result
    pub row_count: usize,
    /// Size of the output file in bytes
    pub output_size: u64,
}

/// Common test setup and utilities
pub struct TestSetup {
    /// Temporary directory for test files
    pub temp_dir: TempDir,
}

impl TestSetup {
    /// Create a new test setup with temporary directory
    #[allow(dead_code)]
    pub fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        Ok(Self { temp_dir })
    }

    /// Get the path to the temporary directory
    #[allow(dead_code)]
    pub fn temp_path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Create a temporary file path with the given name
    #[allow(dead_code)]
    pub fn temp_file_path(&self, name: &str) -> PathBuf {
        self.temp_dir.path().join(name)
    }
}

/// Check if Docker is available for container-based tests
pub fn is_docker_available() -> bool {
    std::process::Command::new("docker")
        .arg("version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Skip test if Docker is not available
#[allow(dead_code)]
pub fn skip_if_no_docker() {
    if !is_docker_available() {
        println!("Skipping test: Docker not available");
    }
}

/// Check if running in CI environment
pub fn is_ci_environment() -> bool {
    std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok()
}

/// Get appropriate timeout for CI vs local execution
#[allow(dead_code)]
pub fn get_test_timeout() -> Duration {
    if is_ci_environment() {
        Duration::from_secs(300) // 5 minutes for CI
    } else {
        Duration::from_secs(60) // 1 minute for local
    }
}
