//! Binary entry point for `gold_digger`.
//!
//! Parses CLI args (with `DATABASE_URL` / `DATABASE_QUERY` / `OUTPUT_FILE`
//! env fallbacks), initialises logging, dispatches subcommands and the
//! `--dump-config` diagnostic, and hands control to [`gold_digger::run`]
//! for the main query-execution pipeline. Exit codes follow the 0-5
//! contract defined in [`gold_digger::exit`].

use clap::Parser;

use gold_digger::cli::{Cli, Commands};
use gold_digger::completion::generate_completion;
use gold_digger::config::{EnvSnapshot, build_configuration_dump};
use gold_digger::exit::{exit_no_rows, exit_success, exit_with_error};
use gold_digger::logging::init_tracing;
use gold_digger::run::{RunOutcome, run};

/// Main entry point for the gold_digger CLI tool.
///
/// Parses CLI arguments and environment variables, executes a database
/// query, and writes the output in the specified format.
fn main() {
    // NOTE (todo #169): the rustls crypto provider is intentionally
    // *not* installed here. Subcommands like `--help`, `--version`,
    // `completion`, and `--dump-config` never touch TLS, so paying the
    // ~5-10 ms install cost on every invocation is pure waste. The
    // provider is installed lazily inside `create_database_connection`
    // immediately before the connection pool is constructed.

    let cli = Cli::parse();

    // Install the tracing subscriber before any work that might log. All
    // `tracing::*!` calls elsewhere in the binary (exit paths, TLS warnings,
    // main-loop progress logs) rely on this being set before they fire
    // (todo #163). Subcommands / `--dump-config` also benefit: error
    // reporting routes through `tracing::error!` even in those branches.
    init_tracing(cli.verbose, cli.quiet);

    // Handle subcommands first.
    //
    // `Commands` is `#[non_exhaustive]` (todo #177) for downstream
    // semver future-proofing; the wildcard arm reports a clear error
    // for any subcommand variant a future build of this binary doesn't
    // yet handle, instead of silently falling through.
    if let Some(command) = cli.command {
        match command {
            Commands::Completion { shell } => {
                generate_completion(shell);
                return;
            }
            #[allow(unreachable_patterns, clippy::wildcard_enum_match_arm)]
            _ => {
                eprintln!(
                    "error: unhandled subcommand; this gold_digger build does not support it"
                );
                std::process::exit(2);
            }
        }
    }

    // Handle --dump-config flag. The `build_configuration_dump` helper
    // returns the JSON value so it can be unit-tested without spawning a
    // subprocess (todo #062). main.rs is responsible for the actual I/O.
    if cli.dump_config {
        let snapshot = EnvSnapshot::from_process_env();
        let value = build_configuration_dump(&cli, &snapshot);
        match serde_json::to_string_pretty(&value) {
            Ok(json) => println!("{}", json),
            Err(e) => exit_with_error(e.into(), Some("Configuration dump failed")),
        }
        return;
    }

    // Hand off to the query-execution pipeline. `run` returns a Result
    // (post-CRITICAL #3) rather than calling `process::exit` itself —
    // doing the exit here means the streaming sink's `Drop` impl runs on
    // every error path (cleaning up the `<output>.tmp` sibling) before
    // the process actually terminates. The single point of `process::exit`
    // is also the single source of error logging via
    // `tracing::error!` inside `exit_with_error`.
    match run(&cli) {
        Ok(RunOutcome::RowsWritten { .. }) => exit_success(None),
        Ok(RunOutcome::EmptyResult) => {
            if cli.quiet {
                exit_no_rows(None);
            } else {
                exit_no_rows(Some("No records found in database"));
            }
        }
        Err(e) => exit_with_error(e, None),
    }
}
