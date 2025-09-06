//! TLS integration tests for Gold Digger
//!
//! This module has been refactored to use the new integration test structure
//! while maintaining backward compatibility for existing CI workflows.
//!
//! The actual TLS tests are now located in tests/integration/tls_tests.rs
//! and use the new TestDatabase abstraction and fixtures system.

mod fixtures;
mod integration;

// The TLS tests are now available through the integration module
// Import is done within test modules as needed

#[cfg(test)]
mod consolidated_tls_tests {
    use super::*;
    use anyhow::Result;
    use fixtures::tls::{CertificateValidator, EphemeralCertificate};
    use integration::{TestDatabase, is_docker_available};

    /// Test that the TLS integration consolidation is working
    #[test]
    fn test_tls_consolidation_works() -> Result<()> {
        // Test certificate generation
        let ephemeral_cert = EphemeralCertificate::generate(Some("consolidation-test"))?;
        CertificateValidator::validate_ephemeral_certificate(&ephemeral_cert)?;

        // Test database type creation
        let mysql_tls = TestDatabase::mysql_tls();
        assert!(mysql_tls.is_tls_enabled());

        let mariadb_plain = TestDatabase::mariadb();
        assert!(!mariadb_plain.is_tls_enabled());

        println!("TLS integration consolidation is working correctly");
        Ok(())
    }

    /// Test Docker availability for container tests
    #[test]
    fn test_docker_availability() {
        let docker_available = is_docker_available();
        println!("Docker available for TLS container tests: {}", docker_available);

        if !docker_available {
            println!("Skipping Docker-dependent TLS tests");
        }
    }

    /// Test TLS configuration compatibility
    #[test]
    fn test_tls_config_compatibility() -> Result<()> {
        // Test that Gold Digger TLS configuration still works
        let config = gold_digger::tls::TlsConfig::new();
        let ssl_opts = config.to_ssl_opts()?;
        assert!(ssl_opts.is_some());

        // Test different TLS modes
        let skip_hostname_config = gold_digger::tls::TlsConfig::with_skip_hostname_verification();
        let skip_hostname_opts = skip_hostname_config.to_ssl_opts()?;
        assert!(skip_hostname_opts.is_some());

        let accept_invalid_config = gold_digger::tls::TlsConfig::with_accept_invalid();
        let accept_invalid_opts = accept_invalid_config.to_ssl_opts()?;
        assert!(accept_invalid_opts.is_some());

        println!("TLS configuration compatibility verified");
        Ok(())
    }
}

// All TLS tests are now available through the integration::tls_tests module
// The tests can be run using the full module path, e.g.:
// cargo test --test tls_integration integration::tls_tests::platform_certificate_tests::test_platform_certificate_validation
//
// This maintains backward compatibility while using the new integration test structure.
