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
use gold_digger::config::dump_configuration;
use gold_digger::exit::exit_with_error;
use gold_digger::logging::init_tracing;
use gold_digger::run::run;

/// Main entry point for the gold_digger CLI tool.
///
/// Parses CLI arguments and environment variables, executes a database
/// query, and writes the output in the specified format.
fn main() {
    // Initialize crypto provider for rustls
    gold_digger::init_crypto_provider();

    let cli = Cli::parse();

    // Install the tracing subscriber before any work that might log. All
    // `tracing::*!` calls elsewhere in the binary (exit paths, TLS warnings,
    // main-loop progress logs) rely on this being set before they fire
    // (todo #163). Subcommands / `--dump-config` also benefit: error
    // reporting routes through `tracing::error!` even in those branches.
    init_tracing(cli.verbose, cli.quiet);

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

    // Hand off to the query-execution pipeline. Never returns.
    run(cli);
}
