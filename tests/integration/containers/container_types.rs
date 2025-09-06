//! Container wrapper types and traits for Gold Digger integration tests
//!
//! This module provides abstracted container types and traits for managing
//! MySQL and MariaDB containers in a unified way.

use anyhow::{Context, Result};
use testcontainers_modules::{mariadb::Mariadb, mysql::Mysql, testcontainers::Container};

/// Trait for abstracting container operations across MySQL and MariaDB
pub trait ContainerInstance {
    /// Get the connection URL for this container
    fn connection_url(&self) -> &str;

    /// Get the container ID for debugging
    fn container_id(&self) -> String;

    /// Check if the container is healthy
    fn is_healthy(&self) -> bool;
}

/// MySQL container wrapper
pub struct MySqlContainer {
    pub container: Container<Mysql>,
    pub connection_url: String,
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
pub struct MariaDbContainer {
    pub container: Container<Mariadb>,
    pub connection_url: String,
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
