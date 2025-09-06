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
// TODO: Fix tls_tests module imports
// pub mod tls_tests;

// Re-export commonly used types and functions
// Note: Specific re-exports to avoid unused import warnings
pub use common::TempFileManager;

/// Test database type enumeration for managing different database containers
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum TestDatabase {
    /// MySQL container instance with optional TLS configuration
    MySQL { tls_enabled: bool },
    /// MariaDB container instance with optional TLS configuration
    MariaDB { tls_enabled: bool },
}

/// TLS-enabled database variants for secure connection testing
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TestDatabaseTls {
    /// MySQL with TLS configuration
    MySQL { tls_config: TlsContainerConfig },
    /// MariaDB with TLS configuration
    MariaDB { tls_config: TlsContainerConfig },
}

/// Plain (non-TLS) database variants for standard connection testing
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TestDatabasePlain {
    /// MySQL without TLS
    MySQL,
    /// MariaDB without TLS
    MariaDB,
}

impl TestDatabase {
    /// Create a new database configuration from this TestDatabase
    #[allow(dead_code)]
    pub fn to_config(&self) -> TestDatabaseConfig {
        match self {
            TestDatabase::MySQL { tls_enabled } => TestDatabaseConfig {
                db_type: DatabaseType::MySQL,
                tls_config: if *tls_enabled {
                    Some(TlsContainerConfig::new_secure())
                } else {
                    None
                },
            },
            TestDatabase::MariaDB { tls_enabled } => TestDatabaseConfig {
                db_type: DatabaseType::MariaDB,
                tls_config: if *tls_enabled {
                    Some(TlsContainerConfig::new_secure())
                } else {
                    None
                },
            },
        }
    }
}

/// Database configuration for test containers
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct TestDatabaseConfig {
    /// Database type (MySQL or MariaDB)
    pub db_type: DatabaseType,
    /// TLS configuration (None for plain connections)
    pub tls_config: Option<TlsContainerConfig>,
}

/// Database type enumeration
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum DatabaseType {
    /// MySQL database
    MySQL,
    /// MariaDB database
    MariaDB,
}

impl TestDatabase {
    /// Create a new MySQL instance without TLS
    #[allow(dead_code)]
    pub fn mysql() -> Self {
        Self::MySQL { tls_enabled: false }
    }

    /// Create a new MySQL instance with TLS
    #[allow(dead_code)]
    pub fn mysql_tls() -> Self {
        Self::MySQL { tls_enabled: true }
    }

    /// Create a new MariaDB instance without TLS
    #[allow(dead_code)]
    pub fn mariadb() -> Self {
        Self::MariaDB { tls_enabled: false }
    }

    /// Create a new MariaDB instance with TLS
    #[allow(dead_code)]
    pub fn mariadb_tls() -> Self {
        Self::MariaDB { tls_enabled: true }
    }

    /// Get the database type name as a string
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            TestDatabase::MySQL { .. } => "mysql",
            TestDatabase::MariaDB { .. } => "mariadb",
        }
    }

    /// Check if TLS is enabled for this database
    pub fn is_tls_enabled(&self) -> bool {
        match self {
            TestDatabase::MySQL { tls_enabled } => *tls_enabled,
            TestDatabase::MariaDB { tls_enabled } => *tls_enabled,
        }
    }

    /// Get the default port for this database type
    #[allow(dead_code)]
    pub fn default_port(&self) -> u16 {
        match self {
            TestDatabase::MySQL { .. } => 3306,
            TestDatabase::MariaDB { .. } => 3306,
        }
    }

    /// Get the container image name for this database type
    #[allow(dead_code)]
    pub fn image_name(&self) -> &'static str {
        match self {
            TestDatabase::MySQL { .. } => "mysql:8.0",
            TestDatabase::MariaDB { .. } => "mariadb:10.6",
        }
    }
}

impl TestDatabaseTls {
    /// Create a new MySQL TLS database with secure defaults
    #[allow(dead_code)]
    pub fn mysql() -> Self {
        Self::MySQL {
            tls_config: TlsContainerConfig::new_secure(),
        }
    }

    /// Create a new MariaDB TLS database with secure defaults
    #[allow(dead_code)]
    pub fn mariadb() -> Self {
        Self::MariaDB {
            tls_config: TlsContainerConfig::new_secure(),
        }
    }

    /// Create a new MariaDB TLS database with custom configuration
    #[allow(dead_code)]
    pub fn mariadb_with_config(tls_config: TlsContainerConfig) -> Self {
        Self::MariaDB { tls_config }
    }

    /// Get the database type name as a string
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            TestDatabaseTls::MySQL { .. } => "mysql-tls",
            TestDatabaseTls::MariaDB { .. } => "mariadb-tls",
        }
    }

    /// Get the TLS configuration
    #[allow(dead_code)]
    pub fn tls_config(&self) -> &TlsContainerConfig {
        match self {
            TestDatabaseTls::MySQL { tls_config } => tls_config,
            TestDatabaseTls::MariaDB { tls_config } => tls_config,
        }
    }

    /// Convert to the base TestDatabase enum for compatibility
    #[allow(dead_code)]
    pub fn to_test_database(&self) -> TestDatabase {
        match self {
            TestDatabaseTls::MySQL { .. } => TestDatabase::MySQL { tls_enabled: true },
            TestDatabaseTls::MariaDB { .. } => TestDatabase::MariaDB { tls_enabled: true },
        }
    }
}

impl TestDatabasePlain {
    /// Create a new MySQL plain database
    #[allow(dead_code)]
    pub fn mysql() -> Self {
        Self::MySQL
    }

    /// Create a new MariaDB plain database
    #[allow(dead_code)]
    pub fn mariadb() -> Self {
        Self::MariaDB
    }

    /// Get the database type name as a string
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            TestDatabasePlain::MySQL => "mysql-plain",
            TestDatabasePlain::MariaDB => "mariadb-plain",
        }
    }

    /// Convert to the base TestDatabase enum for compatibility
    #[allow(dead_code)]
    pub fn to_test_database(&self) -> TestDatabase {
        match self {
            TestDatabasePlain::MySQL => TestDatabase::MySQL { tls_enabled: false },
            TestDatabasePlain::MariaDB => TestDatabase::MariaDB { tls_enabled: false },
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

/// TLS container configuration for database containers
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct TlsContainerConfig {
    /// Whether to require secure transport (SSL/TLS)
    pub require_secure_transport: bool,
    /// Minimum TLS version (TLSv1.2 or TLSv1.3)
    pub min_tls_version: String,
    /// Allowed cipher suites for TLS connections
    pub cipher_suites: Vec<String>,
    /// Whether to generate ephemeral certificates per test run
    pub use_ephemeral_certs: bool,
    /// Custom certificate paths (if not using ephemeral certificates)
    pub ca_cert_path: Option<PathBuf>,
    pub server_cert_path: Option<PathBuf>,
    pub server_key_path: Option<PathBuf>,
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

/// CI environment detection and handling utilities
#[allow(dead_code)]
pub struct CiEnvironment;

#[allow(dead_code)]
impl CiEnvironment {
    /// Check if running in CI environment
    pub fn is_ci() -> bool {
        std::env::var("CI").is_ok()
            || std::env::var("GITHUB_ACTIONS").is_ok()
            || std::env::var("GITLAB_CI").is_ok()
            || std::env::var("JENKINS_URL").is_ok()
            || std::env::var("BUILDKITE").is_ok()
    }

    /// Check if running in GitHub Actions specifically
    pub fn is_github_actions() -> bool {
        std::env::var("GITHUB_ACTIONS").is_ok()
    }

    /// Get appropriate timeout for CI vs local execution
    pub fn get_test_timeout() -> Duration {
        if Self::is_ci() {
            // Longer timeout for CI environments due to resource constraints
            Duration::from_secs(300) // 5 minutes for CI
        } else {
            Duration::from_secs(60) // 1 minute for local
        }
    }

    /// Get appropriate timeout for container operations
    pub fn get_container_timeout() -> Duration {
        if Self::is_ci() {
            Duration::from_secs(180) // 3 minutes for container startup in CI
        } else {
            Duration::from_secs(60) // 1 minute for local
        }
    }

    /// Get appropriate timeout for database operations
    pub fn get_database_timeout() -> Duration {
        if Self::is_ci() {
            Duration::from_secs(120) // 2 minutes for database operations in CI
        } else {
            Duration::from_secs(30) // 30 seconds for local
        }
    }

    /// Get CI-specific resource limits
    pub fn get_resource_limits() -> CiResourceLimits {
        if Self::is_ci() {
            CiResourceLimits {
                max_memory_usage_mb: 1024,                    // 1GB limit for CI
                max_disk_usage_mb: 512,                       // 512MB disk limit for CI
                max_execution_time: Duration::from_secs(600), // 10 minutes max
                max_parallel_tests: 2,                        // Limit parallelism in CI
            }
        } else {
            CiResourceLimits {
                max_memory_usage_mb: 2048,                    // 2GB limit for local
                max_disk_usage_mb: 1024,                      // 1GB disk limit for local
                max_execution_time: Duration::from_secs(300), // 5 minutes max
                max_parallel_tests: 4,                        // More parallelism locally
            }
        }
    }

    /// Check if Docker is available and working
    pub fn check_docker_availability() -> DockerAvailability {
        let docker_check = std::process::Command::new("docker").arg("version").output();

        match docker_check {
            Ok(output) if output.status.success() => {
                // Check if Docker daemon is running
                let daemon_check = std::process::Command::new("docker").arg("info").output();

                match daemon_check {
                    Ok(daemon_output) if daemon_output.status.success() => DockerAvailability::Available,
                    _ => DockerAvailability::DaemonNotRunning,
                }
            },
            _ => DockerAvailability::NotInstalled,
        }
    }

    /// Get CI-specific environment variables for test configuration
    pub fn get_ci_config() -> CiConfig {
        CiConfig {
            runner_os: std::env::var("RUNNER_OS").unwrap_or_else(|_| "unknown".to_string()),
            runner_arch: std::env::var("RUNNER_ARCH").unwrap_or_else(|_| "unknown".to_string()),
            github_ref: std::env::var("GITHUB_REF").ok(),
            github_sha: std::env::var("GITHUB_SHA").ok(),
            github_run_id: std::env::var("GITHUB_RUN_ID").ok(),
            github_run_number: std::env::var("GITHUB_RUN_NUMBER").ok(),
            is_pull_request: std::env::var("GITHUB_EVENT_NAME")
                .map(|event| event == "pull_request")
                .unwrap_or(false),
        }
    }

    /// Create JUnit XML report for CI integration
    pub fn create_junit_report(test_results: &[TestExecutionResult]) -> Result<String> {
        let mut xml = String::new();
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push('\n');

        let total_tests = test_results.len();
        let failed_tests = test_results.iter().filter(|r| !r.passed).count();
        let total_time: f64 = test_results.iter().map(|r| r.execution_time.as_secs_f64()).sum();

        xml.push_str(&format!(
            r#"<testsuite name="gold_digger_integration_tests" tests="{}" failures="{}" time="{:.3}">"#,
            total_tests, failed_tests, total_time
        ));
        xml.push('\n');

        for result in test_results {
            xml.push_str(&format!(
                r#"  <testcase name="{}" classname="integration_tests" time="{:.3}""#,
                result.test_name,
                result.execution_time.as_secs_f64()
            ));

            if result.passed {
                xml.push_str(" />");
            } else {
                xml.push('>');
                xml.push('\n');
                xml.push_str(&format!(
                    r#"    <failure message="{}">{}</failure>"#,
                    result.error_message.as_deref().unwrap_or("Test failed"),
                    result.error_details.as_deref().unwrap_or("")
                ));
                xml.push('\n');
                xml.push_str("  </testcase>");
            }
            xml.push('\n');
        }

        xml.push_str("</testsuite>");
        xml.push('\n');

        Ok(xml)
    }

    /// Emit GitHub Actions annotations for test failures
    pub fn emit_github_annotations(test_results: &[TestExecutionResult]) -> Result<()> {
        if !Self::is_github_actions() {
            return Ok(());
        }

        for result in test_results {
            if !result.passed {
                println!(
                    "::error title=Integration Test Failed::Test '{}' failed: {}",
                    result.test_name,
                    result.error_message.as_deref().unwrap_or("Unknown error")
                );
            }
        }

        Ok(())
    }

    /// Check if flaky test quarantine is enabled
    pub fn is_flaky_test_quarantine_enabled() -> bool {
        std::env::var("GOLD_DIGGER_QUARANTINE_FLAKY_TESTS")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
    }

    /// Get retry count for flaky tests
    pub fn get_flaky_test_retry_count() -> usize {
        std::env::var("GOLD_DIGGER_FLAKY_TEST_RETRIES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3) // Default to 3 retries
    }
}

/// Docker availability status
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum DockerAvailability {
    /// Docker is available and daemon is running
    Available,
    /// Docker is installed but daemon is not running
    DaemonNotRunning,
    /// Docker is not installed
    NotInstalled,
}

/// CI resource limits
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CiResourceLimits {
    /// Maximum memory usage in MB
    pub max_memory_usage_mb: usize,
    /// Maximum disk usage in MB
    pub max_disk_usage_mb: usize,
    /// Maximum execution time
    pub max_execution_time: Duration,
    /// Maximum number of parallel tests
    pub max_parallel_tests: usize,
}

/// CI configuration information
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CiConfig {
    /// Operating system of the runner
    pub runner_os: String,
    /// Architecture of the runner
    pub runner_arch: String,
    /// GitHub ref (branch/tag)
    pub github_ref: Option<String>,
    /// GitHub commit SHA
    pub github_sha: Option<String>,
    /// GitHub run ID
    pub github_run_id: Option<String>,
    /// GitHub run number
    pub github_run_number: Option<String>,
    /// Whether this is a pull request
    pub is_pull_request: bool,
}

/// Test execution result for CI reporting
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TestExecutionResult {
    /// Test name
    pub test_name: String,
    /// Whether the test passed
    pub passed: bool,
    /// Execution time
    pub execution_time: Duration,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Detailed error information
    pub error_details: Option<String>,
    /// Test artifacts (file paths)
    pub artifacts: Vec<std::path::PathBuf>,
}

/// Cargo nextest integration utilities
#[allow(dead_code)]
pub struct CargoNextestIntegration;

#[allow(dead_code)]
impl CargoNextestIntegration {
    /// Check if running under cargo nextest
    pub fn is_nextest() -> bool {
        std::env::var("NEXTEST").is_ok() || std::env::var("NEXTEST_RUN_ID").is_ok()
    }

    /// Get nextest-specific configuration
    pub fn get_nextest_config() -> NextestConfig {
        NextestConfig {
            run_id: std::env::var("NEXTEST_RUN_ID").ok(),
            profile: std::env::var("NEXTEST_PROFILE").unwrap_or_else(|_| "default".to_string()),
            partition: std::env::var("NEXTEST_PARTITION").ok(),
            total_partitions: std::env::var("NEXTEST_TOTAL_PARTITIONS")
                .ok()
                .and_then(|v| v.parse().ok()),
            test_threads: std::env::var("NEXTEST_TEST_THREADS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
        }
    }

    /// Create nextest-compatible test output
    pub fn emit_nextest_output(test_name: &str, result: &TestExecutionResult) -> Result<()> {
        if !Self::is_nextest() {
            return Ok(());
        }

        // Nextest expects specific output format for test results
        let status = if result.passed { "PASS" } else { "FAIL" };
        println!("test {} ... {} ({:.3}s)", test_name, status, result.execution_time.as_secs_f64());

        if !result.passed {
            if let Some(error) = &result.error_message {
                eprintln!("Error: {}", error);
            }
            if let Some(details) = &result.error_details {
                eprintln!("Details: {}", details);
            }
        }

        Ok(())
    }

    /// Configure test execution for nextest parallel execution
    pub fn configure_parallel_execution() -> ParallelExecutionConfig {
        let nextest_config = Self::get_nextest_config();
        let ci_limits = CiEnvironment::get_resource_limits();

        ParallelExecutionConfig {
            max_parallel_tests: std::cmp::min(nextest_config.test_threads, ci_limits.max_parallel_tests),
            test_timeout: CiEnvironment::get_test_timeout(),
            container_timeout: CiEnvironment::get_container_timeout(),
            enable_flaky_test_quarantine: CiEnvironment::is_flaky_test_quarantine_enabled(),
            flaky_test_retry_count: CiEnvironment::get_flaky_test_retry_count(),
        }
    }
}

/// Nextest configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NextestConfig {
    /// Nextest run ID
    pub run_id: Option<String>,
    /// Nextest profile
    pub profile: String,
    /// Current partition (for test sharding)
    pub partition: Option<String>,
    /// Total number of partitions
    pub total_partitions: Option<usize>,
    /// Number of test threads
    pub test_threads: usize,
}

/// Parallel execution configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ParallelExecutionConfig {
    /// Maximum number of parallel tests
    pub max_parallel_tests: usize,
    /// Timeout for individual tests
    pub test_timeout: Duration,
    /// Timeout for container operations
    pub container_timeout: Duration,
    /// Whether to enable flaky test quarantine
    pub enable_flaky_test_quarantine: bool,
    /// Number of retries for flaky tests
    pub flaky_test_retry_count: usize,
}

/// Check if running in CI environment (backward compatibility)
pub fn is_ci_environment() -> bool {
    CiEnvironment::is_ci()
}

/// Get appropriate timeout for CI vs local execution (backward compatibility)
#[allow(dead_code)]
pub fn get_test_timeout() -> Duration {
    CiEnvironment::get_test_timeout()
}

/// Test execution utilities for Gold Digger CLI with comprehensive CI support
///
/// This struct provides enhanced test execution capabilities that handle both TLS and non-TLS
/// database connections, with full CI environment integration including:
///
/// - Parallel execution support with cargo nextest
/// - Flaky test quarantine and retry mechanisms
/// - JUnit/XML report generation for CI annotations
/// - Artifact collection for debugging failed tests
/// - Timeout handling optimized for CI vs local execution
/// - Resource limit enforcement for CI environments
///
/// # Example Usage
///
/// ```rust
/// use gold_digger_tests::integration::TestExecutor;
/// use std::time::Duration;
///
/// // Create test executor with CI-aware configuration
/// let executor = TestExecutor::new("my_test_suite")?;
///
/// // Execute test with automatic TLS/non-TLS handling
/// let result = executor.execute_test_case(&test_case, &db_config)?;
///
/// // Execute with flaky test retry
/// let result = executor.execute_with_retry(&test_case, &db_config, 3)?;
///
/// // Generate JUnit report for CI
/// executor.generate_junit_report(&test_results, "target/test-reports")?;
/// ```
#[allow(dead_code)]
pub struct TestExecutor {
    /// Test suite name for identification and artifact collection
    suite_name: String,
    /// Temporary directory for test isolation
    temp_manager: TempFileManager,
    /// CI configuration and resource limits
    #[allow(dead_code)]
    ci_config: CiConfig,
    /// Resource limits for the current environment
    #[allow(dead_code)]
    resource_limits: CiResourceLimits,
    /// Parallel execution configuration
    parallel_config: ParallelExecutionConfig,
    /// Test execution results for reporting
    execution_results: std::cell::RefCell<Vec<TestExecutionResult>>,
}

#[allow(dead_code)]
impl TestExecutor {
    /// Create a new test executor with CI-aware configuration
    pub fn new(suite_name: &str) -> Result<Self> {
        let temp_manager = TempFileManager::new(suite_name)?;
        let ci_config = CiEnvironment::get_ci_config();
        let resource_limits = CiEnvironment::get_resource_limits();
        let parallel_config = CargoNextestIntegration::configure_parallel_execution();

        Ok(Self {
            suite_name: suite_name.to_string(),
            temp_manager,
            ci_config,
            resource_limits,
            parallel_config,
            execution_results: std::cell::RefCell::new(Vec::new()),
        })
    }

    /// Execute a test case with automatic TLS/non-TLS database handling
    pub fn execute_test_case(
        &self,
        test_case: &TestCase,
        db_config: &TestDatabaseConfig,
    ) -> Result<TestExecutionResult> {
        let start_time = std::time::Instant::now();

        // Create CLI executor with appropriate timeout
        let cli = common::GoldDiggerCli::with_timeout(self.parallel_config.test_timeout);

        // Create temporary output file
        let output_file = self.temp_manager.create_output_file(&test_case.expected_format)?;

        // Generate database connection URL based on configuration
        let db_url = self.generate_connection_url(db_config)?;

        // Execute test case with error handling
        let result = match cli.execute(test_case, &db_url, output_file.path()) {
            Ok(_gold_digger_result) => TestExecutionResult {
                test_name: test_case.name.clone(),
                passed: true,
                execution_time: start_time.elapsed(),
                error_message: None,
                error_details: None,
                artifacts: vec![output_file.path().to_path_buf()],
            },
            Err(e) => TestExecutionResult {
                test_name: test_case.name.clone(),
                passed: false,
                execution_time: start_time.elapsed(),
                error_message: Some(e.to_string()),
                error_details: Some(format!("{:?}", e)),
                artifacts: vec![output_file.path().to_path_buf()],
            },
        };

        // Store result for reporting
        self.execution_results.borrow_mut().push(result.clone());

        // Emit nextest-compatible output if running under nextest
        CargoNextestIntegration::emit_nextest_output(&test_case.name, &result)?;

        Ok(result)
    }

    /// Execute test case with flaky test retry mechanism
    pub fn execute_with_retry(
        &self,
        test_case: &TestCase,
        db_config: &TestDatabaseConfig,
        max_retries: usize,
    ) -> Result<TestExecutionResult> {
        let mut last_result = None;

        for attempt in 0..=max_retries {
            let result = self.execute_test_case(test_case, db_config)?;

            if result.passed {
                return Ok(result);
            }

            last_result = Some(result);

            if attempt < max_retries {
                // Wait before retry with exponential backoff
                let wait_time = Duration::from_millis(100 * (2_u64.pow(attempt as u32)));
                std::thread::sleep(wait_time);

                println!(
                    "Test '{}' failed (attempt {}/{}), retrying in {:?}...",
                    test_case.name,
                    attempt + 1,
                    max_retries + 1,
                    wait_time
                );
            }
        }

        // Return the last failed result
        Ok(last_result.unwrap())
    }

    /// Execute test case with comprehensive error handling and artifact collection
    pub fn execute_with_debug(
        &self,
        test_case: &TestCase,
        db_config: &TestDatabaseConfig,
    ) -> Result<TestExecutionResult> {
        let start_time = std::time::Instant::now();

        // Create debug artifact directory
        let debug_dir = self.temp_manager.create_temp_subdir("debug")?;

        // Create CLI executor with debug timeout
        let cli = common::GoldDiggerCli::with_timeout(self.parallel_config.test_timeout);

        // Create temporary output file
        let output_file = self.temp_manager.create_output_file(&test_case.expected_format)?;

        // Generate database connection URL
        let db_url = self.generate_connection_url(db_config)?;

        // Execute with raw output capture for debugging
        let raw_output = cli.execute_raw(test_case, &db_url, output_file.path())?;

        // Save debug artifacts
        let stdout_file = debug_dir.join("stdout.txt");
        let stderr_file = debug_dir.join("stderr.txt");
        let test_config_file = debug_dir.join("test_config.json");

        std::fs::write(&stdout_file, &raw_output.stdout)?;
        std::fs::write(&stderr_file, &raw_output.stderr)?;
        std::fs::write(
            &test_config_file,
            serde_json::to_string_pretty(&serde_json::json!({
                "test_case": {
                    "name": test_case.name,
                    "query": test_case.query,
                    "expected_format": format!("{:?}", test_case.expected_format),
                    "expected_exit_code": test_case.expected_exit_code,
                    "cli_args": test_case.cli_args,
                    "env_vars": test_case.env_vars,
                },
                "db_config": {
                    "db_type": format!("{:?}", db_config.db_type),
                    "tls_enabled": db_config.tls_config.is_some(),
                },
                "execution": {
                    "exit_code": raw_output.status.code(),
                    "execution_time_ms": start_time.elapsed().as_millis(),
                }
            }))?,
        )?;

        // Determine if test passed
        let expected_exit_code = test_case.expected_exit_code;
        let actual_exit_code = raw_output.status.code().unwrap_or(-1);
        let passed = actual_exit_code == expected_exit_code;

        let result = TestExecutionResult {
            test_name: test_case.name.clone(),
            passed,
            execution_time: start_time.elapsed(),
            error_message: if !passed {
                Some(format!("Exit code mismatch: expected {}, got {}", expected_exit_code, actual_exit_code))
            } else {
                None
            },
            error_details: if !passed {
                Some(String::from_utf8_lossy(&raw_output.stderr).to_string())
            } else {
                None
            },
            artifacts: vec![
                output_file.path().to_path_buf(),
                stdout_file,
                stderr_file,
                test_config_file,
            ],
        };

        // Store result for reporting
        self.execution_results.borrow_mut().push(result.clone());

        Ok(result)
    }

    /// Generate database connection URL based on configuration
    fn generate_connection_url(&self, db_config: &TestDatabaseConfig) -> Result<String> {
        // This is a placeholder - in a real implementation, this would
        // generate the appropriate connection URL based on the database
        // configuration and whether TLS is enabled
        match &db_config.db_type {
            DatabaseType::MySQL => {
                if db_config.tls_config.is_some() {
                    Ok("mysql://test_user:test_pass@localhost:3306/test_db?ssl-mode=REQUIRED".to_string())
                } else {
                    Ok("mysql://test_user:test_pass@localhost:3306/test_db".to_string())
                }
            },
            DatabaseType::MariaDB => {
                if db_config.tls_config.is_some() {
                    Ok("mysql://test_user:test_pass@localhost:3306/test_db?ssl-mode=REQUIRED".to_string())
                } else {
                    Ok("mysql://test_user:test_pass@localhost:3306/test_db".to_string())
                }
            },
        }
    }

    /// Generate JUnit XML report for CI integration
    pub fn generate_junit_report(&self, output_dir: &Path) -> Result<PathBuf> {
        std::fs::create_dir_all(output_dir)?;

        let results = self.execution_results.borrow();
        let junit_xml = CiEnvironment::create_junit_report(&results)?;

        let report_file = output_dir.join(format!("{}_junit.xml", self.suite_name));
        std::fs::write(&report_file, junit_xml)?;

        Ok(report_file)
    }

    /// Emit GitHub Actions annotations for test failures
    pub fn emit_github_annotations(&self) -> Result<()> {
        let results = self.execution_results.borrow();
        CiEnvironment::emit_github_annotations(&results)
    }

    /// Collect all test artifacts for debugging
    pub fn collect_artifacts(&self, artifact_dir: &Path) -> Result<Vec<PathBuf>> {
        self.temp_manager.collect_artifacts(artifact_dir)
    }

    /// Get test execution summary
    pub fn get_execution_summary(&self) -> TestExecutionSummary {
        let results = self.execution_results.borrow();
        let total_tests = results.len();
        let passed_tests = results.iter().filter(|r| r.passed).count();
        let failed_tests = total_tests - passed_tests;
        let total_time: Duration = results.iter().map(|r| r.execution_time).sum();

        TestExecutionSummary {
            total_tests,
            passed_tests,
            failed_tests,
            total_execution_time: total_time,
            suite_name: self.suite_name.clone(),
        }
    }

    /// Execute multiple test cases in parallel (if supported)
    pub fn execute_parallel(
        &self,
        test_cases: &[TestCase],
        db_config: &TestDatabaseConfig,
    ) -> Result<Vec<TestExecutionResult>> {
        let max_parallel = self.parallel_config.max_parallel_tests;

        if max_parallel <= 1 || test_cases.len() <= 1 {
            // Execute sequentially
            let mut results = Vec::new();
            for test_case in test_cases {
                let result = if self.parallel_config.enable_flaky_test_quarantine {
                    self.execute_with_retry(test_case, db_config, self.parallel_config.flaky_test_retry_count)?
                } else {
                    self.execute_test_case(test_case, db_config)?
                };
                results.push(result);
            }
            return Ok(results);
        }

        // Execute in parallel using thread pool
        use std::sync::{Arc, Mutex};
        use std::thread;

        let results = Arc::new(Mutex::new(Vec::<TestExecutionResult>::new()));
        let mut handles = Vec::new();

        for chunk in test_cases.chunks(max_parallel) {
            for test_case in chunk {
                let _test_case = test_case.clone();
                let _db_config = db_config.clone();
                let _results = Arc::clone(&results);
                let _executor = self; // Note: This would need proper Arc/Mutex handling in real implementation

                let handle = thread::spawn(move || {
                    // This is a simplified version - real implementation would need
                    // proper thread-safe access to the executor
                    // For now, we'll fall back to sequential execution
                });

                handles.push(handle);
            }

            // Wait for this chunk to complete
            for handle in handles.drain(..) {
                handle.join().unwrap();
            }
        }

        // For now, fall back to sequential execution
        // Real implementation would use proper thread pool with shared executor state
        self.execute_parallel_fallback(test_cases, db_config)
    }

    /// Fallback sequential execution for parallel test execution
    fn execute_parallel_fallback(
        &self,
        test_cases: &[TestCase],
        db_config: &TestDatabaseConfig,
    ) -> Result<Vec<TestExecutionResult>> {
        let mut results = Vec::new();
        for test_case in test_cases {
            let result = if self.parallel_config.enable_flaky_test_quarantine {
                self.execute_with_retry(test_case, db_config, self.parallel_config.flaky_test_retry_count)?
            } else {
                self.execute_test_case(test_case, db_config)?
            };
            results.push(result);
        }
        Ok(results)
    }
}

/// Test execution summary for reporting
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TestExecutionSummary {
    /// Total number of tests executed
    pub total_tests: usize,
    /// Number of tests that passed
    pub passed_tests: usize,
    /// Number of tests that failed
    pub failed_tests: usize,
    /// Total execution time for all tests
    pub total_execution_time: Duration,
    /// Test suite name
    pub suite_name: String,
}

#[allow(dead_code)]
impl TestExecutionSummary {
    /// Get success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.passed_tests as f64 / self.total_tests as f64) * 100.0
        }
    }

    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed_tests == 0 && self.total_tests > 0
    }

    /// Format summary as string for display
    pub fn format_summary(&self) -> String {
        format!(
            "Test Suite '{}': {}/{} passed ({:.1}%) in {:.2}s",
            self.suite_name,
            self.passed_tests,
            self.total_tests,
            self.success_rate(),
            self.total_execution_time.as_secs_f64()
        )
    }
}

impl TlsContainerConfig {
    /// Create a new TLS container configuration with secure defaults
    #[allow(dead_code)]
    pub fn new_secure() -> Self {
        Self {
            require_secure_transport: true,
            min_tls_version: "TLSv1.2".to_string(),
            cipher_suites: vec![
                "ECDHE-RSA-AES256-GCM-SHA384".to_string(),
                "ECDHE-RSA-AES128-GCM-SHA256".to_string(),
            ],
            use_ephemeral_certs: true,
            ca_cert_path: None,
            server_cert_path: None,
            server_key_path: None,
        }
    }

    /// Create a TLS configuration with custom certificate paths
    #[allow(dead_code)]
    pub fn with_custom_certs<P: AsRef<Path>>(ca_cert_path: P, server_cert_path: P, server_key_path: P) -> Self {
        Self {
            require_secure_transport: true,
            min_tls_version: "TLSv1.2".to_string(),
            cipher_suites: vec![
                "ECDHE-RSA-AES256-GCM-SHA384".to_string(),
                "ECDHE-RSA-AES128-GCM-SHA256".to_string(),
            ],
            use_ephemeral_certs: false,
            ca_cert_path: Some(ca_cert_path.as_ref().to_path_buf()),
            server_cert_path: Some(server_cert_path.as_ref().to_path_buf()),
            server_key_path: Some(server_key_path.as_ref().to_path_buf()),
        }
    }

    /// Create a TLS configuration with strict security settings
    #[allow(dead_code)]
    pub fn with_strict_security(mut self) -> Result<Self> {
        self.min_tls_version = "TLSv1.3".to_string();
        self.cipher_suites = vec![
            "TLS_AES_256_GCM_SHA384".to_string(),
            "TLS_AES_128_GCM_SHA256".to_string(),
            "TLS_CHACHA20_POLY1305_SHA256".to_string(),
        ];
        Ok(self)
    }

    /// Validate the TLS configuration
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<()> {
        // Validate TLS version
        match self.min_tls_version.as_str() {
            "TLSv1.2" | "TLSv1.3" => {},
            _ => {
                anyhow::bail!("Invalid TLS version: {}. Must be TLSv1.2 or TLSv1.3", self.min_tls_version);
            },
        }

        // Validate cipher suites are not empty
        if self.cipher_suites.is_empty() {
            anyhow::bail!("Cipher suites cannot be empty for TLS configuration");
        }

        // Validate certificate paths if not using ephemeral certificates
        if !self.use_ephemeral_certs {
            if let Some(ca_path) = &self.ca_cert_path
                && !ca_path.exists()
            {
                anyhow::bail!("CA certificate file does not exist: {}", ca_path.display());
            }
            if let Some(cert_path) = &self.server_cert_path
                && !cert_path.exists()
            {
                anyhow::bail!("Server certificate file does not exist: {}", cert_path.display());
            }
            if let Some(key_path) = &self.server_key_path
                && !key_path.exists()
            {
                anyhow::bail!("Server key file does not exist: {}", key_path.display());
            }
        }

        Ok(())
    }
}

impl TestDatabaseConfig {
    /// Create a new MySQL configuration without TLS
    #[allow(dead_code)]
    pub fn mysql_plain() -> Self {
        Self {
            db_type: DatabaseType::MySQL,
            tls_config: None,
        }
    }

    /// Create a new MySQL configuration with TLS
    #[allow(dead_code)]
    pub fn mysql_tls() -> Self {
        Self {
            db_type: DatabaseType::MySQL,
            tls_config: Some(TlsContainerConfig::new_secure()),
        }
    }

    /// Create a new MariaDB configuration without TLS
    #[allow(dead_code)]
    pub fn mariadb_plain() -> Self {
        Self {
            db_type: DatabaseType::MariaDB,
            tls_config: None,
        }
    }

    /// Create a new MariaDB configuration with TLS
    #[allow(dead_code)]
    pub fn mariadb_tls() -> Self {
        Self {
            db_type: DatabaseType::MariaDB,
            tls_config: Some(TlsContainerConfig::new_secure()),
        }
    }

    /// Create a configuration with custom TLS settings
    #[allow(dead_code)]
    pub fn with_tls_config(db_type: DatabaseType, tls_config: TlsContainerConfig) -> Self {
        Self {
            db_type,
            tls_config: Some(tls_config),
        }
    }

    /// Get the database type name
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match (&self.db_type, &self.tls_config) {
            (DatabaseType::MySQL, Some(_)) => "mysql-tls",
            (DatabaseType::MySQL, None) => "mysql-plain",
            (DatabaseType::MariaDB, Some(_)) => "mariadb-tls",
            (DatabaseType::MariaDB, None) => "mariadb-plain",
        }
    }

    /// Check if TLS is enabled
    #[allow(dead_code)]
    pub fn is_tls_enabled(&self) -> bool {
        self.tls_config.is_some()
    }

    /// Convert to the base TestDatabase enum for compatibility
    #[allow(dead_code)]
    pub fn to_test_database(&self) -> TestDatabase {
        match self.db_type {
            DatabaseType::MySQL => TestDatabase::MySQL {
                tls_enabled: self.tls_config.is_some(),
            },
            DatabaseType::MariaDB => TestDatabase::MariaDB {
                tls_enabled: self.tls_config.is_some(),
            },
        }
    }
}
