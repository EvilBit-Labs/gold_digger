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
// Note: Specific re-exports to avoid unused import warnings

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
