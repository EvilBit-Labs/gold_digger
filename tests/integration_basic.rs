//! Basic integration tests for Gold Digger
//!
//! These tests verify core functionality using real database containers.

mod integration;
mod test_support;

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::integration::{
        TestDatabase,
        containers::{DatabaseContainer, utils::skip_if_no_docker},
    };

    #[test]
    fn test_basic_mysql_query() -> Result<()> {
        skip_if_no_docker()?;

        // Test basic container creation and health check
        let container = DatabaseContainer::new(TestDatabase::mysql())?;

        // Verify container is created and healthy
        assert!(container.test_connection());

        // Verify database type
        assert_eq!(container.db_type(), &TestDatabase::mysql());

        // Verify connection URL is generated
        assert!(container.connection_url().contains("mysql://"));
        assert!(container.connection_url().contains("127.0.0.1"));

        // Test seeding data
        container.seed_data()?;

        // Verify we can execute a simple query directly on the container
        container.execute_sql("SELECT COUNT(*) FROM test_data")?;

        Ok(())
    }

    #[test]
    fn test_basic_mariadb_query() -> Result<()> {
        skip_if_no_docker()?;

        // Test basic container creation and health check
        let container = DatabaseContainer::new(TestDatabase::mariadb())?;

        // Verify container is created and healthy
        assert!(container.test_connection());

        // Verify database type
        assert_eq!(container.db_type(), &TestDatabase::mariadb());

        // Verify connection URL is generated
        assert!(container.connection_url().contains("mysql://"));
        assert!(container.connection_url().contains("127.0.0.1"));

        // Test seeding data
        container.seed_data()?;

        // Verify we can execute a simple query directly on the container
        container.execute_sql("SELECT COUNT(*) FROM test_data")?;

        Ok(())
    }

    #[test]
    fn test_container_health_info() -> Result<()> {
        skip_if_no_docker()?;

        let container = DatabaseContainer::new(TestDatabase::mysql())?;

        // Test health info functionality
        let health_info = container.health_info();

        // Verify health info contains expected data
        assert!(!health_info.container_id.is_empty());
        assert_eq!(health_info.db_type, TestDatabase::mysql());
        assert!(health_info.is_healthy);
        assert!(health_info.connection_url_redacted.contains("***"));

        Ok(())
    }

    #[test]
    fn test_database_enum_functionality() {
        // Test TestDatabase enum methods
        let mysql_db = TestDatabase::mysql();
        let mariadb_db = TestDatabase::mariadb();
        let mysql_tls_db = TestDatabase::mysql_tls();
        let mariadb_tls_db = TestDatabase::mariadb_tls();

        // Test name method
        assert_eq!(mysql_db.name(), "mysql");
        assert_eq!(mariadb_db.name(), "mariadb");

        // Test TLS enabled check
        assert!(!mysql_db.is_tls_enabled());
        assert!(!mariadb_db.is_tls_enabled());
        assert!(mysql_tls_db.is_tls_enabled());
        assert!(mariadb_tls_db.is_tls_enabled());

        // Test equality
        assert_eq!(mysql_db, TestDatabase::mysql());
        assert_ne!(mysql_db, mariadb_db);
    }
}
