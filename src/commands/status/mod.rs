mod actions;
mod display;
mod types;

use std::process::Command;

use crate::error::Result;
use crate::models::WtConfig;
use crate::tui::TuiAction;

pub fn execute(json: bool, action: Option<String>, task: Option<String>) -> Result<()> {
    // Verify we're in a wt project directory
    WtConfig::load()?;

    // Handle --action parameter
    if let Some(action_name) = action {
        actions::execute_action(&action_name, task);
        return Ok(());
    }

    if json {
        // JSON output for agents/scripts
        display::display_status(true)
    } else if atty::is(atty::Stream::Stdout) {
        // Interactive TUI mode (default for humans)
        let tui_action = crate::tui::run()?;
        handle_tui_action(tui_action)
    } else {
        // Non-TTY: auto-degrade to JSON
        display::display_status(true)
    }
}

fn handle_tui_action(action: TuiAction) -> Result<()> {
    match action {
        TuiAction::Quit => Ok(()),
        TuiAction::SwitchTmuxWindow { .. } => {
            // This should be handled within TUI, not here
            Ok(())
        }
        TuiAction::AttachTmux { session, window: _ } => {
            // Outside tmux: directly attach to session (each task has its own session)
            Command::new("tmux")
                .args(["attach", "-t", &session])
                .status()
                .ok();
            Ok(())
        }
        TuiAction::ShowResume {
            worktree,
            session_id,
            claude_command,
        } => {
            eprintln!("Tmux window closed. Run this command to resume:");
            println!("cd {} && {} -r {}", worktree, claude_command, session_id);
            Ok(())
        }
        TuiAction::Tail { name } => {
            // Execute tail command (default: 1 turn)
            crate::commands::tail::execute(name, 1)
        }
    }
}
