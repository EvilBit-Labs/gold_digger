//! CLI testing utilities for Gold Digger
//!
//! This module provides utilities for executing Gold Digger CLI commands
//! in integration tests with proper error handling and output capture.

#![allow(dead_code)]

use anyhow::{Context, Result};
use assert_cmd::Command;
use std::collections::HashMap;
use std::path::Path;
use std::process::Output;
use std::time::{Duration, Instant};

/// CLI command builder for Gold Digger tests
#[derive(Debug, Clone)]
pub struct GoldDiggerCommand {
    /// Database URL
    pub db_url: Option<String>,
    /// SQL query to execute
    pub query: Option<String>,
    /// Query file path
    pub query_file: Option<String>,
    /// Output file path
    pub output: Option<String>,
    /// Output format override
    pub format: Option<String>,
    /// Verbose flag
    pub verbose: bool,
    /// Quiet flag
    pub quiet: bool,
    /// Allow empty results flag
    pub allow_empty: bool,
    /// Additional CLI arguments
    pub extra_args: Vec<String>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Expected exit code
    pub expected_exit_code: Option<i32>,
}

impl GoldDiggerCommand {
    /// Create a new command builder
    pub fn new() -> Self {
        Self {
            db_url: None,
            query: None,
            query_file: None,
            output: None,
            format: None,
            verbose: false,
            quiet: false,
            allow_empty: false,
            extra_args: Vec::new(),
            env_vars: HashMap::new(),
            expected_exit_code: None,
        }
    }

    /// Set the database URL
    pub fn db_url<S: Into<String>>(mut self, url: S) -> Self {
        self.db_url = Some(url.into());
        self
    }

    /// Set the SQL query
    pub fn query<S: Into<String>>(mut self, query: S) -> Self {
        self.query = Some(query.into());
        self
    }

    /// Set the query file path
    pub fn query_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.query_file = Some(path.as_ref().to_string_lossy().to_string());
        self
    }

    /// Set the output file path
    pub fn output<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.output = Some(path.as_ref().to_string_lossy().to_string());
        self
    }

    /// Set the output format
    pub fn format<S: Into<String>>(mut self, format: S) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Enable verbose output
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Enable quiet mode
    pub fn quiet(mut self) -> Self {
        self.quiet = true;
        self
    }

    /// Allow empty results
    pub fn allow_empty(mut self) -> Self {
        self.allow_empty = true;
        self
    }

    /// Add an extra CLI argument
    pub fn arg<S: Into<String>>(mut self, arg: S) -> Self {
        self.extra_args.push(arg.into());
        self
    }

    /// Add multiple CLI arguments
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.extra_args.extend(args.into_iter().map(|s| s.into()));
        self
    }

    /// Set an environment variable
    pub fn env<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Set expected exit code for validation
    pub fn expect_exit_code(mut self, code: i32) -> Self {
        self.expected_exit_code = Some(code);
        self
    }

    /// Execute the command and return the output
    pub fn execute(self) -> Result<CommandResult> {
        let start_time = Instant::now();

        let mut cmd = Command::cargo_bin("gold_digger")?;

        // Add database URL
        if let Some(url) = &self.db_url {
            cmd.arg("--db-url").arg(url);
        }

        // Add query or query file
        if let Some(query) = &self.query {
            cmd.arg("--query").arg(query);
        } else if let Some(query_file) = &self.query_file {
            cmd.arg("--query-file").arg(query_file);
        }

        // Add output file
        if let Some(output) = &self.output {
            cmd.arg("--output").arg(output);
        }

        // Add format if specified
        if let Some(format) = &self.format {
            cmd.arg("--format").arg(format);
        }

        // Add flags
        if self.verbose {
            cmd.arg("--verbose");
        }

        if self.quiet {
            cmd.arg("--quiet");
        }

        if self.allow_empty {
            cmd.arg("--allow-empty");
        }

        // Add extra arguments
        for arg in &self.extra_args {
            cmd.arg(arg);
        }

        // Set environment variables
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        // Execute command
        let output = cmd.output().context("Failed to execute Gold Digger command")?;

        let execution_time = start_time.elapsed();
        let exit_code = output.status.code().unwrap_or(-1);

        // Validate exit code if expected
        if let Some(expected) = self.expected_exit_code
            && exit_code != expected
        {
            return Err(anyhow::anyhow!(
                "Command exited with code {} (expected {}). Stderr: {}",
                exit_code,
                expected,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(CommandResult {
            exit_code,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time,
            raw_output: output,
        })
    }

    /// Execute the command and expect success (exit code 0)
    pub fn execute_success(self) -> Result<CommandResult> {
        self.expect_exit_code(0).execute()
    }

    /// Execute the command and expect failure (non-zero exit code)
    pub fn execute_failure(self) -> Result<CommandResult> {
        let result = self.execute()?;
        if result.exit_code == 0 {
            return Err(anyhow::anyhow!("Command unexpectedly succeeded. Stdout: {}", result.stdout));
        }
        Ok(result)
    }
}

impl Default for GoldDiggerCommand {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of executing a Gold Digger command
#[derive(Debug)]
pub struct CommandResult {
    /// Exit code from the command
    pub exit_code: i32,
    /// Standard output as string
    pub stdout: String,
    /// Standard error as string
    pub stderr: String,
    /// Command execution time
    pub execution_time: Duration,
    /// Raw process output for advanced inspection
    pub raw_output: Output,
}

impl CommandResult {
    /// Check if the command was successful (exit code 0)
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Check if the command failed (non-zero exit code)
    pub fn is_failure(&self) -> bool {
        self.exit_code != 0
    }

    /// Get stdout lines as vector
    pub fn stdout_lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }

    /// Get stderr lines as vector
    pub fn stderr_lines(&self) -> Vec<&str> {
        self.stderr.lines().collect()
    }

    /// Check if stdout contains a specific string
    pub fn stdout_contains(&self, text: &str) -> bool {
        self.stdout.contains(text)
    }

    /// Check if stderr contains a specific string
    pub fn stderr_contains(&self, text: &str) -> bool {
        self.stderr.contains(text)
    }

    /// Assert that the command was successful
    pub fn assert_success(&self) -> Result<()> {
        if !self.is_success() {
            return Err(anyhow::anyhow!("Command failed with exit code {}. Stderr: {}", self.exit_code, self.stderr));
        }
        Ok(())
    }

    /// Assert that the command failed with a specific exit code
    pub fn assert_exit_code(&self, expected: i32) -> Result<()> {
        if self.exit_code != expected {
            return Err(anyhow::anyhow!(
                "Command exited with code {} (expected {}). Stderr: {}",
                self.exit_code,
                expected,
                self.stderr
            ));
        }
        Ok(())
    }
}

/// Utilities for CLI testing
pub struct CliTestUtils;

impl CliTestUtils {
    /// Create a basic Gold Digger command with common defaults
    pub fn basic_command() -> GoldDiggerCommand {
        GoldDiggerCommand::new()
    }

    /// Create a command for testing database connectivity
    pub fn connectivity_test_command(db_url: &str) -> GoldDiggerCommand {
        GoldDiggerCommand::new().db_url(db_url).query("SELECT 1 as test_column")
    }

    /// Create a command for testing output formats
    pub fn format_test_command(db_url: &str, output_path: &Path, format: &str) -> GoldDiggerCommand {
        GoldDiggerCommand::new()
            .db_url(db_url)
            .query("SELECT 'test' as column1, 123 as column2")
            .output(output_path)
            .format(format)
    }

    /// Create a command for testing error scenarios
    pub fn error_test_command(db_url: &str) -> GoldDiggerCommand {
        GoldDiggerCommand::new()
            .db_url(db_url)
            .query("SELECT * FROM non_existent_table")
    }

    /// Redact sensitive information from command output for logging
    pub fn redact_sensitive_output(output: &str) -> String {
        // Replace database URLs and passwords with redacted versions
        let mut redacted = output.to_string();

        // Redact mysql:// URLs
        if let Ok(re) = regex::Regex::new(r"mysql://[^:]+:[^@]+@[^/]+/[^\s]+") {
            redacted = re.replace_all(&redacted, "mysql://***:***@***/***").to_string();
        }

        // Redact DATABASE_URL references
        redacted = redacted.replace("DATABASE_URL", "***DATABASE_URL***");

        redacted
    }
}
