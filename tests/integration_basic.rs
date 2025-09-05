//! Basic integration tests for Gold Digger
//!
//! These tests verify core functionality using real database containers.

mod integration;
mod test_support;

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use tempfile::NamedTempFile;

    use crate::integration::{
        TestDatabase,
        containers::{DatabaseContainer, utils::skip_if_no_docker},
    };
    use crate::test_support::cli::GoldDiggerCommand;

    #[test]
    fn test_basic_mysql_query() -> Result<()> {
        skip_if_no_docker()?;

        let container = DatabaseContainer::new(TestDatabase::MySQL)?;
        container.seed_data()?;

        let temp_file = NamedTempFile::new()?;
        let output_path = temp_file.path();

        let result = GoldDiggerCommand::new()
            .db_url(container.connection_url())
            .query("SELECT * FROM test_data")
            .output(output_path)
            .format("json")
            .execute_success()?;

        assert!(result.is_success());
        assert!(output_path.exists());

        // Verify output contains expected data
        let content = std::fs::read_to_string(output_path)?;
        assert!(content.contains("test1"));

        Ok(())
    }

    #[test]
    fn test_basic_mariadb_query() -> Result<()> {
        skip_if_no_docker()?;

        let container = DatabaseContainer::new(TestDatabase::MariaDB)?;
        container.seed_data()?;

        let temp_file = NamedTempFile::new()?;
        let output_path = temp_file.path();

        let result = GoldDiggerCommand::new()
            .db_url(container.connection_url())
            .query("SELECT COUNT(*) as row_count FROM test_data")
            .output(output_path)
            .format("csv")
            .execute_success()?;

        assert!(result.is_success());

        // Verify CSV output
        let content = std::fs::read_to_string(output_path)?;
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2); // Header + 1 data row
        assert!(lines[0].contains("row_count"));
        assert!(lines[1].contains("3")); // Should have 3 test rows

        Ok(())
    }
}
