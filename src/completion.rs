//! Shell completion script generation.

use clap::CommandFactory;
use clap_complete::{Shell as CompletionShell, generate};

use crate::cli::{Cli, Shell};

/// Generates shell completion scripts
pub fn generate_completion(shell: Shell) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_completion_bash() {
        // This should not panic
        generate_completion(Shell::Bash);
    }

    #[test]
    fn test_generate_completion_zsh() {
        // This should not panic
        generate_completion(Shell::Zsh);
    }

    #[test]
    fn test_generate_completion_fish() {
        // This should not panic
        generate_completion(Shell::Fish);
    }

    #[test]
    fn test_generate_completion_powershell() {
        // This should not panic
        generate_completion(Shell::PowerShell);
    }
}
