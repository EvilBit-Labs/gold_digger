//! Database version and feature detection for Gold Digger integration tests
//!
//! This module provides database version parsing, feature detection, and compatibility
//! handling for MySQL and MariaDB containers.

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

impl DatabaseInfo {
    /// Extract version number from database version string
    pub fn extract_version_number(version_string: &str) -> DatabaseVersion {
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
    pub fn detect_mysql_features(version: &DatabaseVersion) -> DatabaseFeatures {
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
    pub fn detect_mariadb_features(version: &DatabaseVersion) -> DatabaseFeatures {
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
