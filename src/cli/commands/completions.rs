//! Shell completion generation command

use clap::CommandFactory;
use clap_complete::{Shell, generate};
use clap_complete_nushell::Nushell;

use super::ShellChoice;
use crate::cli::Cli;

const BIN_NAME: &str = "repolens";

/// Write the completion script for the given shell to `out`.
pub fn execute(shell: ShellChoice, mut out: impl std::io::Write) -> anyhow::Result<()> {
    let mut cmd = Cli::command();
    match shell {
        ShellChoice::Bash => generate(Shell::Bash, &mut cmd, BIN_NAME, &mut out),
        ShellChoice::Zsh => generate(Shell::Zsh, &mut cmd, BIN_NAME, &mut out),
        ShellChoice::Fish => generate(Shell::Fish, &mut cmd, BIN_NAME, &mut out),
        ShellChoice::PowerShell => generate(Shell::PowerShell, &mut cmd, BIN_NAME, &mut out),
        ShellChoice::Elvish => generate(Shell::Elvish, &mut cmd, BIN_NAME, &mut out),
        ShellChoice::Nushell => generate(Nushell, &mut cmd, BIN_NAME, &mut out),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn produces_output(shell: ShellChoice) {
        let mut buf: Vec<u8> = Vec::new();
        execute(shell, &mut buf).expect("completion generation must succeed");
        assert!(!buf.is_empty(), "{:?} completions must not be empty", shell);
        let text = String::from_utf8(buf).expect("completion output must be UTF-8");
        assert!(
            text.contains("repolens"),
            "{:?} completions must mention the binary name",
            shell
        );
    }

    #[test]
    fn bash_completions_are_generated() {
        produces_output(ShellChoice::Bash);
    }

    #[test]
    fn zsh_completions_are_generated() {
        produces_output(ShellChoice::Zsh);
    }

    #[test]
    fn fish_completions_are_generated() {
        produces_output(ShellChoice::Fish);
    }

    #[test]
    fn powershell_completions_are_generated() {
        produces_output(ShellChoice::PowerShell);
    }

    #[test]
    fn elvish_completions_are_generated() {
        produces_output(ShellChoice::Elvish);
    }

    #[test]
    fn nushell_completions_are_generated() {
        produces_output(ShellChoice::Nushell);
    }
}
