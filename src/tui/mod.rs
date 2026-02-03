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
use crate::models::TaskStatus;
use crate::services::multiplexer::MultiplexerType;

/// Run the TUI application and return the action to perform
pub fn run(show_all: bool) -> Result<TuiAction> {
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
    let mut app = App::new(show_all)?;
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
                                    TuiAction::OpenWorktreeShell {
                                        multiplexer,
                                        session,
                                        worktree_path,
                                        task_name,
                                    } => {
                                        // Open shell in worktree directory
                                        match multiplexer {
                                            MultiplexerType::Tmux => {
                                                // Create new window with shell in worktree dir
                                                Command::new("tmux")
                                                    .args([
                                                        "new-window",
                                                        "-t",
                                                        &session,
                                                        "-n",
                                                        &task_name,
                                                        "-c",
                                                        &worktree_path,
                                                    ])
                                                    .status()
                                                    .ok();
                                            }
                                            MultiplexerType::Zellij => {
                                                // For Zellij, create new tab
                                                Command::new("zellij")
                                                    .args([
                                                        "-s",
                                                        &session,
                                                        "action",
                                                        "new-tab",
                                                        "--name",
                                                        &task_name,
                                                        "--cwd",
                                                        &worktree_path,
                                                    ])
                                                    .status()
                                                    .ok();
                                            }
                                        }
                                        // Refresh data to show updated state
                                        app.refresh()?;
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // Tail (Active or Idle)
                        KeyCode::Char('t') => {
                            if let Some(action) = app.tail_action() {
                                return Ok(action);
                            }
                        }

                        // Next (Pending/Active/Idle) - advance phase
                        KeyCode::Char('n') => {
                            if let Some(task) = app.selected_task() {
                                if task.status != TaskStatus::Completed {
                                    let name = task.name.clone();
                                    // Temporarily leave TUI to show output
                                    disable_raw_mode().ok();
                                    let mut stdout = io::stdout();
                                    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture).ok();

                                    // Execute wt next
                                    let result = crate::commands::next::execute(name);
                                    if let Err(e) = &result {
                                        eprintln!("Error: {}", e);
                                    }

                                    // Brief pause to see output
                                    std::thread::sleep(std::time::Duration::from_millis(500));

                                    // Re-enter TUI
                                    enable_raw_mode().ok();
                                    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).ok();
                                    app.refresh()?;
                                }
                            }
                        }

                        // Stop (Active only)
                        KeyCode::Char('s') => {
                            if let Some(task) = app.selected_task() {
                                if task.status == TaskStatus::Active {
                                    let name = task.name.clone();
                                    // Execute wt stop (don't need to leave TUI)
                                    crate::commands::stop::execute(name, false)?;
                                    app.refresh()?;
                                }
                            }
                        }

                        // Prev (Active or Idle) - go back to previous phase
                        KeyCode::Char('p') => {
                            if let Some(task) = app.selected_task() {
                                if task.status == TaskStatus::Active || task.status == TaskStatus::Idle {
                                    let name = task.name.clone();
                                    // Temporarily leave TUI to show output
                                    disable_raw_mode().ok();
                                    let mut stdout = io::stdout();
                                    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture).ok();

                                    // Execute wt prev
                                    let result = crate::commands::prev::execute(name);
                                    if let Err(e) = &result {
                                        eprintln!("Error: {}", e);
                                    }

                                    // Brief pause to see output
                                    std::thread::sleep(std::time::Duration::from_millis(500));

                                    // Re-enter TUI
                                    enable_raw_mode().ok();
                                    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).ok();
                                    app.refresh()?;
                                }
                            }
                        }

                        // Log (Active or Idle) - open in new tmux window
                        KeyCode::Char('l') => {
                            if let Some(task) = app.selected_task() {
                                if task.status == TaskStatus::Active || task.status == TaskStatus::Idle {
                                    if task.worktree_path.is_some() {
                                        // Find log file
                                        let log_dir = format!(".wt/logs/{}", task.name);
                                        let phase = task.phase.as_deref().unwrap_or("developing");
                                        let log_path = format!("{}/{}/workflow.log", log_dir, phase);

                                        // Open in new tmux window
                                        if std::env::var("TMUX").is_ok() {
                                            Command::new("tmux")
                                                .args([
                                                    "new-window",
                                                    "-n",
                                                    &format!("log:{}", task.name),
                                                    &format!("less +F {} 2>/dev/null || echo 'No log file found' && read", log_path),
                                                ])
                                                .status()
                                                .ok();
                                        }
                                    }
                                }
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
