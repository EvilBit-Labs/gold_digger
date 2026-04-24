//! Query-execution pipeline.
//!
//! Ties together config resolution, connection establishment, query
//! execution, and output dispatch. The binary entry point (`src/main.rs`)
//! handles CLI parsing, logging init, and subcommand / `--dump-config`
//! dispatch; once those are out of the way it delegates to [`run`].
//!
//! All failures are routed through [`crate::exit::exit_with_error`] so the
//! binary always exits with a stable, documented code.

use mysql::prelude::Queryable;

use crate::cli::Cli;
use crate::config::{resolve_database_query, resolve_database_url, resolve_output_file};
use crate::connection::create_database_connection;
use crate::exit::{exit_no_rows, exit_success, exit_with_error};
use crate::logging::make_progress;
use crate::output::write_output;
use crate::utils::redact_sql_error;

/// Executes the main query pipeline: resolve config → create pool → run
/// query → dispatch to writer.
///
/// Never returns to the caller: every termination path goes through the
/// [`crate::exit`] helpers so the binary always exits with a stable,
/// documented code (0 success, 1 no rows, 2 config, 3 auth, 4 query, 5 I/O).
pub fn run(cli: Cli) -> ! {
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

    // Connect phase: spinner (indeterminate duration). Hidden when
    // `--quiet` or stderr is not a TTY (todo #162).
    let connect_progress = make_progress(cli.quiet, None, "Connecting to database...");
    tracing::info!("Connecting to database...");

    let pool = match create_database_connection(&database_url, &cli) {
        Ok(pool) => pool,
        Err(e) => {
            connect_progress.finish_and_clear();
            exit_with_error(
                anyhow::anyhow!("Database connection pool creation failed: {}", e),
                None,
            )
        }
    };
    let mut conn = match pool.get_conn() {
        Ok(conn) => conn,
        Err(e) => {
            connect_progress.finish_and_clear();
            exit_with_error(anyhow::anyhow!("Database connection failed: {}", e), None)
        }
    };
    connect_progress.finish_and_clear();

    // Query phase: spinner (indeterminate duration).
    let query_progress = make_progress(cli.quiet, None, "Executing query...");

    let result: Vec<mysql::Row> = match conn.query(&database_query) {
        Ok(result) => result,
        Err(e) => {
            query_progress.finish_and_clear();
            exit_with_error(map_query_error(&e), None);
        }
    };
    query_progress.finish_and_clear();

    tracing::info!(
        rows = result.len(),
        file = %output_file.display(),
        "Outputting {} records to {}.",
        result.len(),
        output_file.display()
    );

    if result.is_empty() {
        handle_empty_result(&cli, output_file.as_path());
    } else {
        write_rows(&cli, result, output_file.as_path());
    }

    exit_success(None);
}

/// Handles an empty result set. If `--allow-empty` is set, writes an
/// empty output file and returns; otherwise exits with [`exit_no_rows`].
fn handle_empty_result(cli: &Cli, output_file: &std::path::Path) {
    if cli.allow_empty {
        tracing::info!("No records found in database, but --allow-empty is set.");
        let empty_rows: Vec<mysql::Row> = vec![];
        if let Err(e) = write_output(empty_rows, output_file, cli) {
            exit_with_error(e, Some("Output writing failed"));
        }
    } else {
        tracing::info!("No records found in database.");
        if cli.quiet {
            exit_no_rows(None);
        } else {
            exit_no_rows(Some("No records found in database"));
        }
    }
}

/// Writes a non-empty result set, attaching a progress bar sized to the
/// known row count. `result` is already fully materialised here (streaming
/// is a separate todo — #005), so we size the bar to the row count and
/// advance in a single step after `write_output` returns. This still gives
/// users useful feedback (bar appears briefly, ETA resolves, bar completes)
/// for multi-second writes of large result sets; the bar is hidden under
/// `--quiet` or when stderr is not a TTY.
fn write_rows(cli: &Cli, result: Vec<mysql::Row>, output_file: &std::path::Path) {
    let total_rows = u64::try_from(result.len()).unwrap_or(u64::MAX);
    let write_progress = make_progress(
        cli.quiet,
        Some(total_rows),
        &format!("Writing {} rows...", total_rows),
    );

    if let Err(e) = write_output(result, output_file, cli) {
        write_progress.finish_and_clear();
        exit_with_error(e, Some("Output writing failed"));
    }
    write_progress.set_position(total_rows);
    write_progress.finish_and_clear();
}

/// Maps a [`mysql::Error`] from query execution into a contextual
/// `anyhow::Error`. Known MySQL error codes are translated into
/// operator-facing messages; the underlying error is always appended
/// (via [`redact_sql_error`]) so the caller can diagnose the failure.
fn map_query_error(e: &mysql::Error) -> anyhow::Error {
    // Structured error matching on mysql::Error variants
    let context = match e {
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

    // Always include redacted error detail so users can diagnose
    // issues. `redact_sql_error` is the single canonical redactor
    // (todo #016 / P1-C); any credential embedded in the mysql
    // crate's error string is scrubbed before it reaches the log.
    anyhow::anyhow!("{}: {}", context, redact_sql_error(&e.to_string()))
}
