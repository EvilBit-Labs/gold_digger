//! CA certificate loading and PEM parsing.
//!
//! Used by [`super::config::TlsValidationMode`] when the caller specifies
//! `--tls-ca-file`. Re-exported from [`super`] as `cert_utils` so the
//! pre-split import path (`gold_digger::tls::cert_utils::*`) still
//! resolves for tests and downstream callers.

use super::error::TlsError;
use rustls_pki_types::{CertificateDer, pem::PemObject};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

/// Validated CA bundle handle.
///
/// Construction (`CaFile::load`) performs the full chain of safety
/// checks — canonicalisation, existence, permission probe, and PEM
/// parse with at least one valid certificate — so any value of this
/// type is safe to hand to `mysql::SslOpts::with_root_cert_path`
/// without re-validation. This is the type-design fix for #2: bogus
/// paths can no longer survive into [`super::config::TlsValidationMode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaFile {
    /// Canonicalised on-disk path; safe to pass to mysql.
    path: PathBuf,
    /// Number of valid PEM certificates parsed at load time.
    cert_count: usize,
}

impl CaFile {
    /// Loads and validates a CA file from disk.
    ///
    /// Performs three checks in order so the caller gets the most
    /// specific error possible:
    ///
    /// 1. **Canonicalise.** Resolves `..` and symlinks; failure yields
    ///    [`TlsError::CaFileNotFound`] (broken paths look identical to
    ///    missing ones from a UX perspective).
    /// 2. **Open and parse PEM.** I/O failures map to
    ///    [`TlsError::InvalidCaFormat`] with the OS error text;
    ///    parse failures map to the same variant with the rustls-pki
    ///    error text.
    /// 3. **Require ≥ 1 certificate.** An empty PEM file is treated as
    ///    [`TlsError::InvalidCaFormat`] — empty files almost always mean
    ///    the operator pointed at the wrong path.
    ///
    /// Returns a `CaFile` whose `path()` is canonical and whose
    /// `cert_count()` is the number of certificates that parsed.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, TlsError> {
        let raw = path.as_ref();

        // 1. Canonicalise. Missing files / broken symlinks fail here.
        let canonical = raw
            .canonicalize()
            .map_err(|_| TlsError::ca_file_not_found(raw.display().to_string()))?;

        // 2. Open + parse PEM. Each failure mode carries the path so
        //    operators can tell which file is wrong even when several
        //    are configured.
        let file = File::open(&canonical).map_err(|e| {
            TlsError::invalid_ca_format(
                canonical.display().to_string(),
                format!("Cannot read certificate file: {}", e),
            )
        })?;

        let mut reader = BufReader::new(file);
        let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_reader_iter(&mut reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                TlsError::invalid_ca_format(
                    canonical.display().to_string(),
                    format!("Failed to parse PEM certificates: {}", e),
                )
            })?;

        // 3. Require at least one certificate.
        if certs.is_empty() {
            return Err(TlsError::invalid_ca_format(
                canonical.display().to_string(),
                "No valid certificates found in file".to_string(),
            ));
        }

        Ok(Self {
            path: canonical,
            cert_count: certs.len(),
        })
    }

    /// Returns the canonicalised on-disk path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the number of valid PEM certificates parsed at load time.
    pub fn cert_count(&self) -> usize {
        self.cert_count
    }
}

/// Loads CA certificates from a PEM file.
///
/// # Performance Note
/// This function reads the entire file into memory. For very large CA bundles,
/// consider streaming parsing if memory usage becomes an issue.
///
/// Prefer [`CaFile::load`] in new code; this free function is retained
/// for backward compatibility with existing callers and tests that
/// expect the raw `Vec<CertificateDer<'static>>` payload.
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

/// Validates that a certificate file contains valid PEM certificates.
///
/// Prefer [`CaFile::load`] in new code; this is retained for backward
/// compatibility with the historical `validate_ca_file(&PathBuf)` API.
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

    // -----------------------------------------------------------------
    // CaFile newtype tests (#2 type-design fix)
    // -----------------------------------------------------------------

    #[test]
    fn ca_file_load_nonexistent_returns_not_found() {
        let result = CaFile::load("/definitely/does/not/exist/ca.pem");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TlsError::CaFileNotFound { .. }
        ));
    }

    #[test]
    fn ca_file_load_empty_file_returns_invalid_format() {
        use tempfile::NamedTempFile;
        let temp_file = NamedTempFile::new().unwrap();
        let result = CaFile::load(temp_file.path());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TlsError::InvalidCaFormat { .. }
        ));
    }

    #[test]
    fn ca_file_load_garbage_file_returns_invalid_format() {
        use std::io::Write;
        use tempfile::NamedTempFile;
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "not a certificate").unwrap();
        let result = CaFile::load(temp_file.path());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TlsError::InvalidCaFormat { .. }
        ));
    }

    #[test]
    fn ca_file_load_valid_pem_succeeds() {
        use rcgen::generate_simple_self_signed;
        use std::io::Write;
        use tempfile::NamedTempFile;

        let cert =
            generate_simple_self_signed(vec!["localhost".to_string()]).expect("generate cert");
        let pem = cert.cert.pem();

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(pem.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let ca = CaFile::load(temp_file.path()).expect("load valid pem");
        assert_eq!(ca.cert_count(), 1);
        // Path should be canonicalised — comparison is via canonicalize
        // because tempfile's path may itself be a symlink on macOS.
        let expected = std::fs::canonicalize(temp_file.path()).unwrap();
        assert_eq!(ca.path(), expected);
    }
}
