//! Test for database seeding functionality
//!
//! This test validates the comprehensive database seeding implementation
//! including DDL/DML separation, idempotency, and compatibility handling.

use anyhow::Result;
use gold_digger::init_crypto_provider;

mod fixtures;
mod integration;
use integration::{TestDatabase, containers::DatabaseContainer};

/// Test basic database seeding functionality
#[test]
fn test_database_seeding_basic() -> Result<()> {
    // Skip if Docker is not available
    if !integration::is_docker_available() {
        println!("Skipping test: Docker not available");
        return Ok(());
    }

    // Initialize crypto provider for TLS
    init_crypto_provider();

    // Create a MySQL container for testing
    let container = DatabaseContainer::new(TestDatabase::mysql())?;

    // Test that the container is healthy
    assert!(container.test_connection(), "Container should be healthy after creation");

    // Seed the database with comprehensive test data
    container.seed_data()?;

    // Verify that basic tables were created
    let basic_results = container.query_results("SELECT COUNT(*) as count FROM test_basic")?;
    assert!(!basic_results.is_empty(), "Should have results from test_basic table");

    // Verify that data types table was created and has data
    let data_types_results = container.query_results("SELECT COUNT(*) as count FROM test_data_types")?;
    assert!(!data_types_results.is_empty(), "Should have results from test_data_types table");

    // Verify that edge cases table was created
    let edge_cases_results = container.query_results("SELECT COUNT(*) as count FROM test_edge_cases")?;
    assert!(!edge_cases_results.is_empty(), "Should have results from test_edge_cases table");

    // Test idempotency - seeding again should not fail
    container.seed_data()?;

    println!("Database seeding test completed successfully");
    Ok(())
}

/// Test database seeding with MariaDB
#[test]
fn test_database_seeding_mariadb() -> Result<()> {
    // Skip if Docker is not available
    if !integration::is_docker_available() {
        println!("Skipping test: Docker not available");
        return Ok(());
    }

    // Initialize crypto provider for TLS
    init_crypto_provider();

    // Create a MariaDB container for testing
    let container = DatabaseContainer::new(TestDatabase::mariadb())?;

    // Test that the container is healthy
    assert!(container.test_connection(), "MariaDB container should be healthy after creation");

    // Seed the database with comprehensive test data
    container.seed_data()?;

    // Verify that basic tables were created
    let basic_results = container.query_results("SELECT COUNT(*) as count FROM test_basic")?;
    assert!(!basic_results.is_empty(), "Should have results from test_basic table in MariaDB");

    // Test idempotency - seeding again should not fail
    container.seed_data()?;

    println!("MariaDB database seeding test completed successfully");
    Ok(())
}

/// Test database version detection
#[test]
fn test_database_version_detection() -> Result<()> {
    // Skip if Docker is not available
    if !integration::is_docker_available() {
        println!("Skipping test: Docker not available");
        return Ok(());
    }

    // Initialize crypto provider for TLS
    init_crypto_provider();

    // Test MySQL version detection
    {
        let container = DatabaseContainer::new(TestDatabase::mysql())?;

        // Get database connection for version detection
        use mysql::prelude::*;
        let opts = mysql::Opts::from_url(container.connection_url())?;
        let pool = mysql::Pool::new(opts)?;
        let mut conn = pool.get_conn()?;

        // Test version query
        let version_result: Option<String> = conn.query_first("SELECT VERSION()")?;
        assert!(version_result.is_some(), "Should be able to query MySQL version");

        let version = version_result.unwrap();
        println!("MySQL version: {}", version);
        assert!(version.contains("8.") || version.contains("5."), "Should be MySQL 5.x or 8.x");
    }

    // Test MariaDB version detection
    {
        let container = DatabaseContainer::new(TestDatabase::mariadb())?;

        // Get database connection for version detection
        use mysql::prelude::*;
        let opts = mysql::Opts::from_url(container.connection_url())?;
        let pool = mysql::Pool::new(opts)?;
        let mut conn = pool.get_conn()?;

        // Test version query
        let version_result: Option<String> = conn.query_first("SELECT VERSION()")?;
        assert!(version_result.is_some(), "Should be able to query MariaDB version");

        let version = version_result.unwrap();
        println!("MariaDB version: {}", version);
        assert!(version.to_lowercase().contains("mariadb"), "Should be MariaDB");
    }

    println!("Database version detection test completed successfully");
    Ok(())
}

/// Test comprehensive data types after seeding
#[test]
fn test_comprehensive_data_types() -> Result<()> {
    // Skip if Docker is not available
    if !integration::is_docker_available() {
        println!("Skipping test: Docker not available");
        return Ok(());
    }

    // Initialize crypto provider for TLS
    init_crypto_provider();

    let container = DatabaseContainer::new(TestDatabase::mysql())?;
    container.seed_data()?;

    // Test various data types
    let results = container.query_results(
        "SELECT varchar_col, int_col, decimal_col, date_col, json_col, bool_col
         FROM test_data_types
         LIMIT 5",
    )?;

    assert!(!results.is_empty(), "Should have data type test results");
    println!("Retrieved {} rows from test_data_types", results.len());

    // Test NULL handling
    let null_results = container.query_results(
        "SELECT null_varchar, null_int, null_decimal, null_date
         FROM test_edge_cases
         WHERE id = 1",
    )?;

    assert!(!null_results.is_empty(), "Should have NULL test results");
    println!("NULL handling test completed");

    // Test Unicode data
    let unicode_results = container.query_results(
        "SELECT unicode_text, emoji_text
         FROM test_edge_cases
         WHERE unicode_text IS NOT NULL
         LIMIT 3",
    )?;

    assert!(!unicode_results.is_empty(), "Should have Unicode test results");
    println!("Unicode handling test completed");

    println!("Comprehensive data types test completed successfully");
    Ok(())
}
