//! Container utilities for test support
//!
//! This module provides simplified container management utilities
//! that can be shared across different test modules.

#![allow(dead_code)]

use anyhow::{Context, Result};
use std::time::Duration;

/// Re-export container types from integration module
pub use crate::integration::containers::{ContainerManager, DatabaseContainer};
pub use crate::integration::{TestDatabase, is_ci_environment};

/// Simplified container factory for common test scenarios
pub struct TestContainerFactory;

impl TestContainerFactory {
    /// Create a MySQL container for testing
    pub fn mysql() -> Result<DatabaseContainer> {
        Self::check_docker_available()?;
        DatabaseContainer::new(TestDatabase::mysql())
    }

    /// Create a MariaDB container for testing
    pub fn mariadb() -> Result<DatabaseContainer> {
        Self::check_docker_available()?;
        DatabaseContainer::new(TestDatabase::mariadb())
    }

    /// Create a container of the specified type
    pub fn create(db_type: TestDatabase) -> Result<DatabaseContainer> {
        Self::check_docker_available()?;
        DatabaseContainer::new(db_type)
    }

    /// Check if Docker is available and fail fast if not
    fn check_docker_available() -> Result<()> {
        ContainerManager::check_docker_availability().context("Docker is required for container-based tests")
    }
}

/// Container test utilities
pub struct ContainerTestUtils;

impl ContainerTestUtils {
    /// Skip test if Docker is not available
    pub fn skip_if_no_docker() {
        if ContainerManager::check_docker_availability().is_err() {
            println!("Skipping test: Docker not available");
        }
    }

    /// Get appropriate timeout for container operations
    pub fn container_timeout() -> Duration {
        if is_ci_environment() {
            Duration::from_secs(300) // 5 minutes for CI
        } else {
            Duration::from_secs(60) // 1 minute for local
        }
    }

    /// Wait for container to be ready with custom timeout
    pub fn wait_for_container_ready(container: &DatabaseContainer, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if container.test_connection() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(500));
        }

        Err(anyhow::anyhow!("Container failed to become ready within {} seconds", timeout.as_secs()))
    }
}
