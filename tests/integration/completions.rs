//! Smoke tests for the `repolens completions <shell>` command.
//!
//! For each supported shell we assert the generator produces non-empty
//! output that mentions the binary name. Validating the actual shell
//! syntax is deferred to the upstream `clap_complete` crate.

use repolens::cli::commands::ShellChoice;
use repolens::cli::commands::completions::execute;

fn assert_completions_for(shell: ShellChoice) {
    let mut buf: Vec<u8> = Vec::new();
    execute(shell, &mut buf).expect("completions::execute must succeed");

    assert!(!buf.is_empty(), "{:?} completions must not be empty", shell);

    let text = String::from_utf8(buf).expect("completion output must be UTF-8");
    assert!(
        text.contains("repolens"),
        "{:?} completions must reference the binary name",
        shell
    );
}

#[test]
fn bash_completions_contain_binary_name() {
    assert_completions_for(ShellChoice::Bash);
}

#[test]
fn zsh_completions_contain_binary_name() {
    assert_completions_for(ShellChoice::Zsh);
}

#[test]
fn fish_completions_contain_binary_name() {
    assert_completions_for(ShellChoice::Fish);
}

#[test]
fn powershell_completions_contain_binary_name() {
    assert_completions_for(ShellChoice::PowerShell);
}

#[test]
fn elvish_completions_contain_binary_name() {
    assert_completions_for(ShellChoice::Elvish);
}

#[test]
fn nushell_completions_contain_binary_name() {
    assert_completions_for(ShellChoice::Nushell);
}
