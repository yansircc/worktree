//! UI rendering for TUI v2 - Left/Right split layout.

use ratatui::{prelude::*, widgets::Paragraph};

use crate::models::TaskStatus;

use super::app::{App, TaskDisplay};

/// Minimum terminal size
const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 10;

/// Left panel width (fixed)
const LEFT_WIDTH: u16 = 25;

/// Main draw function
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Check minimum size
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_size_warning(frame, area);
        return;
    }

    // Layout: main content + footer
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Main content
            Constraint::Length(2), // Footer
        ])
        .split(area);

    // Main content: left panel + right panel
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(LEFT_WIDTH), // Left: task list
            Constraint::Min(1),             // Right: details
        ])
        .split(main_chunks[0]);

    draw_left_panel(frame, content_chunks[0], app);
    draw_right_panel(frame, content_chunks[1], app);
    draw_footer(frame, main_chunks[1], app);
}

/// Draw terminal size warning
fn draw_size_warning(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from("Terminal too small"),
        Line::from(format!("Need at least {}x{}", MIN_WIDTH, MIN_HEIGHT)),
        Line::from(format!("Current: {}x{}", area.width, area.height)),
    ];
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(Color::Red))
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

/// Draw left panel: task list
fn draw_left_panel(frame: &mut Frame, area: Rect, app: &App) {
    if app.tasks.is_empty() {
        let text = Paragraph::new(" (no active tasks)")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(text, area);
        return;
    }

    let mut lines = Vec::new();

    for (i, task) in app.tasks.iter().enumerate() {
        let is_selected = i == app.selected;
        let line = format_task_line(task, is_selected, &app.phase_sequence);
        lines.push(line);
    }

    let text = Text::from(lines);
    frame.render_widget(Paragraph::new(text), area);
}

/// Format a single task line for left panel
fn format_task_line(task: &TaskDisplay, selected: bool, phase_sequence: &[String]) -> Line<'static> {
    let mut spans = Vec::new();

    // Selection indicator
    if selected {
        spans.push(Span::styled(" ", Style::default().fg(Color::Yellow)));
    } else {
        spans.push(Span::raw(" "));
    }

    // Index
    spans.push(Span::styled(
        format!("{:>2}", task.index),
        Style::default().fg(Color::DarkGray),
    ));
    spans.push(Span::raw(" "));

    // Status icon with color
    let (icon, icon_color) = get_status_icon(task);
    spans.push(Span::styled(icon, Style::default().fg(icon_color)));
    spans.push(Span::raw(" "));

    // Task name (fixed width 10)
    let name = truncate(&task.name, 10);
    let name_style = if selected {
        Style::default().fg(Color::White).bold()
    } else {
        Style::default().fg(Color::White)
    };
    spans.push(Span::styled(format!("{:<10}", name), name_style));
    spans.push(Span::raw(" "));

    // Phase abbreviation (position-based: p1, p2, p3, etc.)
    let phase_abbr = task
        .phase
        .as_ref()
        .map(|p| abbreviate_phase(p, phase_sequence))
        .unwrap_or_else(|| "---".to_string());
    spans.push(Span::styled(
        format!("{:<3}", phase_abbr),
        Style::default().fg(Color::DarkGray),
    ));

    // Context percentage with color
    let ctx_color = if task.context_percent >= 95 {
        Color::Red
    } else if task.context_percent >= 80 {
        Color::Yellow
    } else {
        Color::Cyan
    };

    if task.context_percent > 0 {
        spans.push(Span::styled(
            format!("{:>3}%", task.context_percent),
            Style::default().fg(ctx_color),
        ));
    } else {
        spans.push(Span::styled("   -", Style::default().fg(Color::DarkGray)));
    }

    Line::from(spans)
}

/// Get status icon and color for a task
fn get_status_icon(task: &TaskDisplay) -> (&'static str, Color) {
    if task.has_conflict {
        return ("⚠", Color::Red);
    }

    match task.status {
        TaskStatus::Pending => ("○", Color::White),
        TaskStatus::Active => {
            if !task.mux_alive {
                ("⚠", Color::Yellow)
            } else if task.active {
                ("●", Color::Green)
            } else {
                ("●", Color::Yellow)
            }
        }
        TaskStatus::Idle => ("◐", Color::Yellow),
        TaskStatus::Completed => ("✓", Color::Magenta),
    }
}

/// Abbreviate phase name using position index (p1, p2, p3, etc.)
/// Falls back to first 3 chars if phase not in sequence.
fn abbreviate_phase(phase: &str, sequence: &[String]) -> String {
    if let Some(pos) = sequence.iter().position(|s| s == phase) {
        format!("p{}", pos + 1)
    } else {
        // Fallback for unknown phases
        phase.chars().take(3).collect()
    }
}

/// Draw right panel: task details
fn draw_right_panel(frame: &mut Frame, area: Rect, app: &App) {
    // Vertical border on the left
    let border_area = Rect::new(area.x, area.y, 1, area.height);
    let border = "│".repeat(area.height as usize);
    frame.render_widget(
        Paragraph::new(border).style(Style::default().fg(Color::DarkGray)),
        border_area,
    );

    // Content area (after border)
    let content_area = Rect::new(area.x + 2, area.y, area.width.saturating_sub(3), area.height);

    match app.selected_task() {
        Some(task) => draw_task_details(frame, content_area, task),
        None => draw_help(frame, content_area),
    }
}

/// Draw task details in right panel
fn draw_task_details(frame: &mut Frame, area: Rect, task: &TaskDisplay) {
    let mut lines = Vec::new();

    // Header: task name + phase + status
    let status_suffix = match task.status {
        TaskStatus::Idle => {
            let reason = task.step_result.as_deref().unwrap_or("idle");
            format!(" ({})", reason)
        }
        _ => String::new(),
    };

    let phase_display = task.phase.as_deref().unwrap_or("none");
    lines.push(Line::from(vec![
        Span::styled(
            format!("{}: {}", task.name, phase_display),
            Style::default().fg(Color::Cyan).bold(),
        ),
        Span::styled(status_suffix, Style::default().fg(Color::Yellow)),
    ]));

    // Separator
    lines.push(Line::from(Span::styled(
        "─".repeat(area.width as usize),
        Style::default().fg(Color::DarkGray),
    )));

    // Content based on status
    match task.status {
        TaskStatus::Pending => draw_pending_details(&mut lines, task, area.width),
        TaskStatus::Active | TaskStatus::Idle => draw_active_details(&mut lines, task, area.width),
        TaskStatus::Completed => draw_completed_details(&mut lines, task),
    }

    let text = Text::from(lines);
    frame.render_widget(Paragraph::new(text), area);
}

/// Draw details for pending task
fn draw_pending_details(lines: &mut Vec<Line<'static>>, task: &TaskDisplay, _width: u16) {
    lines.push(Line::from("No resources allocated"));
    lines.push(Line::from(""));

    if !task.dependencies.is_empty() {
        lines.push(Line::from(Span::styled(
            "Dependencies:",
            Style::default().fg(Color::White),
        )));

        for (dep_name, dep_status) in &task.dependencies {
            let (icon, color) = match dep_status {
                TaskStatus::Completed => ("✓", Color::Green),
                TaskStatus::Active => ("●", Color::Yellow),
                TaskStatus::Idle => ("◐", Color::Yellow),
                TaskStatus::Pending => ("○", Color::DarkGray),
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(icon, Style::default().fg(color)),
                Span::raw(" "),
                Span::raw(dep_name.clone()),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press [n] to start",
        Style::default().fg(Color::DarkGray),
    )));
}

/// Draw details for active/idle task
fn draw_active_details(lines: &mut Vec<Line<'static>>, task: &TaskDisplay, width: u16) {
    // Workflow steps
    if !task.workflow_steps.is_empty() {
        lines.push(Line::from(Span::styled(
            "on_enter workflow",
            Style::default().fg(Color::White),
        )));

        let step_count = task.workflow_steps.len();
        for (i, step) in task.workflow_steps.iter().enumerate() {
            let is_last = i == step_count - 1;
            let prefix = if is_last { "└─ " } else { "├─ " };

            // Script steps are shown as completed (✓), agent step shows current status
            let (icon, icon_color) = if step.is_agent {
                // Agent step - show based on status and step_result
                if task.status == TaskStatus::Active {
                    ("●", Color::Green)  // Running
                } else if task.step_result.as_deref() == Some("done") {
                    ("✓", Color::Green)  // Completed successfully
                } else if task.step_result.as_deref() == Some("human_review") {
                    ("⏸", Color::Yellow) // Blocked
                } else if task.step_result.as_deref() == Some("error") {
                    ("✗", Color::Red)    // Failed
                } else {
                    ("◐", Color::Yellow) // Other idle state
                }
            } else {
                // Script step - assumed completed if we got to agent
                ("✓", Color::Green)
            };

            // Show duration only for agent step
            let duration_part = if step.is_agent {
                let duration_str = task.duration.clone().unwrap_or_else(|| "-".to_string());
                vec![
                    Span::raw("  "),
                    Span::styled(duration_str, Style::default().fg(Color::DarkGray)),
                ]
            } else {
                vec![]
            };

            let mut spans = vec![
                Span::raw(prefix),
                Span::styled(step.name.clone(), Style::default().fg(Color::White)),
                Span::raw("  "),
                Span::styled(icon, Style::default().fg(icon_color)),
            ];
            spans.extend(duration_part);

            lines.push(Line::from(spans));
        }
    }

    lines.push(Line::from(""));

    // Context progress bar
    let bar_width = (width as usize).saturating_sub(15).min(20);
    let filled = (task.context_percent as usize * bar_width) / 100;
    let empty = bar_width - filled;
    let bar = format!("{}{}",
        "█".repeat(filled),
        "░".repeat(empty)
    );

    let ctx_color = if task.context_percent >= 95 {
        Color::Red
    } else if task.context_percent >= 80 {
        Color::Yellow
    } else {
        Color::Cyan
    };

    lines.push(Line::from(vec![
        Span::styled("Context   ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("[{}]", bar), Style::default().fg(ctx_color)),
        Span::raw("  "),
        Span::styled(format!("{}%", task.context_percent), Style::default().fg(ctx_color)),
    ]));

    // Duration
    let duration_display = task.duration.clone().unwrap_or_else(|| "-".to_string());
    lines.push(Line::from(vec![
        Span::styled("Duration  ", Style::default().fg(Color::DarkGray)),
        Span::raw(duration_display),
    ]));

    // Git stats
    if task.commit_count > 0 || task.additions > 0 || task.deletions > 0 {
        lines.push(Line::from(vec![
            Span::styled("Git       ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} commits", task.commit_count),
                Style::default().fg(Color::Magenta),
            ),
            Span::raw("  "),
            Span::styled(
                format!("+{}/-{}", task.additions, task.deletions),
                Style::default().fg(Color::Gray),
            ),
        ]));
    }

    // Current tool
    if let Some(ref tool) = task.current_tool {
        let tool_str = tool.clone();
        lines.push(Line::from(vec![
            Span::styled("Tool      ", Style::default().fg(Color::DarkGray)),
            Span::raw(tool_str),
        ]));
    }

    // Conflict warning
    if task.has_conflict {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "⚠ CONFLICT - merge conflict detected",
            Style::default().fg(Color::Red).bold(),
        )));
    }

    // Latest message separator
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "─".repeat(width as usize),
        Style::default().fg(Color::DarkGray),
    )));

    // Latest message
    let msg_display = task
        .latest_message
        .as_ref()
        .map(|m| format!("> {}", m))
        .unwrap_or_else(|| "> (waiting for output...)".to_string());
    lines.push(Line::from(Span::styled(
        msg_display,
        Style::default().fg(Color::DarkGray).italic(),
    )));
}

/// Draw details for completed task
fn draw_completed_details(lines: &mut Vec<Line<'static>>, task: &TaskDisplay) {
    lines.push(Line::from("Task completed"));
    lines.push(Line::from(""));

    if task.commit_count > 0 {
        lines.push(Line::from(vec![
            Span::styled("Commits   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", task.commit_count),
                Style::default().fg(Color::Magenta),
            ),
        ]));
    }

    if task.additions > 0 || task.deletions > 0 {
        lines.push(Line::from(vec![
            Span::styled("Changes   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("+{}/-{}", task.additions, task.deletions),
                Style::default().fg(Color::Gray),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press [l] to view logs",
        Style::default().fg(Color::DarkGray),
    )));
}

/// Draw help when no task selected
fn draw_help(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled(
            "wt - Worktree Task Manager",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(Span::styled(
            "─".repeat(area.width as usize),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from("No tasks selected"),
        Line::from(""),
        Line::from(Span::styled(
            "Keyboard shortcuts:",
            Style::default().fg(Color::White),
        )),
        Line::from("  j/k     Navigate tasks"),
        Line::from("  Enter   Attach to agent window"),
        Line::from("  n       Next phase (wt next)"),
        Line::from("  p       Prev phase (wt prev)"),
        Line::from("  s       Stop task (wt stop)"),
        Line::from("  l       View logs"),
        Line::from("  t       Tail transcript"),
        Line::from("  q       Quit"),
    ];

    let text = Text::from(lines);
    frame.render_widget(Paragraph::new(text), area);
}

/// Draw footer: keybindings
fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    // Separator
    let sep = "─".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(sep).style(Style::default().fg(Color::DarkGray)),
        Rect::new(area.x, area.y, area.width, 1),
    );

    // Keybindings
    if area.height > 1 {
        let help_area = Rect::new(area.x, area.y + 1, area.width, 1);

        let mut spans = vec![
            Span::raw(" "),
            Span::styled("j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" select  "),
        ];

        // Context-sensitive actions
        if let Some(task) = app.selected_task() {
            // Enter
            if task.status == TaskStatus::Active || task.status == TaskStatus::Idle {
                spans.push(Span::styled("⏎", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" attach  "));
            }

            // n (next) - available for all non-completed
            if task.status != TaskStatus::Completed {
                spans.push(Span::styled("n", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" next  "));
            }

            // s (stop) - only for Active
            if task.status == TaskStatus::Active {
                spans.push(Span::styled("s", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" stop  "));
            }

            // p (prev) - available for Active and Idle
            if task.status == TaskStatus::Active || task.status == TaskStatus::Idle {
                spans.push(Span::styled("p", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" prev  "));
            }

            // l (log) - available for Active and Idle
            if task.status == TaskStatus::Active || task.status == TaskStatus::Idle {
                spans.push(Span::styled("l", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" log  "));
            }

            // t (tail) - available for Active and Idle
            if task.status == TaskStatus::Active || task.status == TaskStatus::Idle {
                spans.push(Span::styled("t", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" tail  "));
            }
        }

        spans.push(Span::styled("q", Style::default().fg(Color::Yellow)));
        spans.push(Span::raw(" quit"));

        frame.render_widget(Paragraph::new(Line::from(spans)), help_area);
    }
}

/// Truncate string to max length
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 1).collect();
        format!("{}…", truncated)
    }
}
