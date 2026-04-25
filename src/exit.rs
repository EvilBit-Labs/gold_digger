//! Exit-code contract for the `gold_digger` binary (0-5).
//!
//! The constants below define the public exit-code surface:
//! `EXIT_SUCCESS=0`, `EXIT_NO_ROWS=1`, `EXIT_CONFIG_ERROR=2`,
//! `EXIT_DB_AUTH_ERROR=3`, `EXIT_QUERY_ERROR=4`, `EXIT_IO_ERROR=5`.
//!
//! Two classification paths feed the exit code:
//!   1. **Typed path (preferred)** — callers construct a [`GoldDiggerError`]
//!      variant and pass it through `anyhow::Error`. [`map_error_to_exit_code`]
//!      downcasts and reads the variant's `exit_code()` directly. This path
//!      is stable: refactoring an error message cannot shift the exit code.
//!   2. **Substring fallback** — for legacy untyped `anyhow!(...)` errors,
//!      the lowercased error string is matched against keyword sets. This
//!      path is brittle (todo #017 tracks replacing the in-tls.rs classifier;
//!      remaining sites migrate per #031/#165). Once all construction sites
//!      use [`GoldDiggerError`], the substring path will be deleted.

use anyhow::Error;
use std::path::PathBuf;
use std::process;
use thiserror::Error as ThisError;

use crate::tls::TlsError;

/// Exit code constants as defined in the product specification
pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_NO_ROWS: i32 = 1;
pub const EXIT_CONFIG_ERROR: i32 = 2;
pub const EXIT_DB_AUTH_ERROR: i32 = 3;
pub const EXIT_QUERY_ERROR: i32 = 4;
pub const EXIT_IO_ERROR: i32 = 5;

/// Typed configuration error sub-enum (todo: type-design #3).
///
/// Splits the previously-unstructured `Config(String)` payload into
/// named variants so error-origin sites can construct typed values
/// instead of free-form strings, and tests can pattern-match the
/// variant rather than asserting on substrings of the rendered message.
///
/// `Display` strings are stable: existing tests that assert
/// `error.to_string().contains("Missing database URL")` continue to pass
/// because each variant emits the same human-readable text the
/// previous `Config(String)` carried.
#[derive(ThisError, Debug)]
pub enum ConfigError {
    /// Neither `--db-url` CLI flag nor `DATABASE_URL` env var was supplied.
    #[error("Missing database URL. Provide --db-url or set DATABASE_URL environment variable")]
    MissingDbUrl,

    /// Neither `--query` / `--query-file` flag nor `DATABASE_QUERY` env
    /// var was supplied.
    #[error(
        "Missing database query. Provide --query, --query-file, or set DATABASE_QUERY environment variable"
    )]
    MissingQuery,

    /// Neither `--output` flag nor `OUTPUT_FILE` env var was supplied.
    #[error("Missing output file. Provide --output or set OUTPUT_FILE environment variable")]
    MissingOutputFile,

    /// `--query-file` rejected by a path-safety guard (extension
    /// deny-list, size cap, format-detection).
    #[error("Invalid query file {path}: {reason}")]
    InvalidQueryFile { path: PathBuf, reason: String },

    /// Two or more mutually-exclusive CLI flags were supplied at once.
    /// Migrated from `TlsError::MutuallyExclusiveFlags` so the routing
    /// is config-class (exit 2), not TLS-class.
    #[error("Mutually exclusive flags provided: {flags}. Use only one option")]
    MutuallyExclusiveFlags { flags: String },

    /// Output target already exists and `--force` was not supplied.
    #[error("Output file already exists: {path}. Pass --force to overwrite.")]
    OutputExists { path: PathBuf },

    /// Format-resolution rejection (unknown extension and no `--format`).
    #[error("{0}")]
    UnresolvableFormat(String),

    /// Catch-all escape hatch for one-off config errors that don't yet
    /// warrant a dedicated variant.
    #[error("{0}")]
    Other(String),
}

/// Typed application error with a stable mapping to the binary's exit-code
/// contract. Constructors at error origin sites (config resolvers, query
/// execution, output writes) should prefer this enum over `anyhow!(...)` so
/// the exit code does not depend on error-message text.
///
/// `From` impls wrap common foreign error types into the `Io`, `Tls`, and
/// `Config` variants so the `?` operator works without manual mapping.
/// `anyhow::Error` preserves the underlying type, so
/// [`map_error_to_exit_code`] can downcast even when the error has been
/// chained with `.context(...)`.
#[derive(ThisError, Debug)]
pub enum GoldDiggerError {
    /// Query executed successfully but returned no rows. Maps to exit 1
    /// unless `--allow-empty` upgrades the disposition to success.
    #[error("query returned no rows")]
    NoRows,

    /// Configuration error — missing required argument, mutually exclusive
    /// flags, malformed value. Maps to exit 2.
    ///
    /// Wraps a typed [`ConfigError`]; see that enum for the per-cause
    /// variants. Use [`ConfigError`] constructors directly at error sites
    /// so the message text becomes a function of the variant rather than
    /// a free-form string passed in by the caller.
    #[error("configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Database authentication or connection-establishment failure (wrong
    /// credentials, unreachable host, TLS handshake failure when the
    /// failure is not a CA/cert validation issue). Maps to exit 3.
    #[error("database auth/connection error: {0}")]
    DbAuth(String),

    /// SQL execution failure (bad syntax, missing table, type-conversion
    /// failure during row processing). Maps to exit 4.
    #[error("query error: {0}")]
    Query(String),

    /// Filesystem or process I/O failure (output file not writable, query
    /// file unreadable). Maps to exit 5.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// TLS configuration or runtime error. Wraps [`TlsError`]; maps to
    /// exit 3 (DB auth) by default since most TLS failures present at
    /// connection time. CA-file-not-found is a config error (exit 2) but
    /// is currently constructed at the `TlsError` layer — see
    /// [`Self::exit_code`] for the per-variant override.
    #[error("TLS error: {0}")]
    Tls(#[from] TlsError),
}

impl GoldDiggerError {
    /// Returns the exit code for this error variant per the public 0-5
    /// contract documented at module level.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::NoRows => EXIT_NO_ROWS,
            Self::Config(_) => EXIT_CONFIG_ERROR,
            Self::DbAuth(_) => EXIT_DB_AUTH_ERROR,
            Self::Query(_) => EXIT_QUERY_ERROR,
            Self::Io(_) => EXIT_IO_ERROR,
            Self::Tls(tls) => tls_exit_code(tls),
        }
    }
}

/// Maps a [`TlsError`] variant to the public exit-code contract. CA-file
/// problems are config errors (the user supplied a bad path / bad PEM);
/// everything else surfaces as a connection/auth failure.
///
/// `MutuallyExclusiveFlags` is also routed to `EXIT_CONFIG_ERROR` for
/// backward compatibility with the legacy [`TlsError`] variant; new code
/// SHOULD construct [`ConfigError::MutuallyExclusiveFlags`] (which routes
/// through [`GoldDiggerError::Config`] → [`EXIT_CONFIG_ERROR`]) so the
/// exit code is established by the typed `ConfigError` enum rather than
/// a TLS-specific special case.
fn tls_exit_code(error: &TlsError) -> i32 {
    match error {
        TlsError::CaFileNotFound { .. }
        | TlsError::InvalidCaFormat { .. }
        | TlsError::MutuallyExclusiveFlags { .. } => EXIT_CONFIG_ERROR,
        _ => EXIT_DB_AUTH_ERROR,
    }
}

/// Maps an error to the appropriate exit code and exits the process
///
/// # Arguments
///
/// * `error` - The error to map to an exit code
/// * `context` - Optional context message to log before exiting
///
/// This function never returns as it calls `process::exit`
pub fn exit_with_error(error: Error, context: Option<&str>) -> ! {
    let exit_code = map_error_to_exit_code(&error);

    // Render the full anyhow chain so context layers (e.g. the
    // `.context("TLS configuration error")` wrapping a typed `TlsError`
    // payload added in connection.rs after CRITICAL #5) surface to the
    // user — `to_string()` alone shows only the topmost message and
    // hides the underlying typed error's Display text.
    //
    // Format: `outer: middle: inner` (joined with `: `), matching the
    // convention `anyhow` itself uses for chain rendering.
    let error_msg = error
        .chain()
        .map(|cause| cause.to_string())
        .collect::<Vec<_>>()
        .join(": ");

    // Log the error with context if provided. Routed through
    // `tracing::error!` so the subscriber (installed by `main()` before
    // any work begins) applies level filtering, formatting, and future
    // redaction layers. Errors emit at ERROR level, so they fire even
    // under `--quiet` (the documented contract: quiet suppresses
    // everything except errors).
    if let Some(ctx) = context {
        tracing::error!("{}: {}", ctx, error_msg);
    } else {
        tracing::error!("{}", error_msg);
    }

    process::exit(exit_code);
}

/// Maps an error to the appropriate exit code without exiting.
///
/// # WARNING: error-message text is part of the public API
///
/// The substring-fallback path (reached only when no typed
/// [`GoldDiggerError`], [`TlsError`], or [`std::io::Error`] can be
/// downcast from the error chain) classifies by keyword match on
/// `error.to_string().to_lowercase()`. That makes the **exact text** of
/// un-migrated `anyhow!(...)` call sites a versioned contract — rewording
/// a `bail!("Missing database URL")` message can silently shift the exit
/// code from 2 to 4, breaking CI/automation that switches on exit codes.
///
/// New error-origin sites SHOULD construct a [`GoldDiggerError`] variant
/// so the typed downcast path handles classification. Once all sites are
/// typed the substring matcher can be deleted (tracked under todos
/// #017, #031, #165).
///
/// # Arguments
///
/// * `error` - The error to map to an exit code
///
/// # Returns
///
/// The appropriate exit code for the given error
pub fn map_error_to_exit_code(error: &Error) -> i32 {
    // Typed path (preferred): walk the anyhow chain looking for a
    // `GoldDiggerError` or a directly-wrapped `TlsError` / `io::Error`.
    // This is stable against error-message refactors.
    for cause in error.chain() {
        if let Some(typed) = cause.downcast_ref::<GoldDiggerError>() {
            return typed.exit_code();
        }
        if let Some(tls) = cause.downcast_ref::<TlsError>() {
            return tls_exit_code(tls);
        }
        if cause.downcast_ref::<std::io::Error>().is_some() {
            return EXIT_IO_ERROR;
        }
    }

    // Substring fallback (legacy): used for untyped `anyhow!(...)` errors
    // produced by sites that have not yet migrated to `GoldDiggerError`.
    // Slated for removal once all construction sites are typed
    // (see todos #017, #031, #165).
    //
    // The `to_lowercase()` + repeated `contains(..)` pattern is an
    // intentional cold-path design choice (todo #072). This branch only
    // executes when the typed downcast above failed, which on the
    // happy-path is never; on the error path it runs exactly once per
    // process before `process::exit`. The architectural fix is the
    // typed classifier ([C2] / `GoldDiggerError`), not perf tuning
    // here — micro-optimising the substring matcher would just delay
    // its eventual removal.
    let error_string = error.to_string().to_lowercase();

    // Check for specific error patterns
    if error_string.contains("no records found") || error_string.contains("no rows") {
        return EXIT_NO_ROWS;
    }

    if error_string.contains("missing")
        || (error_string.contains("invalid")
            && !error_string.contains("invalid certificate format")
            && !error_string.contains("type conversion"))
        || error_string.contains("configuration")
        || error_string.contains("mutually exclusive")
        || error_string.contains("tls feature not enabled")
        || error_string.contains("certificate file not found")
    {
        return EXIT_CONFIG_ERROR;
    }

    if error_string.contains("access denied")
        || error_string.contains("authentication")
        || error_string.contains("connection")
        || error_string.contains("tls connection failed")
        || error_string.contains("tls handshake failed")
        || error_string.contains("certificate validation failed")
        || error_string.contains("unsupported tls version")
        || error_string.contains("mysql")
            && (error_string.contains("auth") || error_string.contains("connect"))
    {
        return EXIT_DB_AUTH_ERROR;
    }

    if error_string.contains("query")
        || error_string.contains("sql")
        || error_string.contains("syntax")
        || error_string.contains("type conversion")
        || error_string.contains("from_value")
    {
        return EXIT_QUERY_ERROR;
    }

    if error_string.contains("file")
        || error_string.contains("io")
        || error_string.contains("read")
        || error_string.contains("write")
        || error_string.contains("permission")
        || error_string.contains("invalid certificate format")
    {
        return EXIT_IO_ERROR;
    }

    // Default to query error for unknown errors
    EXIT_QUERY_ERROR
}

/// Exits with success code (0)
///
/// # Arguments
///
/// * `message` - Optional success message to print before exiting
pub fn exit_success(message: Option<&str>) -> ! {
    if let Some(msg) = message {
        tracing::info!("{}", msg);
    }
    process::exit(EXIT_SUCCESS);
}

/// Exits with no rows code (1)
///
/// # Arguments
///
/// * `message` - Optional message to print before exiting
pub fn exit_no_rows(message: Option<&str>) -> ! {
    if let Some(msg) = message {
        // Empty-result-set is a warning, not an error: the query ran,
        // it just didn't match anything. Emitted at WARN so it survives
        // the default level filter but is suppressed under `--quiet`.
        tracing::warn!("{}", msg);
    }
    process::exit(EXIT_NO_ROWS);
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn test_map_error_to_exit_code_no_rows() {
        let error = anyhow!("No records found in database");
        assert_eq!(map_error_to_exit_code(&error), EXIT_NO_ROWS);

        let error = anyhow!("Query returned no rows");
        assert_eq!(map_error_to_exit_code(&error), EXIT_NO_ROWS);
    }

    #[test]
    fn test_map_error_to_exit_code_config() {
        let error = anyhow!("Missing database URL");
        assert_eq!(map_error_to_exit_code(&error), EXIT_CONFIG_ERROR);

        let error = anyhow!("Invalid configuration");
        assert_eq!(map_error_to_exit_code(&error), EXIT_CONFIG_ERROR);

        let error = anyhow!("Mutually exclusive flags");
        assert_eq!(map_error_to_exit_code(&error), EXIT_CONFIG_ERROR);

        let error = anyhow!("TLS feature not enabled");
        assert_eq!(map_error_to_exit_code(&error), EXIT_CONFIG_ERROR);

        let error = anyhow!("Certificate file not found");
        assert_eq!(map_error_to_exit_code(&error), EXIT_CONFIG_ERROR);
    }

    #[test]
    fn test_map_error_to_exit_code_db_auth() {
        let error = anyhow!("Access denied for user");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);

        let error = anyhow!("Authentication failed");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);

        let error = anyhow!("Connection refused");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);

        let error = anyhow!("MySQL authentication error");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);

        let error = anyhow!("TLS connection failed");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);

        let error = anyhow!("TLS handshake failed");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);

        let error = anyhow!("Certificate validation failed");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);

        let error = anyhow!("Unsupported TLS version");
        assert_eq!(map_error_to_exit_code(&error), EXIT_DB_AUTH_ERROR);
    }

    #[test]
    fn test_map_error_to_exit_code_query() {
        let error = anyhow!("Query execution failed");
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

        let error = anyhow!("SQL syntax error");
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

        let error = anyhow!("Type conversion error");
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

        let error = anyhow!("from_value error");
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

        // Test specific type conversion errors from our implementation
        let error = anyhow!("Type conversion error: Invalid month value 13 in date");
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);

        let error = anyhow!("Type conversion failed during row processing");
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);
    }

    #[test]
    fn test_map_error_to_exit_code_io() {
        let error = anyhow!("File not found");
        assert_eq!(map_error_to_exit_code(&error), EXIT_IO_ERROR);

        let error = anyhow!("IO error occurred");
        assert_eq!(map_error_to_exit_code(&error), EXIT_IO_ERROR);

        let error = anyhow!("Permission denied");
        assert_eq!(map_error_to_exit_code(&error), EXIT_IO_ERROR);

        let error = anyhow!("Failed to read file");
        assert_eq!(map_error_to_exit_code(&error), EXIT_IO_ERROR);

        let error = anyhow!("Failed to write file");
        assert_eq!(map_error_to_exit_code(&error), EXIT_IO_ERROR);

        let error = anyhow!("Invalid certificate format");
        assert_eq!(map_error_to_exit_code(&error), EXIT_IO_ERROR);
    }

    #[test]
    fn test_map_error_to_exit_code_default() {
        let error = anyhow!("Unknown error occurred");
        assert_eq!(map_error_to_exit_code(&error), EXIT_QUERY_ERROR);
    }

    #[test]
    fn test_exit_code_constants() {
        assert_eq!(EXIT_SUCCESS, 0);
        assert_eq!(EXIT_NO_ROWS, 1);
        assert_eq!(EXIT_CONFIG_ERROR, 2);
        assert_eq!(EXIT_DB_AUTH_ERROR, 3);
        assert_eq!(EXIT_QUERY_ERROR, 4);
        assert_eq!(EXIT_IO_ERROR, 5);
    }

    // ---------------------------------------------------------------------
    // Typed-path tests for `GoldDiggerError`. Unlike the substring tests
    // above (which assert that the matcher matches its own keyword set),
    // these tests verify the *contract* — every variant maps to a stable
    // exit code regardless of the message text inside it.
    // ---------------------------------------------------------------------

    #[test]
    fn test_typed_error_exit_codes_direct() {
        assert_eq!(GoldDiggerError::NoRows.exit_code(), EXIT_NO_ROWS);
        assert_eq!(
            GoldDiggerError::Config(ConfigError::Other("anything".into())).exit_code(),
            EXIT_CONFIG_ERROR
        );
        assert_eq!(
            GoldDiggerError::DbAuth("any text".into()).exit_code(),
            EXIT_DB_AUTH_ERROR
        );
        assert_eq!(
            GoldDiggerError::Query("any wording".into()).exit_code(),
            EXIT_QUERY_ERROR
        );
        let io_err = std::io::Error::other("disk full");
        assert_eq!(GoldDiggerError::Io(io_err).exit_code(), EXIT_IO_ERROR);
    }

    #[test]
    fn test_typed_error_through_anyhow_chain() {
        // Construct a GoldDiggerError, wrap it in anyhow, add layers of
        // context — the classifier should still find the typed variant.
        let typed = GoldDiggerError::Config(ConfigError::MissingDbUrl);
        let anyhow_err: Error = typed.into();
        assert_eq!(map_error_to_exit_code(&anyhow_err), EXIT_CONFIG_ERROR);

        let with_context = anyhow_err.context("while resolving CLI arguments");
        assert_eq!(map_error_to_exit_code(&with_context), EXIT_CONFIG_ERROR);
    }

    #[test]
    fn test_config_error_display_text_unchanged() {
        // Each variant's Display string must remain the same as the
        // legacy `Config(String)` payload it replaces — historical tests
        // assert on the rendered message text.
        assert!(
            ConfigError::MissingDbUrl
                .to_string()
                .contains("Missing database URL")
        );
        assert!(
            ConfigError::MissingQuery
                .to_string()
                .contains("Missing database query")
        );
        assert!(
            ConfigError::MissingOutputFile
                .to_string()
                .contains("Missing output file")
        );
        let path: PathBuf = "/tmp/out.csv".into();
        assert!(
            ConfigError::OutputExists { path: path.clone() }
                .to_string()
                .contains("Output file already exists")
        );
        assert!(
            ConfigError::OutputExists { path }
                .to_string()
                .contains("--force to overwrite")
        );
        assert!(
            ConfigError::MutuallyExclusiveFlags {
                flags: "--a, --b".into()
            }
            .to_string()
            .contains("Mutually exclusive flags")
        );
    }

    #[test]
    fn test_config_error_routes_to_config_exit_code() {
        // Every ConfigError variant must route through GoldDiggerError::Config
        // to EXIT_CONFIG_ERROR (2), regardless of the variant's payload.
        for variant in [
            GoldDiggerError::Config(ConfigError::MissingDbUrl),
            GoldDiggerError::Config(ConfigError::MissingQuery),
            GoldDiggerError::Config(ConfigError::MissingOutputFile),
            GoldDiggerError::Config(ConfigError::InvalidQueryFile {
                path: "/x".into(),
                reason: "bad".into(),
            }),
            GoldDiggerError::Config(ConfigError::MutuallyExclusiveFlags {
                flags: "--a, --b".into(),
            }),
            GoldDiggerError::Config(ConfigError::OutputExists { path: "/x".into() }),
            GoldDiggerError::Config(ConfigError::UnresolvableFormat("x".into())),
            GoldDiggerError::Config(ConfigError::Other("anything".into())),
        ] {
            assert_eq!(variant.exit_code(), EXIT_CONFIG_ERROR);
        }
    }

    #[test]
    fn test_typed_path_overrides_substring_match() {
        // Typed Query error whose message would otherwise look like an
        // I/O failure to the substring matcher. Typed path must win.
        let typed =
            GoldDiggerError::Query("permission denied while reading file from query result".into());
        let anyhow_err: Error = typed.into();
        assert_eq!(map_error_to_exit_code(&anyhow_err), EXIT_QUERY_ERROR);
    }

    #[test]
    fn test_io_error_downcast_path() {
        // A bare std::io::Error wrapped in anyhow should map to EXIT_IO_ERROR
        // via downcast, regardless of message text.
        let io_err = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "cannot access something with the word 'authentication' in it",
        );
        let anyhow_err: Error = io_err.into();
        assert_eq!(map_error_to_exit_code(&anyhow_err), EXIT_IO_ERROR);
    }

    #[test]
    fn test_no_rows_variant_overrides_default_query_classification() {
        let typed = GoldDiggerError::NoRows;
        let anyhow_err: Error = typed.into();
        assert_eq!(map_error_to_exit_code(&anyhow_err), EXIT_NO_ROWS);
    }

    // ---------------------------------------------------------------------
    // Property tests (todo #032). Two invariants:
    //   1. The substring classifier never panics and always returns a code in
    //      [EXIT_NO_ROWS..=EXIT_IO_ERROR] (i.e. 1..=5; never 0, never -1, never
    //      anything outside the documented public contract).
    //   2. The typed `GoldDiggerError` variants are wording-stable: any random
    //      string passed into `Config(..)` always produces EXIT_CONFIG_ERROR,
    //      `DbAuth(..)` always EXIT_DB_AUTH_ERROR, `Query(..)` always
    //      EXIT_QUERY_ERROR. These guard against future refactors that might
    //      accidentally make the typed path consult the message text.
    // ---------------------------------------------------------------------
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig {
            // 1k cases keeps the test under a second locally while still
            // covering a broad input space. The acceptance criterion in the
            // todo asks for "no panics" and "bounded codes", both of which
            // converge well before 10k.
            cases: 1024,
            ..ProptestConfig::default()
        })]

        /// Substring classifier must never panic and must always return a
        /// code inside the documented public range, regardless of input.
        #[test]
        fn proptest_substring_classifier_returns_valid_code(s in any::<String>()) {
            let err = anyhow!("{}", s);
            let code = map_error_to_exit_code(&err);
            prop_assert!(
                (EXIT_NO_ROWS..=EXIT_IO_ERROR).contains(&code),
                "exit code {} outside contract [1..=5] for input {:?}",
                code,
                s,
            );
        }

        /// Typed `Config` variant maps to `EXIT_CONFIG_ERROR` for ALL message
        /// payloads — the typed path is defined to ignore message text.
        #[test]
        fn proptest_typed_config_is_stable(s in any::<String>()) {
            let typed = GoldDiggerError::Config(ConfigError::Other(s));
            prop_assert_eq!(typed.exit_code(), EXIT_CONFIG_ERROR);
        }

        /// Typed `DbAuth` variant maps to `EXIT_DB_AUTH_ERROR` for ALL
        /// message payloads.
        #[test]
        fn proptest_typed_db_auth_is_stable(s in any::<String>()) {
            let typed = GoldDiggerError::DbAuth(s);
            prop_assert_eq!(typed.exit_code(), EXIT_DB_AUTH_ERROR);
        }

        /// Typed `Query` variant maps to `EXIT_QUERY_ERROR` for ALL message
        /// payloads, even ones that look like I/O or auth failures to the
        /// substring classifier.
        #[test]
        fn proptest_typed_query_is_stable(s in any::<String>()) {
            let typed = GoldDiggerError::Query(s);
            prop_assert_eq!(typed.exit_code(), EXIT_QUERY_ERROR);
        }

        /// Wrapping a typed variant inside an `anyhow::Error` (with or
        /// without `.context(..)` layers) must preserve the typed exit code.
        #[test]
        fn proptest_typed_through_anyhow_chain(s in any::<String>(), ctx in any::<String>()) {
            let typed = GoldDiggerError::Config(ConfigError::Other(s));
            let anyhow_err: Error = anyhow::Error::from(typed).context(ctx);
            prop_assert_eq!(map_error_to_exit_code(&anyhow_err), EXIT_CONFIG_ERROR);
        }
    }
}
