use std::path::Path;
use std::time::SystemTime;

use serde::Serialize;

use crate::error::{Result, WtError};
use crate::services::command::CommandRunner;

/// Git worktree statistics
#[derive(Debug, Clone, Default, Serialize)]
pub struct GitMetrics {
    pub additions: i32,
    pub deletions: i32,
    pub commits: i32,
    pub has_conflict: bool,
}

/// Result of a rebase operation
#[derive(Debug, Clone, Serialize)]
pub enum RebaseResult {
    Success,
    AlreadyUpToDate,
    Conflicts,
}

/// Get git statistics for a worktree
pub fn get_worktree_metrics(worktree_path: &str) -> Option<GitMetrics> {
    let path = Path::new(worktree_path);
    if !path.exists() {
        return None;
    }

    let (additions, deletions) = get_diff_stats(worktree_path).unwrap_or((0, 0));
    let commits = get_commit_count(worktree_path, "main")
        .or_else(|| get_commit_count(worktree_path, "master"))
        .unwrap_or(0);
    let has_conflict = has_conflicts(worktree_path);

    Some(GitMetrics {
        additions,
        deletions,
        commits,
        has_conflict,
    })
}

pub fn create_worktree(branch: &str, path: &str) -> Result<()> {
    let worktree_path = Path::new(path);
    if let Some(parent) = worktree_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| WtError::Git(e.to_string()))?;
        }
    }

    CommandRunner::git().run(&["worktree", "add", "-b", branch, path])
}

pub fn remove_worktree(path: &str) -> Result<()> {
    CommandRunner::git().run(&["worktree", "remove", "--force", path])
}

pub fn delete_branch_in(branch: &str, cwd: &str) -> Result<()> {
    CommandRunner::git()
        .current_dir(cwd)
        .run(&["branch", "-D", branch])
}

/// Get the main repository root path (works from worktree).
pub fn get_repo_root() -> Result<String> {
    let git_dir = CommandRunner::git().output(&[
        "rev-parse",
        "--path-format=absolute",
        "--git-common-dir",
    ])?;
    let git_dir = git_dir.trim();
    // Strip /.git suffix
    Ok(git_dir.strip_suffix("/.git").unwrap_or(git_dir).to_string())
}

pub fn branch_exists(branch: &str) -> bool {
    CommandRunner::git().success(&[
        "show-ref",
        "--verify",
        "--quiet",
        &format!("refs/heads/{}", branch),
    ])
}

/// Find branches matching a pattern (e.g., "wt/task-*")
pub fn find_branches(pattern: &str) -> Vec<String> {
    let output = CommandRunner::git().output(&["branch", "--list", pattern]);
    match output {
        Ok(stdout) => stdout
            .lines()
            .map(|line| line.trim().trim_start_matches("* ").to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Get diff stats (additions, deletions) for a worktree compared to main branch.
/// Shows all changes on the branch, including committed ones.
pub fn get_diff_stats(worktree_path: &str) -> Option<(i32, i32)> {
    // Try to find the base branch (main or master)
    let base = get_default_branch(worktree_path).unwrap_or_else(|| "main".to_string());

    // Try committed changes first (main...HEAD)
    let output = CommandRunner::new("git")
        .current_dir(worktree_path)
        .output(&["diff", "--shortstat", &format!("{}...HEAD", base)]);

    if let Ok(stdout) = output {
        if let Some(stats) = parse_diff_stats(&stdout) {
            return Some(stats);
        }
    }

    // Fallback: show uncommitted changes (diff HEAD)
    let output = CommandRunner::new("git")
        .current_dir(worktree_path)
        .output(&["diff", "--shortstat", "HEAD"]);
    output.ok().and_then(|s| parse_diff_stats(&s))
}

/// Get the default branch name (main or master)
fn get_default_branch(worktree_path: &str) -> Option<String> {
    // Try main first
    let result = CommandRunner::new("git")
        .current_dir(worktree_path)
        .success(&["rev-parse", "--verify", "main"]);
    if result {
        return Some("main".to_string());
    }

    // Try master
    let result = CommandRunner::new("git")
        .current_dir(worktree_path)
        .success(&["rev-parse", "--verify", "master"]);
    if result {
        return Some("master".to_string());
    }

    None
}

/// Parse git diff --shortstat output like "3 files changed, 10 insertions(+), 5 deletions(-)"
fn parse_diff_stats(output: &str) -> Option<(i32, i32)> {
    let output = output.trim();
    if output.is_empty() {
        return None; // Return None to trigger fallback to uncommitted changes
    }

    let mut insertions = 0;
    let mut deletions = 0;

    for part in output.split(',') {
        let part = part.trim();
        if part.contains("insertion") {
            if let Some(num) = part.split_whitespace().next() {
                insertions = num.parse().unwrap_or(0);
            }
        } else if part.contains("deletion") {
            if let Some(num) = part.split_whitespace().next() {
                deletions = num.parse().unwrap_or(0);
            }
        }
    }

    Some((insertions, deletions))
}

/// Get the number of commits ahead of the base branch.
pub fn get_commit_count(worktree_path: &str, base_branch: &str) -> Option<i32> {
    let range = format!("{}..HEAD", base_branch);
    let output = CommandRunner::new("git")
        .current_dir(worktree_path)
        .output(&["rev-list", "--count", &range]);

    if let Ok(stdout) = output {
        stdout.trim().parse().ok()
    } else {
        None
    }
}

/// Check if the worktree has merge conflicts.
pub fn has_conflicts(worktree_path: &str) -> bool {
    // Check for unmerged files via git status
    let output = CommandRunner::new("git")
        .current_dir(worktree_path)
        .output(&["status", "--porcelain"]);

    if let Ok(stdout) = output {
        // Unmerged files have status like "UU", "AA", "DD", etc.
        stdout.lines().any(|line| {
            let chars: Vec<char> = line.chars().collect();
            if chars.len() >= 2 {
                let x = chars[0];
                let y = chars[1];
                // Unmerged statuses
                matches!((x, y), ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D'))
            } else {
                false
            }
        })
    } else {
        false
    }
}

/// Get the last modification time of any file in the worktree.
pub fn get_last_activity(worktree_path: &str) -> Option<SystemTime> {
    let path = Path::new(worktree_path);
    if !path.exists() {
        return None;
    }

    path.metadata().ok()?.modified().ok()
}

// ============================================================================
// Atomic Git Operations (for workflows)
// ============================================================================

/// Fetch from a remote repository.
pub fn fetch(repo_root: &str, remote: &str) -> Result<()> {
    CommandRunner::new("git")
        .current_dir(repo_root)
        .run(&["fetch", remote])
}

/// Rebase the current branch onto target.
pub fn rebase(worktree_path: &str, target: &str) -> Result<RebaseResult> {
    // First check if we're already up to date
    let merge_base = CommandRunner::new("git")
        .current_dir(worktree_path)
        .output(&["merge-base", "HEAD", target]);

    let target_commit = CommandRunner::new("git")
        .current_dir(worktree_path)
        .output(&["rev-parse", target]);

    if let (Ok(base), Ok(target_rev)) = (merge_base, target_commit) {
        if base.trim() == target_rev.trim() {
            // HEAD is already based on target
            let head = CommandRunner::new("git")
                .current_dir(worktree_path)
                .output(&["rev-parse", "HEAD"]);
            if let Ok(head_rev) = head {
                if head_rev.trim() == target_rev.trim() {
                    return Ok(RebaseResult::AlreadyUpToDate);
                }
            }
        }
    }

    let result = CommandRunner::new("git")
        .current_dir(worktree_path)
        .success(&["rebase", target]);

    if result {
        Ok(RebaseResult::Success)
    } else {
        // Check if there are conflicts
        if has_conflicts(worktree_path) {
            // Abort the rebase to leave clean state
            let _ = CommandRunner::new("git")
                .current_dir(worktree_path)
                .run(&["rebase", "--abort"]);
            Ok(RebaseResult::Conflicts)
        } else {
            Err(WtError::Git("rebase failed".to_string()))
        }
    }
}

/// Squash merge a branch into the current branch.
pub fn squash_merge(repo_root: &str, branch: &str) -> Result<()> {
    CommandRunner::new("git")
        .current_dir(repo_root)
        .run(&["merge", "--squash", branch])
}

/// Create a commit with the given message.
pub fn commit(path: &str, message: &str) -> Result<()> {
    CommandRunner::new("git")
        .current_dir(path)
        .run(&["commit", "-m", message])
}

/// Push a branch to a remote.
pub fn push(repo_root: &str, branch: &str, remote: &str) -> Result<()> {
    CommandRunner::new("git")
        .current_dir(repo_root)
        .run(&["push", remote, branch])
}

/// Check if the working directory has uncommitted changes.
pub fn has_changes(path: &str) -> Result<bool> {
    let output = CommandRunner::new("git")
        .current_dir(path)
        .output(&["status", "--porcelain"])?;

    Ok(!output.trim().is_empty())
}

/// Stash changes in the working directory.
pub fn stash(path: &str) -> Result<()> {
    CommandRunner::new("git")
        .current_dir(path)
        .run(&["stash", "push", "-u"])
}

/// Pop the most recent stash.
pub fn stash_pop(path: &str) -> Result<()> {
    CommandRunner::new("git")
        .current_dir(path)
        .run(&["stash", "pop"])
}

/// Create a new branch at the current HEAD.
pub fn create_branch(repo_root: &str, branch_name: &str) -> Result<()> {
    CommandRunner::new("git")
        .current_dir(repo_root)
        .run(&["branch", branch_name])
}

/// Delete a branch.
pub fn delete_branch(repo_root: &str, branch_name: &str) -> Result<()> {
    CommandRunner::new("git")
        .current_dir(repo_root)
        .run(&["branch", "-D", branch_name])
}

/// Checkout a branch.
pub fn checkout(path: &str, branch: &str) -> Result<()> {
    CommandRunner::new("git")
        .current_dir(path)
        .run(&["checkout", branch])
}

/// Get the current branch name.
pub fn current_branch(path: &str) -> Result<String> {
    let output = CommandRunner::new("git").current_dir(path).output(&[
        "rev-parse",
        "--abbrev-ref",
        "HEAD",
    ])?;
    Ok(output.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== parse_diff_stats Tests ====================

    #[test]
    fn test_parse_diff_stats_empty() {
        // Empty output returns None to trigger fallback to uncommitted changes
        assert_eq!(parse_diff_stats(""), None);
        assert_eq!(parse_diff_stats("  "), None);
    }

    #[test]
    fn test_parse_diff_stats_insertions_only() {
        assert_eq!(
            parse_diff_stats("1 file changed, 10 insertions(+)"),
            Some((10, 0))
        );
    }

    #[test]
    fn test_parse_diff_stats_deletions_only() {
        assert_eq!(
            parse_diff_stats("1 file changed, 5 deletions(-)"),
            Some((0, 5))
        );
    }

    #[test]
    fn test_parse_diff_stats_both() {
        assert_eq!(
            parse_diff_stats("3 files changed, 10 insertions(+), 5 deletions(-)"),
            Some((10, 5))
        );
    }

    #[test]
    fn test_parse_diff_stats_singular() {
        assert_eq!(
            parse_diff_stats("1 file changed, 1 insertion(+), 1 deletion(-)"),
            Some((1, 1))
        );
    }

    #[test]
    fn test_parse_diff_stats_large_numbers() {
        assert_eq!(
            parse_diff_stats("100 files changed, 9999 insertions(+), 5432 deletions(-)"),
            Some((9999, 5432))
        );
    }

    #[test]
    fn test_parse_diff_stats_files_only() {
        // Only files changed, no insertions or deletions (e.g., binary files)
        assert_eq!(
            parse_diff_stats("2 files changed"),
            Some((0, 0))
        );
    }

    // ==================== GitMetrics Tests ====================

    #[test]
    fn test_git_metrics_default() {
        let metrics = GitMetrics::default();
        assert_eq!(metrics.additions, 0);
        assert_eq!(metrics.deletions, 0);
        assert_eq!(metrics.commits, 0);
        assert!(!metrics.has_conflict);
    }

    #[test]
    fn test_git_metrics_clone() {
        let metrics = GitMetrics {
            additions: 10,
            deletions: 5,
            commits: 3,
            has_conflict: true,
        };
        let cloned = metrics.clone();
        assert_eq!(cloned.additions, 10);
        assert_eq!(cloned.deletions, 5);
        assert_eq!(cloned.commits, 3);
        assert!(cloned.has_conflict);
    }

    // ==================== RebaseResult Tests ====================

    #[test]
    fn test_rebase_result_debug() {
        assert!(format!("{:?}", RebaseResult::Success).contains("Success"));
        assert!(format!("{:?}", RebaseResult::AlreadyUpToDate).contains("AlreadyUpToDate"));
        assert!(format!("{:?}", RebaseResult::Conflicts).contains("Conflicts"));
    }

    // ==================== get_worktree_metrics Tests ====================

    #[test]
    fn test_get_worktree_metrics_nonexistent_path() {
        let result = get_worktree_metrics("/nonexistent/path/that/does/not/exist");
        assert!(result.is_none());
    }

    // ==================== get_last_activity Tests ====================

    #[test]
    fn test_get_last_activity_nonexistent_path() {
        let result = get_last_activity("/nonexistent/path/that/does/not/exist");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_last_activity_current_dir() {
        // Current directory should have a valid modification time
        let result = get_last_activity(".");
        assert!(result.is_some());
    }
}
