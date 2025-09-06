//! Test for database seeding functionality
//!
//! This test verifies that the seed_data method works correctly with
//! both MySQL and MariaDB containers.

use anyhow::Result;
use gold_digger::init_crypto_provider;

mod fixtures;
mod integration;
use integration::TestDatabase;
use integration::containers::database_container::DatabaseContainer;

#[test]
fn test_seed_data_mysql() -> Result<()> {
    // Skip if Docker is not available
    if !integration::is_docker_available() {
        println!("Skipping test: Docker not available");
        return Ok(());
    }

    // Initialize crypto provider for rustls
    init_crypto_provider();

    // Create MySQL container
    let db_type = TestDatabase::mysql();
    let container = DatabaseContainer::new(db_type)?;

    // Test that container is ready
    assert!(container.test_connection(), "Container should be ready for connections");

    // Seed the database
    container.seed_data()?;

    println!("Database seeding completed successfully for MySQL");
    Ok(())
}

#[test]
fn test_seed_data_mariadb() -> Result<()> {
    // Skip if Docker is not available
    if !integration::is_docker_available() {
        println!("Skipping test: Docker not available");
        return Ok(());
    }

    // Initialize crypto provider for rustls
    init_crypto_provider();

    // Create MariaDB container
    let db_type = TestDatabase::mariadb();
    let container = DatabaseContainer::new(db_type)?;

    // Test that container is ready
    assert!(container.test_connection(), "Container should be ready for connections");

    // Seed the database
    container.seed_data()?;

    println!("Database seeding completed successfully for MariaDB");
    Ok(())
}
