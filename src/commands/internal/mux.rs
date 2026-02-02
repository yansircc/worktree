//! Multiplexer atomic operations for hooks.
//!
//! Usage:
//!   wt internal mux:create-window <session> <window> <cwd> <command>
//!   wt internal mux:close-window <session> <window>
//!   wt internal mux:focus-window <session> <window>
//!   wt internal mux:window-exists <session> <window>
//!   wt internal mux:send-keys <session> <window> <keys>
//!   wt internal mux:list-windows <session>

use crate::error::{Result, WtError};
use crate::models::WtConfig;

/// Execute a mux operation
pub fn execute(action: &str, args: Vec<String>) -> Result<()> {
    let config = WtConfig::load()?;
    let mux = config.create_multiplexer();

    match action {
        "create-window" => {
            if args.len() < 4 {
                return Err(WtError::InvalidInput(
                    "mux:create-window requires 4 arguments: <session> <window> <cwd> <command>"
                        .to_string(),
                ));
            }
            mux.create_window(&args[0], &args[1], &args[2], &args[3])
        }

        "close-window" => {
            if args.len() < 2 {
                return Err(WtError::InvalidInput(
                    "mux:close-window requires 2 arguments: <session> <window>".to_string(),
                ));
            }
            let _ = mux.kill_window_if_exists(&args[0], &args[1])?;
            Ok(())
        }

        "focus-window" => {
            if args.len() < 2 {
                return Err(WtError::InvalidInput(
                    "mux:focus-window requires 2 arguments: <session> <window>".to_string(),
                ));
            }
            mux.focus_window(&args[0], &args[1])
        }

        "window-exists" => {
            if args.len() < 2 {
                return Err(WtError::InvalidInput(
                    "mux:window-exists requires 2 arguments: <session> <window>".to_string(),
                ));
            }
            if mux.window_exists(&args[0], &args[1]) {
                Ok(())
            } else {
                std::process::exit(1);
            }
        }

        "send-keys" => {
            if args.len() < 3 {
                return Err(WtError::InvalidInput(
                    "mux:send-keys requires 3 arguments: <session> <window> <keys>".to_string(),
                ));
            }
            mux.send_keys(&args[0], &args[1], &args[2])
        }

        "list-windows" => {
            if args.is_empty() {
                return Err(WtError::InvalidInput(
                    "mux:list-windows requires 1 argument: <session>".to_string(),
                ));
            }
            let windows = mux.list_windows(&args[0])?;
            for window in windows {
                println!("{}", window);
            }
            Ok(())
        }

        _ => Err(WtError::InvalidInput(format!(
            "Unknown mux operation '{}'. Available: create-window, close-window, focus-window, window-exists, send-keys, list-windows",
            action
        ))),
    }
}
