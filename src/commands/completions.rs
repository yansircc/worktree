use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::cli::Cli;
use crate::error::{Result, WtError};

/// Generate completions to stdout
pub fn generate_completions(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "wt", &mut io::stdout());
    Ok(())
}

/// Install completions to shell config
pub fn install() -> Result<()> {
    let shell = detect_shell()?;
    let rc_file = get_rc_file(&shell)?;
    let eval_line = get_eval_line(&shell);

    // Check if already installed
    let rc_content = fs::read_to_string(&rc_file).unwrap_or_default();
    if rc_content.contains(&eval_line) {
        println!("Completions already installed in {}", rc_file.display());
        return Ok(());
    }

    // Ensure parent directory exists (especially for fish: ~/.config/fish/)
    if let Some(parent) = rc_file.parent() {
        fs::create_dir_all(parent).map_err(|e| WtError::Io {
            operation: "create_dir_all".to_string(),
            path: parent.to_string_lossy().to_string(),
            message: e.to_string(),
        })?;
    }

    // Append eval line to rc file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rc_file)
        .map_err(|e| WtError::Io {
            operation: "open".to_string(),
            path: rc_file.to_string_lossy().to_string(),
            message: e.to_string(),
        })?;

    // Add newline before if file doesn't end with one
    let prefix = if rc_content.ends_with('\n') || rc_content.is_empty() {
        ""
    } else {
        "\n"
    };

    writeln!(file, "{}\n# wt shell completions\n{}", prefix, eval_line).map_err(|e| WtError::Io {
        operation: "write".to_string(),
        path: rc_file.to_string_lossy().to_string(),
        message: e.to_string(),
    })?;

    println!("Installed completions to {}", rc_file.display());
    println!("Run `exec {}` or restart your terminal to activate.", shell_name(&shell));

    Ok(())
}

/// Check if completions are already installed
pub fn is_installed() -> bool {
    let shell = match detect_shell() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let rc_file = match get_rc_file(&shell) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let eval_line = get_eval_line(&shell);
    let rc_content = fs::read_to_string(&rc_file).unwrap_or_default();
    rc_content.contains(&eval_line)
}

fn detect_shell() -> Result<Shell> {
    let shell_path = env::var("SHELL").unwrap_or_default();
    let shell_name = shell_path.rsplit('/').next().unwrap_or("");

    match shell_name {
        "zsh" => Ok(Shell::Zsh),
        "bash" => Ok(Shell::Bash),
        "fish" => Ok(Shell::Fish),
        _ => Err(WtError::InvalidInput(format!(
            "Unsupported shell: {}. Supported: zsh, bash, fish",
            shell_name
        ))),
    }
}

fn get_rc_file(shell: &Shell) -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        WtError::InvalidInput("Cannot determine home directory".to_string())
    })?;

    let rc_file = match shell {
        Shell::Zsh => home.join(".zshrc"),
        Shell::Bash => {
            // Prefer .bashrc, fall back to .bash_profile
            let bashrc = home.join(".bashrc");
            if bashrc.exists() {
                bashrc
            } else {
                home.join(".bash_profile")
            }
        }
        Shell::Fish => home.join(".config/fish/config.fish"),
        _ => return Err(WtError::InvalidInput(format!(
            "Unsupported shell: {:?}",
            shell
        ))),
    };

    Ok(rc_file)
}

fn get_eval_line(shell: &Shell) -> String {
    match shell {
        Shell::Zsh => r#"eval "$(wt completions generate zsh)""#.to_string(),
        Shell::Bash => r#"eval "$(wt completions generate bash)""#.to_string(),
        Shell::Fish => "wt completions generate fish | source".to_string(),
        _ => String::new(),
    }
}

fn shell_name(shell: &Shell) -> &'static str {
    match shell {
        Shell::Zsh => "zsh",
        Shell::Bash => "bash",
        Shell::Fish => "fish",
        _ => "shell",
    }
}
