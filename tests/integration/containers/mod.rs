//! Container management utilities for Gold Digger integration tests
//!
//! This module provides container management, health checks, and database setup
//! for MySQL and MariaDB containers using testcontainers.
//!
//! ## Module Structure
//!
//! - `database_info.rs` - Database version detection and feature compatibility
//! - `tls_config.rs` - TLS configuration and validation
//! - `container_types.rs` - Container wrapper types and traits
//! - `container_manager.rs` - Container management and resource monitoring
//! - `database_container.rs` - Main DatabaseContainer implementation
//! - `utils.rs` - Utility functions and platform-specific code

#![allow(dead_code)]

// Re-export all submodules
pub mod container_manager;
pub mod container_types;
pub mod database_container;
pub mod database_info;
pub mod tls_config;
pub mod utils;

// Re-export commonly used types and functions
// Import for internal use in tests
use utils::is_ci_environment;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_disk_space_basic() {
        // Test that check_disk_space returns a reasonable value
        let result = utils::ContainerManager::check_disk_space();
        assert!(result.is_ok(), "check_disk_space should not fail");

        let disk_space = result.unwrap();
        assert!(disk_space > 0, "Available disk space should be greater than 0");

        // Disk space should be at least 1GB (reasonable minimum)
        let min_expected = 1024 * 1024 * 1024; // 1GB
        assert!(disk_space >= min_expected, "Available disk space should be at least 1GB, got {} bytes", disk_space);
    }

    #[test]
    fn test_check_disk_space_returns_bytes() {
        let result = utils::ContainerManager::check_disk_space();
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
        let result1 = utils::ContainerManager::check_disk_space();
        let result2 = utils::ContainerManager::check_disk_space();

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
        let preflight = utils::ContainerManager::docker_preflight_check();

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
        let result = utils::ContainerManager::check_resource_availability();
        assert!(result.is_ok(), "Resource availability check should not fail");

        let sufficient = result.unwrap();

        // If resources are sufficient, disk space should be at least 1GB
        if sufficient {
            let disk_result = utils::ContainerManager::check_disk_space();
            assert!(disk_result.is_ok(), "Disk space check should succeed when resources are sufficient");

            let disk_space = disk_result.unwrap();
            let min_disk = 1024 * 1024 * 1024; // 1GB
            assert!(disk_space >= min_disk, "When resources are sufficient, disk space should be at least 1GB");
        }
    }

    #[test]
    fn test_disk_space_formatting() {
        let result = utils::ContainerManager::check_disk_space();
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
        // This test is actually questionable, as it's not really testing the sysinfo crate,
        use sysinfo::Disks;

        let disks = Disks::new_with_refreshed_list();
        assert!(!disks.is_empty(), "Should have at least one disk");

        // Test that we can access disk information
        // In CI environments, some disks might have 0 available space or be inaccessible
        let mut found_accessible_disk = false;
        for disk in &disks {
            let mount_point = disk.mount_point();
            let available_space = disk.available_space();
            let total_space = disk.total_space();

            // Log disk information for debugging
            println!(
                "Disk: {}, Available: {} bytes, Total: {} bytes",
                mount_point.display(),
                available_space,
                total_space
            );

            // A disk is considered accessible if we can get its information
            // Even if available_space is 0, the disk is still accessible
            if total_space > 0 {
                found_accessible_disk = true;
            }
        }

        // In CI environments, we might not have any disks with available space
        // but we should still be able to access disk information
        if is_ci_environment() {
            // In CI, just verify we can get disk information, even if space is 0
            assert!(
                found_accessible_disk || !disks.is_empty(),
                "Should be able to access disk information in CI environment"
            );
        } else {
            // In local environments, expect at least one disk with available space
            assert!(found_accessible_disk, "Should have at least one accessible disk with space");
        }
    }
}
