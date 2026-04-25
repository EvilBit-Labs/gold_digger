//! Query-execution pipeline.
//!
//! Ties together config resolution, connection establishment, query
//! execution, and output dispatch. The binary entry point (`src/main.rs`)
//! handles CLI parsing, logging init, and subcommand / `--dump-config`
//! dispatch; once those are out of the way it delegates to [`run`].
//!
//! All failures are routed through [`crate::exit::exit_with_error`] so the
//! binary always exits with a stable, documented code.
//!
//! # Streaming (F007, todo #005)
//!
//! The pipeline uses `mysql::Queryable::query_iter` and feeds each row
//! into a [`crate::sink::RowSink`] chosen by the requested output format.
//! Peak memory is linear in the **column count**, not the row count:
//! only the current `mysql::Row` (plus per-row conversion scratch) is
//! live at a time. The streaming sinks write to a sibling `<output>.tmp`
//! path and rename to the final path on full success, preserving the
//! atomic-output guarantee even when a type-conversion error fires on
//! row N.

use mysql::prelude::Queryable;

use crate::cli::Cli;
use crate::config::{
    EnvSnapshot, resolve_database_query_with_env, resolve_database_url_with_env,
    resolve_output_file_with_env,
};
use crate::connection::create_database_connection;
use crate::logging::make_progress;
use crate::mysql_errors::{
    CR_CONN_HOST_ERROR, CR_CONNECTION_ERROR, CR_SERVER_GONE_ERROR, CR_SERVER_LOST,
    ER_ACCESS_DENIED_ERROR, ER_BAD_DB_ERROR, ER_BAD_FIELD_ERROR, ER_COLUMNACCESS_DENIED_ERROR,
    ER_DBACCESS_DENIED_ERROR, ER_NO_SUCH_TABLE, ER_PARSE_ERROR, ER_TABLEACCESS_DENIED_ERROR,
};
use crate::output::build_sink;
use crate::utils::redact_sql_error;

/// Outcome of the main query pipeline. `RowsWritten` corresponds to a
/// stream that produced one or more rows; `EmptyResult` corresponds to a
/// stream where the query parsed and ran but matched zero rows AND
/// `--allow-empty` was NOT supplied (the disposition that maps to exit 1
/// in the binary). Successful empty results with `--allow-empty` are
/// folded into `RowsWritten { count: 0 }` because they commit a valid
/// envelope to disk and exit successfully.
#[derive(Debug)]
pub enum RunOutcome {
    /// The pipeline streamed `count` data rows and finalized the output.
    /// Maps to `EXIT_SUCCESS` at the binary boundary.
    RowsWritten { count: u64 },
    /// The query returned no rows and `--allow-empty` was NOT set. The
    /// sink was dropped without committing, so no `<output>` file was
    /// produced. Maps to `EXIT_NO_ROWS` at the binary boundary.
    EmptyResult,
}

/// Executes the main query pipeline: resolve config → create pool → run
/// streaming query → feed sink.
///
/// Returns `Result<RunOutcome, anyhow::Error>` rather than calling
/// `process::exit` directly so the binary entry point in `src/main.rs`
/// can perform the `process::exit` once, AFTER stack unwinding has run
/// every `Drop` impl on the path. In particular, the streaming sink's
/// `Drop` impl removes the `<output>.tmp` sibling file on error — calling
/// `process::exit` mid-pipeline (the previous behaviour) skipped that
/// cleanup and orphaned the `.tmp` file (CRITICAL #3).
pub fn run(cli: &Cli) -> anyhow::Result<RunOutcome> {
    // Read every env-var fallback exactly once into an immutable
    // snapshot (todo #068, S15). Downstream resolvers consume the
    // snapshot rather than calling `std::env::var` again, so a hostile
    // parent process that mutates the environment between resolution
    // steps cannot influence the values gold_digger uses.
    let env_snapshot = EnvSnapshot::from_process_env();

    // Resolve configuration with precedence: CLI flags > snapshot env > error.
    let database_url = resolve_database_url_with_env(cli, &env_snapshot)?;
    let database_query = resolve_database_query_with_env(cli, &env_snapshot)?;
    let output_file = resolve_output_file_with_env(cli, &env_snapshot)?;

    // Connect phase: spinner (indeterminate duration). Hidden when
    // `--quiet` or stderr is not a TTY (todo #162).
    let connect_progress = make_progress(cli.quiet, None, "Connecting to database...");
    tracing::info!("Connecting to database...");

    // Connection-pool construction failures already carry typed errors
    // (`anyhow::Error::from(TlsError).context(...)` from `connection.rs`
    // after CRITICAL #5); propagate verbatim so the downcast classifier
    // in `crate::exit::map_error_to_exit_code` can read the variant.
    let pool = match create_database_connection(&database_url, cli) {
        Ok(pool) => pool,
        Err(e) => {
            connect_progress.finish_and_clear();
            return Err(e);
        }
    };
    // HIGH #10: route `pool.get_conn()` failures through the same
    // typed classifier `Pool::new` uses (`classify_mysql_pool_error`).
    // The previous path wrapped the raw `mysql::Error` into a
    // `GoldDiggerError::DbAuth(...)` string via free-form `anyhow!`,
    // which (a) skipped the typed `TlsError` routing required for
    // accurate exit codes (handshake/cert/hostname failures need exit
    // 3 with TLS-specific framing, not generic "DB auth"), and (b)
    // bypassed the credential-redaction guarantees the classifier
    // bakes in for every variant. The classifier internally routes
    // through `redact_sql_error`, so the user-facing message can never
    // embed an un-scrubbed `mysql::Error::to_string()`.
    let mut conn = match pool.get_conn() {
        Ok(conn) => conn,
        Err(e) => {
            connect_progress.finish_and_clear();
            let typed = crate::tls::pool::classify_mysql_pool_error(e);
            return Err(anyhow::Error::from(typed).context("Database connection failed"));
        }
    };
    connect_progress.finish_and_clear();

    stream_query(cli, &mut conn, &database_query, output_file.as_path())
}

/// Streams the query result into the appropriate sink.
///
/// Row count is unknown until the stream completes, so progress is
/// shown as a spinner (indeterminate). After every row the spinner's
/// message is updated with the running count. On empty results the
/// branch defers to [`handle_empty_result`] which still honours
/// `--allow-empty`.
///
/// All errors return via `Err(...)?` so the sink's `Drop` impl runs
/// during stack unwinding and cleans up the `<output>.tmp` sibling
/// file (CRITICAL #3).
fn stream_query(
    cli: &Cli,
    conn: &mut mysql::PooledConn,
    database_query: &str,
    output_file: &std::path::Path,
) -> anyhow::Result<RunOutcome> {
    // Query phase: spinner (indeterminate duration).
    let progress = make_progress(cli.quiet, None, "Executing query...");

    // `query_iter` returns a streaming QueryResult that yields
    // `Result<Row, mysql::Error>` as rows arrive — no full materialisation.
    let mut result = match conn.query_iter(database_query) {
        Ok(r) => r,
        Err(e) => {
            progress.finish_and_clear();
            return Err(map_query_error(&e));
        }
    };

    // Columns are known up-front from the first result-set metadata. We
    // snapshot them once, *before* pulling any rows, so `on_headers` can
    // fire even when the stream turns out to be empty.
    let columns: Vec<String> = result
        .columns()
        .as_ref()
        .iter()
        .map(|c| c.name_str().to_string())
        .collect();

    // Build the sink lazily: only after we know the query parsed and we
    // have column metadata. This preserves the previous behaviour where
    // the output file was never created on a bad query.
    let mut sink = match build_sink(output_file, cli) {
        Ok(s) => s,
        Err(e) => {
            progress.finish_and_clear();
            return Err(e.context("Output sink creation failed"));
        }
    };

    if let Err(e) = sink.on_headers(&columns) {
        progress.finish_and_clear();
        // Sink dropped on `?`-unwind -> .tmp removed.
        return Err(e.context("Failed to write output headers"));
    }

    let mut rows_seen: u64 = 0;
    progress.set_message("Streaming rows...");

    for row_result in result.by_ref() {
        let row = match row_result {
            Ok(row) => row,
            Err(e) => {
                progress.finish_and_clear();
                // Any mysql::Error from row fetch flows through the same
                // credential-redacting classifier as the initial query
                // error so streaming failures don't leak creds.
                // Sink dropped on `?`-unwind -> .tmp removed.
                return Err(map_query_error(&e));
            }
        };

        if let Err(e) = sink.on_row(&row) {
            progress.finish_and_clear();
            // Sink dropped on `?`-unwind -> .tmp removed.
            return Err(e.context("Row processing failed"));
        }

        rows_seen = rows_seen.saturating_add(1);
        // Update the spinner message every 1000 rows to avoid redraw
        // pressure on huge result sets while still giving users feedback.
        if rows_seen.is_multiple_of(1000) {
            progress.set_message(format!("Streaming rows... ({} so far)", rows_seen));
        }
    }

    // Drop the streaming result *before* finalize so any lingering server
    // traffic is drained and the connection is returned to a clean state.
    drop(result);

    if rows_seen == 0 {
        // Empty result: we already wrote `on_headers`, so the sink holds
        // a `.tmp` file with an empty envelope / header row. For
        // `--allow-empty` we finalize (commit the empty file); otherwise
        // we drop the sink (the tmp gets cleaned up) and signal
        // `EmptyResult` so the binary exits with 1.
        progress.finish_and_clear();
        return handle_empty_result(cli, sink);
    }

    sink.finalize()
        .map_err(|e| e.context("Output finalisation failed"))?;

    progress.finish_and_clear();
    tracing::info!(
        rows = rows_seen,
        file = %output_file.display(),
        "Outputting {} records to {}.",
        rows_seen,
        output_file.display()
    );
    Ok(RunOutcome::RowsWritten { count: rows_seen })
}

/// Handles an empty result set. If `--allow-empty` is set, finalizes the
/// sink (committing an empty `{"data":[]}` or header-only CSV/TSV file)
/// and returns `RowsWritten { count: 0 }`; otherwise drops the sink
/// (which cleans up its `.tmp`) and returns `EmptyResult` so the binary
/// can exit with [`crate::exit::EXIT_NO_ROWS`].
fn handle_empty_result(
    cli: &Cli,
    sink: Box<dyn crate::sink::RowSink>,
) -> anyhow::Result<RunOutcome> {
    if cli.allow_empty {
        tracing::info!("No records found in database, but --allow-empty is set.");
        sink.finalize()
            .map_err(|e| e.context("Output writing failed"))?;
        Ok(RunOutcome::RowsWritten { count: 0 })
    } else {
        // Drop the streaming sink; its `Drop` impl removes the `.tmp`
        // file so the filesystem shows no partial output.
        drop(sink);
        tracing::info!("No records found in database.");
        Ok(RunOutcome::EmptyResult)
    }
}

/// Maps a [`mysql::Error`] from query execution into a contextual
/// `anyhow::Error`. Known MySQL error codes are translated into
/// operator-facing messages; the underlying error is always appended
/// (via [`redact_sql_error`]) so the caller can diagnose the failure.
///
/// # Authentication errors (todo #064, S9)
///
/// Codes 1045 (`ER_ACCESS_DENIED_ERROR`) and 1044
/// (`ER_DBACCESS_DENIED_ERROR`) fire with a server-side message of the
/// form `"Access denied for user 'alice'@'10.0.0.5' (using password:
/// YES)"`. That string leaks the database username, the *client source
/// IP* the server saw the connection from, and a confirmation that a
/// password was supplied (CWE-209 — sensitive information in an error
/// message). The username and IP are not under the redactor's control
/// (`redact_sql_error` only scrubs `password=`, `token=`, URL userinfo,
/// etc.), so we discard the server message wholesale on those codes
/// and emit a static, action-oriented sentence instead. The original
/// (redacted) error still flows to `tracing::debug!` for operators
/// running with `-vv` who genuinely need to triage.
fn map_query_error(e: &mysql::Error) -> anyhow::Error {
    // Authentication-class MySQL errors get a hard-coded, leak-free
    // message. Everything else falls through to the general path below.
    // Codes are routed through `crate::mysql_errors` so contributors can
    // grep for the canonical `ER_*` symbol (todo #054).
    if let mysql::Error::MySqlError(mysql_err) = e
        && matches!(
            mysql_err.code,
            ER_ACCESS_DENIED_ERROR | ER_DBACCESS_DENIED_ERROR
        )
    {
        // Route the original (redacted) error to debug-level so it is
        // available with `-vv` but never reaches the default
        // user-facing log/exit path. The substrings "authentication" /
        // "access denied" must stay in the public message so
        // `crate::exit` still classifies this as EXIT_DB_AUTH_ERROR (3).
        tracing::debug!(
            code = mysql_err.code,
            detail = redact_sql_error(&e.to_string()),
            "MySQL authentication error (full server message redacted)"
        );
        return anyhow::anyhow!(
            "Database authentication failed: access denied. \
             Verify credentials via your secret manager."
        );
    }

    // Structured error matching on mysql::Error variants. Numeric codes
    // are routed through `crate::mysql_errors` so contributors can grep
    // for the canonical `ER_*` / `CR_*` symbol instead of decoding bare
    // numeric literals (todo #054).
    let context = match e {
        mysql::Error::MySqlError(mysql_err) => classify_mysql_error_code(mysql_err.code),
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

/// Maps a MySQL/MariaDB error code to an operator-facing context message.
///
/// Extracted from [`map_query_error`] so the named constants in
/// [`crate::mysql_errors`] keep their grep-ability and the call site
/// stays readable (todo #054). Returns a fallback message for codes the
/// classifier does not recognise.
///
/// `ER_ACCESS_DENIED_ERROR` / `ER_DBACCESS_DENIED_ERROR` are handled
/// upstream in [`map_query_error`] with a static, leak-free message
/// (todo #064) and never reach this function in practice.
fn classify_mysql_error_code(code: u16) -> &'static str {
    match code {
        ER_PARSE_ERROR => "SQL syntax error in query",
        ER_NO_SUCH_TABLE => "Table does not exist",
        ER_BAD_FIELD_ERROR => "Column does not exist or is ambiguous",
        ER_TABLEACCESS_DENIED_ERROR => "Insufficient privileges for query execution",
        ER_COLUMNACCESS_DENIED_ERROR => "Insufficient column privileges",
        ER_BAD_DB_ERROR => "Unknown database",
        CR_CONNECTION_ERROR => "Connection failed - server not reachable",
        CR_CONN_HOST_ERROR => "Connection failed - server not responding",
        CR_SERVER_GONE_ERROR => "Connection lost - server has gone away",
        CR_SERVER_LOST => "Connection lost during query",
        _ => "Query execution failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_known_server_errors() {
        assert_eq!(
            classify_mysql_error_code(ER_PARSE_ERROR),
            "SQL syntax error in query"
        );
        assert_eq!(
            classify_mysql_error_code(ER_NO_SUCH_TABLE),
            "Table does not exist"
        );
        assert_eq!(
            classify_mysql_error_code(ER_BAD_FIELD_ERROR),
            "Column does not exist or is ambiguous"
        );
    }

    #[test]
    fn classify_known_client_errors() {
        assert_eq!(
            classify_mysql_error_code(CR_SERVER_LOST),
            "Connection lost during query"
        );
        assert_eq!(
            classify_mysql_error_code(CR_SERVER_GONE_ERROR),
            "Connection lost - server has gone away"
        );
        assert_eq!(
            classify_mysql_error_code(CR_CONNECTION_ERROR),
            "Connection failed - server not reachable"
        );
    }

    #[test]
    fn classify_unknown_error_falls_back() {
        // 9999 is not a documented MySQL/MariaDB error code; fallback
        // message must be returned rather than an empty / panicking arm.
        assert_eq!(classify_mysql_error_code(9999), "Query execution failed");
    }
}
