//! Minimal test for database seeding functionality

use anyhow::Result;
use mysql::prelude::*;
use testcontainers_modules::{mysql::Mysql, testcontainers::runners::SyncRunner};

/// Simple test to verify database seeding works
#[test]
fn test_minimal_seed_data() -> Result<()> {
    // Skip if Docker is not available
    if !is_docker_available() {
        println!("Skipping test: Docker not available");
        return Ok(());
    }

    // Initialize crypto provider for rustls
    gold_digger::init_crypto_provider();

    // Start MySQL container
    let container = Mysql::default().start()?;
    let host_port = container.get_host_port_ipv4(3306)?;
    let connection_url = format!("mysql://root@127.0.0.1:{}/mysql", host_port);

    // Wait for container to be ready
    wait_for_mysql_ready(&connection_url)?;

    // Test database seeding
    seed_test_database(&connection_url)?;

    // Verify seeding worked
    verify_seeded_data(&connection_url)?;

    println!("Database seeding test completed successfully");
    Ok(())
}

/// Check if Docker is available
fn is_docker_available() -> bool {
    std::process::Command::new("docker")
        .arg("version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Wait for MySQL to be ready for connections
fn wait_for_mysql_ready(connection_url: &str) -> Result<()> {
    use std::time::{Duration, Instant};

    let start_time = Instant::now();
    let timeout = Duration::from_secs(60);

    while start_time.elapsed() < timeout {
        if test_mysql_connection(connection_url) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    Err(anyhow::anyhow!("MySQL container failed to become ready within 60 seconds"))
}

/// Test MySQL connection
fn test_mysql_connection(connection_url: &str) -> bool {
    let opts = match mysql::Opts::from_url(connection_url) {
        Ok(opts) => opts,
        Err(_) => return false,
    };

    let pool = match mysql::Pool::new(opts) {
        Ok(pool) => pool,
        Err(_) => return false,
    };

    match pool.get_conn() {
        Ok(mut conn) => matches!(conn.query_first::<i32, _>("SELECT 1"), Ok(Some(1))),
        Err(_) => false,
    }
}

/// Seed the test database with basic schema and data
fn seed_test_database(connection_url: &str) -> Result<()> {
    let opts = mysql::Opts::from_url(connection_url)?;
    let pool = mysql::Pool::new(opts)?;
    let mut conn = pool.get_conn()?;

    // Create a simple test table
    conn.exec_drop(
        "CREATE TABLE IF NOT EXISTS test_basic (
            id INT PRIMARY KEY AUTO_INCREMENT,
            item_name VARCHAR(255),
            table_value INT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        (),
    )?;

    // Insert test data with upsert behavior
    conn.exec_drop(
        "INSERT INTO test_basic (item_name, table_value)
         VALUES ('test1', 100)
         ON DUPLICATE KEY UPDATE table_value = VALUES(table_value)",
        (),
    )?;

    conn.exec_drop(
        "INSERT INTO test_basic (item_name, table_value)
         VALUES ('test2', 200)
         ON DUPLICATE KEY UPDATE table_value = VALUES(table_value)",
        (),
    )?;

    // Create a data types test table
    conn.exec_drop(
        "CREATE TABLE IF NOT EXISTS test_data_types (
            id INT PRIMARY KEY AUTO_INCREMENT,
            varchar_col VARCHAR(255),
            int_col INT,
            decimal_col DECIMAL(10,2),
            date_col DATE,
            json_col JSON
        )",
        (),
    )?;

    // Insert test data with various data types
    conn.exec_drop(
        "INSERT INTO test_data_types (varchar_col, int_col, decimal_col, date_col, json_col)
         VALUES ('Sample text', 42, 99.99, '2024-01-15', '{\"test\": true}')
         ON DUPLICATE KEY UPDATE
         varchar_col = VALUES(varchar_col),
         int_col = VALUES(int_col),
         decimal_col = VALUES(decimal_col),
         date_col = VALUES(date_col),
         json_col = VALUES(json_col)",
        (),
    )?;

    Ok(())
}

/// Verify that the seeded data is present
fn verify_seeded_data(connection_url: &str) -> Result<()> {
    let opts = mysql::Opts::from_url(connection_url)?;
    let pool = mysql::Pool::new(opts)?;
    let mut conn = pool.get_conn()?;

    // Check basic table
    let count: Option<i64> = conn.query_first("SELECT COUNT(*) FROM test_basic")?;
    assert!(count.unwrap_or(0) >= 2, "test_basic should have at least 2 rows");

    // Check data types table
    let count: Option<i64> = conn.query_first("SELECT COUNT(*) FROM test_data_types")?;
    assert!(count.unwrap_or(0) >= 1, "test_data_types should have at least 1 row");

    // Verify specific data
    let item_name: Option<String> = conn.query_first("SELECT item_name FROM test_basic WHERE table_value = 100")?;
    assert_eq!(item_name, Some("test1".to_string()), "Should find test1 with value 100");

    let varchar_value: Option<String> =
        conn.query_first("SELECT varchar_col FROM test_data_types WHERE int_col = 42")?;
    assert_eq!(varchar_value, Some("Sample text".to_string()), "Should find sample text with int_col 42");

    Ok(())
}
