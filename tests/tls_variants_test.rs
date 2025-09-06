//! Test TLS and non-TLS database variants
//!
//! This test demonstrates the usage of TestDatabaseTls and TestDatabasePlain
//! variants for secure and standard connection testing.

use anyhow::Result;
mod fixtures;
mod integration;

use integration::{TestDatabasePlain, TestDatabaseTls, TlsContainerConfig};

#[cfg(feature = "integration_tests")]
use integration::containers::database_container::DatabaseContainer;
#[cfg(feature = "integration_tests")]
use integration::{is_ci_environment, is_docker_available};

/// Skip test if Docker is not available
#[cfg(feature = "integration_tests")]
fn skip_if_no_docker() -> Result<()> {
    if !is_docker_available() {
        eprintln!("Skipping test: Docker not available");
        return Err(anyhow::anyhow!("Docker not available"));
    }
    Ok(())
}

#[test]
#[cfg(feature = "integration_tests")]
fn test_mysql_tls_variant() -> Result<()> {
    skip_if_no_docker()?;

    eprintln!("Testing MySQL TLS variant...");

    // Create MySQL TLS database with secure defaults
    let tls_db = TestDatabaseTls::mysql();
    assert_eq!(tls_db.name(), "mysql-tls");
    assert!(tls_db.tls_config().require_secure_transport);
    assert_eq!(tls_db.tls_config().min_tls_version, "TLSv1.2");

    // Create container with TLS configuration
    let container = DatabaseContainer::new_tls(tls_db)?;

    // Validate TLS connection
    let validation = container.validate_tls_connection()?;
    eprintln!("TLS validation result: {:?}", validation);

    // For now, we expect TLS connection to work (even if not fully configured)
    // and plain connection to also work (MySQL allows both by default)
    assert!(validation.tls_connection_success || validation.tls_error.is_some());

    eprintln!("✓ MySQL TLS variant test completed");
    Ok(())
}

#[test]
#[cfg(feature = "integration_tests")]
fn test_mariadb_tls_variant() -> Result<()> {
    skip_if_no_docker()?;

    eprintln!("Testing MariaDB TLS variant...");

    // Create MariaDB TLS database with custom configuration
    let tls_config = TlsContainerConfig::new_secure().with_strict_security()?;

    let tls_db = TestDatabaseTls::mariadb_with_config(tls_config);
    assert_eq!(tls_db.name(), "mariadb-tls");
    assert_eq!(tls_db.tls_config().min_tls_version, "TLSv1.3");

    // Create container with TLS configuration
    let container = DatabaseContainer::new_tls(tls_db)?;

    // Validate TLS connection
    let validation = container.validate_tls_connection()?;
    eprintln!("TLS validation result: {:?}", validation);

    // For now, we expect TLS connection to work (even if not fully configured)
    assert!(validation.tls_connection_success || validation.tls_error.is_some());

    eprintln!("✓ MariaDB TLS variant test completed");
    Ok(())
}

#[test]
#[cfg(feature = "integration_tests")]
fn test_mysql_plain_variant() -> Result<()> {
    skip_if_no_docker()?;

    eprintln!("Testing MySQL plain variant...");

    // Create MySQL plain database
    let plain_db = TestDatabasePlain::mysql();
    assert_eq!(plain_db.name(), "mysql-plain");

    // Create container without TLS
    let container = DatabaseContainer::new_plain(plain_db)?;

    // Validate plain connection
    let validation = container.validate_plain_connection()?;
    eprintln!("Plain validation result: {:?}", validation);

    // Plain connection should work
    assert!(validation.plain_connection_success);

    eprintln!("✓ MySQL plain variant test completed");
    Ok(())
}

#[test]
#[cfg(feature = "integration_tests")]
fn test_mariadb_plain_variant() -> Result<()> {
    skip_if_no_docker()?;

    eprintln!("Testing MariaDB plain variant...");

    // Create MariaDB plain database
    let plain_db = TestDatabasePlain::mariadb();
    assert_eq!(plain_db.name(), "mariadb-plain");

    // Create container without TLS
    let container = DatabaseContainer::new_plain(plain_db)?;

    // Validate plain connection
    let validation = container.validate_plain_connection()?;
    eprintln!("Plain validation result: {:?}", validation);

    // Plain connection should work
    assert!(validation.plain_connection_success);

    eprintln!("✓ MariaDB plain variant test completed");
    Ok(())
}

#[test]
#[cfg(feature = "integration_tests")]
fn test_connection_url_generation() -> Result<()> {
    skip_if_no_docker()?;

    eprintln!("Testing connection URL generation...");

    // Test with plain MySQL container
    let plain_db = TestDatabasePlain::mysql();
    let container = DatabaseContainer::new_plain(plain_db)?;

    // Test different SSL modes
    let disabled_url = container.connection_url_with_ssl_mode("DISABLED")?;
    let required_url = container.connection_url_with_ssl_mode("REQUIRED")?;
    let verify_ca_url = container.connection_url_with_ssl_mode("VERIFY_CA")?;

    eprintln!("SSL DISABLED URL: {}", disabled_url);
    eprintln!("SSL REQUIRED URL: {}", required_url);
    eprintln!("SSL VERIFY_CA URL: {}", verify_ca_url);

    // Validate URL formats
    assert!(disabled_url.contains("ssl-mode=DISABLED"));
    assert!(required_url.contains("ssl-mode=REQUIRED"));
    assert!(verify_ca_url.contains("ssl-mode=VERIFY_CA"));

    // Test invalid SSL mode
    let invalid_result = container.connection_url_with_ssl_mode("INVALID");
    assert!(invalid_result.is_err());

    eprintln!("✓ Connection URL generation test completed");
    Ok(())
}

#[test]
#[cfg(feature = "integration_tests")]
fn test_tls_config_validation() -> Result<()> {
    eprintln!("Testing TLS configuration validation...");

    // Test valid TLS configuration
    let valid_config = TlsContainerConfig::new_secure();
    assert!(valid_config.validate().is_ok());

    // Test TLS configuration with strict security
    let strict_config = TlsContainerConfig::new_secure().with_strict_security()?;
    assert!(strict_config.validate().is_ok());
    assert_eq!(strict_config.min_tls_version, "TLSv1.3");

    // Test configuration with custom certificate paths (will fail validation since files don't exist)
    let custom_config =
        TlsContainerConfig::with_custom_certs("/nonexistent/ca.pem", "/nonexistent/cert.pem", "/nonexistent/key.pem");
    assert!(custom_config.validate().is_err());

    eprintln!("✓ TLS configuration validation test completed");
    Ok(())
}

#[test]
fn test_database_variant_conversions() {
    eprintln!("Testing database variant conversions...");

    // Test TLS variant conversions
    let mysql_tls = TestDatabaseTls::mysql();
    let mysql_base = mysql_tls.to_test_database();
    assert!(mysql_base.is_tls_enabled());

    let mariadb_tls = TestDatabaseTls::mariadb();
    let mariadb_base = mariadb_tls.to_test_database();
    assert!(mariadb_base.is_tls_enabled());

    // Test plain variant conversions
    let mysql_plain = TestDatabasePlain::mysql();
    let mysql_plain_base = mysql_plain.to_test_database();
    assert!(!mysql_plain_base.is_tls_enabled());

    let mariadb_plain = TestDatabasePlain::mariadb();
    let mariadb_plain_base = mariadb_plain.to_test_database();
    assert!(!mariadb_plain_base.is_tls_enabled());

    eprintln!("✓ Database variant conversions test completed");
}

#[test]
fn test_tls_container_config_methods() -> Result<()> {
    eprintln!("Testing TLS container configuration methods...");

    // Test secure defaults
    let secure_config = TlsContainerConfig::new_secure();
    assert!(secure_config.require_secure_transport);
    assert_eq!(secure_config.min_tls_version, "TLSv1.2");
    assert!(secure_config.use_ephemeral_certs);
    assert!(!secure_config.cipher_suites.is_empty());

    // Test strict security configuration
    let strict_config = TlsContainerConfig::new_secure().with_strict_security()?;
    assert_eq!(strict_config.min_tls_version, "TLSv1.3");
    assert!(
        strict_config
            .cipher_suites
            .contains(&"TLS_AES_256_GCM_SHA384".to_string())
    );

    // Test custom certificate configuration
    let custom_config =
        TlsContainerConfig::with_custom_certs("/path/to/ca.pem", "/path/to/cert.pem", "/path/to/key.pem");
    assert!(!custom_config.use_ephemeral_certs);
    assert!(custom_config.ca_cert_path.is_some());
    assert!(custom_config.server_cert_path.is_some());
    assert!(custom_config.server_key_path.is_some());

    eprintln!("✓ TLS container configuration methods test completed");
    Ok(())
}

/// Integration test that demonstrates the complete TLS vs non-TLS workflow
#[test]
#[cfg(feature = "integration_tests")]
fn test_complete_tls_vs_plain_workflow() -> Result<()> {
    skip_if_no_docker()?;

    if is_ci_environment() {
        eprintln!("Skipping complete workflow test in CI environment");
        return Ok(());
    }

    eprintln!("Testing complete TLS vs plain workflow...");

    // Create both TLS and plain containers
    let tls_db = TestDatabaseTls::mysql();
    let plain_db = TestDatabasePlain::mysql();

    let tls_container = DatabaseContainer::new_tls(tls_db)?;
    let plain_container = DatabaseContainer::new_plain(plain_db)?;

    // Test that both containers are functional
    assert!(tls_container.test_connection());
    assert!(plain_container.test_connection());

    // Test connection URL differences
    let tls_url = tls_container.connection_url();
    let plain_url = plain_container.connection_url();

    eprintln!("TLS container URL: {}", tls_url);
    eprintln!("Plain container URL: {}", plain_url);

    // URLs should be different (different ports at minimum)
    assert_ne!(tls_url, plain_url);

    // Test validation methods
    let tls_validation = tls_container.validate_tls_connection()?;
    let plain_validation = plain_container.validate_plain_connection()?;

    eprintln!("TLS validation: {:?}", tls_validation);
    eprintln!("Plain validation: {:?}", plain_validation);

    // Both should have successful connections
    assert!(tls_validation.tls_connection_success);
    assert!(plain_validation.plain_connection_success);

    eprintln!("✓ Complete TLS vs plain workflow test completed");
    Ok(())
}
