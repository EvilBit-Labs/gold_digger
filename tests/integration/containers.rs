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
        // Actually test database connectivity
        use mysql::prelude::*;
        match mysql::Opts::from_url(&self.connection_url) {
            Ok(opts) => match mysql::Pool::new(opts) {
                Ok(pool) => match pool.get_conn() {
                    Ok(mut conn) => conn.query_drop("SELECT 1").is_ok(),
                    Err(_) => false,
                },
                Err(_) => false,
            },
            Err(_) => false,
        }
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
        // Actually test database connectivity
        use mysql::prelude::*;
        match mysql::Opts::from_url(&self.connection_url) {
            Ok(opts) => match mysql::Pool::new(opts) {
                Ok(pool) => match pool.get_conn() {
                    Ok(mut conn) => conn.query_drop("SELECT 1").is_ok(),
                    Err(_) => false,
                },
                Err(_) => false,
            },
            Err(_) => false,
        }
    }
}

impl DatabaseContainer {
    /// Create a new database container of the specified type
    pub fn new(db_type: TestDatabase) -> Result<Self> {
        let temp_dir = tempfile::tempdir().context("Failed to create temporary directory")?;

        let (container, connection_url) = match db_type {
            TestDatabase::MySQL => {
                let mysql_container = Self::create_mysql_container()?;
                let url = mysql_container.connection_url.clone();
                (Box::new(mysql_container) as Box<dyn ContainerInstance>, url)
            },
            TestDatabase::MariaDB => {
                let mariadb_container = Self::create_mariadb_container()?;
                let url = mariadb_container.connection_url.clone();
                (Box::new(mariadb_container) as Box<dyn ContainerInstance>, url)
            },
        };

        let db_container = Self {
            db_type,
            container,
            connection_url,
            temp_dir,
        };

        // Wait for container to be ready
        db_container.wait_for_readiness()?;

        Ok(db_container)
    }

    /// Create a MySQL container
    fn create_mysql_container() -> Result<MySqlContainer> {
        let mysql_image = Mysql::default()
            .with_env_var("MYSQL_ROOT_PASSWORD", "test_root_password")
            .with_env_var("MYSQL_DATABASE", "gold_digger_test")
            .with_env_var("MYSQL_USER", "test_user")
            .with_env_var("MYSQL_PASSWORD", "test_password");

        let container = mysql_image.start().context("Failed to start MySQL container")?;

        let host_port = container
            .get_host_port_ipv4(3306)
            .context("Failed to get MySQL container port")?;

        let connection_url = format!("mysql://test_user:test_password@127.0.0.1:{}/gold_digger_test", host_port);

        Ok(MySqlContainer {
            container,
            connection_url,
        })
    }

    /// Create a MariaDB container
    fn create_mariadb_container() -> Result<MariaDbContainer> {
        let mariadb_image = Mariadb::default()
            .with_env_var("MARIADB_ROOT_PASSWORD", "test_root_password")
            .with_env_var("MARIADB_DATABASE", "gold_digger_test")
            .with_env_var("MARIADB_USER", "test_user")
            .with_env_var("MARIADB_PASSWORD", "test_password");

        let container = mariadb_image.start().context("Failed to start MariaDB container")?;

        let host_port = container
            .get_host_port_ipv4(3306)
            .context("Failed to get MariaDB container port")?;

        let connection_url = format!("mysql://test_user:test_password@127.0.0.1:{}/gold_digger_test", host_port);

        Ok(MariaDbContainer {
            container,
            connection_url,
        })
    }

    /// Wait for the database container to be ready for connections
    fn wait_for_readiness(&self) -> Result<()> {
        let timeout = if is_ci_environment() {
            Duration::from_secs(120) // 2 minutes for CI
        } else {
            Duration::from_secs(30) // 30 seconds for local
        };

        let start_time = Instant::now();

        while start_time.elapsed() < timeout {
            if self.test_connection() {
                return Ok(());
            }

            std::thread::sleep(Duration::from_millis(500));
        }

        Err(anyhow::anyhow!("Database container failed to become ready within {} seconds", timeout.as_secs()))
    }

    /// Test database connection
    pub fn test_connection(&self) -> bool {
        use mysql::prelude::*;

        match mysql::Opts::from_url(&self.connection_url) {
            Ok(opts) => match mysql::Pool::new(opts) {
                Ok(pool) => {
                    match pool.get_conn() {
                        Ok(mut conn) => {
                            // Try a simple query
                            conn.query_drop("SELECT 1").is_ok()
                        },
                        Err(_) => false,
                    }
                },
                Err(_) => false,
            },
            Err(_) => false,
        }
    }

    /// Get the database type
    pub fn db_type(&self) -> &TestDatabase {
        &self.db_type
    }

    /// Get the connection URL
    pub fn connection_url(&self) -> &str {
        &self.connection_url
    }

    /// Get the temporary directory path
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Seed the database with test schema and data
    pub fn seed_data(&self) -> Result<()> {
        use mysql::prelude::*;

        let opts = mysql::Opts::from_url(&self.connection_url).context("Failed to parse database URL")?;
        let pool = mysql::Pool::new(opts).context("Failed to create database connection pool")?;

        let mut conn = pool.get_conn().context("Failed to get database connection")?;

        // Create basic test table for initial testing
        conn.exec_drop(
            r"CREATE TABLE IF NOT EXISTS test_data (
                id INT PRIMARY KEY AUTO_INCREMENT,
                name VARCHAR(255),
                value INT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            (),
        )
        .context("Failed to create test table")?;

        // Insert some test data
        conn.exec_drop(
            r"INSERT INTO test_data (name, value) VALUES
              ('test1', 100),
              ('test2', 200),
              ('test3', 300)
              ON DUPLICATE KEY UPDATE value = VALUES(value)",
            (),
        )
        .context("Failed to insert test data")?;

        Ok(())
    }

    /// Execute a SQL statement on the database
    pub fn execute_sql(&self, sql: &str) -> Result<()> {
        use mysql::prelude::*;

        let opts = mysql::Opts::from_url(&self.connection_url).context("Failed to parse database URL")?;
        let pool = mysql::Pool::new(opts).context("Failed to create database connection pool")?;

        let mut conn = pool.get_conn().context("Failed to get database connection")?;

        conn.exec_drop(sql, ())
            .with_context(|| format!("Failed to execute SQL: {}", sql))?;

        Ok(())
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

    /// Redact sensitive information from connection URL
    fn redact_connection_url(&self) -> String {
        // Replace password with ***
        let url = &self.connection_url;
        if let Some(at_pos) = url.find('@')
            && let Some(colon_pos) = url[..at_pos].rfind(':')
        {
            let mut redacted = url.to_string();
            redacted.replace_range(colon_pos + 1..at_pos, "***");
            return redacted;
        }
        "***REDACTED***".to_string()
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

/// Container manager for handling multiple database types
pub struct ContainerManager {
    /// Available containers
    containers: Vec<DatabaseContainer>,
}

impl ContainerManager {
    /// Create a new container manager
    pub fn new() -> Self {
        Self { containers: Vec::new() }
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

/// Utility functions for container management
pub mod utils {
    use super::*;

    /// Skip test if Docker is not available
    pub fn skip_if_no_docker() -> Result<()> {
        ContainerManager::check_docker_availability().context("Docker not available - skipping container-based test")
    }

    /// Create a test database container with error handling
    pub fn create_test_database(db_type: TestDatabase) -> Result<DatabaseContainer> {
        skip_if_no_docker()?;
        DatabaseContainer::new(db_type)
    }

    /// Get appropriate container timeout for environment
    pub fn get_container_timeout() -> Duration {
        if is_ci_environment() {
            Duration::from_secs(300) // 5 minutes for CI
        } else {
            Duration::from_secs(60) // 1 minute for local
        }
    }
}
