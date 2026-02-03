mod actions;
mod display;
mod types;

use std::process::Command;

use crate::error::Result;
use crate::models::{UserAction, WtConfig};
use crate::services::multiplexer::MultiplexerType;

pub fn execute(json: bool, verbose: bool, all: bool, action: Option<String>, task: Option<String>) -> Result<()> {
    // Verify we're in a wt project directory
    WtConfig::load()?;

    // Handle --action parameter
    if let Some(action_name) = action {
        actions::execute_action(&action_name, task);
        return Ok(());
    }

    if json {
        // JSON output for agents/scripts
        display::display_status(json, verbose)
    } else if verbose {
        // Verbose output (non-TUI)
        display::display_status(false, true)
    } else if atty::is(atty::Stream::Stdout) {
        // Interactive TUI mode (default for humans)
        let tui_action = crate::tui::run(all)?;
        handle_tui_action(tui_action)
    } else {
        // Non-TTY: auto-degrade to JSON
        display::display_status(true, false)
    }
}

fn handle_tui_action(action: UserAction) -> Result<()> {
    match action {
        UserAction::Quit => Ok(()),
        UserAction::SwitchWindow { .. } => {
            // This should be handled within TUI, not here
            Ok(())
        }
        UserAction::AttachSession {
            multiplexer,
            session,
            window,
        } => {
            // Outside multiplexer: directly attach to session
            match multiplexer {
                MultiplexerType::Tmux => {
                    Command::new("tmux")
                        .args(["attach", "-t", &format!("{}:{}", session, window)])
                        .status()
                        .ok();
                }
                MultiplexerType::Zellij => {
                    Command::new("zellij")
                        .args(["attach", &session])
                        .status()
                        .ok();
                }
            }
            Ok(())
        }
        UserAction::ShowResume {
            worktree,
            session_id,
            claude_command,
        } => {
            eprintln!("Multiplexer window closed. Run this command to resume:");
            println!("cd {} && {} -r {}", worktree, claude_command, session_id);
            Ok(())
        }
        UserAction::Tail { name } => {
            // Execute tail command (default: 1 turn)
            crate::commands::tail::execute(name, 1)
        }
        UserAction::OpenWorktreeShell { .. } => {
            // This is handled within TUI, not here
            Ok(())
        }
    }
}
