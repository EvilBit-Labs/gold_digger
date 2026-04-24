//! Configuration resolution and dumping.
//!
//! CLI flags take precedence over environment variables. Resolution errors
//! are wrapped in [`GoldDiggerError::Config`] (or [`GoldDiggerError::Io`] for
//! filesystem interactions) so the exit-code classifier can identify them
//! via downcast, independent of error-message text.

use std::env;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::cli::Cli;
use crate::exit::GoldDiggerError;
use crate::utils::redact_dump_query;

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
pub fn resolve_database_url(cli: &Cli) -> Result<String> {
    if let Some(url) = &cli.db_url {
        Ok(url.clone())
    } else {
        std::env::var("DATABASE_URL").map_err(|_| {
            GoldDiggerError::Config(
                "Missing database URL. Provide --db-url or set DATABASE_URL environment variable"
                    .into(),
            )
            .into()
        })
    }
}

/// Validates a `--query-file` path (todo #023).
///
/// Applies three path-safety guards before returning the canonical path
/// the caller should pass to `read_to_string`:
///
/// 1. **Canonicalize.** `std::fs::canonicalize` resolves `..` and symlinks,
///    giving the caller a stable path that matches what the OS will open.
///    Any failure (including broken symlinks or missing files) maps to
///    [`GoldDiggerError::Io`] so the exit code (5) reflects the filesystem
///    interaction. Permissive traversal handling is acceptable because
///    the size / extension checks below are the real safety net.
/// 2. **Extension deny-list.** Refuse obvious binary extensions (`.exe`,
///    `.dll`, `.so`, `.dylib`, `.bin`, `.bat`, `.cmd`, `.com`) with a
///    configuration error so the operator sees the problem, not a cryptic
///    SQL syntax error from the server. `.sql` / `.txt` / missing are
///    allowed. Comparison is case-insensitive (`query.SQL` works).
/// 3. **Size cap.** Refuse files larger than [`MAX_QUERY_FILE_SIZE_BYTES`]
///    to bound memory use and avoid OOM on attacker-chosen large inputs.
///
/// Failures use [`GoldDiggerError::Config`] (exit 2) for policy rejections
/// and [`GoldDiggerError::Io`] (exit 5) for filesystem / stat failures.
pub fn validate_query_file_path(path: &Path) -> Result<PathBuf> {
    // 1. Canonicalize. Broken symlinks and missing files fail here with
    //    a real `io::Error`, which gets wrapped via `GoldDiggerError::Io`
    //    (exit 5) for stable routing.
    let canonical = std::fs::canonicalize(path).map_err(|e| {
        anyhow::Error::from(GoldDiggerError::Io(e)).context(format!(
            "Failed to canonicalize query file path {}",
            path.display()
        ))
    })?;

    // 2. Extension deny-list (case-insensitive; missing is allowed).
    if let Some(ext) = canonical.extension().and_then(|s| s.to_str()) {
        let lower = ext.to_ascii_lowercase();
        if REFUSED_QUERY_FILE_EXTENSIONS.iter().any(|r| *r == lower) {
            return Err(GoldDiggerError::Config(format!(
                "Refusing to read query file with disallowed extension '.{}': {}. \
                 Use --query-file with .sql, .txt, or no extension.",
                lower,
                canonical.display()
            ))
            .into());
        }
    }

    // 3. Size cap. Use metadata on the canonical path.
    let metadata = std::fs::metadata(&canonical).map_err(|e| {
        anyhow::Error::from(GoldDiggerError::Io(e))
            .context(format!("Failed to stat query file {}", canonical.display()))
    })?;
    if metadata.len() > MAX_QUERY_FILE_SIZE_BYTES {
        return Err(GoldDiggerError::Config(format!(
            "Query file {} is {} bytes; maximum allowed is {} bytes (10 MiB). \
             Split the query or raise MAX_QUERY_FILE_SIZE_BYTES.",
            canonical.display(),
            metadata.len(),
            MAX_QUERY_FILE_SIZE_BYTES
        ))
        .into());
    }

    Ok(canonical)
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
    if let Some(query) = &cli.query {
        Ok(query.clone())
    } else if let Some(query_file) = &cli.query_file {
        let canonical = validate_query_file_path(query_file)?;
        std::fs::read_to_string(&canonical).map_err(|e| {
            // Preserve both the typed I/O classification and the original
            // path context that previous integration tests assert on.
            anyhow::Error::from(GoldDiggerError::Io(e)).context(format!(
                "Failed to read query file {}",
                query_file.display()
            ))
        })
    } else {
        std::env::var("DATABASE_QUERY").map_err(|_| {
            GoldDiggerError::Config(
                "Missing database query. Provide --query, --query-file, or set DATABASE_QUERY environment variable".into(),
            )
            .into()
        })
    }
}

/// Resolves the output file path from CLI arguments or environment variables.
///
/// Errors are wrapped in [`GoldDiggerError::Config`] for stable exit-code
/// classification.
pub fn resolve_output_file(cli: &Cli) -> Result<PathBuf> {
    if let Some(output) = &cli.output {
        Ok(output.clone())
    } else {
        std::env::var("OUTPUT_FILE")
            .map(PathBuf::from)
            .map_err(|_| {
                GoldDiggerError::Config(
                    "Missing output file. Provide --output or set OUTPUT_FILE environment variable"
                        .into(),
                )
                .into()
            })
    }
}

/// Dumps current configuration as JSON with proper credential redaction.
///
/// The query (whether from `--query` or `DATABASE_QUERY`) is routed
/// through [`redact_dump_query`], which delegates to the same regex set
/// that scrubs MySQL error messages. There is exactly one redactor in
/// the codebase ([`crate::utils`]) so a fix to a missed pattern
/// (e.g. `passwd=`, `pwd=`, `Kennwort=`, `mot_de_passe=`) lands in every
/// surface at once.
pub fn dump_configuration(cli: &Cli) -> Result<()> {
    use serde_json::json;

    // Route the query through the canonical redactor. The previous
    // implementation used a substring check for "password" / "identified
    // by" and replaced the entire query with a sentinel — that missed
    // `pwd=`, `passwd=`, GRANT/SET PASSWORD, base64/JWT/hex blobs,
    // non-English labels, and erased the legitimate query when it
    // matched. The shared regex set in `utils::redact_sql_error` covers
    // all of those and only redacts the offending substrings.
    let query_from_env = env::var("DATABASE_QUERY").ok();
    let redacted_query = cli
        .query
        .as_ref()
        .or(query_from_env.as_ref())
        .map(|q| redact_dump_query(q))
        .unwrap_or_default();

    let config = json!({
        "database_url": "***REDACTED***", // Always redact database URLs
        "query": redacted_query,
        "query_file": cli.query_file.as_ref().map(|p| p.display().to_string()),
        "output": cli.output.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| env::var("OUTPUT_FILE").unwrap_or_default()),
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
            "verbose": cfg!(feature = "verbose"),
            "additional_mysql_types": cfg!(feature = "additional_mysql_types"),
            "tls": true  // TLS is always available (rustls-only implementation)
        }
    });

    println!("{}", serde_json::to_string_pretty(&config)?);
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
        let cli = Cli::parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--output",
            "test.json",
        ]);

        // Set environment variable using temp_env
        temp_env::with_var("DATABASE_QUERY", Some("SELECT * FROM env_table"), || {
            let result = resolve_database_query(&cli);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "SELECT * FROM env_table");
        });
    }

    #[test]
    fn test_resolve_database_query_missing() {
        let cli = Cli::parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--output",
            "test.json",
        ]);

        // Ensure env var is not set using temp_env
        temp_env::with_var("DATABASE_QUERY", None::<&str>, || {
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
        let cli = Cli::parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
        ]);

        // Set environment variable using temp_env
        temp_env::with_var("OUTPUT_FILE", Some("/tmp/env_output.csv"), || {
            let result = resolve_output_file(&cli);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), PathBuf::from("/tmp/env_output.csv"));
        });
    }

    #[test]
    fn test_resolve_output_file_missing() {
        let cli = Cli::parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
        ]);

        // Ensure env var is not set using temp_env
        temp_env::with_var("OUTPUT_FILE", None::<&str>, || {
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
