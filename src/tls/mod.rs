//! Rustls-only TLS configuration for MySQL/MariaDB connections.
//!
//! Supports four validation modes selectable via [`TlsConfig`]:
//!   1. Platform defaults with strict hostname + certificate validation.
//!   2. `--insecure-skip-hostname-verify`: trust the CA, skip hostname check.
//!   3. `--allow-invalid-certificate`: accept self-signed or expired certs
//!      (testing only — full MITM exposure).
//!   4. Custom CA bundle via `--tls-ca-file` (PEM format).
//!
//! All errors flow through [`TlsError`], which carries actionable guidance
//! back to the user. No OpenSSL or native-tls — rustls is mandatory.
//!
//! # Module layout
//!
//! The TLS surface is split across several submodules; the public API is
//! re-exported here so callers continue to use `gold_digger::tls::*` paths.
//!
//! - [`error`] — [`TlsError`] enum, constructors, and classifiers.
//! - [`classifier`] — [`rustls::Error`] → [`TlsError`] mapping
//!   ([`TlsError::from_rustls_error`]).
//! - [`config`] — [`TlsConfig`] DTO, [`TlsValidationMode`], CLI adapter.
//! - [`ca`] — CA file loading / PEM parsing (re-exported as `cert_utils`
//!   for backward compatibility with tests that reference
//!   `gold_digger::tls::cert_utils`).
//! - [`pool`] — [`create_tls_connection`] MySQL pool builder and the
//!   [`redact_url`] backward-compat wrapper.

pub mod ca;
pub mod classifier;
pub mod config;
pub mod error;
pub mod pool;

// Public re-exports preserve the `gold_digger::tls::*` import paths used by
// `src/main.rs`, `src/cli.rs`, `src/exit.rs`, and the `tests/` suite.
pub use config::{TlsConfig, TlsValidationMode, tls_config_from_url};
pub use error::TlsError;
pub use pool::{create_tls_connection, redact_url};

/// Backward-compat alias for the CA-loading helpers. Tests and external
/// callers reference `gold_digger::tls::cert_utils::load_ca_certificates`;
/// the implementation now lives in [`ca`] but the old path still works.
pub use ca as cert_utils;
