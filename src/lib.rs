//! Gold Digger: MySQL/MariaDB query tool with structured output.
//!
//! The crate is organised into the [`cli`] argument parser, output writers
//! ([`csv`], [`json`], [`tab`]), the panic-free value converter
//! ([`type_transformer`]), the exit-code contract ([`exit`]), rustls-backed
//! TLS ([`tls`]), and shared helpers ([`utils`]). The library exposes
//! [`rows_to_strings`] and [`get_extension_from_filename`] for callers.
//!
//! Each writer module exposes a single `write(...)` free function. CSV and
//! TSV take `IntoIterator<Item = IntoIterator<Item = String>>`; JSON takes
//! pre-converted `BTreeMap<String, serde_json::Value>` maps so the caller
//! can validate every row before truncating the output file.
//!
//! TLS is always rustls; consumers must invoke [`init_crypto_provider`] once
//! before opening a connection pool so rustls has a default crypto provider.
//! Exit codes follow the 0-5 contract defined in [`exit`].

use std::{env, ffi::OsStr, path::Path, sync::OnceLock};

use anyhow::{Context, Result};
use mysql::Row;

/// Default buffer capacity (64 KiB) for output writers.
///
/// Used by streaming sinks (`src/sink.rs`) and the legacy non-streaming
/// writers (`src/csv.rs`, `src/tab.rs`, `src/json.rs`) so a single tuning
/// knob covers every output path. Sized to amortise syscall cost on
/// modern filesystems while keeping memory pressure modest for very wide
/// rows (todo #059).
pub const OUTPUT_BUFFER_CAPACITY: usize = 64 * 1024;

static INIT: OnceLock<()> = OnceLock::new();

/// Install the ring crypto provider as rustls's process-wide default.
///
/// Must be called exactly once, before any `Pool::new`, `Opts::from_url`,
/// or other code path that constructs a rustls `ClientConfig`. The
/// [`OnceLock`] guard makes repeated calls safe — only the first call does
/// anything, later calls are no-ops. Migrated from `std::sync::Once` so
/// the initialiser is consistent with the [`OnceLock`] used by
/// [`crate::utils`] (todo #106).
///
/// # Provider detection (#4 medium fix)
///
/// Before attempting installation, this function checks
/// [`rustls::crypto::CryptoProvider::get_default`]. When another path
/// (test harness, library consumer, embedding application) has already
/// installed a provider, we defer to it rather than racing for the
/// process-wide default. When no provider is installed, we install
/// ring; an error from `install_default()` at that point is genuinely
/// surprising — it means another path won the race between our check
/// and our install — and we surface it via `tracing::warn!` instead of
/// silently dropping it. The `OnceLock` guard still prevents repeated
/// install attempts within this process.
pub fn init_crypto_provider() {
    INIT.get_or_init(|| {
        if rustls::crypto::CryptoProvider::get_default().is_some() {
            // Another path (test harness, library consumer) already
            // installed a provider — defer to it.
            return;
        }
        if let Err(_existing) = rustls::crypto::ring::default_provider().install_default() {
            tracing::warn!(
                "rustls crypto provider was installed by another path between our \
                 check and our install; using the one already in place"
            );
        }
    });
}

/// CLI interface module.
pub mod cli;
/// Shell completion script generation.
pub mod completion;
/// Configuration resolution and dumping.
pub mod config;
/// Database connection pool construction with rustls-only TLS.
pub mod connection;
/// CSV output module.
pub mod csv;
/// Shared delimited-output helper used by CSV and TSV writers.
pub mod delimited;
/// Exit code helper module.
pub mod exit;
/// JSON output module.
pub mod json;
/// Structured logging, colour, and progress helpers.
pub mod logging;
/// Named constants for MySQL/MariaDB server and client error codes.
pub mod mysql_errors;
/// Output format dispatch and safe output-file creation.
pub mod output;
/// Query-execution pipeline (binary entry point glue).
pub mod run;
/// Streaming row-sink trait and per-format implementations (F007).
pub mod sink;
/// Tab-delimited output module.
pub mod tab;
/// TLS configuration module.
pub mod tls;
/// Type transformation module for safe MySQL value conversion.
pub mod type_transformer;
/// Utility functions module.
pub mod utils;

pub use type_transformer::TypeTransformer;

/// Converts MySQL rows to a vector of string vectors, with the first row as headers.
///
/// This function safely handles all MySQL data types including NULL values without panicking.
/// It uses safe iteration over row values instead of indexed access to prevent runtime panics.
///
/// # Empty input
///
/// When `rows` is empty, this function returns `Ok(Vec::new())` — column
/// names are not available at this layer, so no header row is emitted.
/// Live query execution does **not** flow through this function: the
/// streaming pipeline in [`crate::run`] reads column metadata directly
/// from the streaming `mysql::QueryResult` and feeds it to the sink's
/// `on_headers` hook so headers are always written even on empty
/// result sets. The empty-vector outcome here only affects snapshot
/// tests / benchmarks that drive the legacy materialised path
/// (todo #056). JSON callers that need a `{"data":[]}` envelope should
/// use [`crate::sink::json_sink`] which always emits the envelope
/// regardless of row count.
///
/// # Arguments
///
/// * `rows` - A vector of MySQL rows.
///
/// # Returns
///
/// A Result containing a vector of string vectors (header row first when
/// non-empty), or an error.
///
/// # Safety
///
/// This function replaces the dangerous pattern of using `row[column.name_str().as_ref()]`
/// which can panic on NULL values or type mismatches. Instead, it uses safe iteration
/// over `row.as_ref()` to handle all value types gracefully.
pub fn rows_to_strings(mut rows: Vec<Row>) -> anyhow::Result<Vec<Vec<String>>> {
    // Use `first()` rather than `rows[0]` so the workspace
    // `clippy::indexing_slicing` lint stays clean (todo #149); the
    // `let-else` short-circuit also makes the empty-input branch
    // explicit instead of relying on a separate `is_empty` guard.
    let Some(first_row) = rows.first() else {
        return Ok(Vec::new());
    };

    // Pre-allocate with known capacity for better performance.
    // +1 accounts for the header row prepended below.
    let mut result_rows = Vec::with_capacity(rows.len().saturating_add(1));

    // Extract headers from the first row before draining so we retain access
    // to column metadata while the row values are still owned by `rows`.
    let header_row: Vec<String> = first_row
        .columns_ref()
        .iter()
        .map(|column| column.name_str().to_string())
        .collect();
    result_rows.push(header_row);

    // Drain each row by value so the `Row` is dropped as soon as its string
    // representation has been extracted. This halves peak memory during
    // conversion compared to `rows.iter()` which kept every source row live
    // alongside the fully-materialised result set.
    //
    // Delegate per-row conversion to [`TypeTransformer::row_to_strings`]
    // so the inner column-level error message ("Failed to convert column
    // N to string") is consistent across the streaming sink path and
    // this legacy bulk path (todo #063). The outer `.context()` adds the
    // row index — `TypeTransformer` only sees one row at a time so the
    // row context has to be supplied here.
    for (row_index, row) in rows.drain(..).enumerate() {
        let data_row = match TypeTransformer::row_to_strings(row) {
            Ok(values) => values,
            Err(e) => {
                return Err(e.context(format!("Type conversion failed at row {}", row_index + 1)));
            }
        };
        result_rows.push(data_row);
    }

    Ok(result_rows)
}

/// Extracts the file extension from a filename, if present.
///
/// # Case sensitivity
///
/// Returns the extension exactly as it appears on disk — `"FOO.CSV"`
/// yields `Some("CSV")`, not `Some("csv")`. Callers that need a
/// case-insensitive dispatch (e.g., [`crate::cli::OutputFormat::from_extension`])
/// must lowercase the returned string themselves. See todo
/// `explicit-error-for-non-utf-8-extension-paths` for the planned
/// handling of paths whose extension is not valid UTF-8.
///
/// # Arguments
///
/// * `filename` - The filename as a string slice.
///
/// # Returns
///
/// An Option containing the extension as a string slice, or None if
/// the path has no extension or the extension is not valid UTF-8.
pub fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension().and_then(OsStr::to_str)
}

/// Gets a required environment variable with contextual error information.
///
/// This is the low-level helper that underpins the CLI config resolvers
/// ([`crate::config::resolve_database_url`],
/// [`crate::config::resolve_database_query`],
/// [`crate::config::resolve_output_file`]). Those resolvers prefer CLI
/// flags and only call into this function as a fallback, so most callers
/// should go through them rather than invoking `get_required_env`
/// directly.
///
/// # Arguments
///
/// * `var_name` - The name of the environment variable to retrieve.
///
/// # Returns
///
/// A `Result` containing the environment variable value as a `String`,
/// or an `anyhow::Error` with a `"Missing required environment variable: NAME"`
/// context message when the variable is unset or not valid Unicode.
///
/// # Example
///
/// ```
/// use gold_digger::get_required_env;
///
/// // SAFETY: test is single-threaded; no other code reads this var.
/// unsafe { std::env::set_var("GD_EXAMPLE_VAR", "hello"); }
/// let value = get_required_env("GD_EXAMPLE_VAR").unwrap();
/// assert_eq!(value, "hello");
///
/// let missing = get_required_env("GD_DEFINITELY_UNSET_VAR_XYZ");
/// assert!(missing.is_err());
/// ```
pub fn get_required_env(var_name: &str) -> Result<String> {
    env::var(var_name)
        .with_context(|| format!("Missing required environment variable: {}", var_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_required_env_missing() {
        let result = get_required_env("NONEXISTENT_ENV_VAR");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Missing required environment variable: NONEXISTENT_ENV_VAR")
        );
    }

    #[test]
    fn test_get_required_env_present() {
        temp_env::with_var("TEST_ENV_VAR", Some("test_value"), || {
            let result = get_required_env("TEST_ENV_VAR");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "test_value");
        });
    }

    #[test]
    fn test_rows_to_strings_empty() {
        let result = rows_to_strings(vec![]).expect("empty rows should succeed");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_rows_to_strings_type_conversion_error() {
        // Verify that type conversion errors map to exit code 4
        let error = anyhow::anyhow!("Type conversion error: Invalid month value 13 in date");
        use crate::exit::map_error_to_exit_code;
        assert_eq!(map_error_to_exit_code(&error), 4);
        assert!(error.to_string().contains("Type conversion error"));
    }

    #[test]
    fn test_get_extension_from_filename() {
        assert_eq!(get_extension_from_filename("test.json"), Some("json"));
        assert_eq!(get_extension_from_filename("test.csv"), Some("csv"));
        assert_eq!(get_extension_from_filename("test.tsv"), Some("tsv"));
        assert_eq!(get_extension_from_filename("test"), None);
        assert_eq!(get_extension_from_filename(""), None);
        assert_eq!(get_extension_from_filename(".hidden"), None);
    }
}
