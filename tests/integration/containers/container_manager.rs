//! Container manager for Gold Digger integration tests
//!
//! This module provides container management, resource monitoring, and cleanup
//! functionality for multiple database containers.

use crate::integration::TestDatabase;
use std::time::Duration;

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
