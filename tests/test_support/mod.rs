//! Test support module for Gold Digger integration tests
//!
//! This module provides shared utilities, fixtures, and helper functions
//! that can be used across different test modules and integration test suites.

#![allow(dead_code)]

pub mod cli;
pub mod containers;
pub mod fixtures;
pub mod parsing;

// Re-export commonly used items
#[allow(unused_imports)]
pub use cli::*;
#[allow(unused_imports)]
pub use fixtures::*;

use anyhow::Result;
use std::path::Path;

/// Test support utilities and constants
pub struct TestSupport;

impl TestSupport {
    /// Get the path to test fixtures directory
    pub fn fixtures_dir() -> &'static str {
        "tests/fixtures"
    }

    /// Get the path to test data directory
    pub fn test_data_dir() -> &'static str {
        "tests/test_data"
    }

    /// Check if we're running in a CI environment
    pub fn is_ci() -> bool {
        std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok()
    }

    /// Get appropriate test timeout based on environment
    pub fn test_timeout() -> std::time::Duration {
        if Self::is_ci() {
            std::time::Duration::from_secs(300) // 5 minutes for CI
        } else {
            std::time::Duration::from_secs(60) // 1 minute for local
        }
    }
}

/// Common test assertions and utilities
pub struct TestUtils;

impl TestUtils {
    /// Assert that a file exists and has content
    pub fn assert_file_has_content(path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(anyhow::anyhow!("File does not exist: {}", path.display()));
        }

        let metadata = std::fs::metadata(path)?;
        if metadata.len() == 0 {
            return Err(anyhow::anyhow!("File is empty: {}", path.display()));
        }

        Ok(())
    }

    /// Create a temporary directory for test files
    pub fn create_temp_dir() -> Result<tempfile::TempDir> {
        tempfile::tempdir().map_err(|e| anyhow::anyhow!("Failed to create temp dir: {}", e))
    }

    /// Read file content as string with error context
    pub fn read_file_content(path: &Path) -> Result<String> {
        std::fs::read_to_string(path).map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path.display(), e))
    }
}
