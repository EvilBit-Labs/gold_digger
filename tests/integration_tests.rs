//! Integration tests for Gold Digger
//!
//! This module provides comprehensive integration testing using real MySQL/MariaDB containers
//! via testcontainers. Tests validate the complete query-to-output pipeline with seeded data.

mod fixtures;
mod integration;
mod test_support;

use integration::*;
use test_support::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// Basic smoke test to ensure integration test infrastructure works
    #[test]
    fn test_integration_infrastructure() {
        // Test that we can create test setup
        let setup = TestSetup::new().expect("Failed to create test setup");
        assert!(setup.temp_path().exists(), "Temporary directory should exist");

        // Test Docker availability check
        let docker_available = is_docker_available();
        println!("Docker available: {}", docker_available);

        // Test CI environment detection
        let is_ci = is_ci_environment();
        println!("Running in CI: {}", is_ci);

        // Test timeout configuration
        let timeout = get_test_timeout();
        println!("Test timeout: {:?}", timeout);

        // Integration test infrastructure is ready - all checks passed
    }

    /// Test TLS integration test consolidation
    #[test]
    fn test_tls_integration_consolidation() {
        // Verify that TLS tests are accessible through the new integration structure
        // This test ensures the consolidation was successful

        // Test that we can access TLS certificate generation
        use fixtures::tls::EphemeralCertificate;
        let ephemeral_cert = EphemeralCertificate::generate(Some("test-consolidation"));
        assert!(ephemeral_cert.is_ok(), "Should be able to generate ephemeral certificates");

        // Test that we can create TestDatabase instances for TLS
        let mysql_tls = TestDatabase::mysql_tls();
        assert!(mysql_tls.is_tls_enabled(), "MySQL TLS database should have TLS enabled");

        let mariadb_tls = TestDatabase::mariadb_tls();
        assert!(mariadb_tls.is_tls_enabled(), "MariaDB TLS database should have TLS enabled");

        // Test that we can create plain database instances
        let mysql_plain = TestDatabase::mysql();
        assert!(!mysql_plain.is_tls_enabled(), "MySQL plain database should not have TLS enabled");

        let mariadb_plain = TestDatabase::mariadb();
        assert!(!mariadb_plain.is_tls_enabled(), "MariaDB plain database should not have TLS enabled");

        println!("TLS integration test consolidation successful");
    }

    /// Test that we can create test cases
    #[test]
    fn test_test_case_creation() {
        let test_case = TestCase::new("basic_test", "SELECT 1 as test_column")
            .with_format(OutputFormat::Json)
            .with_exit_code(0)
            .with_arg("--verbose")
            .with_env("TEST_VAR", "test_value")
            .with_validation(ValidationRule::RowCount(1));

        assert_eq!(test_case.name, "basic_test");
        assert_eq!(test_case.query, "SELECT 1 as test_column");
        assert_eq!(test_case.expected_format, OutputFormat::Json);
        assert_eq!(test_case.expected_exit_code, 0);
        assert!(test_case.cli_args.contains(&"--verbose".to_string()));
        assert_eq!(test_case.env_vars.get("TEST_VAR"), Some(&"test_value".to_string()));
        assert_eq!(test_case.validation_rules.len(), 1);
    }

    /// Test CLI command builder
    #[test]
    fn test_cli_command_builder() {
        let cmd = GoldDiggerCommand::new()
            .db_url("mysql://test:test@localhost:3306/test")
            .query("SELECT 1")
            .format("json")
            .verbose()
            .expect_exit_code(0);

        assert!(cmd.db_url.is_some());
        assert!(cmd.query.is_some());
        assert!(cmd.format.is_some());
        assert!(cmd.verbose);
        assert_eq!(cmd.expected_exit_code, Some(0));
    }

    /// Test fixture utilities
    #[test]
    fn test_fixture_utilities() {
        // Test sample queries
        let basic_query = SampleQueries::basic_select();
        assert!(!basic_query.is_empty());
        assert!(basic_query.contains("SELECT"));

        let data_types_query = SampleQueries::data_types_query();
        assert!(data_types_query.contains("int_col"));
        assert!(data_types_query.contains("varchar_col"));

        // Test schema utilities
        let basic_table = TestSchema::basic_test_table();
        assert!(basic_table.contains("CREATE TABLE"));
        assert!(basic_table.contains("test_basic"));

        // Test data utilities
        let test_data = TestData::basic_test_data();
        assert!(!test_data.is_empty());
        assert!(test_data[0].contains("INSERT INTO"));
    }
}
