//! Display formatting utilities.

// ANSI color codes
pub const RESET: &str = "\x1b[0m";
pub const WHITE: &str = "\x1b[37m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const MAGENTA: &str = "\x1b[35m";
pub const GRAY: &str = "\x1b[90m";

/// Format index with gray color for display.
pub fn colored_index(idx: usize) -> String {
    format!("{}{}{}", GRAY, idx, RESET)
}

/// Running status icon with color based on tmux state and activity.
///
/// Returns (icon, color_code) tuple for terminal display.
/// - tmux dead: ⚠ yellow (warning)
/// - active: ● green
/// - idle: ● yellow
/// - unknown: ● green (default)
pub fn running_icon(tmux_alive: Option<bool>, active: Option<bool>) -> (&'static str, &'static str) {
    match tmux_alive {
        Some(false) => ("⚠", YELLOW), // tmux window closed
        _ => match active {
            Some(true) => ("●", GREEN),  // actively working
            Some(false) => ("●", YELLOW), // idle
            None => ("●", GREEN),         // unknown, default to green
        },
    }
}

/// Format duration in human-readable form (e.g., "1h 30m", "45s").
pub fn format_duration(secs: i64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        let mins = secs / 60;
        let remaining_secs = secs % 60;
        if remaining_secs == 0 {
            format!("{}m", mins)
        } else {
            format!("{}m {}s", mins, remaining_secs)
        }
    } else {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        if mins == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h {}m", hours, mins)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(59), "59s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(60), "1m");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3599), "59m 59s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3600), "1h");
        assert_eq!(format_duration(5400), "1h 30m");
        assert_eq!(format_duration(7200), "2h");
    }
}
