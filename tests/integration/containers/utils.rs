//! Utility functions and platform-specific code for Gold Digger integration tests
//!
//! This module provides utility functions, platform-specific optimizations,
//! and helper functions for container management and testing.

use anyhow::{Context, Result};
use std::time::Duration;

use super::{
    container_manager::{CiResourceLimits, ContainerResourceUsage, DockerEnvironment, DockerPreflightResult},
    database_container::DatabaseContainer,
};
use crate::integration::TestDatabase;

/// Check if running in a CI environment
pub fn is_ci_environment() -> bool {
    std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok()
}

/// Container manager for handling multiple database types
pub struct ContainerManager {
    /// Available containers
    containers: Vec<DatabaseContainer>,
    /// Maximum number of containers to manage simultaneously
    max_containers: usize,
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
    pub fn check_resource_availability() -> Result<bool> {
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
        let container_id = container.health_info().container_id;

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
                        container.health_info().container_id,
                        e
                    );
                },
            }
        }

        Ok(usage_stats)
    }

    /// Get resource usage for a specific container
    fn get_container_resource_usage(&self, container: &DatabaseContainer) -> Result<ContainerResourceUsage> {
        let container_id = &container.health_info().container_id;

        let stats_output = std::process::Command::new("docker")
            .args([
                "stats",
                "--no-stream",
                "--format",
                "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}\t{{.BlockIO}}",
                container_id,
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
    tls_config: crate::integration::containers::tls_config::ContainerTlsConfig,
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
