use std::{env, fs::File, path::PathBuf};

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use clap_complete::{Shell as CompletionShell, generate};
use mysql::Pool;
use mysql::prelude::Queryable;

use gold_digger::cli::{Cli, Commands, OutputFormat, Shell};
use gold_digger::exit::{exit_no_rows, exit_success, exit_with_error};
use gold_digger::rows_to_strings;

use gold_digger::tls::{TlsConfig, create_tls_connection};

/// Redacts sensitive information from SQL error messages
fn redact_sql_error(message: &str) -> String {
    // Simple redaction using string replacement for common sensitive patterns
    let mut redacted = message.to_string();
    let lower_msg = message.to_lowercase();

    // Redact common sensitive patterns
    if lower_msg.contains("password") {
        redacted = redacted.replace("password", "***REDACTED***");
    }
    if lower_msg.contains("identified by") {
        redacted = redacted.replace("identified by", "***REDACTED***");
    }
    if lower_msg.contains("token") {
        redacted = redacted.replace("token", "***REDACTED***");
    }
    if lower_msg.contains("secret") {
        redacted = redacted.replace("secret", "***REDACTED***");
    }
    if lower_msg.contains("key") && lower_msg.contains("=") {
        redacted = redacted.replace("key", "***REDACTED***");
    }

    redacted
}

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
            },
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

    let pool = match create_database_connection(&database_url, &cli) {
        Ok(pool) => pool,
        Err(e) => exit_with_error(anyhow::anyhow!("Database connection pool creation failed: {}", e), None),
    };
    let mut conn = match pool.get_conn() {
        Ok(conn) => conn,
        Err(e) => exit_with_error(anyhow::anyhow!("Database connection failed: {}", e), None),
    };

    if cli.verbose > 0 && !cli.quiet {
        println!("Connecting to database...");
    }

    let result: Vec<mysql::Row> = match conn.query(&database_query) {
        Ok(result) => result,
        Err(e) => {
            // Structured error matching on mysql::Error variants
            let (context, _should_show_details) = match &e {
                mysql::Error::MySqlError(mysql_err) => {
                    // Map known MySQL error codes to contextual messages
                    let context = match mysql_err.code {
                        1064 => "SQL syntax error in query",                   // ER_PARSE_ERROR
                        1146 => "Table does not exist",                        // ER_NO_SUCH_TABLE
                        1054 => "Column does not exist or is ambiguous",       // ER_BAD_FIELD_ERROR
                        1045 => "Access denied - invalid credentials",         // ER_ACCESS_DENIED_ERROR
                        1044 => "Access denied to database",                   // ER_DBACCESS_DENIED_ERROR
                        1142 => "Insufficient privileges for query execution", // ER_TABLEACCESS_DENIED_ERROR
                        1143 => "Insufficient column privileges",              // ER_COLUMNACCESS_DENIED_ERROR
                        1049 => "Unknown database",                            // ER_BAD_DB_ERROR
                        2002 => "Connection failed - server not reachable",    // CR_CONNECTION_ERROR
                        2003 => "Connection failed - server not responding",   // CR_CONN_HOST_ERROR
                        2006 => "Connection lost - server has gone away",      // CR_SERVER_GONE_ERROR
                        2013 => "Connection lost during query",                // CR_SERVER_LOST
                        _ => "Query execution failed",
                    };
                    (context, true)
                },
                mysql::Error::IoError(_) => ("Network I/O error during query execution", false),
                mysql::Error::UrlError(_) => ("Invalid database URL format", false),
                mysql::Error::DriverError(_) => ("Database driver error", false),
                _ => ("Query execution failed", false),
            };

            // Create error message with appropriate level of detail
            let error_message = if cli.verbose > 0 && _should_show_details {
                format!("{}: {}", context, redact_sql_error(&e.to_string()))
            } else {
                context.to_string()
            };

            exit_with_error(anyhow::anyhow!("{}", error_message), Some("Database query failed"));
        },
    };

    if cli.verbose > 0 && !cli.quiet {
        println!("Outputting {} records to {}.", result.len(), output_file.display());
    }

    if result.is_empty() {
        if cli.allow_empty {
            if cli.verbose > 0 && !cli.quiet {
                println!("No records found in database, but --allow-empty is set.");
            }
            // Create empty output file
            let output = match File::create(&output_file) {
                Ok(output) => output,
                Err(e) => exit_with_error(anyhow::anyhow!("Failed to create output file: {}", e), None),
            };
            let empty_rows: Vec<Vec<String>> = vec![];
            if let Err(e) = write_output(empty_rows, output, output_file.as_path(), &cli) {
                exit_with_error(e, Some("Output writing failed"));
            }
        } else {
            if cli.verbose > 0 && !cli.quiet {
                println!("No records found in database.");
            }
            exit_no_rows(Some("No records found in database"));
        }
    } else {
        let rows = match rows_to_strings(result) {
            Ok(rows) => rows,
            Err(e) => exit_with_error(e, Some("Row conversion failed")),
        };
        let output = match File::create(&output_file) {
            Ok(output) => output,
            Err(e) => exit_with_error(anyhow::anyhow!("Failed to create output file: {}", e), None),
        };
        if let Err(e) = write_output(rows, output, output_file.as_path(), &cli) {
            exit_with_error(e, Some("Output writing failed"));
        }
    }

    exit_success(None);
}

/// Creates a database connection pool with rustls-only TLS configuration from CLI
fn create_database_connection(database_url: &str, cli: &Cli) -> Result<Pool> {
    // Create TLS configuration from CLI options
    let tls_config = if cli.tls_options.tls_ca_file.is_some()
        || cli.tls_options.insecure_skip_hostname_verify
        || cli.tls_options.allow_invalid_certificate
    {
        let config = TlsConfig::from_tls_options(&cli.tls_options)
            .map_err(|e| anyhow::anyhow!("TLS configuration error: {}", e))?;

        // Display security warnings for insecure modes
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
            },
            gold_digger::tls::TlsError::HostnameVerificationFailed { .. } => {
                // Hostname verification errors - suggest skip hostname flag
                anyhow::anyhow!(
                    "{}. Suggestion: {}",
                    tls_error,
                    tls_error
                        .suggest_cli_flag()
                        .unwrap_or("--insecure-skip-hostname-verify")
                )
            },
            gold_digger::tls::TlsError::CaFileNotFound { .. }
            | gold_digger::tls::TlsError::InvalidCaFormat { .. }
            | gold_digger::tls::TlsError::MutuallyExclusiveFlags { .. } => {
                // Client configuration errors - no additional context needed
                anyhow::anyhow!("{}", tls_error)
            },
            _ => {
                // Other TLS errors (handshake, connection, server issues)
                anyhow::anyhow!("Database connection failed: {}", tls_error)
            },
        }
    })
}

/// Resolves the database URL from CLI arguments or environment variables
fn resolve_database_url(cli: &Cli) -> Result<String> {
    if let Some(url) = &cli.db_url {
        Ok(url.clone())
    } else {
        gold_digger::get_required_env("DATABASE_URL")
            .context("Missing database URL. Provide --db-url or set DATABASE_URL environment variable")
    }
}

/// Resolves the database query from CLI arguments or environment variables
fn resolve_database_query(cli: &Cli) -> Result<String> {
    if let Some(query) = &cli.query {
        Ok(query.clone())
    } else if let Some(query_file) = &cli.query_file {
        std::fs::read_to_string(query_file)
            .map_err(|e| anyhow::anyhow!("Failed to read query file {}: {}", query_file.display(), e))
    } else {
        gold_digger::get_required_env("DATABASE_QUERY").context(
            "Missing database query. Provide --query, --query-file, or set DATABASE_QUERY environment variable",
        )
    }
}

/// Resolves the output file path from CLI arguments or environment variables
fn resolve_output_file(cli: &Cli) -> Result<PathBuf> {
    if let Some(output) = &cli.output {
        Ok(output.clone())
    } else {
        let output = gold_digger::get_required_env("OUTPUT_FILE")
            .context("Missing output file. Provide --output or set OUTPUT_FILE environment variable")?;
        Ok(PathBuf::from(output))
    }
}

/// Writes output in the specified format
fn write_output(rows: Vec<Vec<String>>, output: File, output_file: &std::path::Path, cli: &Cli) -> Result<()> {
    let format = if let Some(format) = &cli.format {
        format.clone()
    } else {
        OutputFormat::from_extension(output_file)
    };

    match format {
        #[cfg(feature = "csv")]
        OutputFormat::Csv => gold_digger::csv::write(rows, output)?,
        #[cfg(feature = "json")]
        OutputFormat::Json => gold_digger::json::write_with_pretty(rows, output, cli.pretty)?,
        OutputFormat::Tsv => gold_digger::tab::write(rows, output)?,
        #[cfg(not(feature = "csv"))]
        OutputFormat::Csv => anyhow::bail!("CSV support not compiled in"),
        #[cfg(not(feature = "json"))]
        OutputFormat::Json => anyhow::bail!("JSON support not compiled in"),
    }

    Ok(())
}

/// Generates shell completion scripts
fn generate_completion(shell: Shell) {
    let mut cmd = Cli::command();
    let bin_name = "gold_digger";

    match shell {
        Shell::Bash => generate(CompletionShell::Bash, &mut cmd, bin_name, &mut std::io::stdout()),
        Shell::Zsh => generate(CompletionShell::Zsh, &mut cmd, bin_name, &mut std::io::stdout()),
        Shell::Fish => generate(CompletionShell::Fish, &mut cmd, bin_name, &mut std::io::stdout()),
        Shell::PowerShell => generate(CompletionShell::PowerShell, &mut cmd, bin_name, &mut std::io::stdout()),
    }
}

/// Dumps current configuration as JSON with proper credential redaction
fn dump_configuration(cli: &Cli) -> Result<()> {
    use serde_json::json;

    // Safely redact query content that might contain sensitive data
    let query_from_env = env::var("DATABASE_QUERY").ok();
    let redacted_query = cli
        .query
        .as_ref()
        .or(query_from_env.as_ref())
        .map(|q| {
            // Redact potential passwords in SQL queries
            if q.to_lowercase().contains("password") || q.to_lowercase().contains("identified by") {
                "***QUERY_WITH_CREDENTIALS_REDACTED***".to_string()
            } else {
                q.clone()
            }
        })
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
            "json": cfg!(feature = "json"),
            "csv": cfg!(feature = "csv"),
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
        let result = create_database_connection("mysql://invalid:invalid@nonexistent:3306/test", &cli);
        // Should fail due to invalid connection details, but not panic
        assert!(result.is_err());
    }

    #[test]
    fn test_create_database_connection_with_tls_options() {
        // Test TLS configuration path
        let cli = build_test_cli_with_tls();
        let result = create_database_connection("mysql://invalid:invalid@nonexistent:3306/test", &cli);
        // Should fail due to invalid connection details, but TLS config should be processed
        assert!(result.is_err());
    }

    #[test]
    fn test_redact_sql_error() {
        // Test that sensitive information is redacted from error messages
        let error_with_password = "Error: Access denied for user 'test' (using password: YES)";
        let redacted = redact_sql_error(error_with_password);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("password"));

        let error_with_identified_by = "Error: CREATE USER failed with identified by 'secret123'";
        let redacted = redact_sql_error(error_with_identified_by);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("identified by"));

        let error_with_token = "Error: Invalid token abc123";
        let redacted = redact_sql_error(error_with_token);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("token"));

        let error_with_secret = "Error: Invalid secret key";
        let redacted = redact_sql_error(error_with_secret);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("secret"));

        let error_with_key = "Error: api_key=sensitive_value";
        let redacted = redact_sql_error(error_with_key);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("key"));

        let normal_error = "Error: Table 'test.users' doesn't exist";
        let redacted = redact_sql_error(normal_error);
        assert_eq!(redacted, normal_error); // Should be unchanged
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
        let cli = Cli::parse_from(["gold_digger", "--query", "SELECT 1", "--output", "test.json"]);

        // Set environment variable using temp_env
        temp_env::with_var("DATABASE_URL", Some("mysql://env_test"), || {
            let result = resolve_database_url(&cli);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "mysql://env_test");
        });
    }

    #[test]
    fn test_resolve_database_url_missing() {
        let cli = Cli::parse_from(["gold_digger", "--query", "SELECT 1", "--output", "test.json"]);

        // Ensure env var is not set using temp_env
        temp_env::with_var("DATABASE_URL", None::<&str>, || {
            let result = resolve_database_url(&cli);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Missing database URL"));
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
        let cli = Cli::parse_from(["gold_digger", "--db-url", "mysql://test", "--output", "test.json"]);

        // Set environment variable using temp_env
        temp_env::with_var("DATABASE_QUERY", Some("SELECT * FROM env_table"), || {
            let result = resolve_database_query(&cli);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "SELECT * FROM env_table");
        });
    }

    #[test]
    fn test_resolve_database_query_missing() {
        let cli = Cli::parse_from(["gold_digger", "--db-url", "mysql://test", "--output", "test.json"]);

        // Ensure env var is not set using temp_env
        temp_env::with_var("DATABASE_QUERY", None::<&str>, || {
            let result = resolve_database_query(&cli);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Missing database query"));
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
        assert!(result.unwrap_err().to_string().contains("Failed to read query file"));
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
        let cli = Cli::parse_from(["gold_digger", "--db-url", "mysql://test", "--query", "SELECT 1"]);

        // Set environment variable using temp_env
        temp_env::with_var("OUTPUT_FILE", Some("/tmp/env_output.csv"), || {
            let result = resolve_output_file(&cli);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), PathBuf::from("/tmp/env_output.csv"));
        });
    }

    #[test]
    fn test_resolve_output_file_missing() {
        let cli = Cli::parse_from(["gold_digger", "--db-url", "mysql://test", "--query", "SELECT 1"]);

        // Ensure env var is not set using temp_env
        temp_env::with_var("OUTPUT_FILE", None::<&str>, || {
            let result = resolve_output_file(&cli);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Missing output file"));
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
            let cli = Cli::parse_from(["gold_digger", "--db-url", "mysql://test", "--output", "test.json"]);

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
