//! Configuration resolution and dumping.
//!
//! CLI flags take precedence over environment variables. Resolution errors
//! are wrapped in [`GoldDiggerError::Config`] (or [`GoldDiggerError::Io`] for
//! filesystem interactions) so the exit-code classifier can identify them
//! via downcast, independent of error-message text.
//!
//! # Single-read environment snapshot (todo #068)
//!
//! [`EnvSnapshot::from_process_env`] reads every supported `DATABASE_URL` /
//! `DATABASE_QUERY` / `OUTPUT_FILE` value from the process environment
//! exactly once at startup. Downstream code should consult that snapshot
//! rather than calling `std::env::var` again — this guarantees the
//! resolved values are stable for the lifetime of the run even if a
//! parent process mutates the environment mid-execution (CWE-284), and
//! makes [`dump_configuration`] deterministic against the same inputs.

use std::env;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::cli::Cli;
use crate::exit::{ConfigError, GoldDiggerError};
use crate::utils::redact_dump_query;

/// Frozen view of the three legacy environment-variable fallbacks
/// (`DATABASE_URL`, `DATABASE_QUERY`, `OUTPUT_FILE`).
///
/// Constructed once at startup via [`EnvSnapshot::from_process_env`] so
/// downstream resolvers see a consistent set of values regardless of
/// concurrent mutations to the process environment (todo #068).
#[derive(Debug, Clone, Default)]
pub struct EnvSnapshot {
    /// Value of `DATABASE_URL`, if set at snapshot time.
    pub database_url: Option<String>,
    /// Value of `DATABASE_QUERY`, if set at snapshot time.
    pub database_query: Option<String>,
    /// Value of `OUTPUT_FILE`, if set at snapshot time.
    pub output_file: Option<String>,
}

impl EnvSnapshot {
    /// Reads the supported env vars exactly once and returns the frozen
    /// snapshot. Subsequent mutations to the environment do not affect
    /// the returned struct.
    pub fn from_process_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL").ok(),
            database_query: env::var("DATABASE_QUERY").ok(),
            output_file: env::var("OUTPUT_FILE").ok(),
        }
    }
}

/// Generic CLI > env > error resolver shared by the three legacy
/// resolvers below (todo #101).
///
/// Returns the CLI value when present; otherwise the env value;
/// otherwise a typed [`ConfigError`] (wrapped via [`GoldDiggerError::Config`]).
fn resolve_or_missing<T: Clone>(
    cli_value: Option<&T>,
    env_value: Option<&T>,
    missing: ConfigError,
) -> Result<T> {
    if let Some(v) = cli_value {
        return Ok(v.clone());
    }
    if let Some(v) = env_value {
        return Ok(v.clone());
    }
    Err(GoldDiggerError::Config(missing).into())
}

/// Maximum accepted size for a `--query-file` payload, in bytes. A query
/// file larger than this is refused at resolve time to cap DoS risk from
/// an attacker pointing `--query-file` at a huge file on a shared host
/// (todo #023). 10 MiB is far larger than any legitimate hand-written SQL
/// query while still bounding memory and response latency.
pub const MAX_QUERY_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024;

/// Extensions refused for `--query-file` to stop accidental reads of
/// binary artefacts as SQL (todo #023). Comparison is case-insensitive.
/// The check is intentionally deny-list style: allow `.sql`, `.txt`, or
/// missing extensions (plain filename like `query` is common for wrapper
/// scripts); refuse recognised binary extensions explicitly so the error
/// message tells the caller which extension tripped the guard.
pub const REFUSED_QUERY_FILE_EXTENSIONS: &[&str] =
    &["exe", "dll", "so", "dylib", "bin", "bat", "cmd", "com"];

/// Resolves the database URL from CLI arguments or environment variables.
///
/// Errors are wrapped in [`GoldDiggerError::Config`] so the exit-code
/// classifier can identify them via downcast (stable across message text
/// refactors).
///
/// Reads `DATABASE_URL` from the process environment as a fallback. New
/// code should prefer [`resolve_database_url_with_env`] with a snapshot
/// captured at startup so the resolution is deterministic against env
/// mutations during the run (todo #068).
pub fn resolve_database_url(cli: &Cli) -> Result<String> {
    let snapshot = EnvSnapshot::from_process_env();
    resolve_database_url_with_env(cli, &snapshot)
}

/// Snapshot-aware variant of [`resolve_database_url`] that reads the
/// `DATABASE_URL` fallback from a frozen [`EnvSnapshot`] instead of
/// `std::env`. (todo #068, #101)
pub fn resolve_database_url_with_env(cli: &Cli, env: &EnvSnapshot) -> Result<String> {
    resolve_or_missing(
        cli.db_url.as_ref(),
        env.database_url.as_ref(),
        ConfigError::MissingDbUrl,
    )
}

/// Validates a `--query-file` path (todo #023).
///
/// Applies three path-safety guards before returning the canonical path
/// the caller should pass to `read_to_string`:
///
/// 1. **Size cap (pre-canonicalise).** `std::fs::symlink_metadata` reads
///    only the entry the path names — never following symlinks — so we
///    can refuse oversize files via [`MAX_QUERY_FILE_SIZE_BYTES`] before
///    paying the traversal cost of `canonicalize` (#6 medium fix). On a
///    hostile filesystem, `canonicalize` can read deeply-nested symlink
///    chain entries; capping the size first bounds that work too. Stat
///    failures map to [`GoldDiggerError::Io`] (exit 5).
/// 2. **Canonicalize.** `std::fs::canonicalize` resolves `..` and symlinks,
///    giving the caller a stable path that matches what the OS will open.
///    Any failure (including broken symlinks or missing files) maps to
///    [`GoldDiggerError::Io`] so the exit code (5) reflects the filesystem
///    interaction.
/// 3. **Extension deny-list.** Refuse obvious binary extensions (`.exe`,
///    `.dll`, `.so`, `.dylib`, `.bin`, `.bat`, `.cmd`, `.com`) with a
///    configuration error so the operator sees the problem, not a cryptic
///    SQL syntax error from the server. `.sql` / `.txt` / missing are
///    allowed. Comparison is case-insensitive (`query.SQL` works). The
///    extension is checked on the canonicalised path so symlinks pointing
///    at refused targets are caught.
///
/// Failures use [`GoldDiggerError::Config`] (exit 2) for policy rejections
/// and [`GoldDiggerError::Io`] (exit 5) for filesystem / stat failures.
///
/// Calls through to [`validate_query_file_path_with_limit`] using the
/// default [`MAX_QUERY_FILE_SIZE_BYTES`] cap. New code should prefer the
/// `_with_limit` variant so the user-supplied `--max-query-file-size`
/// value is honoured.
pub fn validate_query_file_path(path: &Path) -> Result<PathBuf> {
    validate_query_file_path_with_limit(path, MAX_QUERY_FILE_SIZE_BYTES)
}

/// Like [`validate_query_file_path`] but accepts a custom size cap so the
/// user-supplied `--max-query-file-size` flag has a recovery path. Error
/// messages reference the exact configured limit and the CLI flag, not
/// the internal const, so the operator can act on the failure without
/// reading the source.
pub fn validate_query_file_path_with_limit(path: &Path, max_bytes: u64) -> Result<PathBuf> {
    // 1. Size cap, pre-canonicalise. `symlink_metadata` does NOT follow
    //    symlinks, so a deeply-nested or attacker-controlled symlink
    //    chain cannot trick us into expensive traversal before we've
    //    even confirmed the file is small enough to read. If the entry
    //    itself is a symlink we still want to reject when its TARGET is
    //    huge, so this guard runs again post-canonicalise below — but
    //    the cheap check up front bounds the worst case for non-symlink
    //    files (todo #6).
    let entry_metadata = std::fs::symlink_metadata(path).map_err(|e| {
        anyhow::Error::from(GoldDiggerError::Io(e))
            .context(format!("Failed to stat query file {}", path.display()))
    })?;
    if entry_metadata.is_file() && entry_metadata.len() > max_bytes {
        return Err(GoldDiggerError::Config(ConfigError::InvalidQueryFile {
            path: path.to_path_buf(),
            reason: oversize_query_file_message(entry_metadata.len(), max_bytes),
        })
        .into());
    }

    // 2. Canonicalize. Broken symlinks and missing files fail here with
    //    a real `io::Error`, which gets wrapped via `GoldDiggerError::Io`
    //    (exit 5) for stable routing.
    let canonical = std::fs::canonicalize(path).map_err(|e| {
        anyhow::Error::from(GoldDiggerError::Io(e)).context(format!(
            "Failed to canonicalize query file path {}",
            path.display()
        ))
    })?;

    // 3. Extension deny-list (case-insensitive; missing is allowed).
    if let Some(ext) = canonical.extension().and_then(|s| s.to_str()) {
        let lower = ext.to_ascii_lowercase();
        if REFUSED_QUERY_FILE_EXTENSIONS.iter().any(|r| *r == lower) {
            return Err(GoldDiggerError::Config(ConfigError::InvalidQueryFile {
                path: canonical.clone(),
                reason: format!(
                    "Refusing to read query file with disallowed extension '.{}'. \
                     Use --query-file with .sql, .txt, or no extension.",
                    lower,
                ),
            })
            .into());
        }
    }

    // 4. Size cap, post-canonicalise. The pre-canonicalise check above
    //    only inspected the entry the path named (no symlink follow);
    //    re-check on the canonical target so a symlink pointing at a
    //    huge file is still caught.
    let metadata = std::fs::metadata(&canonical).map_err(|e| {
        anyhow::Error::from(GoldDiggerError::Io(e))
            .context(format!("Failed to stat query file {}", canonical.display()))
    })?;
    if metadata.len() > max_bytes {
        return Err(GoldDiggerError::Config(ConfigError::InvalidQueryFile {
            path: canonical.clone(),
            reason: oversize_query_file_message(metadata.len(), max_bytes),
        })
        .into());
    }

    Ok(canonical)
}

/// Builds the user-facing "oversize query file" error string. Centralised
/// so the size cap and the CLI flag for raising it stay in one place.
fn oversize_query_file_message(actual: u64, configured: u64) -> String {
    format!(
        "Query file is {} bytes; refused because the limit is {} bytes. \
         Split the query, or raise the limit with --max-query-file-size <BYTES>.",
        actual, configured
    )
}

/// Resolves the database query from CLI arguments, an external file, or
/// environment variables.
///
/// The `--query-file` path is canonicalized and validated (extension
/// deny-list, size cap) via [`validate_query_file_path`] before being
/// read. Policy rejections map to [`GoldDiggerError::Config`] (exit 2);
/// filesystem failures map to [`GoldDiggerError::Io`] (exit 5); missing
/// configuration maps to [`GoldDiggerError::Config`] (exit 2).
pub fn resolve_database_query(cli: &Cli) -> Result<String> {
    let snapshot = EnvSnapshot::from_process_env();
    resolve_database_query_with_env(cli, &snapshot)
}

/// Snapshot-aware variant of [`resolve_database_query`]. Reads the
/// `DATABASE_QUERY` fallback from the supplied [`EnvSnapshot`] rather
/// than the process environment so the resolution is deterministic
/// against env mutations (todo #068).
pub fn resolve_database_query_with_env(cli: &Cli, env: &EnvSnapshot) -> Result<String> {
    let max_bytes = cli.max_query_file_size.unwrap_or(MAX_QUERY_FILE_SIZE_BYTES);
    resolve_database_query_with_env_and_limit(cli, env, max_bytes)
}

/// Like [`resolve_database_query_with_env`] but accepts an explicit max
/// size for the query-file payload so callers can plumb the
/// `--max-query-file-size` value through.
pub fn resolve_database_query_with_env_and_limit(
    cli: &Cli,
    env: &EnvSnapshot,
    max_bytes: u64,
) -> Result<String> {
    if let Some(query) = &cli.query {
        return Ok(query.clone());
    }
    if let Some(query_file) = &cli.query_file {
        let canonical = validate_query_file_path_with_limit(query_file, max_bytes)?;
        return std::fs::read_to_string(&canonical).map_err(|e| {
            // Preserve both the typed I/O classification and the original
            // path context that previous integration tests assert on.
            anyhow::Error::from(GoldDiggerError::Io(e)).context(format!(
                "Failed to read query file {}",
                query_file.display()
            ))
        });
    }
    env.database_query
        .clone()
        .ok_or_else(|| GoldDiggerError::Config(ConfigError::MissingQuery).into())
}

/// Resolves the output file path from CLI arguments or environment variables.
///
/// Errors are wrapped in [`GoldDiggerError::Config`] for stable exit-code
/// classification.
pub fn resolve_output_file(cli: &Cli) -> Result<PathBuf> {
    let snapshot = EnvSnapshot::from_process_env();
    resolve_output_file_with_env(cli, &snapshot)
}

/// Snapshot-aware variant of [`resolve_output_file`].
pub fn resolve_output_file_with_env(cli: &Cli, env: &EnvSnapshot) -> Result<PathBuf> {
    if let Some(output) = &cli.output {
        return Ok(output.clone());
    }
    env.output_file
        .as_ref()
        .map(PathBuf::from)
        .ok_or_else(|| GoldDiggerError::Config(ConfigError::MissingOutputFile).into())
}

/// Fully-resolved query source. Either an inline `--query` / env-supplied
/// string, or the contents of a `--query-file` that has been canonicalised
/// and read at config-resolution time so IO failures surface up-front.
#[derive(Debug, Clone)]
pub enum ResolvedQuery {
    /// Query supplied as a string (CLI `--query` or `DATABASE_QUERY`).
    Inline(String),
    /// Query loaded from a file. `path` is the canonical path; `contents`
    /// is the file body, eagerly read so the run path never blocks on IO
    /// after configuration is settled.
    File { path: PathBuf, contents: String },
}

impl ResolvedQuery {
    /// Returns the query body regardless of whether it came from `--query`
    /// or `--query-file`.
    pub fn body(&self) -> &str {
        match self {
            Self::Inline(s) => s.as_str(),
            Self::File { contents, .. } => contents.as_str(),
        }
    }

    /// Consumes the query and returns the body string.
    pub fn into_body(self) -> String {
        match self {
            Self::Inline(s) => s,
            Self::File { contents, .. } => contents,
        }
    }
}

/// Output target after resolving format + path together. The format is
/// determined exactly once at config time (`--format` > extension > error)
/// so no downstream code has to re-run the dispatch.
#[derive(Debug, Clone)]
pub struct OutputTarget {
    /// Path the output sink will write to.
    pub path: PathBuf,
    /// Resolved output format (CSV, JSON, or TSV).
    pub format: crate::cli::OutputFormat,
}

impl OutputTarget {
    /// Builds an [`OutputTarget`] from the resolved output path and the
    /// CLI-supplied `--format` flag. Fails when neither the extension nor
    /// `--format` can determine the output format (typed
    /// [`ConfigError::UnresolvableFormat`], exit 2).
    pub fn from_path_and_format(
        path: PathBuf,
        format_override: Option<crate::cli::OutputFormat>,
    ) -> Result<Self> {
        let format = if let Some(format) = format_override {
            format
        } else {
            crate::cli::OutputFormat::from_extension(&path).ok_or_else(|| {
                GoldDiggerError::Config(ConfigError::UnresolvableFormat(format!(
                    "Cannot infer output format from '{}'. Recognised extensions: .csv, .json, .tsv, .tab, .txt. Pass --format <csv|json|tsv> to select explicitly.",
                    path.display()
                )))
            })?
        };
        Ok(Self { path, format })
    }
}

/// Fully-resolved, validated configuration for a single `gold_digger` run.
///
/// Built once at the binary boundary by [`ResolvedConfig::from_cli`] (or
/// [`ResolvedConfig::from_cli_with_env`] in tests). All resolution,
/// validation, and IO that depend only on `(Cli, EnvSnapshot)` happens at
/// construction time: the query file is read, the output format is
/// determined, the TLS configuration is materialised. Downstream code
/// (`run.rs`, `connection.rs`, `output.rs`) consumes a `&ResolvedConfig`
/// rather than re-walking the parsed `Cli` and re-running validation.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    /// Resolved database connection URL (`--db-url` > `DATABASE_URL`).
    pub database_url: String,
    /// Resolved query source (inline string or canonical file + contents).
    pub query: ResolvedQuery,
    /// Resolved output target (path + format).
    pub output: OutputTarget,
    /// Resolved TLS configuration (defaults to platform-validation when
    /// no TLS flags were supplied).
    pub tls: crate::tls::TlsConfig,
    /// Whether to exit successfully on empty result sets.
    pub allow_empty: bool,
    /// Whether to pretty-print JSON output.
    pub pretty: bool,
    /// Whether to overwrite an existing output file.
    pub force: bool,
    /// Verbosity counter from `-v` (0..).
    pub verbose: u8,
    /// Whether `--quiet` was supplied.
    pub quiet: bool,
}

impl ResolvedConfig {
    /// Builds a fully-validated [`ResolvedConfig`] from parsed CLI args
    /// using the live process environment. Convenience wrapper around
    /// [`Self::from_cli_with_env`] that snapshots
    /// [`EnvSnapshot::from_process_env`] internally.
    pub fn from_cli(cli: &Cli) -> Result<Self> {
        Self::from_cli_with_env(cli, &EnvSnapshot::from_process_env())
    }

    /// Builds a fully-validated [`ResolvedConfig`] from parsed CLI args
    /// and an explicit environment snapshot.
    ///
    /// Calls every resolver, validates each step, parses the query file,
    /// determines the output format, and materialises the TLS
    /// configuration. Returns a typed [`GoldDiggerError`] on any failure
    /// so the exit-code classifier can route to the correct exit code
    /// without inspecting message text.
    pub fn from_cli_with_env(cli: &Cli, env: &EnvSnapshot) -> Result<Self> {
        let database_url = resolve_database_url_with_env(cli, env)?;

        let max_query_file_size = cli.max_query_file_size.unwrap_or(MAX_QUERY_FILE_SIZE_BYTES);
        let query = resolve_query_source(cli, env, max_query_file_size)?;

        let output_path = resolve_output_file_with_env(cli, env)?;
        let output = OutputTarget::from_path_and_format(output_path, cli.format)?;

        let tls = cli
            .tls_options
            .to_tls_config()
            .map_err(|e| anyhow::Error::from(e).context("TLS configuration error"))?;

        Ok(Self {
            database_url,
            query,
            output,
            tls,
            allow_empty: cli.allow_empty,
            pretty: cli.pretty,
            force: cli.force,
            verbose: cli.verbose,
            quiet: cli.quiet,
        })
    }
}

/// Builds a [`ResolvedQuery`] from CLI args + env snapshot, with the
/// `--max-query-file-size` cap honoured for `--query-file` reads.
fn resolve_query_source(cli: &Cli, env: &EnvSnapshot, max_bytes: u64) -> Result<ResolvedQuery> {
    if let Some(query) = &cli.query {
        return Ok(ResolvedQuery::Inline(query.clone()));
    }
    if let Some(query_file) = &cli.query_file {
        let canonical = validate_query_file_path_with_limit(query_file, max_bytes)?;
        let contents = std::fs::read_to_string(&canonical).map_err(|e| {
            anyhow::Error::from(GoldDiggerError::Io(e)).context(format!(
                "Failed to read query file {}",
                query_file.display()
            ))
        })?;
        return Ok(ResolvedQuery::File {
            path: canonical,
            contents,
        });
    }
    if let Some(query) = &env.database_query {
        return Ok(ResolvedQuery::Inline(query.clone()));
    }
    Err(GoldDiggerError::Config(ConfigError::MissingQuery).into())
}

/// Builds the configuration-dump JSON value for `--dump-config`.
///
/// Returns a [`serde_json::Value`] so callers can serialise wherever they
/// need (stdout, a string buffer, a unit test) instead of being forced
/// through `println!` (todo #062, T-C3). The binary entry point in
/// `src/main.rs` writes the pretty-printed result to stdout; tests
/// inspect the value directly.
///
/// Reads `DATABASE_QUERY` and `OUTPUT_FILE` from the supplied
/// [`EnvSnapshot`] (a one-shot read taken at startup, todo #068). Missing
/// CLI / env values surface as JSON `null` rather than the empty string
/// so consumers can distinguish "explicitly empty" from "unresolved"
/// (todo #057).
///
/// The query (whether from `--query` or `DATABASE_QUERY`) is routed
/// through [`redact_dump_query`], which delegates to the same regex set
/// that scrubs MySQL error messages. There is exactly one redactor in
/// the codebase ([`crate::utils`]) so a fix to a missed pattern
/// (e.g. `passwd=`, `pwd=`, `Kennwort=`, `mot_de_passe=`) lands in every
/// surface at once.
pub fn build_configuration_dump(cli: &Cli, env: &EnvSnapshot) -> serde_json::Value {
    use serde_json::json;

    // Route the query through the canonical redactor. The previous
    // implementation used a substring check for "password" / "identified
    // by" and replaced the entire query with a sentinel — that missed
    // `pwd=`, `passwd=`, GRANT/SET PASSWORD, base64/JWT/hex blobs,
    // non-English labels, and erased the legitimate query when it
    // matched. The shared regex set in `utils::redact_sql_error` covers
    // all of those and only redacts the offending substrings.
    let redacted_query: serde_json::Value = cli
        .query
        .as_ref()
        .or(env.database_query.as_ref())
        .map(|q| serde_json::Value::String(redact_dump_query(q)))
        .unwrap_or(serde_json::Value::Null);

    // Output: prefer CLI, fall back to env snapshot, otherwise JSON null
    // so dumps stay distinguishable from "explicitly the empty string"
    // (todo #057).
    let output_value: serde_json::Value = match cli.output.as_ref() {
        Some(p) => serde_json::Value::String(p.display().to_string()),
        None => env
            .output_file
            .clone()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    };

    json!({
        "database_url": "***REDACTED***", // Always redact database URLs
        "query": redacted_query,
        "query_file": cli.query_file.as_ref().map(|p| p.display().to_string()),
        "output": output_value,
        "format": cli.format.as_ref().map(|f| f.as_str()),
        "verbose": cli.verbose,
        "quiet": cli.quiet,
        "pretty": cli.pretty,
        "allow_empty": cli.allow_empty,
        "features": {
            // JSON and CSV output are built into the binary unconditionally
            // (todo #011 removed the vestigial feature flags).
            "json": true,
            "csv": true,
            // The `verbose` feature was removed (todo #180) — verbosity is
            // a runtime flag, not a build-time toggle. Reported as
            // `false` so older callers still see a stable schema.
            "verbose": false,
            "additional_mysql_types": cfg!(feature = "additional_mysql_types"),
            "tls": true  // TLS is always available (rustls-only implementation)
        }
    })
}

/// Backwards-compatible entry point that prints the dump JSON to stdout.
///
/// Internally delegates to [`build_configuration_dump`] so unit tests can
/// inspect the JSON without spawning a subprocess. The binary entry
/// point uses [`build_configuration_dump`] directly to control where the
/// JSON is written.
pub fn dump_configuration(cli: &Cli) -> Result<()> {
    let snapshot = EnvSnapshot::from_process_env();
    let value = build_configuration_dump(cli, &snapshot);
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// Creates a CLI instance with common test arguments
    fn build_test_cli() -> Cli {
        Cli::parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
        ])
    }

    #[test]
    fn test_resolve_database_url_from_cli() {
        let cli = build_test_cli();
        let result = resolve_database_url(&cli);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "mysql://test");
    }

    #[test]
    fn test_resolve_database_url_from_env() {
        // Parse inside temp_env so clap picks up our env var, not the user's
        temp_env::with_var("DATABASE_URL", Some("mysql://env_test"), || {
            let cli = Cli::parse_from([
                "gold_digger",
                "--query",
                "SELECT 1",
                "--output",
                "test.json",
            ]);
            let result = resolve_database_url(&cli);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "mysql://env_test");
        });
    }

    #[test]
    fn test_resolve_database_url_missing() {
        // Parse inside temp_env so clap does not pick up the user's env var
        temp_env::with_var("DATABASE_URL", None::<&str>, || {
            let cli = Cli::parse_from([
                "gold_digger",
                "--query",
                "SELECT 1",
                "--output",
                "test.json",
            ]);
            let result = resolve_database_url(&cli);
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Missing database URL")
            );
        });
    }

    #[test]
    fn test_resolve_database_query_from_cli() {
        let cli = build_test_cli();
        let result = resolve_database_query(&cli);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "SELECT 1");
    }

    #[test]
    fn test_resolve_database_query_from_file() -> anyhow::Result<()> {
        let mut temp_file = tempfile::NamedTempFile::new()?;
        std::io::Write::write_all(&mut temp_file, b"SELECT * FROM users")?;

        let cli = Cli::parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query-file",
            temp_file.path().to_str().unwrap(),
            "--output",
            "test.json",
        ]);

        let result = resolve_database_query(&cli);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "SELECT * FROM users");
        Ok(())
    }

    #[test]
    fn test_resolve_database_query_from_env() {
        // Parse inside temp_env so clap's `#[arg(env = "DATABASE_QUERY")]`
        // reads the temp value rather than racing with sibling tests that
        // mutate the same env var.
        temp_env::with_var("DATABASE_QUERY", Some("SELECT * FROM env_table"), || {
            let cli = Cli::parse_from([
                "gold_digger",
                "--db-url",
                "mysql://test",
                "--output",
                "test.json",
            ]);
            let result = resolve_database_query(&cli);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "SELECT * FROM env_table");
        });
    }

    #[test]
    fn test_resolve_database_query_missing() {
        // Parse inside temp_env so clap does not pick up a sibling test's
        // DATABASE_QUERY value before this test gets to clear it.
        temp_env::with_var("DATABASE_QUERY", None::<&str>, || {
            let cli = Cli::parse_from([
                "gold_digger",
                "--db-url",
                "mysql://test",
                "--output",
                "test.json",
            ]);
            let result = resolve_database_query(&cli);
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Missing database query")
            );
        });
    }

    #[test]
    fn test_resolve_database_query_invalid_file() {
        let cli = Cli::parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query-file",
            "/nonexistent/file.sql",
            "--output",
            "test.json",
        ]);

        let result = resolve_database_query(&cli);
        assert!(result.is_err());
        // After todo #023 the path is canonicalized first; a nonexistent
        // path fails in the canonicalize step, so the error wording is
        // "Failed to canonicalize query file path". Either message is
        // acceptable as long as we surface *some* query-file context.
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("query file") || err_msg.contains("canonicalize"),
            "error message should mention the query file path, got: {err_msg}"
        );
    }

    #[test]
    fn test_resolve_output_file_from_cli() {
        let cli = build_test_cli();
        let result = resolve_output_file(&cli);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("test.json"));
    }

    #[test]
    fn test_resolve_output_file_from_env() {
        // Parse inside temp_env so clap's `#[arg(env = "OUTPUT_FILE")]` reads
        // the temp value rather than racing with sibling tests that mutate
        // the same env var.
        temp_env::with_var("OUTPUT_FILE", Some("/tmp/env_output.csv"), || {
            let cli = Cli::parse_from([
                "gold_digger",
                "--db-url",
                "mysql://test",
                "--query",
                "SELECT 1",
            ]);
            let result = resolve_output_file(&cli);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), PathBuf::from("/tmp/env_output.csv"));
        });
    }

    #[test]
    fn test_resolve_output_file_missing() {
        // Parse inside temp_env so clap does not pick up a sibling test's
        // OUTPUT_FILE value before this test gets to clear it.
        temp_env::with_var("OUTPUT_FILE", None::<&str>, || {
            let cli = Cli::parse_from([
                "gold_digger",
                "--db-url",
                "mysql://test",
                "--query",
                "SELECT 1",
            ]);
            let result = resolve_output_file(&cli);
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Missing output file")
            );
        });
    }

    // ---------------------------------------------------------------------
    // CLI-vs-env precedence regression tests (todo #036).
    //
    // These pin the historical "Some(&_) pattern bug" so that any future
    // regression in the resolve_* functions where CLI flags failed to
    // override the corresponding env var would surface immediately.
    // ---------------------------------------------------------------------

    #[test]
    fn test_resolve_database_url_cli_overrides_env() {
        temp_env::with_var("DATABASE_URL", Some("mysql://from-env"), || {
            let cli = Cli::parse_from([
                "gold_digger",
                "--db-url",
                "mysql://from-cli",
                "--query",
                "SELECT 1",
                "--output",
                "test.json",
            ]);
            let result = resolve_database_url(&cli);
            assert!(result.is_ok(), "resolve failed: {:?}", result.err());
            assert_eq!(
                result.unwrap(),
                "mysql://from-cli",
                "CLI --db-url must win over DATABASE_URL env var"
            );
        });
    }

    #[test]
    fn test_resolve_database_query_cli_overrides_env() {
        temp_env::with_var("DATABASE_QUERY", Some("SELECT 'from_env'"), || {
            let cli = Cli::parse_from([
                "gold_digger",
                "--db-url",
                "mysql://test",
                "--query",
                "SELECT 'from_cli'",
                "--output",
                "test.json",
            ]);
            let result = resolve_database_query(&cli);
            assert!(result.is_ok(), "resolve failed: {:?}", result.err());
            assert_eq!(
                result.unwrap(),
                "SELECT 'from_cli'",
                "CLI --query must win over DATABASE_QUERY env var"
            );
        });
    }

    #[test]
    fn test_resolve_output_file_cli_overrides_env() {
        temp_env::with_var("OUTPUT_FILE", Some("/tmp/from_env.csv"), || {
            let cli = Cli::parse_from([
                "gold_digger",
                "--db-url",
                "mysql://test",
                "--query",
                "SELECT 1",
                "--output",
                "/tmp/from_cli.json",
            ]);
            let result = resolve_output_file(&cli);
            assert!(result.is_ok(), "resolve failed: {:?}", result.err());
            assert_eq!(
                result.unwrap(),
                PathBuf::from("/tmp/from_cli.json"),
                "CLI --output must win over OUTPUT_FILE env var"
            );
        });
    }

    #[test]
    fn test_dump_configuration() -> anyhow::Result<()> {
        let cli = build_test_cli();

        // This should not panic and should return Ok
        let result = dump_configuration(&cli);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_dump_configuration_with_sensitive_query() -> anyhow::Result<()> {
        let cli = Cli::parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "CREATE USER 'test' IDENTIFIED BY 'secret123'",
            "--output",
            "test.json",
        ]);

        // This should redact the sensitive query
        let result = dump_configuration(&cli);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_dump_configuration_with_env_query() -> anyhow::Result<()> {
        temp_env::with_var("DATABASE_QUERY", Some("SELECT password FROM users"), || {
            let cli = Cli::parse_from([
                "gold_digger",
                "--db-url",
                "mysql://test",
                "--output",
                "test.json",
            ]);

            let result = dump_configuration(&cli);
            assert!(result.is_ok());
        });

        Ok(())
    }
}
