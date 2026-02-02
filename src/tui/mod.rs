//! Terminal User Interface for wt status.

mod app;
mod ui;

pub use app::{App, TuiAction};

use std::io;
use std::process::Command;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use crate::error::Result;
use crate::services::multiplexer::MultiplexerType;

/// Run the TUI application and return the action to perform
pub fn run() -> Result<TuiAction> {
    // Setup terminal
    enable_raw_mode().map_err(|e| crate::error::WtError::Io {
        operation: "enable raw mode".to_string(),
        path: "terminal".to_string(),
        message: e.to_string(),
    })?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(|e| {
        crate::error::WtError::Io {
            operation: "enter alternate screen".to_string(),
            path: "terminal".to_string(),
            message: e.to_string(),
        }
    })?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| crate::error::WtError::Io {
        operation: "create terminal".to_string(),
        path: "terminal".to_string(),
        message: e.to_string(),
    })?;

    // Create app and run
    let mut app = App::new()?;
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode().ok();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .ok();
    terminal.show_cursor().ok();

    result
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<TuiAction> {
    let tick_rate = Duration::from_secs(2);

    loop {
        terminal
            .draw(|f| ui::draw(f, app))
            .map_err(|e| crate::error::WtError::Io {
                operation: "draw terminal".to_string(),
                path: "terminal".to_string(),
                message: e.to_string(),
            })?;

        // Poll for events with timeout
        if event::poll(tick_rate).map_err(|e| crate::error::WtError::Io {
            operation: "poll events".to_string(),
            path: "terminal".to_string(),
            message: e.to_string(),
        })? {
            if let Event::Key(key) = event::read().map_err(|e| crate::error::WtError::Io {
                operation: "read event".to_string(),
                path: "terminal".to_string(),
                message: e.to_string(),
            })? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        // Quit
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return Ok(TuiAction::Quit);
                        }

                        // Navigate
                        KeyCode::Up | KeyCode::Char('k') => app.previous(),
                        KeyCode::Down | KeyCode::Char('j') => app.next(),

                        // Enter: switch/attach multiplexer or show resume command
                        KeyCode::Enter => {
                            if let Some(action) = app.enter_action() {
                                match &action {
                                    TuiAction::SwitchWindow {
                                        multiplexer,
                                        session,
                                        window,
                                    } => {
                                        // Inside multiplexer: temporarily leave TUI to switch window
                                        disable_raw_mode().ok();
                                        let mut stdout = io::stdout();
                                        execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)
                                            .ok();

                                        // Switch to target window based on multiplexer type
                                        match multiplexer {
                                            MultiplexerType::Tmux => {
                                                Command::new("tmux")
                                                    .args([
                                                        "select-window",
                                                        "-t",
                                                        &format!("{}:{}", session, window),
                                                    ])
                                                    .status()
                                                    .ok();
                                            }
                                            MultiplexerType::Zellij => {
                                                Command::new("zellij")
                                                    .args([
                                                        "-s",
                                                        session,
                                                        "action",
                                                        "go-to-tab-name",
                                                        window,
                                                    ])
                                                    .status()
                                                    .ok();
                                            }
                                        }

                                        // Re-enter TUI (user can switch back with keybind)
                                        enable_raw_mode().ok();
                                        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
                                            .ok();

                                        // Refresh data after returning
                                        app.refresh()?;
                                    }
                                    TuiAction::AttachSession { .. }
                                    | TuiAction::ShowResume { .. } => {
                                        // Exit TUI and handle in status.rs
                                        return Ok(action);
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // Tail (Running or Done)
                        KeyCode::Char('t') => {
                            if let Some(action) = app.tail_action() {
                                return Ok(action);
                            }
                        }

                        // Mark for review (Running only)
                        KeyCode::Char('r') => {
                            if app.can_mark_review() {
                                app.mark_review()?;
                            }
                        }

                        // Resume (Review only)
                        KeyCode::Char('u') => {
                            if app.can_resume() {
                                // Call resume command
                                if let Some(task) = app.selected_task() {
                                    let name = task.name.clone();
                                    crate::commands::resume::execute(name)?;
                                    app.refresh()?;
                                }
                            }
                        }

                        // Complete (Review only)
                        KeyCode::Char('c') => {
                            if app.can_complete() {
                                app.mark_completed()?;
                            }
                        }

                        _ => {}
                    }
                }
            }
        } else {
            // Tick: refresh data
            app.refresh()?;
        }
    }
}
