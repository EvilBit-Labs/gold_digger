//! CA certificate loading and PEM parsing.
//!
//! Used by [`super::config::TlsConfig`] when the caller specifies
//! `--tls-ca-file`. Re-exported from [`super`] as `cert_utils` so the
//! pre-split import path (`gold_digger::tls::cert_utils::*`) still
//! resolves for tests and downstream callers.

use super::error::TlsError;
use rustls_pki_types::{CertificateDer, pem::PemObject};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

/// Loads CA certificates from a PEM file
///
/// # Performance Note
/// This function reads the entire file into memory. For very large CA bundles,
/// consider streaming parsing if memory usage becomes an issue.
pub fn load_ca_certificates(
    ca_file_path: &PathBuf,
) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    // Check if file exists
    if !ca_file_path.exists() {
        return Err(TlsError::ca_file_not_found(
            ca_file_path.display().to_string(),
        ));
    }

    // Open and read the file
    let file = File::open(ca_file_path).map_err(|e| {
        TlsError::invalid_ca_format(
            ca_file_path.display().to_string(),
            format!("Cannot read certificate file: {}", e),
        )
    })?;

    let mut reader = BufReader::new(file);

    // Parse PEM certificates using rustls-pki-types
    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_reader_iter(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            TlsError::invalid_ca_format(
                ca_file_path.display().to_string(),
                format!("Failed to parse PEM certificates: {}", e),
            )
        })?;

    if certs.is_empty() {
        return Err(TlsError::invalid_ca_format(
            ca_file_path.display().to_string(),
            "No valid certificates found in file".to_string(),
        ));
    }

    Ok(certs)
}

/// Validates that a certificate file contains valid PEM certificates
pub fn validate_ca_file(ca_file_path: &PathBuf) -> Result<(), TlsError> {
    load_ca_certificates(ca_file_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cert_utils_load_nonexistent_file() {
        let nonexistent_path = PathBuf::from("/nonexistent/ca.pem");
        let result = load_ca_certificates(&nonexistent_path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("CA certificate file not found")
        );
    }

    #[test]
    fn test_cert_utils_load_invalid_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temporary file with invalid content
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "This is not a valid PEM certificate").unwrap();

        let result = load_ca_certificates(&temp_file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid CA certificate format")
        );
    }

    #[test]
    fn test_cert_utils_load_empty_file() {
        use tempfile::NamedTempFile;

        // Create an empty temporary file
        let temp_file = NamedTempFile::new().unwrap();

        let result = load_ca_certificates(&temp_file.path().to_path_buf());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No valid certificates found")
        );
    }

    #[test]
    fn test_cert_utils_validate_ca_file() {
        let nonexistent_path = PathBuf::from("/nonexistent/ca.pem");
        let result = validate_ca_file(&nonexistent_path);
        assert!(result.is_err());
    }
}
