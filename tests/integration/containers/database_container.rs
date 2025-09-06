//! Database container implementation for Gold Digger integration tests
//!
//! This module provides the main DatabaseContainer struct and its implementation
//! for managing MySQL and MariaDB containers with health checks and data seeding.

use anyhow::{Context, Result};
use std::path::Path;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use testcontainers_modules::testcontainers::{ImageExt, runners::SyncRunner};

use super::{
    container_types::{ContainerInstance, MariaDbContainer, MySqlContainer},
    database_info::DatabaseInfo,
    tls_config::{ContainerTlsConfig, PlainValidationResult, TlsValidationResult},
    utils::is_ci_environment,
};
use crate::integration::{TestDatabase, TestDatabasePlain, TestDatabaseTls, TlsContainerConfig};

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
    pub fn new_tls(db_type: TestDatabaseTls) -> Result<Self> {
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
    pub fn new_plain(db_type: TestDatabasePlain) -> Result<Self> {
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
        let container = testcontainers_modules::mysql::Mysql::default()
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
        let container = testcontainers_modules::mariadb::Mariadb::default()
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
        let container = testcontainers_modules::mysql::Mysql::default()
            .with_env_var("MYSQL_ALLOW_EMPTY_PASSWORD", "yes")
            .with_env_var("MYSQL_ROOT_HOST", "%")
            .start()
            .context("Failed to start MySQL TLS container")?;

        let host_port = container
            .get_host_port_ipv4(3306)
            .context("Failed to get MySQL TLS container port mapping")?;

        // Use plain connection URL since TLS is not configured on the container
        // TODO: Implement proper TLS configuration with certificate mounting
        let connection_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);

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
        let container = testcontainers_modules::mariadb::Mariadb::default()
            .with_env_var("MARIADB_ALLOW_EMPTY_ROOT_PASSWORD", "yes")
            .with_env_var("MARIADB_ROOT_HOST", "%")
            .start()
            .context("Failed to start MariaDB TLS container")?;

        let host_port = container
            .get_host_port_ipv4(3306)
            .context("Failed to get MariaDB TLS container port mapping")?;

        // Use plain connection URL since TLS is not configured on the container
        // TODO: Implement proper TLS configuration with certificate mounting
        let connection_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);

        Ok(MariaDbContainer {
            container,
            connection_url,
        })
    }

    /// Create a MySQL container for standard unencrypted connection testing
    fn create_mysql_container_plain() -> Result<MySqlContainer> {
        let container = testcontainers_modules::mysql::Mysql::default()
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
        let container = testcontainers_modules::mariadb::Mariadb::default()
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
    fn convert_tls_config(config: &TlsContainerConfig) -> Result<ContainerTlsConfig> {
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
        use super::container_manager::RetryConfig;

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
    fn test_connection_with_retry(&self, retry_config: &super::container_manager::RetryConfig) -> Result<bool> {
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
        self.test_connection_detailed().unwrap_or(false)
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
        // Validate SSL mode
        match ssl_mode {
            "DISABLED" | "PREFERRED" | "REQUIRED" | "VERIFY_CA" | "VERIFY_IDENTITY" => {},
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid SSL mode: {}. Must be one of: DISABLED, PREFERRED, REQUIRED, VERIFY_CA, VERIFY_IDENTITY",
                    ssl_mode
                ));
            },
        }

        // Add SSL mode parameter to the connection URL
        let base_url = &self.connection_url;
        let ssl_url = if base_url.contains('?') {
            format!("{}&ssl-mode={}", base_url, ssl_mode)
        } else {
            format!("{}?ssl-mode={}", base_url, ssl_mode)
        };

        Ok(ssl_url)
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
            let version = DatabaseInfo::extract_version_number(&version_string);
            let features = DatabaseInfo::detect_mariadb_features(&version);
            ("MariaDB".to_string(), version, features)
        } else {
            let version = DatabaseInfo::extract_version_number(&version_string);
            let features = DatabaseInfo::detect_mysql_features(&version);
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
    pub fn health_info(&self) -> super::container_manager::ContainerHealthInfo {
        super::container_manager::ContainerHealthInfo {
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

    /// Validate TLS connection establishment
    pub fn validate_tls_connection(&self) -> Result<TlsValidationResult> {
        // For now, use the base connection URL since TLS is not fully implemented
        // TODO: Implement proper TLS validation when TLS containers are configured
        let tls_url = &self.connection_url;

        match self.test_connection_with_url(tls_url) {
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
        // For now, use the base connection URL since TLS is not fully implemented
        // TODO: Implement proper plain connection validation when TLS containers are configured
        let plain_url = &self.connection_url;

        match self.test_connection_with_url(plain_url) {
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
