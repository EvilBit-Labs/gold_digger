//! Binary entry point for `gold_digger`.
//!
//! Parses CLI args (with `DATABASE_URL` / `DATABASE_QUERY` / `OUTPUT_FILE`
//! env fallbacks), opens a rustls-backed MySQL pool, executes the query, and
//! dispatches to the writer selected by `--format` or by file extension.
//! Exit codes follow the 0-5 contract defined in [`gold_digger::exit`].

use std::{env, fs::File, path::PathBuf};

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use clap_complete::{Shell as CompletionShell, generate};
use mysql::Pool;
use mysql::prelude::Queryable;

use gold_digger::cli::{Cli, Commands, OutputFormat, Shell};
use gold_digger::exit::{GoldDiggerError, exit_no_rows, exit_success, exit_with_error};
use gold_digger::rows_to_strings;
use gold_digger::utils::{redact_dump_query, redact_sql_error};
use std::collections::BTreeMap;

use gold_digger::tls::create_tls_connection;

/// Main entry point for the gold_digger CLI tool.
///
/// Parses CLI arguments and environment variables, executes a database query, and writes the output in the specified format.
fn main() {
    // Initialize crypto provider for rustls
    gold_digger::init_crypto_provider();

    let cli = Cli::parse();

    // Handle subcommands first
    if let Some(command) = cli.command {
        match command {
            Commands::Completion { shell } => {
                generate_completion(shell);
                return;
            }
        }
    }

    // Handle --dump-config flag
    if cli.dump_config {
        if let Err(e) = dump_configuration(&cli) {
            exit_with_error(e, Some("Configuration dump failed"));
        }
        return;
    }

    // Resolve configuration with precedence: CLI flags > environment variables
    let database_url = match resolve_database_url(&cli) {
        Ok(url) => url,
        Err(e) => exit_with_error(e, Some("Database URL resolution failed")),
    };
    let database_query = match resolve_database_query(&cli) {
        Ok(query) => query,
        Err(e) => exit_with_error(e, Some("Database query resolution failed")),
    };
    let output_file = match resolve_output_file(&cli) {
        Ok(file) => file,
        Err(e) => exit_with_error(e, Some("Output file resolution failed")),
    };

    if cli.verbose > 0 && !cli.quiet {
        eprintln!("Connecting to database...");
    }

    let pool = match create_database_connection(&database_url, &cli) {
        Ok(pool) => pool,
        Err(e) => exit_with_error(
            anyhow::anyhow!("Database connection pool creation failed: {}", e),
            None,
        ),
    };
    let mut conn = match pool.get_conn() {
        Ok(conn) => conn,
        Err(e) => exit_with_error(anyhow::anyhow!("Database connection failed: {}", e), None),
    };

    let result: Vec<mysql::Row> = match conn.query(&database_query) {
        Ok(result) => result,
        Err(e) => {
            // Structured error matching on mysql::Error variants
            let context = match &e {
                mysql::Error::MySqlError(mysql_err) => {
                    // Map known MySQL error codes to contextual messages
                    match mysql_err.code {
                        1064 => "SQL syntax error in query", // ER_PARSE_ERROR
                        1146 => "Table does not exist",      // ER_NO_SUCH_TABLE
                        1054 => "Column does not exist or is ambiguous", // ER_BAD_FIELD_ERROR
                        1045 => "Access denied - invalid credentials", // ER_ACCESS_DENIED_ERROR
                        1044 => "Access denied to database", // ER_DBACCESS_DENIED_ERROR
                        1142 => "Insufficient privileges for query execution", // ER_TABLEACCESS_DENIED_ERROR
                        1143 => "Insufficient column privileges", // ER_COLUMNACCESS_DENIED_ERROR
                        1049 => "Unknown database",               // ER_BAD_DB_ERROR
                        2002 => "Connection failed - server not reachable", // CR_CONNECTION_ERROR
                        2003 => "Connection failed - server not responding", // CR_CONN_HOST_ERROR
                        2006 => "Connection lost - server has gone away", // CR_SERVER_GONE_ERROR
                        2013 => "Connection lost during query",   // CR_SERVER_LOST
                        _ => "Query execution failed",
                    }
                }
                mysql::Error::IoError(_) => "Network I/O error during query execution",
                mysql::Error::UrlError(_) => "Invalid database URL format",
                mysql::Error::DriverError(_) => "Database driver error",
                _ => "Query execution failed",
            };

            // Always include redacted error detail so users can diagnose issues
            let error_message = format!("{}: {}", context, redact_sql_error(&e.to_string()));

            exit_with_error(anyhow::anyhow!("{}", error_message), None);
        }
    };

    if cli.verbose > 0 && !cli.quiet {
        eprintln!(
            "Outputting {} records to {}.",
            result.len(),
            output_file.display()
        );
    }

    if result.is_empty() {
        if cli.allow_empty {
            if cli.verbose > 0 && !cli.quiet {
                eprintln!("No records found in database, but --allow-empty is set.");
            }
            let empty_rows: Vec<mysql::Row> = vec![];
            if let Err(e) = write_output(empty_rows, output_file.as_path(), &cli) {
                exit_with_error(e, Some("Output writing failed"));
            }
        } else {
            if cli.verbose > 0 && !cli.quiet {
                eprintln!("No records found in database.");
            }
            if cli.quiet {
                exit_no_rows(None);
            } else {
                exit_no_rows(Some("No records found in database"));
            }
        }
    } else if let Err(e) = write_output(result, output_file.as_path(), &cli) {
        exit_with_error(e, Some("Output writing failed"));
    }

    exit_success(None);
}

/// Creates a database connection pool with rustls-only TLS configuration from CLI.
///
/// The `TlsOptions` → `TlsConfig` adapter now lives in [`gold_digger::cli`]
/// (todo #045) so the `tls` module has zero dependency on CLI types. The
/// second-confirmation gate for `--allow-invalid-certificate` is enforced by
/// [`gold_digger::cli::TlsOptions::to_tls_config`] (todo #022); bare
/// `--allow-invalid-certificate` without `--i-understand-this-is-insecure`
/// (or the `GOLD_DIGGER_ALLOW_INVALID=1` env var) returns a config error here.
fn create_database_connection(database_url: &str, cli: &Cli) -> Result<Pool> {
    // Create TLS configuration from CLI options
    let tls_config = if cli.tls_options.tls_ca_file.is_some()
        || cli.tls_options.insecure_skip_hostname_verify
        || cli.tls_options.allow_invalid_certificate
    {
        let config = cli
            .tls_options
            .to_tls_config()
            .map_err(|e| anyhow::anyhow!("TLS configuration error: {}", e))?;

        // Display security warnings for insecure modes (includes the
        // mandatory DANGER delay for AcceptInvalid — see #022).
        config.display_security_warnings();

        Some(config)
    } else {
        // Use default TLS behavior when no explicit TLS flags are provided
        // This will use platform certificate store with rustls
        None
    };

    // Use rustls-only TLS connection creation with enhanced error handling
    create_tls_connection(database_url, tls_config, cli.verbose > 0).map_err(|tls_error| {
        // Convert TLS errors to anyhow errors with appropriate context
        match &tls_error {
            gold_digger::tls::TlsError::CertificateValidationFailed { .. }
            | gold_digger::tls::TlsError::CertificateTimeInvalid { .. }
            | gold_digger::tls::TlsError::InvalidSignature { .. }
            | gold_digger::tls::TlsError::UnknownCertificateAuthority { .. }
            | gold_digger::tls::TlsError::InvalidCertificatePurpose { .. }
            | gold_digger::tls::TlsError::CertificateChainInvalid { .. }
            | gold_digger::tls::TlsError::CertificateRevoked { .. } => {
                // Certificate validation errors - suggest appropriate CLI flag
                if let Some(suggestion) = tls_error.suggest_cli_flag() {
                    anyhow::anyhow!("{}. Suggestion: {}", tls_error, suggestion)
                } else {
                    anyhow::anyhow!("{}", tls_error)
                }
            }
            gold_digger::tls::TlsError::HostnameVerificationFailed { .. } => {
                // Hostname verification errors - suggest skip hostname flag
                anyhow::anyhow!(
                    "{}. Suggestion: {}",
                    tls_error,
                    tls_error
                        .suggest_cli_flag()
                        .unwrap_or("--insecure-skip-hostname-verify")
                )
            }
            gold_digger::tls::TlsError::CaFileNotFound { .. }
            | gold_digger::tls::TlsError::InvalidCaFormat { .. }
            | gold_digger::tls::TlsError::MutuallyExclusiveFlags { .. } => {
                // Client configuration errors - no additional context needed
                anyhow::anyhow!("{}", tls_error)
            }
            _ => {
                // Other TLS errors (handshake, connection, server issues)
                anyhow::anyhow!("Database connection failed: {}", tls_error)
            }
        }
    })
}

/// Resolves the database URL from CLI arguments or environment variables.
///
/// Errors are wrapped in [`GoldDiggerError::Config`] so the exit-code
/// classifier can identify them via downcast (stable across message text
/// refactors).
fn resolve_database_url(cli: &Cli) -> Result<String> {
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

/// Resolves the database query from CLI arguments, an external file, or
/// environment variables.
///
/// File-read failures map to [`GoldDiggerError::Io`] (exit 5); missing
/// configuration maps to [`GoldDiggerError::Config`] (exit 2).
fn resolve_database_query(cli: &Cli) -> Result<String> {
    if let Some(query) = &cli.query {
        Ok(query.clone())
    } else if let Some(query_file) = &cli.query_file {
        std::fs::read_to_string(query_file).map_err(|e| {
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
fn resolve_output_file(cli: &Cli) -> Result<PathBuf> {
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

/// Writes output in the specified format.
///
/// For JSON output, uses `TypeTransformer` to preserve native MySQL types (integers
/// as JSON numbers, NULLs as JSON null, etc.). For CSV and TSV, converts rows to
/// strings first via `rows_to_strings`, ensuring conversion succeeds before
/// creating/truncating the output file.
///
/// Format selection (todo #019): if `--format` is absent AND the output file
/// extension is unknown (or missing), returns [`GoldDiggerError::Config`]
/// instead of silently defaulting to TSV. The previous behaviour surfaced a
/// "silent format selection" hazard — an `.xml` or `.yaml` output path would
/// quietly emit tab-separated data with no signal to the caller.
fn write_output(rows: Vec<mysql::Row>, output_file: &std::path::Path, cli: &Cli) -> Result<()> {
    let format = if let Some(format) = &cli.format {
        format.clone()
    } else {
        OutputFormat::from_extension(output_file).ok_or_else(|| {
            GoldDiggerError::Config(format!(
                "Cannot infer output format from '{}'. Recognised extensions: .csv, .json, .tsv, .tab, .txt. Pass --format <csv|json|tsv> to select explicitly.",
                output_file.display()
            ))
        })?
    };

    match format {
        OutputFormat::Csv => {
            let string_rows = rows_to_strings(rows)?;
            let output = File::create(output_file).context("Failed to create output file")?;
            gold_digger::csv::write(string_rows, output)?;
        }
        OutputFormat::Json => {
            use gold_digger::TypeTransformer;

            // Convert rows to JSON maps before creating the file to avoid
            // leaving an empty/truncated file on conversion failure.
            let json_maps: Vec<BTreeMap<String, serde_json::Value>> = rows
                .into_iter()
                .enumerate()
                .map(|(i, row)| {
                    TypeTransformer::row_to_json(row)
                        .with_context(|| format!("Failed to convert row {}", i + 1))
                })
                .collect::<Result<Vec<_>>>()?;

            let output = File::create(output_file).context("Failed to create output file")?;
            gold_digger::json::write(json_maps, output, cli.pretty)?;
        }
        OutputFormat::Tsv => {
            let string_rows = rows_to_strings(rows)?;
            let output = File::create(output_file).context("Failed to create output file")?;
            gold_digger::tab::write(string_rows, output)?;
        }
    }

    Ok(())
}

/// Generates shell completion scripts
fn generate_completion(shell: Shell) {
    let mut cmd = Cli::command();
    let bin_name = "gold_digger";

    match shell {
        Shell::Bash => generate(
            CompletionShell::Bash,
            &mut cmd,
            bin_name,
            &mut std::io::stdout(),
        ),
        Shell::Zsh => generate(
            CompletionShell::Zsh,
            &mut cmd,
            bin_name,
            &mut std::io::stdout(),
        ),
        Shell::Fish => generate(
            CompletionShell::Fish,
            &mut cmd,
            bin_name,
            &mut std::io::stdout(),
        ),
        Shell::PowerShell => generate(
            CompletionShell::PowerShell,
            &mut cmd,
            bin_name,
            &mut std::io::stdout(),
        ),
    }
}

/// Dumps current configuration as JSON with proper credential redaction.
///
/// The query (whether from `--query` or `DATABASE_QUERY`) is routed
/// through [`redact_dump_query`], which delegates to the same regex set
/// that scrubs MySQL error messages. There is exactly one redactor in
/// the codebase ([`gold_digger::utils`]) so a fix to a missed pattern
/// (e.g. `passwd=`, `pwd=`, `Kennwort=`, `mot_de_passe=`) lands in every
/// surface at once.
fn dump_configuration(cli: &Cli) -> Result<()> {
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
    use tempfile::NamedTempFile;

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

    /// Creates a CLI instance with TLS options for testing
    fn build_test_cli_with_tls() -> Cli {
        Cli::parse_from([
            "gold_digger",
            "--db-url",
            "mysql://test",
            "--query",
            "SELECT 1",
            "--output",
            "test.json",
            "--insecure-skip-hostname-verify",
        ])
    }

    #[test]
    fn test_create_database_connection_invalid_url() {
        // Test with invalid URL to ensure error handling works
        let cli = build_test_cli();
        let result = create_database_connection("invalid://url", &cli);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_database_connection() {
        // Test that the function exists and handles errors properly
        let cli = build_test_cli();
        let result =
            create_database_connection("mysql://invalid:invalid@nonexistent:3306/test", &cli);
        // Should fail due to invalid connection details, but not panic
        assert!(result.is_err());
    }

    #[test]
    fn test_create_database_connection_with_tls_options() {
        // Test TLS configuration path
        let cli = build_test_cli_with_tls();
        let result =
            create_database_connection("mysql://invalid:invalid@nonexistent:3306/test", &cli);
        // Should fail due to invalid connection details, but TLS config should be processed
        assert!(result.is_err());
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
        let mut temp_file = NamedTempFile::new()?;
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
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to read query file")
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
    #[test]
    fn test_generate_completion_bash() {
        use gold_digger::cli::Shell;
        // This should not panic
        generate_completion(Shell::Bash);
    }

    #[test]
    fn test_generate_completion_zsh() {
        use gold_digger::cli::Shell;
        // This should not panic
        generate_completion(Shell::Zsh);
    }

    #[test]
    fn test_generate_completion_fish() {
        use gold_digger::cli::Shell;
        // This should not panic
        generate_completion(Shell::Fish);
    }

    #[test]
    fn test_generate_completion_powershell() {
        use gold_digger::cli::Shell;
        // This should not panic
        generate_completion(Shell::PowerShell);
    }
}
