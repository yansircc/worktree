//! UI rendering for TUI.

use ratatui::{prelude::*, widgets::Paragraph};

use crate::models::TaskStatus;

use super::app::{App, TaskDisplay};

/// Main draw function
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Layout: header, task list, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Header
            Constraint::Min(1),    // Task list
            Constraint::Length(2), // Footer
        ])
        .split(area);

    draw_header(frame, chunks[0], app);
    draw_tasks(frame, chunks[1], app);
    draw_footer(frame, chunks[2], app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let running = app
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Running)
        .count();
    let done = app
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Done)
        .count();
    let merged = app
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Merged)
        .count();

    let mut spans = vec![
        Span::styled(" wt status", Style::default().fg(Color::Cyan).bold()),
        Span::raw("                              "),
        Span::styled(format!("{} running", running), Style::default().fg(Color::Green)),
        Span::raw(" · "),
        Span::styled(format!("{} done", done), Style::default().fg(Color::Blue)),
    ];
    if merged > 0 {
        spans.push(Span::raw(" · "));
        spans.push(Span::styled(format!("{} merged", merged), Style::default().fg(Color::Magenta)));
    }
    let text = Line::from(spans);

    frame.render_widget(Paragraph::new(text), area);

    // Draw separator line
    if area.height > 1 {
        let sep_area = Rect::new(area.x, area.y + 1, area.width, 1);
        let sep = "─".repeat(area.width as usize);
        frame.render_widget(
            Paragraph::new(sep).style(Style::default().fg(Color::DarkGray)),
            sep_area,
        );
    }
}

fn draw_tasks(frame: &mut Frame, area: Rect, app: &App) {
    if app.tasks.is_empty() {
        let text = Paragraph::new(" No running or done tasks.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(text, area);
        return;
    }

    let mut lines = Vec::new();

    for (i, task) in app.tasks.iter().enumerate() {
        let is_selected = i == app.selected;
        let line = format_task_line(task, is_selected, area.width as usize);
        lines.push(line);
    }

    let text = Text::from(lines);
    frame.render_widget(Paragraph::new(text), area);
}

fn format_task_line(task: &TaskDisplay, selected: bool, _width: usize) -> Line<'static> {
    let mut spans = Vec::new();

    // Selection indicator
    if selected {
        spans.push(Span::styled(" ▸ ", Style::default().fg(Color::Yellow)));
    } else {
        spans.push(Span::raw("   "));
    }

    // Index (gray color - auxiliary info)
    spans.push(Span::styled(
        format!("{:>2}", task.index),
        Style::default().fg(Color::DarkGray),
    ));
    spans.push(Span::raw(" "));

    // Status icon with color (conflict overrides normal status)
    let (icon, icon_color) = if task.has_conflict {
        ("⚠", Color::Red)
    } else {
        get_status_icon(task)
    };
    spans.push(Span::styled(icon, Style::default().fg(icon_color)));
    spans.push(Span::raw(" "));

    // Task name (fixed width)
    let name = format!("{:<12}", truncate(&task.name, 12));
    let name_style = if selected {
        Style::default().fg(Color::White).bold()
    } else {
        Style::default().fg(Color::White)
    };
    spans.push(Span::styled(name, name_style));

    // Duration
    let duration = task
        .duration
        .as_ref()
        .map(|d| format!("{:>6}", d))
        .unwrap_or_else(|| "     -".to_string());
    spans.push(Span::styled(duration, Style::default().fg(Color::DarkGray)));

    // Context percent (colored by usage level)
    let ctx_color = if task.context_percent >= 95 {
        Color::Red
    } else if task.context_percent >= 80 {
        Color::Yellow
    } else {
        Color::Cyan
    };
    spans.push(Span::styled(
        format!(" {:>3}%", task.context_percent),
        Style::default().fg(ctx_color),
    ));

    // Commit count
    let commit_str = if task.commit_count > 0 {
        format!(" {:>2}c", task.commit_count)
    } else {
        "   -".to_string()
    };
    spans.push(Span::styled(commit_str, Style::default().fg(Color::Magenta)));

    // Changes (compact format)
    let changes = if task.additions > 0 || task.deletions > 0 {
        format!(" +{}/-{}", task.additions, task.deletions)
    } else {
        " -".to_string()
    };
    let changes_color = if task.additions > 0 || task.deletions > 0 {
        Color::Gray
    } else {
        Color::DarkGray
    };
    spans.push(Span::styled(format!("{:<12}", changes), Style::default().fg(changes_color)));

    // Conflict or current tool
    if task.has_conflict {
        spans.push(Span::styled(" ⚡CONFLICT", Style::default().fg(Color::Red).bold()));
    } else if let Some(tool) = &task.current_tool {
        let tool_display = format_tool_name(tool);
        spans.push(Span::styled(
            format!(" {}", tool_display),
            Style::default().fg(Color::DarkGray),
        ));
    }

    Line::from(spans)
}

fn format_tool_name(tool: &str) -> String {
    // Shorten common tool names for display
    let short = match tool {
        "Read" => "Read",
        "Write" => "Write",
        "Edit" => "Edit",
        "Bash" => "Bash",
        "Glob" => "Glob",
        "Grep" => "Grep",
        "Task" => "Task",
        "WebFetch" => "Web",
        "WebSearch" => "Search",
        other => {
            // Handle MCP tools like mcp__server__tool
            if other.starts_with("mcp__") {
                other.rsplit("__").next().unwrap_or(other)
            } else {
                other
            }
        }
    };
    truncate(short, 12)
}

fn get_status_icon(task: &TaskDisplay) -> (&'static str, Color) {
    match task.status {
        TaskStatus::Done => ("✓", Color::Green),
        TaskStatus::Merged => ("✓✓", Color::Magenta),
        TaskStatus::Archived => ("☑", Color::DarkGray),
        TaskStatus::Running => {
            if !task.tmux_alive {
                ("⚠", Color::Yellow)
            } else if task.active {
                ("●", Color::Green)
            } else {
                ("●", Color::Yellow)
            }
        }
        _ => ("○", Color::White),
    }
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    // Separator
    let sep = "─".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(sep).style(Style::default().fg(Color::DarkGray)),
        Rect::new(area.x, area.y, area.width, 1),
    );

    // Keybindings - context sensitive
    if area.height > 1 {
        let help_area = Rect::new(area.x, area.y + 1, area.width, 1);

        let mut spans = vec![
            Span::raw(" "),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" navigate  "),
            Span::styled("⏎", Style::default().fg(Color::Yellow)),
            Span::raw(" cd  "),
        ];

        // Context-sensitive actions based on selected task
        if let Some(task) = app.selected_task() {
            // t (tail) available for Running and Done
            if task.status == TaskStatus::Running || task.status == TaskStatus::Done {
                spans.push(Span::styled("t", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" tail  "));
            }

            if task.status == TaskStatus::Running {
                // Running: d (done)
                spans.push(Span::styled("d", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" done  "));
            } else if task.status == TaskStatus::Done {
                // Done: m (merged)
                spans.push(Span::styled("m", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" merged  "));
            } else if task.status == TaskStatus::Merged {
                // Merged: a (archive)
                spans.push(Span::styled("a", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" archive  "));
            }
        }

        spans.push(Span::styled("q", Style::default().fg(Color::Yellow)));
        spans.push(Span::raw(" quit"));

        frame.render_widget(Paragraph::new(Line::from(spans)), help_area);
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}
