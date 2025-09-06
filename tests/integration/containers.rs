//! Container management utilities for Gold Digger integration tests
//!
//! This module provides container management, health checks, and database setup
//! for MySQL and MariaDB containers using testcontainers.

#![allow(dead_code)]

use anyhow::{Context, Result};
use std::path::Path;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use testcontainers_modules::{
    mariadb::Mariadb,
    mysql::Mysql,
    testcontainers::{Container, ImageExt, runners::SyncRunner},
};

use super::{TestDatabase, is_ci_environment};

/// Database version information for compatibility handling
#[derive(Debug, Clone)]
pub struct DatabaseInfo {
    /// Database type (MySQL or MariaDB)
    pub db_type: String,
    /// Parsed version number
    pub version: DatabaseVersion,
    /// Raw version string from database
    pub version_string: String,
    /// Supported features for this database version
    pub features: DatabaseFeatures,
}

/// Parsed database version for comparison
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DatabaseVersion {
    /// Major version number
    pub major: u32,
    /// Minor version number
    pub minor: u32,
    /// Patch version number
    pub patch: u32,
}

impl DatabaseVersion {
    /// Create a new database version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
}

impl std::fmt::Display for DatabaseVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Database feature support flags
#[derive(Debug, Clone)]
pub struct DatabaseFeatures {
    /// JSON column type support
    pub supports_json: bool,
    /// Window functions support
    pub supports_window_functions: bool,
    /// Common Table Expressions (CTE) support
    pub supports_cte: bool,
    /// Generated columns support
    pub supports_generated_columns: bool,
    /// Full-text search support
    pub supports_fulltext: bool,
    /// Spatial data types support
    pub supports_spatial: bool,
}

/// Container-specific error types for better error handling
#[derive(Debug, thiserror::Error)]
pub enum ContainerError {
    #[error("Docker is not available: {0}")]
    DockerUnavailable(String),
    #[error("Platform not supported: {0}")]
    PlatformUnsupported(String),
    #[error("Container startup timeout after {timeout}s")]
    StartupTimeout { timeout: u64 },
    #[error("TLS configuration error: {0}")]
    TlsConfiguration(String),
    #[error("Database connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),
}

/// Retry configuration for container operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Number of connection retries per attempt
    pub connection_retries: usize,
    /// Delay between retries in milliseconds
    pub retry_delay_ms: u64,
    /// Maximum consecutive failures before reset
    pub max_consecutive_failures: usize,
    /// Log interval for progress updates
    pub log_interval: usize,
    /// Base backoff time in milliseconds
    pub base_backoff_ms: u64,
    /// Maximum backoff time in milliseconds
    pub max_backoff_ms: u64,
}

impl RetryConfig {
    /// Create retry configuration for CI environments
    pub fn ci() -> Self {
        Self {
            connection_retries: 5,
            retry_delay_ms: 500,
            max_consecutive_failures: 20,
            log_interval: 10,
            base_backoff_ms: 1000,
            max_backoff_ms: 10000,
        }
    }

    /// Create retry configuration for local development
    pub fn local() -> Self {
        Self {
            connection_retries: 3,
            retry_delay_ms: 200,
            max_consecutive_failures: 10,
            log_interval: 5,
            base_backoff_ms: 500,
            max_backoff_ms: 5000,
        }
    }

    /// Calculate adaptive backoff based on consecutive failures
    pub fn calculate_backoff(&self, consecutive_failures: usize) -> u64 {
        let exponential_backoff = self.base_backoff_ms * 2_u64.pow(consecutive_failures.min(10) as u32);
        exponential_backoff.min(self.max_backoff_ms)
    }
}

// Certificate generation will be handled inline for now
// TODO: Import certificate generation utilities when module structure is fixed

/// Test database connection with a simple query and detailed error reporting
fn test_database_connection(connection_url: &str) -> bool {
    test_database_connection_detailed(connection_url).unwrap_or(false)
}

/// Test database connection with detailed error information for debugging
fn test_database_connection_detailed(connection_url: &str) -> Result<bool> {
    use mysql::prelude::*;

    let opts = mysql::Opts::from_url(connection_url).context("Failed to parse connection URL")?;

    let pool = mysql::Pool::new(opts).context("Failed to create connection pool")?;

    let mut conn = pool.get_conn().context("Failed to get database connection")?;

    // Use a more comprehensive health check query
    let result: Option<i32> = conn
        .query_first("SELECT 1 AS health_check")
        .context("Failed to execute health check query")?;

    match result {
        Some(1) => Ok(true),
        Some(other) => {
            eprintln!("Unexpected health check result: {}", other);
            Ok(false)
        },
        None => {
            eprintln!("Health check query returned no results");
            Ok(false)
        },
    }
}

/// Configuration for TLS-enabled database containers
#[derive(Debug, Clone)]
pub struct ContainerTlsConfig {
    /// Whether TLS is enabled
    pub enabled: bool,
    /// Path to CA certificate file
    pub ca_cert_path: Option<std::path::PathBuf>,
    /// Whether to require secure transport
    pub require_secure_transport: bool,
    /// Minimum TLS version (TLSv1.2 or TLSv1.3)
    pub min_tls_version: String,
    /// Allowed cipher suites
    pub cipher_suites: Vec<String>,
    /// Whether to generate ephemeral certificates per test run
    pub use_ephemeral_certs: bool,
    /// Custom server certificate path (if not using ephemeral certificates)
    pub server_cert_path: Option<std::path::PathBuf>,
    /// Custom server key path (if not using ephemeral certificates)
    pub server_key_path: Option<std::path::PathBuf>,
}

impl Default for ContainerTlsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ca_cert_path: None,
            require_secure_transport: false,
            min_tls_version: "TLSv1.2".to_string(),
            cipher_suites: vec![
                "ECDHE-RSA-AES256-GCM-SHA384".to_string(),
                "ECDHE-RSA-AES128-GCM-SHA256".to_string(),
            ],
            use_ephemeral_certs: false,
            server_cert_path: None,
            server_key_path: None,
        }
    }
}

impl ContainerTlsConfig {
    /// Create a new TLS configuration with secure defaults
    pub fn new_secure() -> Self {
        Self {
            enabled: true,
            ca_cert_path: None,
            require_secure_transport: true,
            min_tls_version: "TLSv1.2".to_string(),
            cipher_suites: vec![
                "ECDHE-RSA-AES256-GCM-SHA384".to_string(),
                "ECDHE-RSA-AES128-GCM-SHA256".to_string(),
            ],
            use_ephemeral_certs: true,
            server_cert_path: None,
            server_key_path: None,
        }
    }

    /// Create a new TLS configuration with custom CA certificate path
    pub fn new_with_ca_cert<P: AsRef<std::path::Path>>(ca_cert_path: P) -> Self {
        Self {
            enabled: true,
            ca_cert_path: Some(ca_cert_path.as_ref().to_path_buf()),
            require_secure_transport: true,
            min_tls_version: "TLSv1.2".to_string(),
            cipher_suites: vec![
                "ECDHE-RSA-AES256-GCM-SHA384".to_string(),
                "ECDHE-RSA-AES128-GCM-SHA256".to_string(),
            ],
            use_ephemeral_certs: false,
            server_cert_path: None,
            server_key_path: None,
        }
    }

    /// Create a TLS configuration with minimum TLS version enforcement
    pub fn with_min_tls_version(mut self, version: &str) -> Result<Self> {
        match version {
            "TLSv1.2" | "TLSv1.3" => {
                self.min_tls_version = version.to_string();
                Ok(self)
            },
            _ => Err(anyhow::anyhow!("Invalid TLS version: {}. Must be TLSv1.2 or TLSv1.3", version)),
        }
    }

    /// Create a TLS configuration with strict cipher suite policy
    pub fn with_strict_ciphers(mut self) -> Self {
        self.cipher_suites = vec![
            "ECDHE-RSA-AES256-GCM-SHA384".to_string(),
            "ECDHE-RSA-AES128-GCM-SHA256".to_string(),
            "ECDHE-ECDSA-AES256-GCM-SHA384".to_string(),
            "ECDHE-ECDSA-AES128-GCM-SHA256".to_string(),
        ];
        self
    }

    /// Disable older TLS versions and weak ciphers
    pub fn with_security_hardening(mut self) -> Result<Self> {
        self.min_tls_version = "TLSv1.3".to_string();
        self.cipher_suites = vec![
            "TLS_AES_256_GCM_SHA384".to_string(),
            "TLS_AES_128_GCM_SHA256".to_string(),
            "TLS_CHACHA20_POLY1305_SHA256".to_string(),
        ];
        Ok(self)
    }

    /// Set the CA certificate path with validation
    pub fn with_ca_cert<P: AsRef<std::path::Path>>(mut self, path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Err(anyhow::anyhow!("CA certificate file does not exist: {}", path_ref.display()));
        }
        self.ca_cert_path = Some(path_ref.to_path_buf());
        Ok(self)
    }

    /// Validate TLS configuration
    pub fn validate(&self) -> Result<()> {
        if self.enabled {
            // Validate TLS version
            match self.min_tls_version.as_str() {
                "TLSv1.2" | "TLSv1.3" => {},
                _ => {
                    return Err(anyhow::anyhow!(
                        "Invalid TLS version: {}. Must be TLSv1.2 or TLSv1.3",
                        self.min_tls_version
                    ));
                },
            }

            // Validate cipher suites are not empty for secure configurations
            if self.cipher_suites.is_empty() && self.require_secure_transport {
                return Err(anyhow::anyhow!("Cipher suites cannot be empty when secure transport is required"));
            }

            // Validate CA certificate path if provided
            if let Some(ca_path) = &self.ca_cert_path
                && !ca_path.exists()
            {
                return Err(anyhow::anyhow!("CA certificate file does not exist: {}", ca_path.display()));
            }
        }
        Ok(())
    }
}

/// Database container manager for integration tests
pub struct DatabaseContainer {
    /// The type of database (MySQL or MariaDB)
    db_type: TestDatabase,
    /// The running container instance
    container: Box<dyn ContainerInstance>,
    /// Connection URL for the database
    connection_url: String,
    /// Temporary directory for test files
    temp_dir: TempDir,
    /// TLS configuration for this container
    tls_config: ContainerTlsConfig,
}

/// Trait for abstracting container operations across MySQL and MariaDB
trait ContainerInstance {
    /// Get the connection URL for this container
    fn connection_url(&self) -> &str;

    /// Get the container ID for debugging
    fn container_id(&self) -> String;

    /// Check if the container is healthy
    fn is_healthy(&self) -> bool;
}

/// MySQL container wrapper
struct MySqlContainer {
    container: Container<Mysql>,
    connection_url: String,
}

impl ContainerInstance for MySqlContainer {
    fn connection_url(&self) -> &str {
        &self.connection_url
    }

    fn container_id(&self) -> String {
        format!("mysql-{}", self.container.id())
    }

    fn is_healthy(&self) -> bool {
        test_database_connection(&self.connection_url)
    }
}

/// MariaDB container wrapper
struct MariaDbContainer {
    container: Container<Mariadb>,
    connection_url: String,
}

impl ContainerInstance for MariaDbContainer {
    fn connection_url(&self) -> &str {
        &self.connection_url
    }

    fn container_id(&self) -> String {
        format!("mariadb-{}", self.container.id())
    }

    fn is_healthy(&self) -> bool {
        test_database_connection(&self.connection_url)
    }
}

/// TLS connection validation result
#[derive(Debug, Clone)]
pub struct TlsValidationResult {
    /// Whether TLS connection was successful
    pub tls_connection_success: bool,
    /// TLS error message if connection failed
    pub tls_error: Option<String>,
}

/// Plain connection validation result
#[derive(Debug, Clone)]
pub struct PlainValidationResult {
    /// Whether plain connection was successful
    pub plain_connection_success: bool,
    /// Plain connection error message if connection failed
    pub plain_error: Option<String>,
}

impl Drop for DatabaseContainer {
    fn drop(&mut self) {
        // Ensure container cleanup on drop to prevent resource leaks
        if let Err(e) = self.cleanup() {
            eprintln!("Warning: Failed to cleanup container on drop: {}", e);
        }
    }
}

impl DatabaseContainer {
    /// Create a new database container of the specified type
    pub fn new(db_type: TestDatabase) -> Result<Self> {
        // Initialize crypto provider for rustls
        gold_digger::init_crypto_provider();

        let temp_dir = tempfile::tempdir().context("Failed to create temporary directory")?;

        let tls_config = if db_type.is_tls_enabled() {
            ContainerTlsConfig::new_secure()
        } else {
            ContainerTlsConfig::default()
        };

        let (container, connection_url) = match &db_type {
            TestDatabase::MySQL { tls_enabled } => {
                let mysql_container = Self::create_mysql_container(*tls_enabled, &tls_config)?;
                let url = mysql_container.connection_url().to_string();
                (Box::new(mysql_container) as Box<dyn ContainerInstance>, url)
            },
            TestDatabase::MariaDB { tls_enabled } => {
                let mariadb_container = Self::create_mariadb_container(*tls_enabled, &tls_config)?;
                let url = mariadb_container.connection_url().to_string();
                (Box::new(mariadb_container) as Box<dyn ContainerInstance>, url)
            },
        };

        let db_container = Self {
            db_type,
            container,
            connection_url,
            temp_dir,
            tls_config,
        };

        // Wait for container to be ready
        db_container.wait_for_readiness()?;

        Ok(db_container)
    }

    /// Create a new TLS-enabled database container
    pub fn new_tls(db_type: super::TestDatabaseTls) -> Result<Self> {
        // Initialize crypto provider for rustls
        gold_digger::init_crypto_provider();

        let temp_dir = tempfile::tempdir().context("Failed to create temporary directory")?;

        // Convert TLS container config to internal format
        let tls_config = Self::convert_tls_config(db_type.tls_config())?;

        // Validate TLS configuration
        tls_config.validate().context("Invalid TLS configuration")?;

        let base_db_type = db_type.to_test_database();
        let (container, connection_url) = match &base_db_type {
            TestDatabase::MySQL { .. } => {
                let mysql_container = Self::create_mysql_container_with_tls(&tls_config)?;
                let url = mysql_container.connection_url().to_string();
                (Box::new(mysql_container) as Box<dyn ContainerInstance>, url)
            },
            TestDatabase::MariaDB { .. } => {
                let mariadb_container = Self::create_mariadb_container_with_tls(&tls_config)?;
                let url = mariadb_container.connection_url().to_string();
                (Box::new(mariadb_container) as Box<dyn ContainerInstance>, url)
            },
        };

        let db_container = Self {
            db_type: base_db_type,
            container,
            connection_url,
            temp_dir,
            tls_config,
        };

        // Wait for container to be ready
        db_container.wait_for_readiness()?;

        Ok(db_container)
    }

    /// Create a new non-TLS database container
    pub fn new_plain(db_type: super::TestDatabasePlain) -> Result<Self> {
        // Initialize crypto provider for rustls
        gold_digger::init_crypto_provider();

        let temp_dir = tempfile::tempdir().context("Failed to create temporary directory")?;

        let tls_config = ContainerTlsConfig::default();
        let base_db_type = db_type.to_test_database();

        let (container, connection_url) = match &base_db_type {
            TestDatabase::MySQL { .. } => {
                let mysql_container = Self::create_mysql_container_plain()?;
                let url = mysql_container.connection_url().to_string();
                (Box::new(mysql_container) as Box<dyn ContainerInstance>, url)
            },
            TestDatabase::MariaDB { .. } => {
                let mariadb_container = Self::create_mariadb_container_plain()?;
                let url = mariadb_container.connection_url().to_string();
                (Box::new(mariadb_container) as Box<dyn ContainerInstance>, url)
            },
        };

        let db_container = Self {
            db_type: base_db_type,
            container,
            connection_url,
            temp_dir,
            tls_config,
        };

        // Wait for container to be ready
        db_container.wait_for_readiness()?;

        Ok(db_container)
    }

    /// Create a new database container with custom TLS configuration
    pub fn new_with_tls(db_type: TestDatabase, tls_config: ContainerTlsConfig) -> Result<Self> {
        // Initialize crypto provider for rustls
        gold_digger::init_crypto_provider();

        // Validate TLS configuration
        tls_config.validate().context("Invalid TLS configuration")?;

        let temp_dir = tempfile::tempdir().context("Failed to create temporary directory")?;

        let (container, connection_url) = match &db_type {
            TestDatabase::MySQL { .. } => {
                let mysql_container = Self::create_mysql_container(tls_config.enabled, &tls_config)?;
                let url = mysql_container.connection_url().to_string();
                (Box::new(mysql_container) as Box<dyn ContainerInstance>, url)
            },
            TestDatabase::MariaDB { .. } => {
                let mariadb_container = Self::create_mariadb_container(tls_config.enabled, &tls_config)?;
                let url = mariadb_container.connection_url().to_string();
                (Box::new(mariadb_container) as Box<dyn ContainerInstance>, url)
            },
        };

        let db_container = Self {
            db_type,
            container,
            connection_url,
            temp_dir,
            tls_config,
        };

        // Wait for container to be ready
        db_container.wait_for_readiness()?;

        Ok(db_container)
    }

    /// Create a MySQL container with optional TLS configuration
    fn create_mysql_container(tls_enabled: bool, _tls_config: &ContainerTlsConfig) -> Result<MySqlContainer> {
        // For now, use a simple MySQL container without TLS configuration
        // TODO: Add TLS configuration once the basic container setup is working
        let container = Mysql::default()
            .with_env_var("MYSQL_ALLOW_EMPTY_PASSWORD", "yes")
            .with_env_var("MYSQL_ROOT_HOST", "%")
            .start()
            .with_context(|| format!("Failed to start MySQL container with TLS={}", tls_enabled))?;

        let host_port = container
            .get_host_port_ipv4(3306)
            .context("Failed to get MySQL container port mapping")?;

        let connection_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);

        Ok(MySqlContainer {
            container,
            connection_url,
        })
    }

    /// Create a MariaDB container with optional TLS configuration
    fn create_mariadb_container(tls_enabled: bool, _tls_config: &ContainerTlsConfig) -> Result<MariaDbContainer> {
        // For now, use a simple MariaDB container without TLS configuration
        // TODO: Add TLS configuration once the basic container setup is working
        let container = Mariadb::default()
            .with_env_var("MARIADB_ALLOW_EMPTY_ROOT_PASSWORD", "yes")
            .with_env_var("MARIADB_ROOT_HOST", "%")
            .start()
            .with_context(|| format!("Failed to start MariaDB container with TLS={}", tls_enabled))?;

        let host_port = container
            .get_host_port_ipv4(3306)
            .context("Failed to get MariaDB container port mapping")?;

        let connection_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);

        Ok(MariaDbContainer {
            container,
            connection_url,
        })
    }

    /// Create a MySQL container with TLS configuration and SSL certificate mounting
    fn create_mysql_container_with_tls(tls_config: &ContainerTlsConfig) -> Result<MySqlContainer> {
        // Generate ephemeral certificates if configured
        let (_ca_cert, _server_cert, _server_key) = if tls_config.use_ephemeral_certs {
            // For now, use placeholder certificates
            // TODO: Integrate with EphemeralCertificate when module structure is fixed
            (
                "ca_cert_placeholder".to_string(),
                "server_cert_placeholder".to_string(),
                "server_key_placeholder".to_string(),
            )
        } else {
            // Use provided certificate paths
            let ca_cert = if let Some(ca_path) = &tls_config.ca_cert_path {
                std::fs::read_to_string(ca_path)
                    .with_context(|| format!("Failed to read CA certificate from {}", ca_path.display()))?
            } else {
                return Err(anyhow::anyhow!("CA certificate path required for non-ephemeral TLS configuration"));
            };

            let server_cert = if let Some(cert_path) = &tls_config.server_cert_path {
                std::fs::read_to_string(cert_path)
                    .with_context(|| format!("Failed to read server certificate from {}", cert_path.display()))?
            } else {
                return Err(anyhow::anyhow!("Server certificate path required for non-ephemeral TLS configuration"));
            };

            let server_key = if let Some(key_path) = &tls_config.server_key_path {
                std::fs::read_to_string(key_path)
                    .with_context(|| format!("Failed to read server key from {}", key_path.display()))?
            } else {
                return Err(anyhow::anyhow!("Server key path required for non-ephemeral TLS configuration"));
            };

            (ca_cert, server_cert, server_key)
        };

        // For now, create a basic MySQL container
        // TODO: Mount certificates and configure TLS settings
        let container = Mysql::default()
            .with_env_var("MYSQL_ALLOW_EMPTY_PASSWORD", "yes")
            .with_env_var("MYSQL_ROOT_HOST", "%")
            .start()
            .context("Failed to start MySQL TLS container")?;

        let host_port = container
            .get_host_port_ipv4(3306)
            .context("Failed to get MySQL TLS container port mapping")?;

        // Generate TLS-enabled connection URL
        let connection_url = format!("mysql://root@127.0.0.1:{}/mysql?ssl-mode=REQUIRED", host_port);

        Ok(MySqlContainer {
            container,
            connection_url,
        })
    }

    /// Create a MariaDB container with TLS configuration and SSL certificate mounting
    fn create_mariadb_container_with_tls(tls_config: &ContainerTlsConfig) -> Result<MariaDbContainer> {
        // Generate ephemeral certificates if configured
        let (_ca_cert, _server_cert, _server_key) = if tls_config.use_ephemeral_certs {
            // For now, use placeholder certificates
            // TODO: Integrate with EphemeralCertificate when module structure is fixed
            (
                "ca_cert_placeholder".to_string(),
                "server_cert_placeholder".to_string(),
                "server_key_placeholder".to_string(),
            )
        } else {
            // Use provided certificate paths
            let ca_cert = if let Some(ca_path) = &tls_config.ca_cert_path {
                std::fs::read_to_string(ca_path)
                    .with_context(|| format!("Failed to read CA certificate from {}", ca_path.display()))?
            } else {
                return Err(anyhow::anyhow!("CA certificate path required for non-ephemeral TLS configuration"));
            };

            let server_cert = if let Some(cert_path) = &tls_config.server_cert_path {
                std::fs::read_to_string(cert_path)
                    .with_context(|| format!("Failed to read server certificate from {}", cert_path.display()))?
            } else {
                return Err(anyhow::anyhow!("Server certificate path required for non-ephemeral TLS configuration"));
            };

            let server_key = if let Some(key_path) = &tls_config.server_key_path {
                std::fs::read_to_string(key_path)
                    .with_context(|| format!("Failed to read server key from {}", key_path.display()))?
            } else {
                return Err(anyhow::anyhow!("Server key path required for non-ephemeral TLS configuration"));
            };

            (ca_cert, server_cert, server_key)
        };

        // For now, create a basic MariaDB container
        // TODO: Mount certificates and configure TLS settings
        let container = Mariadb::default()
            .with_env_var("MARIADB_ALLOW_EMPTY_ROOT_PASSWORD", "yes")
            .with_env_var("MARIADB_ROOT_HOST", "%")
            .start()
            .context("Failed to start MariaDB TLS container")?;

        let host_port = container
            .get_host_port_ipv4(3306)
            .context("Failed to get MariaDB TLS container port mapping")?;

        // Generate TLS-enabled connection URL
        let connection_url = format!("mysql://root@127.0.0.1:{}/mysql?ssl-mode=REQUIRED", host_port);

        Ok(MariaDbContainer {
            container,
            connection_url,
        })
    }

    /// Create a MySQL container for standard unencrypted connection testing
    fn create_mysql_container_plain() -> Result<MySqlContainer> {
        let container = Mysql::default()
            .with_env_var("MYSQL_ALLOW_EMPTY_PASSWORD", "yes")
            .with_env_var("MYSQL_ROOT_HOST", "%")
            .start()
            .context("Failed to start MySQL plain container")?;

        let host_port = container
            .get_host_port_ipv4(3306)
            .context("Failed to get MySQL plain container port mapping")?;

        // Generate standard connection URL without TLS
        let connection_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);

        Ok(MySqlContainer {
            container,
            connection_url,
        })
    }

    /// Create a MariaDB container for standard unencrypted connection testing
    fn create_mariadb_container_plain() -> Result<MariaDbContainer> {
        let container = Mariadb::default()
            .with_env_var("MARIADB_ALLOW_EMPTY_ROOT_PASSWORD", "yes")
            .with_env_var("MARIADB_ROOT_HOST", "%")
            .start()
            .context("Failed to start MariaDB plain container")?;

        let host_port = container
            .get_host_port_ipv4(3306)
            .context("Failed to get MariaDB plain container port mapping")?;

        // Generate standard connection URL without TLS
        let connection_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);

        Ok(MariaDbContainer {
            container,
            connection_url,
        })
    }

    /// Convert TLS container config from public API to internal format
    fn convert_tls_config(config: &super::TlsContainerConfig) -> Result<ContainerTlsConfig> {
        Ok(ContainerTlsConfig {
            enabled: true,
            ca_cert_path: config.ca_cert_path.clone(),
            require_secure_transport: config.require_secure_transport,
            min_tls_version: config.min_tls_version.clone(),
            cipher_suites: config.cipher_suites.clone(),
            use_ephemeral_certs: config.use_ephemeral_certs,
            server_cert_path: config.server_cert_path.clone(),
            server_key_path: config.server_key_path.clone(),
        })
    }

    /// Wait for the database container to be ready for connections with enhanced retry logic
    fn wait_for_readiness(&self) -> Result<()> {
        let (timeout, retry_config) = if is_ci_environment() {
            // CI environment: longer timeout, more aggressive retries
            (Duration::from_secs(300), RetryConfig::ci())
        } else {
            // Local environment: Both MySQL and MariaDB need more time on some systems
            let base_timeout = Duration::from_secs(180); // 3 minutes for all containers locally
            (base_timeout, RetryConfig::local())
        };

        let start_time = Instant::now();
        let mut attempt = 0;
        let mut consecutive_failures = 0;

        eprintln!(
            "Waiting for {} container to become ready (timeout: {}s, TLS: {})...",
            self.db_type.name(),
            timeout.as_secs(),
            self.db_type.is_tls_enabled()
        );

        while start_time.elapsed() < timeout {
            attempt += 1;

            match self.test_connection_with_retry(&retry_config) {
                Ok(true) => {
                    eprintln!(
                        "Container ready after {} attempts in {:.2}s (consecutive failures: {})",
                        attempt,
                        start_time.elapsed().as_secs_f64(),
                        consecutive_failures
                    );
                    return Ok(());
                },
                Ok(false) => {
                    consecutive_failures += 1;
                },
                Err(e) => {
                    consecutive_failures += 1;
                    if attempt % 20 == 0 {
                        eprintln!("Connection error (attempt {}): {}", attempt, e);
                    }
                },
            }

            // Adaptive backoff based on consecutive failures
            let backoff_ms = retry_config.calculate_backoff(consecutive_failures);

            if attempt % retry_config.log_interval == 0 {
                eprintln!(
                    "Still waiting for container (attempt {}, elapsed: {:.1}s, consecutive failures: {})...",
                    attempt,
                    start_time.elapsed().as_secs_f64(),
                    consecutive_failures
                );
            }

            std::thread::sleep(Duration::from_millis(backoff_ms));

            // Reset consecutive failures if we've been trying for a while
            if consecutive_failures > retry_config.max_consecutive_failures {
                eprintln!("Too many consecutive failures, resetting counter and increasing backoff");
                consecutive_failures = 0;
            }
        }

        Err(anyhow::anyhow!(
            "Database container {} failed to become ready within {} seconds after {} attempts (consecutive failures: {})",
            self.container.container_id(),
            timeout.as_secs(),
            attempt,
            consecutive_failures
        ))
    }

    /// Test database connection with enhanced retry logic
    fn test_connection_with_retry(&self, retry_config: &RetryConfig) -> Result<bool> {
        for retry_attempt in 0..retry_config.connection_retries {
            match self.test_connection_detailed() {
                Ok(true) => return Ok(true),
                Ok(false) => {
                    if retry_attempt < retry_config.connection_retries - 1 {
                        std::thread::sleep(Duration::from_millis(retry_config.retry_delay_ms));
                    }
                },
                Err(e) => {
                    if retry_attempt == retry_config.connection_retries - 1 {
                        return Err(e);
                    }
                    std::thread::sleep(Duration::from_millis(retry_config.retry_delay_ms));
                },
            }
        }
        Ok(false)
    }

    /// Test database connection with detailed error reporting
    fn test_connection_detailed(&self) -> Result<bool> {
        use mysql::prelude::*;

        let opts =
            mysql::Opts::from_url(&self.connection_url).context("Failed to parse database URL for connection test")?;

        let pool = mysql::Pool::new(opts).context("Failed to create connection pool for connection test")?;

        match pool.get_conn() {
            Ok(mut conn) => {
                // Test basic connectivity with a simple query that returns a known value
                match conn.query_first::<i32, _>("SELECT 1") {
                    Ok(Some(1)) => Ok(true),
                    Ok(Some(val)) => {
                        eprintln!("Unexpected value from connection test: {}", val);
                        Ok(false)
                    },
                    Ok(None) => {
                        eprintln!("Connection test returned no results");
                        Ok(false)
                    },
                    Err(e) => {
                        // Log specific SQL errors for debugging
                        eprintln!("SQL query failed during connection test: {}", e);
                        Ok(false)
                    },
                }
            },
            Err(e) => {
                // Distinguish between different connection error types
                let error_msg = e.to_string();
                if error_msg.contains("Connection refused") {
                    // Container not ready yet
                    Ok(false)
                } else if error_msg.contains("Access denied") {
                    // Authentication issue - this is a configuration problem
                    Err(anyhow::anyhow!("Authentication failed: {}", e))
                } else if error_msg.contains("SSL") || error_msg.contains("TLS") {
                    // TLS-related issue
                    Err(anyhow::anyhow!("TLS connection failed: {}", e))
                } else {
                    // Other connection errors
                    eprintln!("Connection error during test: {}", e);
                    Ok(false)
                }
            },
        }
    }

    /// Test database connection
    pub fn test_connection(&self) -> bool {
        test_database_connection(&self.connection_url)
    }

    /// Get the database type
    pub fn db_type(&self) -> &TestDatabase {
        &self.db_type
    }

    /// Get the connection URL
    pub fn connection_url(&self) -> &str {
        &self.connection_url
    }

    /// Generate a TLS-enabled connection URL with SSL parameters
    pub fn tls_connection_url(&self) -> Result<String> {
        if !self.db_type.is_tls_enabled() {
            return Err(anyhow::anyhow!("Cannot generate TLS connection URL for non-TLS container"));
        }

        // Parse the base connection URL
        let base_url = &self.connection_url;

        // Add TLS parameters to the connection URL
        let tls_url = if base_url.contains('?') {
            format!("{}&ssl-mode=REQUIRED&ssl-verify-server-cert=true", base_url)
        } else {
            format!("{}?ssl-mode=REQUIRED&ssl-verify-server-cert=true", base_url)
        };

        Ok(tls_url)
    }

    /// Generate a non-TLS connection URL explicitly disabling SSL
    pub fn plain_connection_url(&self) -> Result<String> {
        // Parse the base connection URL and ensure SSL is disabled
        let base_url = &self.connection_url;

        // Add SSL disabled parameters to the connection URL
        let plain_url = if base_url.contains('?') {
            format!("{}&ssl-mode=DISABLED", base_url)
        } else {
            format!("{}?ssl-mode=DISABLED", base_url)
        };

        Ok(plain_url)
    }

    /// Generate connection URL with custom SSL mode
    pub fn connection_url_with_ssl_mode(&self, ssl_mode: &str) -> Result<String> {
        // For now, just return the base connection URL since the MySQL crate
        // doesn't support ssl-mode URL parameters in the way we're trying to use them.
        // The SSL configuration should be handled through mysql::SslOpts instead.

        // Validate SSL mode for future use
        match ssl_mode {
            "DISABLED" | "PREFERRED" | "REQUIRED" | "VERIFY_CA" | "VERIFY_IDENTITY" => {},
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid SSL mode: {}. Must be one of: DISABLED, PREFERRED, REQUIRED, VERIFY_CA, VERIFY_IDENTITY",
                    ssl_mode
                ));
            },
        }

        // For now, return the base URL without SSL mode parameters
        // TODO: Implement proper SSL configuration through mysql::SslOpts
        Ok(self.connection_url.clone())
    }

    /// Get the temporary directory path
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Seed the database with comprehensive test schema and data
    ///
    /// This method implements idempotent database seeding with separate DDL and DML phases:
    /// 1. DDL (Data Definition Language) - Schema creation executed outside transactions
    /// 2. DML (Data Manipulation Language) - Data seeding executed inside transactions
    /// 3. Database compatibility detection and handling for MySQL vs MariaDB differences
    pub fn seed_data(&self) -> Result<()> {
        let opts = mysql::Opts::from_url(&self.connection_url).context("Failed to parse database URL")?;
        let pool = mysql::Pool::new(opts).context("Failed to create database connection pool")?;

        let mut conn = pool.get_conn().context("Failed to get database connection")?;

        // Detect database version and type for compatibility handling
        let db_info = self.detect_database_version(&mut conn)?;

        eprintln!("Seeding database: {} version {}", db_info.db_type, db_info.version);

        // Phase 1: DDL execution (outside transactions - auto-committed by MySQL/MariaDB)
        self.execute_schema_ddl(&mut conn, &db_info)?;

        // Phase 2: DML execution (inside explicit transactions for atomicity)
        self.execute_data_seeding(&mut conn, &db_info)?;

        eprintln!("Database seeding completed successfully");
        Ok(())
    }

    /// Detect database version and type for compatibility handling
    fn detect_database_version(&self, conn: &mut mysql::PooledConn) -> Result<DatabaseInfo> {
        use mysql::prelude::*;

        // Get version information
        let version_result: Option<String> = conn
            .query_first("SELECT VERSION()")
            .context("Failed to query database version")?;

        let version_string = version_result.unwrap_or_else(|| "unknown".to_string());

        // Determine database type and version
        let (db_type, version, features) = if version_string.to_lowercase().contains("mariadb") {
            let version = Self::extract_version_number(&version_string);
            let features = Self::detect_mariadb_features(&version);
            ("MariaDB".to_string(), version, features)
        } else {
            let version = Self::extract_version_number(&version_string);
            let features = Self::detect_mysql_features(&version);
            ("MySQL".to_string(), version, features)
        };

        Ok(DatabaseInfo {
            db_type,
            version,
            version_string,
            features,
        })
    }

    /// Execute DDL statements for schema creation (outside transactions)
    fn execute_schema_ddl(&self, conn: &mut mysql::PooledConn, db_info: &DatabaseInfo) -> Result<()> {
        use mysql::prelude::*;
        use std::fs;

        // Load schema.sql file
        let schema_path = std::path::Path::new("tests/fixtures/schema.sql");
        if !schema_path.exists() {
            return Err(anyhow::anyhow!("Schema file not found: {}", schema_path.display()));
        }

        let schema_sql = fs::read_to_string(schema_path)
            .with_context(|| format!("Failed to read schema file: {}", schema_path.display()))?;

        eprintln!("Executing DDL statements for schema creation...");

        // Split SQL into individual statements and execute each one
        let statements = Self::split_sql_statements(&schema_sql);
        let mut ddl_count = 0;

        for (i, statement) in statements.iter().enumerate() {
            let trimmed = statement.trim();

            // Skip empty statements and comments
            if trimmed.is_empty() || trimmed.starts_with("--") || trimmed.starts_with("/*") {
                continue;
            }

            // Apply database-specific compatibility adjustments
            let adjusted_statement = self.apply_compatibility_adjustments(trimmed, db_info)?;

            // Execute DDL statement (auto-committed, not wrapped in transaction)
            if let Err(e) = conn.exec_drop(&adjusted_statement, ()) {
                // Log the error but continue with other statements for idempotency
                eprintln!("Warning: DDL statement {} failed (may be expected for idempotency): {}", i + 1, e);
                eprintln!("Statement: {}", adjusted_statement.chars().take(100).collect::<String>());
            } else {
                ddl_count += 1;
            }
        }

        eprintln!("Executed {} DDL statements successfully", ddl_count);
        Ok(())
    }

    /// Execute DML statements for data seeding (inside explicit transactions)
    fn execute_data_seeding(&self, conn: &mut mysql::PooledConn, db_info: &DatabaseInfo) -> Result<()> {
        use mysql::prelude::*;
        use std::fs;

        // Load seed_data.sql file
        let seed_path = std::path::Path::new("tests/fixtures/seed_data.sql");
        if !seed_path.exists() {
            return Err(anyhow::anyhow!("Seed data file not found: {}", seed_path.display()));
        }

        let seed_sql = fs::read_to_string(seed_path)
            .with_context(|| format!("Failed to read seed data file: {}", seed_path.display()))?;

        eprintln!("Executing DML statements for data seeding...");

        // Begin explicit transaction for atomic data seeding
        // Use the connection's transaction method instead of SQL commands
        let mut tx = conn
            .start_transaction(mysql::TxOpts::default())
            .context("Failed to start transaction for data seeding")?;

        let statements = Self::split_sql_statements(&seed_sql);
        let mut dml_count = 0;
        let mut error_count = 0;

        for (i, statement) in statements.iter().enumerate() {
            let trimmed = statement.trim();

            // Skip empty statements and comments
            if trimmed.is_empty() || trimmed.starts_with("--") || trimmed.starts_with("/*") {
                continue;
            }

            // Apply database-specific compatibility adjustments
            let adjusted_statement = match self.apply_compatibility_adjustments(trimmed, db_info) {
                Ok(stmt) => stmt,
                Err(e) => {
                    eprintln!("Warning: Failed to adjust statement {}: {}", i + 1, e);
                    continue;
                },
            };

            // Execute DML statement inside transaction
            match tx.exec_drop(&adjusted_statement, ()) {
                Ok(_) => {
                    dml_count += 1;
                },
                Err(e) => {
                    error_count += 1;
                    eprintln!("Warning: DML statement {} failed: {}", i + 1, e);
                    eprintln!("Statement: {}", adjusted_statement.chars().take(100).collect::<String>());

                    // For critical errors, rollback and fail
                    if adjusted_statement.to_uppercase().contains("INSERT") && error_count > 10 {
                        tx.rollback().ok();
                        return Err(anyhow::anyhow!(
                            "Too many DML errors ({}), rolling back transaction",
                            error_count
                        ));
                    }
                },
            }
        }

        // Commit transaction if we have successful operations
        if dml_count > 0 {
            tx.commit().context("Failed to commit data seeding transaction")?;
            eprintln!("Committed {} DML statements successfully ({} errors)", dml_count, error_count);
        } else {
            tx.rollback().ok();
            return Err(anyhow::anyhow!("No DML statements executed successfully"));
        }

        Ok(())
    }

    /// Apply database-specific compatibility adjustments to SQL statements
    fn apply_compatibility_adjustments(&self, statement: &str, db_info: &DatabaseInfo) -> Result<String> {
        let mut adjusted = statement.to_string();

        // Handle MySQL vs MariaDB differences
        match db_info.db_type.as_str() {
            "MySQL" => {
                // MySQL-specific adjustments
                if db_info.version.major >= 8 {
                    // MySQL 8.0+ specific features
                    // No adjustments needed for now
                } else {
                    // MySQL 5.7 and earlier compatibility
                    // Replace JSON functions that might not be available
                    if adjusted.contains("JSON_OBJECT") && !db_info.features.supports_json {
                        // Fallback for older MySQL versions without JSON support
                        adjusted = adjusted.replace("JSON_OBJECT", "CONCAT");
                    }
                }
            },
            "MariaDB" => {
                // MariaDB-specific adjustments
                if db_info.version.major >= 10 && db_info.version.minor >= 2 {
                    // MariaDB 10.2+ has JSON support
                    // No adjustments needed
                } else {
                    // Older MariaDB versions - replace JSON with TEXT
                    if adjusted.contains("JSON") {
                        adjusted = adjusted.replace("JSON", "TEXT");
                    }
                }
            },
            _ => {
                // Unknown database type - use conservative approach
                eprintln!("Warning: Unknown database type {}, using conservative SQL", db_info.db_type);
            },
        }

        // Handle CREATE TABLE IF NOT EXISTS for idempotency
        if adjusted.to_uppercase().contains("CREATE TABLE") && !adjusted.to_uppercase().contains("IF NOT EXISTS") {
            adjusted = adjusted.replace("CREATE TABLE", "CREATE TABLE IF NOT EXISTS");
        }

        // Handle CREATE INDEX for idempotency (MySQL doesn't support IF NOT EXISTS for indexes)
        if adjusted.to_uppercase().contains("CREATE INDEX IF NOT EXISTS") {
            // MySQL doesn't support IF NOT EXISTS for CREATE INDEX, so we'll skip these
            // or convert them to a different approach
            let _index_name = if let Some(start) = adjusted.find("CREATE INDEX IF NOT EXISTS ") {
                let remaining = &adjusted[start + 28..];
                if let Some(space_pos) = remaining.find(' ') {
                    remaining[..space_pos].to_string()
                } else {
                    "unknown_index".to_string()
                }
            } else {
                "unknown_index".to_string()
            };

            // For now, just remove IF NOT EXISTS from CREATE INDEX statements
            adjusted = adjusted.replace("CREATE INDEX IF NOT EXISTS", "CREATE INDEX");
        }

        Ok(adjusted)
    }

    /// Split SQL content into individual statements with improved parsing
    fn split_sql_statements(sql_content: &str) -> Vec<String> {
        let mut statements = Vec::new();
        let lines: Vec<&str> = sql_content.lines().collect();
        let mut current_statement = String::new();
        let mut in_procedure = false;

        for line in lines {
            let trimmed_line = line.trim();

            // Skip DELIMITER statements
            if trimmed_line.to_uppercase().starts_with("DELIMITER") {
                continue;
            }

            // Skip stored procedure definitions (they're complex and not needed for basic seeding)
            if trimmed_line.to_uppercase().contains("CREATE PROCEDURE")
                || trimmed_line.to_uppercase().contains("CREATE FUNCTION")
            {
                in_procedure = true;
                continue;
            }

            if in_procedure {
                // Skip everything until we see END followed by delimiter
                if trimmed_line.to_uppercase().starts_with("END$$")
                    || trimmed_line.to_uppercase().starts_with("END$")
                    || (trimmed_line.to_uppercase() == "DELIMITER ;" && in_procedure)
                {
                    in_procedure = false;
                }
                continue;
            }

            // Skip empty lines and comments
            if trimmed_line.is_empty() || trimmed_line.starts_with("--") || trimmed_line.starts_with("/*") {
                continue;
            }

            // Add line to current statement
            if !current_statement.is_empty() {
                current_statement.push('\n');
            }
            current_statement.push_str(line);

            // Check for statement terminator
            if line.trim_end().ends_with(';') {
                let trimmed = current_statement.trim();
                if !trimmed.is_empty() {
                    statements.push(trimmed.to_string());
                }
                current_statement.clear();
            }
        }

        // Add the last statement if it doesn't end with semicolon
        let trimmed = current_statement.trim();
        if !trimmed.is_empty() {
            statements.push(trimmed.to_string());
        }

        statements
    }

    /// Execute a SQL statement on the database
    /// Execute a SQL statement on the database
    pub fn execute_sql(&self, sql: &str) -> Result<()> {
        use mysql::prelude::*;

        // Validate SQL is not empty or just whitespace
        if sql.trim().is_empty() {
            return Err(anyhow::anyhow!("SQL statement cannot be empty"));
        }

        let opts = mysql::Opts::from_url(&self.connection_url).context("Failed to parse database URL")?;
        let pool = mysql::Pool::new(opts).context("Failed to create database connection pool")?;

        let mut conn = pool.get_conn().context("Failed to get database connection")?;

        conn.exec_drop(sql, ())
            .with_context(|| format!("Failed to execute SQL: {}", sql.chars().take(100).collect::<String>()))?;

        Ok(())
    }

    /// Execute a SQL query and return results safely
    pub fn query_results(&self, sql: &str) -> Result<Vec<mysql::Row>> {
        use mysql::prelude::*;

        // Validate SQL is not empty or just whitespace
        if sql.trim().is_empty() {
            return Err(anyhow::anyhow!("SQL query cannot be empty"));
        }

        let opts = mysql::Opts::from_url(&self.connection_url).context("Failed to parse database URL")?;
        let pool = mysql::Pool::new(opts).context("Failed to create database connection pool")?;

        let mut conn = pool.get_conn().context("Failed to get database connection")?;

        let results: Vec<mysql::Row> = conn
            .query(sql)
            .with_context(|| format!("Failed to execute query: {}", sql.chars().take(100).collect::<String>()))?;

        Ok(results)
    }

    /// Get container health information
    pub fn health_info(&self) -> ContainerHealthInfo {
        ContainerHealthInfo {
            container_id: self.container.container_id(),
            db_type: self.db_type.clone(),
            is_healthy: self.container.is_healthy(),
            connection_url_redacted: self.redact_connection_url(),
        }
    }

    /// Clean up the database container
    pub fn cleanup(&self) -> Result<()> {
        // Note: testcontainers handles cleanup automatically when the container goes out of scope
        // This method is provided for explicit cleanup if needed
        Ok(())
    }

    /// Redact sensitive information from connection URL
    fn redact_connection_url(&self) -> String {
        // Replace password with *** in URLs like mysql://user:password@host:port/db
        let url = &self.connection_url;

        // Handle standard MySQL URL format: mysql://user:password@host:port/db
        if let Some(at_pos) = url.find('@') {
            if let Some(colon_pos) = url[..at_pos].rfind(':') {
                // Check if there's a protocol prefix
                if let Some(protocol_end) = url.find("://")
                    && colon_pos > protocol_end + 3
                {
                    let mut redacted = url.to_string();
                    redacted.replace_range(colon_pos + 1..at_pos, "***");
                    return redacted;
                }
            }
            // If no colon found, might be user-only format: mysql://user@host:port/db
            if let Some(protocol_end) = url.find("://") {
                let user_part = &url[protocol_end + 3..at_pos];
                if !user_part.is_empty() {
                    return format!("{}://***@{}", &url[..protocol_end], &url[at_pos + 1..]);
                }
            }
        }

        // Fallback: completely redact if parsing fails
        "***REDACTED***".to_string()
    }

    /// Validate that no sensitive information is logged
    fn validate_no_credentials_in_logs(&self, log_message: &str) -> bool {
        let sensitive_patterns = [
            "password",
            "passwd",
            "pwd",
            "secret",
            "token",
            "key",
            "auth",
            // Common database credential patterns
            "identified by",
            "grant",
            "create user",
        ];

        let lower_message = log_message.to_lowercase();
        !sensitive_patterns.iter().any(|pattern| lower_message.contains(pattern))
    }

    /// Extract version number from database version string
    fn extract_version_number(version_string: &str) -> DatabaseVersion {
        // Extract version numbers from strings like "8.0.35" or "10.6.16-MariaDB"
        let version_part = version_string.split_whitespace().next().unwrap_or("0.0.0");

        let parts: Vec<&str> = version_part.split('.').take(3).collect();

        let major = parts.first().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);

        let minor = parts.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);

        let patch = parts
            .get(2)
            .and_then(|s| {
                // Handle cases like "16-MariaDB" - extract just the number part
                s.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u32>()
                    .ok()
            })
            .unwrap_or(0);

        DatabaseVersion::new(major, minor, patch)
    }

    /// Detect MySQL-specific features based on version
    fn detect_mysql_features(version: &DatabaseVersion) -> DatabaseFeatures {
        DatabaseFeatures {
            supports_json: version >= &DatabaseVersion::new(5, 7, 8),
            supports_window_functions: version >= &DatabaseVersion::new(8, 0, 0),
            supports_cte: version >= &DatabaseVersion::new(8, 0, 1),
            supports_generated_columns: version >= &DatabaseVersion::new(5, 7, 6),
            supports_fulltext: version >= &DatabaseVersion::new(5, 6, 0),
            supports_spatial: version >= &DatabaseVersion::new(5, 7, 0),
        }
    }

    /// Detect MariaDB-specific features based on version
    fn detect_mariadb_features(version: &DatabaseVersion) -> DatabaseFeatures {
        DatabaseFeatures {
            supports_json: version >= &DatabaseVersion::new(10, 2, 7),
            supports_window_functions: version >= &DatabaseVersion::new(10, 2, 0),
            supports_cte: version >= &DatabaseVersion::new(10, 2, 1),
            supports_generated_columns: version >= &DatabaseVersion::new(10, 2, 0),
            supports_fulltext: version >= &DatabaseVersion::new(10, 0, 0),
            supports_spatial: version >= &DatabaseVersion::new(10, 0, 0),
        }
    }
}

/// Container health information for debugging
#[derive(Debug, Clone)]
pub struct ContainerHealthInfo {
    /// Container ID
    pub container_id: String,
    /// Database type
    pub db_type: TestDatabase,
    /// Health status
    pub is_healthy: bool,
    /// Redacted connection URL
    pub connection_url_redacted: String,
}

/// CI resource limits for container management
#[derive(Debug, Clone)]
pub struct CiResourceLimits {
    /// Maximum number of containers to run simultaneously
    pub max_containers: usize,
    /// Maximum memory per container in bytes
    pub max_memory_per_container: u64,
    /// Maximum total memory usage in bytes
    pub max_total_memory: u64,
    /// Container startup timeout
    pub container_startup_timeout: Duration,
}

/// Container manager for handling multiple database types
pub struct ContainerManager {
    /// Available containers
    containers: Vec<DatabaseContainer>,
    /// Maximum number of containers to manage simultaneously
    max_containers: usize,
}

/// Docker environment information for CI compatibility
#[derive(Debug, Clone)]
pub struct DockerEnvironment {
    /// Docker daemon version
    pub docker_version: String,
    /// Available disk space in bytes
    pub available_disk_space: u64,
    /// Available memory in bytes
    pub available_memory: u64,
    /// Whether running in CI environment
    pub is_ci: bool,
    /// Platform information (Linux, macOS, Windows)
    pub platform: String,
}

/// Docker preflight check results
#[derive(Debug, Clone)]
pub struct DockerPreflightResult {
    /// Whether Docker is available
    pub docker_available: bool,
    /// Whether platform is supported (Linux only for containers)
    pub platform_supported: bool,
    /// Whether sufficient resources are available
    pub sufficient_resources: bool,
    /// Docker environment information
    pub environment: Option<DockerEnvironment>,
    /// Error messages for failed checks
    pub error_messages: Vec<String>,
    /// Actionable skip messages for users
    pub skip_messages: Vec<String>,
}

impl ContainerManager {
    /// Create a new container manager
    pub fn new() -> Self {
        Self {
            containers: Vec::new(),
            max_containers: if is_ci_environment() { 2 } else { 4 },
        }
    }

    /// Create a new container manager with custom limits
    pub fn with_limits(max_containers: usize) -> Self {
        Self {
            containers: Vec::new(),
            max_containers,
        }
    }

    /// Perform comprehensive Docker preflight checks
    pub fn docker_preflight_check() -> DockerPreflightResult {
        let mut result = DockerPreflightResult {
            docker_available: false,
            platform_supported: false,
            sufficient_resources: false,
            environment: None,
            error_messages: Vec::new(),
            skip_messages: Vec::new(),
        };

        // Check if Docker daemon is available
        match Self::check_docker_daemon() {
            Ok(docker_info) => {
                result.docker_available = true;
                result.environment = Some(docker_info);
            },
            Err(e) => {
                result
                    .error_messages
                    .push(format!("Docker daemon not available: {}", e));
                result.skip_messages.push(
                    "Install Docker and ensure the Docker daemon is running. \
                     On Linux: sudo systemctl start docker. \
                     On macOS/Windows: Start Docker Desktop."
                        .to_string(),
                );
                return result;
            },
        }

        // Check platform support (restrict to Linux for container tests)
        if Self::is_platform_supported() {
            result.platform_supported = true;
        } else {
            let platform = std::env::consts::OS;
            result
                .error_messages
                .push(format!("Platform {} not supported for container tests", platform));
            result.skip_messages.push(format!(
                "Container-based tests are supported on Linux and macOS only. \
                        Current platform: {}. Run tests on a Linux or macOS system.",
                platform
            ));
            return result;
        }

        // Check resource availability
        match Self::check_resource_availability() {
            Ok(sufficient) => {
                result.sufficient_resources = sufficient;
                if !sufficient {
                    result
                        .error_messages
                        .push("Insufficient system resources for containers".to_string());
                    result.skip_messages.push(
                        "Ensure at least 2GB RAM and 1GB disk space are available. \
                         Close other applications or use a system with more resources."
                            .to_string(),
                    );
                }
            },
            Err(e) => {
                result
                    .error_messages
                    .push(format!("Failed to check system resources: {}", e));
                result
                    .skip_messages
                    .push("Unable to verify system resources. Ensure sufficient RAM and disk space.".to_string());
            },
        }

        result
    }

    /// Check Docker daemon availability and get environment information
    fn check_docker_daemon() -> Result<DockerEnvironment> {
        // Ping Docker daemon with enhanced macOS support
        let ping_output = std::process::Command::new("docker")
            .args(["system", "info", "--format", "{{json .}}"])
            .output()
            .context("Failed to execute 'docker system info' command")?;

        if !ping_output.status.success() {
            let stderr = String::from_utf8_lossy(&ping_output.stderr);

            // Provide platform-specific error messages and troubleshooting
            let platform_hint = match std::env::consts::OS {
                "macos" => {
                    "On macOS, ensure Docker Desktop is installed and running:\n\
                     - Install Docker Desktop from https://docker.com/products/docker-desktop\n\
                     - Start Docker Desktop from Applications or run 'open -a Docker'\n\
                     - Wait for Docker Desktop to fully start (whale icon in menu bar)\n\
                     - Verify with 'docker version' in terminal"
                },
                "linux" => {
                    "On Linux, ensure Docker daemon is running:\n\
                     - Start daemon: 'sudo systemctl start docker' or 'sudo service docker start'\n\
                     - Enable on boot: 'sudo systemctl enable docker'\n\
                     - Add user to docker group: 'sudo usermod -aG docker $USER' (requires logout/login)\n\
                     - Verify with 'docker version'"
                },
                _ => "Ensure Docker is installed and the daemon is running.",
            };

            return Err(anyhow::anyhow!("Docker daemon not responding. {}\nError: {}", platform_hint, stderr));
        }

        // Get Docker version
        let version_output = std::process::Command::new("docker")
            .args(["version", "--format", "{{.Server.Version}}"])
            .output()
            .context("Failed to get Docker version")?;

        let docker_version = if version_output.status.success() {
            String::from_utf8_lossy(&version_output.stdout).trim().to_string()
        } else {
            "unknown".to_string()
        };

        // Parse system info for resource information with platform-specific handling
        let system_info = String::from_utf8_lossy(&ping_output.stdout);
        let available_memory = Self::parse_memory_from_docker_info(&system_info)?;
        let available_disk_space = Self::check_disk_space()?;

        Ok(DockerEnvironment {
            docker_version,
            available_disk_space,
            available_memory,
            is_ci: is_ci_environment(),
            platform: std::env::consts::OS.to_string(),
        })
    }

    /// Check if the current platform supports container tests
    fn is_platform_supported() -> bool {
        // Support both Linux and macOS for container-based tests
        // Windows is excluded due to Docker Desktop complexity in CI
        matches!(std::env::consts::OS, "linux" | "macos")
    }

    /// Check system resource availability
    fn check_resource_availability() -> Result<bool> {
        let min_memory_gb = 2; // Minimum 2GB RAM
        let min_disk_gb = 1; // Minimum 1GB disk space

        let available_memory = Self::get_available_memory()?;
        let available_disk = Self::check_disk_space()?;

        let memory_sufficient = available_memory >= (min_memory_gb * 1024 * 1024 * 1024);
        let disk_sufficient = available_disk >= (min_disk_gb * 1024 * 1024 * 1024);

        Ok(memory_sufficient && disk_sufficient)
    }

    /// Parse memory information from Docker system info
    fn parse_memory_from_docker_info(info: &str) -> Result<u64> {
        // Try to parse JSON output from docker system info
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(info)
            && let Some(memory) = json.get("MemTotal").and_then(|m| m.as_u64())
        {
            return Ok(memory);
        }

        // Fallback to system memory check
        Self::get_available_memory()
    }

    /// Get available system memory with cross-platform support
    fn get_available_memory() -> Result<u64> {
        match std::env::consts::OS {
            "linux" => {
                // Use /proc/meminfo on Linux
                if std::path::Path::new("/proc/meminfo").exists() {
                    let meminfo = std::fs::read_to_string("/proc/meminfo").context("Failed to read /proc/meminfo")?;

                    for line in meminfo.lines() {
                        if line.starts_with("MemAvailable:") {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 2 {
                                let kb = parts[1].parse::<u64>().context("Failed to parse memory value")?;
                                return Ok(kb * 1024); // Convert KB to bytes
                            }
                        }
                    }
                }
                // Fallback for Linux
                Ok(4 * 1024 * 1024 * 1024)
            },
            "macos" => {
                // Use sysctl on macOS to get memory information
                let output = std::process::Command::new("sysctl")
                    .args(["-n", "hw.memsize"])
                    .output()
                    .context("Failed to execute sysctl command on macOS")?;

                if output.status.success() {
                    let memsize_output = String::from_utf8_lossy(&output.stdout);
                    let memsize_str = memsize_output.trim();
                    let total_memory = memsize_str
                        .parse::<u64>()
                        .context("Failed to parse memory size from sysctl")?;

                    // Estimate available memory as 75% of total (conservative estimate)
                    let available_memory = (total_memory as f64 * 0.75) as u64;
                    return Ok(available_memory);
                }

                // Fallback: try vm_stat for more detailed memory info
                let vm_output = std::process::Command::new("vm_stat")
                    .output()
                    .context("Failed to execute vm_stat command on macOS")?;

                if vm_output.status.success() {
                    let vm_stat = String::from_utf8_lossy(&vm_output.stdout);

                    // Parse vm_stat output to get free and inactive memory
                    let mut free_pages = 0u64;
                    let mut inactive_pages = 0u64;
                    let mut page_size = 4096u64; // Default page size

                    for line in vm_stat.lines() {
                        if line.starts_with("Mach Virtual Memory Statistics:") {
                            // Extract page size if available
                            if let Some(size_start) = line.find("page size of ") {
                                let size_part = &line[size_start + 13..];
                                if let Some(size_end) = size_part.find(" bytes")
                                    && let Ok(size) = size_part[..size_end].parse::<u64>()
                                {
                                    page_size = size;
                                }
                            }
                        } else if line.starts_with("Pages free:") {
                            if let Some(num_str) = line.split_whitespace().nth(2)
                                && let Ok(pages) = num_str.trim_end_matches('.').parse::<u64>()
                            {
                                free_pages = pages;
                            }
                        } else if line.starts_with("Pages inactive:")
                            && let Some(num_str) = line.split_whitespace().nth(2)
                            && let Ok(pages) = num_str.trim_end_matches('.').parse::<u64>()
                        {
                            inactive_pages = pages;
                        }
                    }

                    if free_pages > 0 || inactive_pages > 0 {
                        let available_memory = (free_pages + inactive_pages) * page_size;
                        return Ok(available_memory);
                    }
                }

                // Fallback for macOS (assume 8GB available, typical for macOS systems)
                Ok(8 * 1024 * 1024 * 1024)
            },
            _ => {
                // Fallback for other platforms
                Ok(4 * 1024 * 1024 * 1024)
            },
        }
    }

    /// Check available disk space with cross-platform support using sysinfo crate
    pub fn check_disk_space() -> Result<u64> {
        use sysinfo::Disks;

        let disks = Disks::new_with_refreshed_list();

        // Look for /tmp mount point first
        for disk in &disks {
            if disk.mount_point() == std::path::Path::new("/tmp") {
                return Ok(disk.available_space());
            }
        }

        // Fallback to first available disk
        if let Some(disk) = disks.first() {
            return Ok(disk.available_space());
        }

        // Final fallback estimate if no disks found
        Ok(10 * 1024 * 1024 * 1024) // 10GB
    }

    /// Create and add a database container
    pub fn create_container(&mut self, db_type: TestDatabase) -> Result<&DatabaseContainer> {
        let container = DatabaseContainer::new(db_type)?;
        self.containers.push(container);
        Ok(self.containers.last().unwrap())
    }

    /// Get a container by database type
    pub fn get_container(&self, db_type: &TestDatabase) -> Option<&DatabaseContainer> {
        self.containers.iter().find(|c| c.db_type() == db_type)
    }

    /// Get all containers
    pub fn containers(&self) -> &[DatabaseContainer] {
        &self.containers
    }

    /// Clean up all containers and resources
    pub fn cleanup_all(&mut self) -> Result<()> {
        let mut cleanup_errors = Vec::new();

        for (index, container) in self.containers.iter().enumerate() {
            if let Err(e) = self.cleanup_container(container) {
                cleanup_errors.push(format!("Container {}: {}", index, e));
            }
        }

        self.containers.clear();

        if !cleanup_errors.is_empty() {
            return Err(anyhow::anyhow!("Failed to clean up some containers: {}", cleanup_errors.join(", ")));
        }

        Ok(())
    }

    /// Clean up a specific container with platform-specific optimizations
    fn cleanup_container(&self, container: &DatabaseContainer) -> Result<()> {
        let container_id = container.container.container_id();

        // Log cleanup attempt with platform info
        eprintln!("Cleaning up container: {} (platform: {})", container_id, std::env::consts::OS);

        // Platform-specific cleanup optimizations
        match std::env::consts::OS {
            "macos" => {
                // On macOS, Docker Desktop may need more time for cleanup
                self.cleanup_container_macos(&container_id)?;
            },
            "linux" => {
                // Standard Linux cleanup
                self.cleanup_container_linux(&container_id)?;
            },
            _ => {
                // Fallback cleanup for other platforms
                self.cleanup_container_generic(&container_id)?;
            },
        }

        // Verify container is actually removed
        self.verify_container_cleanup(&container_id)?;

        Ok(())
    }

    /// macOS-specific container cleanup with Docker Desktop considerations
    fn cleanup_container_macos(&self, container_id: &str) -> Result<()> {
        eprintln!("Using macOS-optimized cleanup for container: {}", container_id);

        // First, try graceful stop with longer timeout for Docker Desktop
        let stop_output = std::process::Command::new("docker")
            .args(["stop", "--time", "30", container_id]) // 30 second timeout
            .output();

        match stop_output {
            Ok(output) if output.status.success() => {
                eprintln!("Successfully stopped container: {}", container_id);
            },
            Ok(output) => {
                eprintln!(
                    "Failed to gracefully stop container {}: {}",
                    container_id,
                    String::from_utf8_lossy(&output.stderr)
                );

                // Try force kill on macOS if graceful stop fails
                let kill_output = std::process::Command::new("docker")
                    .args(["kill", container_id])
                    .output();

                if let Ok(kill_result) = kill_output {
                    if kill_result.status.success() {
                        eprintln!("Force killed container: {}", container_id);
                    } else {
                        eprintln!(
                            "Failed to force kill container {}: {}",
                            container_id,
                            String::from_utf8_lossy(&kill_result.stderr)
                        );
                    }
                }
            },
            Err(e) => {
                eprintln!("Error stopping container {} on macOS: {}", container_id, e);
            },
        }

        // Remove container with force flag
        let rm_output = std::process::Command::new("docker")
            .args(["rm", "-f", "-v", container_id]) // -v removes associated volumes
            .output();

        match rm_output {
            Ok(output) if output.status.success() => {
                eprintln!("Successfully removed container and volumes: {}", container_id);
            },
            Ok(output) => {
                eprintln!("Failed to remove container {}: {}", container_id, String::from_utf8_lossy(&output.stderr));
            },
            Err(e) => {
                eprintln!("Error removing container {} on macOS: {}", container_id, e);
            },
        }

        Ok(())
    }

    /// Linux-specific container cleanup
    fn cleanup_container_linux(&self, container_id: &str) -> Result<()> {
        eprintln!("Using Linux-optimized cleanup for container: {}", container_id);

        // Standard stop with shorter timeout for Linux
        let stop_output = std::process::Command::new("docker")
            .args(["stop", "--time", "10", container_id]) // 10 second timeout
            .output();

        match stop_output {
            Ok(output) if output.status.success() => {
                eprintln!("Successfully stopped container: {}", container_id);
            },
            Ok(output) => {
                eprintln!("Failed to stop container {}: {}", container_id, String::from_utf8_lossy(&output.stderr));
            },
            Err(e) => {
                eprintln!("Error stopping container {} on Linux: {}", container_id, e);
            },
        }

        // Remove container
        let rm_output = std::process::Command::new("docker")
            .args(["rm", "-f", "-v", container_id])
            .output();

        match rm_output {
            Ok(output) if output.status.success() => {
                eprintln!("Successfully removed container and volumes: {}", container_id);
            },
            Ok(output) => {
                eprintln!("Failed to remove container {}: {}", container_id, String::from_utf8_lossy(&output.stderr));
            },
            Err(e) => {
                eprintln!("Error removing container {} on Linux: {}", container_id, e);
            },
        }

        Ok(())
    }

    /// Generic container cleanup for other platforms
    fn cleanup_container_generic(&self, container_id: &str) -> Result<()> {
        eprintln!("Using generic cleanup for container: {}", container_id);

        // Basic stop and remove
        let _ = std::process::Command::new("docker")
            .args(["stop", container_id])
            .output();

        let _ = std::process::Command::new("docker")
            .args(["rm", "-f", container_id])
            .output();

        Ok(())
    }

    /// Verify that container cleanup was successful
    fn verify_container_cleanup(&self, container_id: &str) -> Result<()> {
        // Check if container still exists
        let inspect_output = std::process::Command::new("docker")
            .args(["inspect", container_id])
            .output();

        match inspect_output {
            Ok(output) if !output.status.success() => {
                // Container doesn't exist - cleanup successful
                eprintln!("Verified container {} has been removed", container_id);
                Ok(())
            },
            Ok(_) => {
                // Container still exists
                eprintln!("Warning: Container {} may still exist after cleanup", container_id);
                Ok(()) // Don't fail the test, just warn
            },
            Err(e) => {
                eprintln!("Error verifying container cleanup for {}: {}", container_id, e);
                Ok(()) // Don't fail the test for verification errors
            },
        }
    }

    /// Get resource usage statistics for all containers
    pub fn get_resource_usage(&self) -> Result<Vec<ContainerResourceUsage>> {
        let mut usage_stats = Vec::new();

        for container in &self.containers {
            match self.get_container_resource_usage(container) {
                Ok(usage) => usage_stats.push(usage),
                Err(e) => {
                    eprintln!(
                        "Failed to get resource usage for container {}: {}",
                        container.container.container_id(),
                        e
                    );
                },
            }
        }

        Ok(usage_stats)
    }

    /// Get resource usage for a specific container
    fn get_container_resource_usage(&self, container: &DatabaseContainer) -> Result<ContainerResourceUsage> {
        let container_id = container.container.container_id();

        let stats_output = std::process::Command::new("docker")
            .args([
                "stats",
                "--no-stream",
                "--format",
                "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}\t{{.BlockIO}}",
                &container_id,
            ])
            .output()
            .context("Failed to get container stats")?;

        if !stats_output.status.success() {
            return Err(anyhow::anyhow!(
                "Docker stats command failed: {}",
                String::from_utf8_lossy(&stats_output.stderr)
            ));
        }

        let stats_output_str = String::from_utf8_lossy(&stats_output.stdout);
        let lines: Vec<&str> = stats_output_str.lines().collect();

        if lines.len() < 2 {
            return Err(anyhow::anyhow!("Unexpected docker stats output format"));
        }

        // Parse the stats line (skip header)
        let stats_line = lines[1];
        let parts: Vec<&str> = stats_line.split('\t').collect();

        if parts.len() < 5 {
            return Err(anyhow::anyhow!("Failed to parse docker stats output"));
        }

        Ok(ContainerResourceUsage {
            container_id: container_id.clone(),
            cpu_percent: parts[1].to_string(),
            memory_usage: parts[2].to_string(),
            network_io: parts[3].to_string(),
            block_io: parts[4].to_string(),
        })
    }

    /// Check if Docker is available
    pub fn check_docker_availability() -> Result<()> {
        let output = std::process::Command::new("docker")
            .arg("version")
            .output()
            .context("Failed to execute 'docker version' command")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Docker is not available or not running. Stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }
}

impl Default for ContainerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Container resource usage information
#[derive(Debug, Clone)]
pub struct ContainerResourceUsage {
    /// Container ID
    pub container_id: String,
    /// CPU usage percentage
    pub cpu_percent: String,
    /// Memory usage (e.g., "123MiB / 2GiB")
    pub memory_usage: String,
    /// Network I/O (e.g., "1.2kB / 3.4kB")
    pub network_io: String,
    /// Block I/O (e.g., "5.6MB / 7.8MB")
    pub block_io: String,
}

/// Utility functions for container management
pub mod utils {
    use super::*;

    /// Skip test if Docker is not available with comprehensive checks
    pub fn skip_if_no_docker() -> Result<()> {
        let preflight = ContainerManager::docker_preflight_check();

        if !preflight.docker_available {
            for message in &preflight.skip_messages {
                eprintln!("SKIP: {}", message);
            }
            // Return Ok to skip the test gracefully instead of failing
            return Ok(());
        }

        if !preflight.platform_supported {
            for message in &preflight.skip_messages {
                eprintln!("SKIP: {}", message);
            }
            // Return Ok to skip the test gracefully instead of failing
            return Ok(());
        }

        if !preflight.sufficient_resources {
            for message in &preflight.skip_messages {
                eprintln!("SKIP: {}", message);
            }
            // Return Ok to skip the test gracefully instead of failing
            return Ok(());
        }

        Ok(())
    }

    /// Create a test database container with comprehensive error handling
    pub fn create_test_database(db_type: TestDatabase) -> Result<DatabaseContainer> {
        skip_if_no_docker()?;

        // Log container creation attempt
        eprintln!("Creating {} container with TLS={}", db_type.name(), db_type.is_tls_enabled());

        let start_time = std::time::Instant::now();
        let container = DatabaseContainer::new(db_type)?;
        let creation_time = start_time.elapsed();

        eprintln!("Container created successfully in {:.2}s", creation_time.as_secs_f64());

        Ok(container)
    }

    /// Create a test database container with custom TLS configuration
    pub fn create_test_database_with_tls(
        db_type: TestDatabase,
        tls_config: ContainerTlsConfig,
    ) -> Result<DatabaseContainer> {
        skip_if_no_docker()?;

        eprintln!("Creating {} container with custom TLS configuration", db_type.name());

        let start_time = std::time::Instant::now();
        let container = DatabaseContainer::new_with_tls(db_type, tls_config)?;
        let creation_time = start_time.elapsed();

        eprintln!("Container with custom TLS created successfully in {:.2}s", creation_time.as_secs_f64());

        Ok(container)
    }

    /// Get appropriate container timeout for environment
    pub fn get_container_timeout() -> Duration {
        if is_ci_environment() {
            Duration::from_secs(300) // 5 minutes for CI
        } else {
            Duration::from_secs(60) // 1 minute for local
        }
    }

    /// Get CI-specific resource limits
    pub fn get_ci_resource_limits() -> CiResourceLimits {
        CiResourceLimits {
            max_containers: if is_ci_environment() { 2 } else { 5 },
            max_memory_per_container: 1024 * 1024 * 1024, // 1GB
            max_total_memory: 2 * 1024 * 1024 * 1024,     // 2GB
            container_startup_timeout: get_container_timeout(),
        }
    }

    /// Check if running on macOS and provide platform-specific guidance
    pub fn check_macos_docker_setup() -> Result<()> {
        if std::env::consts::OS != "macos" {
            return Ok(());
        }

        eprintln!("Detected macOS platform - checking Docker Desktop setup...");

        // Check if Docker Desktop is running
        let docker_info = std::process::Command::new("docker").args(["system", "info"]).output();

        match docker_info {
            Ok(output) if output.status.success() => {
                let info_str = String::from_utf8_lossy(&output.stdout);

                // Check for Docker Desktop specific indicators
                if info_str.contains("Docker Desktop") {
                    eprintln!("✓ Docker Desktop detected and running");
                } else {
                    eprintln!("✓ Docker daemon running (may be Docker Desktop or other)");
                }

                // Check available resources on macOS
                if let Ok(memory_output) = std::process::Command::new("sysctl").args(["-n", "hw.memsize"]).output()
                    && let Ok(memory_str) = String::from_utf8(memory_output.stdout)
                    && let Ok(total_memory) = memory_str.trim().parse::<u64>()
                {
                    let memory_gb = total_memory / (1024 * 1024 * 1024);
                    eprintln!("✓ System memory: {} GB", memory_gb);

                    if memory_gb < 8 {
                        eprintln!("⚠ Warning: Less than 8GB RAM detected. Container tests may be slower.");
                    }
                }

                Ok(())
            },
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(anyhow::anyhow!(
                    "Docker not available on macOS. Please install and start Docker Desktop:\n\
                     1. Download from https://docker.com/products/docker-desktop\n\
                     2. Install Docker Desktop\n\
                     3. Start Docker Desktop and wait for it to be ready\n\
                     4. Verify with 'docker version'\n\
                     Error: {}",
                    stderr
                ))
            },
            Err(e) => Err(anyhow::anyhow!(
                "Failed to check Docker on macOS: {}\n\
                     Please ensure Docker Desktop is installed and running.",
                e
            )),
        }
    }

    /// Get platform-specific container resource recommendations
    pub fn get_platform_resource_recommendations() -> String {
        match std::env::consts::OS {
            "macos" => "macOS Docker Desktop recommendations:\n\
                 - Allocate at least 4GB RAM to Docker Desktop\n\
                 - Ensure at least 10GB free disk space\n\
                 - Use Docker Desktop settings to adjust resource limits\n\
                 - Consider using Rosetta 2 emulation if on Apple Silicon"
                .to_string(),
            "linux" => "Linux Docker recommendations:\n\
                 - Ensure Docker daemon is running: sudo systemctl start docker\n\
                 - Add user to docker group: sudo usermod -aG docker $USER\n\
                 - Ensure at least 2GB RAM and 5GB disk space available\n\
                 - Consider using cgroups v2 for better resource management"
                .to_string(),
            _ => "General Docker recommendations:\n\
                 - Ensure Docker is installed and running\n\
                 - Allocate sufficient resources for container tests\n\
                 - Check Docker documentation for platform-specific setup"
                .to_string(),
        }
    }

    /// Perform platform-specific Docker optimization
    pub fn optimize_docker_for_platform() -> Result<()> {
        match std::env::consts::OS {
            "macos" => {
                eprintln!("Applying macOS Docker optimizations...");

                // Check Docker Desktop resource allocation
                if let Ok(output) = std::process::Command::new("docker")
                    .args(["system", "info", "--format", "{{json .}}"])
                    .output()
                    && output.status.success()
                {
                    let info_str = String::from_utf8_lossy(&output.stdout);
                    eprintln!("Docker system info retrieved for optimization analysis");

                    // Parse JSON to check memory allocation (basic check)
                    if info_str.contains("\"MemTotal\"") {
                        eprintln!("✓ Docker memory allocation detected");
                    }
                }

                // Suggest Docker Desktop settings optimization
                eprintln!("💡 For optimal performance on macOS:");
                eprintln!("   - Open Docker Desktop preferences");
                eprintln!("   - Go to Resources > Advanced");
                eprintln!("   - Set Memory to at least 4GB");
                eprintln!("   - Set Disk image size to at least 64GB");

                Ok(())
            },
            "linux" => {
                eprintln!("Applying Linux Docker optimizations...");

                // Check if user is in docker group
                if let Ok(output) = std::process::Command::new("groups").output() {
                    let groups = String::from_utf8_lossy(&output.stdout);
                    if groups.contains("docker") {
                        eprintln!("✓ User is in docker group");
                    } else {
                        eprintln!("⚠ Consider adding user to docker group: sudo usermod -aG docker $USER");
                    }
                }

                Ok(())
            },
            _ => {
                eprintln!("No platform-specific optimizations available");
                Ok(())
            },
        }
    }

    /// Perform Docker environment validation for CI
    pub fn validate_ci_environment() -> Result<()> {
        if !is_ci_environment() {
            return Ok(()); // Skip validation for local development
        }

        let preflight = ContainerManager::docker_preflight_check();

        if let Some(env) = &preflight.environment {
            eprintln!("CI Docker Environment:");
            eprintln!("  Docker version: {}", env.docker_version);
            eprintln!("  Platform: {}", env.platform);
            eprintln!("  Available memory: {:.2} GB", env.available_memory as f64 / (1024.0 * 1024.0 * 1024.0));
            eprintln!("  Available disk: {:.2} GB", env.available_disk_space as f64 / (1024.0 * 1024.0 * 1024.0));
        }

        if !preflight.docker_available || !preflight.platform_supported || !preflight.sufficient_resources {
            for error in &preflight.error_messages {
                eprintln!("ERROR: {}", error);
            }
            for skip_msg in &preflight.skip_messages {
                eprintln!("SKIP: {}", skip_msg);
            }
            return Err(anyhow::anyhow!("CI environment validation failed"));
        }

        Ok(())
    }
}

impl DatabaseContainer {
    /// Validate TLS connection establishment
    pub fn validate_tls_connection(&self) -> Result<TlsValidationResult> {
        // Try to connect with TLS-enabled URL
        let tls_url = if self.tls_config.enabled {
            self.connection_url_with_ssl_mode("REQUIRED")?
        } else {
            // For non-TLS containers, try to enable TLS anyway to test capability
            self.connection_url_with_ssl_mode("PREFERRED")?
        };

        match self.test_connection_with_url(&tls_url) {
            Ok(()) => Ok(TlsValidationResult {
                tls_connection_success: true,
                tls_error: None,
            }),
            Err(e) => Ok(TlsValidationResult {
                tls_connection_success: false,
                tls_error: Some(e.to_string()),
            }),
        }
    }

    /// Validate non-TLS connection establishment
    pub fn validate_plain_connection(&self) -> Result<PlainValidationResult> {
        // Try to connect with TLS disabled
        let plain_url = self.connection_url_with_ssl_mode("DISABLED")?;

        match self.test_connection_with_url(&plain_url) {
            Ok(()) => Ok(PlainValidationResult {
                plain_connection_success: true,
                plain_error: None,
            }),
            Err(e) => Ok(PlainValidationResult {
                plain_connection_success: false,
                plain_error: Some(e.to_string()),
            }),
        }
    }

    /// Test connection with a specific URL
    fn test_connection_with_url(&self, url: &str) -> Result<()> {
        use mysql::prelude::*;

        let opts = mysql::Opts::from_url(url).with_context(|| format!("Failed to parse connection URL: {}", url))?;

        let pool = mysql::Pool::new(opts).context("Failed to create connection pool")?;

        let mut conn = pool.get_conn().context("Failed to get database connection")?;

        // Test with a simple query
        let result: Option<i32> = conn.query_first("SELECT 1").context("Failed to execute test query")?;

        match result {
            Some(1) => Ok(()),
            Some(other) => Err(anyhow::anyhow!("Unexpected query result: {}", other)),
            None => Err(anyhow::anyhow!("Query returned no results")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_disk_space_basic() {
        // Test that check_disk_space returns a reasonable value
        let result = ContainerManager::check_disk_space();
        assert!(result.is_ok(), "check_disk_space should not fail");

        let disk_space = result.unwrap();
        assert!(disk_space > 0, "Available disk space should be greater than 0");

        // Disk space should be at least 1GB (reasonable minimum)
        let min_expected = 1024 * 1024 * 1024; // 1GB
        assert!(disk_space >= min_expected, "Available disk space should be at least 1GB, got {} bytes", disk_space);
    }

    #[test]
    fn test_check_disk_space_returns_bytes() {
        let result = ContainerManager::check_disk_space();
        assert!(result.is_ok(), "check_disk_space should not fail");

        let disk_space = result.unwrap();

        // Verify it's a reasonable size (not too small, not impossibly large)
        let min_bytes = 1024 * 1024; // 1MB minimum
        let max_bytes = 100 * 1024 * 1024 * 1024 * 1024; // 100TB maximum (reasonable upper bound for CI/CD servers)

        assert!(disk_space >= min_bytes, "Disk space too small: {} bytes", disk_space);
        assert!(disk_space <= max_bytes, "Disk space too large (likely error): {} bytes", disk_space);
    }

    #[test]
    fn test_check_disk_space_consistency() {
        // Test that multiple calls return consistent results
        let result1 = ContainerManager::check_disk_space();
        let result2 = ContainerManager::check_disk_space();

        assert!(result1.is_ok(), "First call should succeed");
        assert!(result2.is_ok(), "Second call should succeed");

        let space1 = result1.unwrap();
        let space2 = result2.unwrap();

        // Results should be within 10% of each other (allowing for some system activity)
        let diff = space1.abs_diff(space2);
        let max_diff = space1 / 10; // 10% tolerance

        assert!(
            diff <= max_diff,
            "Disk space results should be consistent: {} vs {} (diff: {})",
            space1,
            space2,
            diff
        );
    }

    #[test]
    fn test_docker_preflight_check_disk_space() {
        let preflight = ContainerManager::docker_preflight_check();

        // If Docker is available, disk space should be checked
        if preflight.docker_available {
            assert!(
                preflight.environment.is_some(),
                "Docker environment info should be available when Docker is available"
            );

            if let Some(env) = &preflight.environment {
                assert!(env.available_disk_space > 0, "Available disk space should be greater than 0");

                // Should be at least 1GB
                let min_disk = 1024 * 1024 * 1024;
                assert!(
                    env.available_disk_space >= min_disk,
                    "Available disk space should be at least 1GB, got {} bytes",
                    env.available_disk_space
                );
            }
        }
    }

    #[test]
    fn test_resource_availability_check() {
        let result = ContainerManager::check_resource_availability();
        assert!(result.is_ok(), "Resource availability check should not fail");

        let sufficient = result.unwrap();

        // If resources are sufficient, disk space should be at least 1GB
        if sufficient {
            let disk_result = ContainerManager::check_disk_space();
            assert!(disk_result.is_ok(), "Disk space check should succeed when resources are sufficient");

            let disk_space = disk_result.unwrap();
            let min_disk = 1024 * 1024 * 1024; // 1GB
            assert!(disk_space >= min_disk, "When resources are sufficient, disk space should be at least 1GB");
        }
    }

    #[test]
    fn test_disk_space_formatting() {
        let result = ContainerManager::check_disk_space();
        assert!(result.is_ok(), "check_disk_space should not fail");

        let disk_space = result.unwrap();

        // Test that we can format the disk space in a human-readable way
        let gb = disk_space as f64 / (1024.0 * 1024.0 * 1024.0);
        let formatted = format!("{:.2} GB", gb);

        assert!(!formatted.is_empty(), "Formatted disk space should not be empty");
        assert!(formatted.contains("GB"), "Formatted disk space should contain 'GB'");
        assert!(gb > 0.0, "Disk space in GB should be greater than 0");
    }

    #[test]
    fn test_sysinfo_integration() {
        // Test that sysinfo crate is working correctly
        use sysinfo::Disks;

        let disks = Disks::new_with_refreshed_list();
        assert!(!disks.is_empty(), "Should have at least one disk");

        // Test that we can access disk information
        for disk in &disks {
            let mount_point = disk.mount_point();
            let available_space = disk.available_space();

            assert!(available_space > 0, "Disk {} should have available space", mount_point.display());
        }
    }
}
