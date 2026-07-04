//! Shell completion script generation.

use std::io::Write;

use clap::CommandFactory;
use clap_complete::{Shell as CompletionShell, generate};

use crate::cli::{Cli, Shell};

/// Generates shell completion scripts to the provided writer.
///
/// Tests should pass a `Vec<u8>` here so the generated payload (which
/// is multi-KB per shell) does not pollute `cargo test` / `nextest`
/// output. The user-facing CLI entry point is [`generate_completion`],
/// which delegates to this helper with `std::io::stdout()`.
///
/// # Arguments
/// * `shell` - Target shell variant (Bash, Zsh, Fish, or PowerShell)
/// * `writer` - Sink that receives the generated completion script bytes
///
/// # Example
///
///     use gold_digger::cli::Shell;
///     use gold_digger::completion::generate_completion_to;
///
///     let mut buf: Vec<u8> = Vec::new();
///     generate_completion_to(Shell::Bash, &mut buf);
///     assert!(!buf.is_empty());
pub fn generate_completion_to<W: Write>(shell: Shell, writer: &mut W) {
    let mut cmd = Cli::command();
    let bin_name = "gold_digger";

    match shell {
        Shell::Bash => generate(CompletionShell::Bash, &mut cmd, bin_name, writer),
        Shell::Zsh => generate(CompletionShell::Zsh, &mut cmd, bin_name, writer),
        Shell::Fish => generate(CompletionShell::Fish, &mut cmd, bin_name, writer),
        Shell::PowerShell => generate(CompletionShell::PowerShell, &mut cmd, bin_name, writer),
    }
}

/// Generates shell completion scripts to standard output.
///
/// Public CLI entry point invoked from `gold_digger completion <SHELL>`.
/// Tests should call [`generate_completion_to`] with an in-memory
/// writer to keep test output quiet.
///
/// # Arguments
/// * `shell` - Target shell variant (Bash, Zsh, Fish, or PowerShell)
///
/// # Example
///
///     use gold_digger::cli::Shell;
///     use gold_digger::completion::generate_completion;
///
///     // Writes the Bash completion script to stdout.
///     generate_completion(Shell::Bash);
pub fn generate_completion(shell: Shell) {
    generate_completion_to(shell, &mut std::io::stdout());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generates completion into an in-memory buffer (keeps test output
    /// quiet) and asserts the payload is non-empty for the given shell.
    fn generate_to_vec(shell: Shell) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8 * 1024);
        generate_completion_to(shell, &mut buf);
        assert!(
            !buf.is_empty(),
            "completion script for {:?} should be non-empty",
            shell
        );
        buf
    }

    #[test]
    fn test_generate_completion_bash() {
        let _ = generate_to_vec(Shell::Bash);
    }

    #[test]
    fn test_generate_completion_zsh() {
        let _ = generate_to_vec(Shell::Zsh);
    }

    #[test]
    fn test_generate_completion_fish() {
        let _ = generate_to_vec(Shell::Fish);
    }

    #[test]
    fn test_generate_completion_powershell() {
        let _ = generate_to_vec(Shell::PowerShell);
    }
}
