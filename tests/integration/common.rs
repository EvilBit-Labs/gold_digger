//! Common test utilities for Gold Digger integration tests
//!
//! This module provides shared utilities for CLI execution, output parsing,
//! and test data management across all integration tests.

#![allow(dead_code)]

use anyhow::{Context, Result};
use assert_cmd::Command;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Output;
use std::time::Instant;
use tempfile::NamedTempFile;

use super::{GoldDiggerResult, OutputFormat, TestCase};

/// CLI execution utilities for Gold Digger
pub struct GoldDiggerCli {
    /// Path to the Gold Digger binary
    binary_path: String,
}

impl GoldDiggerCli {
    /// Create a new CLI executor
    pub fn new() -> Self {
        Self {
            binary_path: "gold_digger".to_string(),
        }
    }

    /// Execute Gold Digger with the given test case
    pub fn execute(&self, test_case: &TestCase, db_url: &str, output_path: &Path) -> Result<GoldDiggerResult> {
        let start_time = Instant::now();

        // Build command
        let mut cmd = Command::cargo_bin("gold_digger")?;

        // Set database URL
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

        // Execute command
        let output = cmd
            .output()
            .with_context(|| format!("Failed to execute Gold Digger for test case: {}", test_case.name))?;

        let _execution_time = start_time.elapsed();

        // Check exit code
        let actual_exit_code = output.status.code().unwrap_or(-1);
        if actual_exit_code != test_case.expected_exit_code {
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

    /// Execute Gold Digger and capture raw output for error testing
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

        cmd.output()
            .with_context(|| format!("Failed to execute Gold Digger for test case: {}", test_case.name))
    }
}

impl Default for GoldDiggerCli {
    fn default() -> Self {
        Self::new()
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
            unsafe {
                match original_value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
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
