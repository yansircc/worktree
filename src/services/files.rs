//! File operations for hooks system.

use std::fs;
use std::path::Path;

use chrono::Local;

use crate::constants::BACKUPS_DIR;
use crate::error::{Result, WtError};

/// Backup worktree to .wt/backups/.
///
/// Creates a timestamped backup directory and copies the worktree contents.
/// Returns the backup path.
pub fn backup(task: &str, worktree_path: &str, backup_dir: Option<&str>) -> Result<String> {
    let backup_base = backup_dir.unwrap_or(BACKUPS_DIR);
    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let backup_path = format!("{}/{}-{}", backup_base, task, timestamp);

    // Create backup directory
    fs::create_dir_all(&backup_path).map_err(|e| WtError::Io {
        operation: "create backup directory".to_string(),
        path: backup_path.clone(),
        message: e.to_string(),
    })?;

    // Copy worktree contents to backup
    copy_dir_recursive(Path::new(worktree_path), Path::new(&backup_path))?;

    Ok(backup_path)
}

/// Clean directories matching patterns (e.g., target/, node_modules/).
///
/// Patterns can be relative paths like "target" or "node_modules".
pub fn clean(worktree_path: &str, patterns: &[&str]) -> Result<()> {
    let base = Path::new(worktree_path);

    for pattern in patterns {
        let path = base.join(pattern);
        if path.exists() {
            if path.is_dir() {
                fs::remove_dir_all(&path).map_err(|e| WtError::Io {
                    operation: "remove directory".to_string(),
                    path: path.to_string_lossy().to_string(),
                    message: e.to_string(),
                })?;
            } else {
                fs::remove_file(&path).map_err(|e| WtError::Io {
                    operation: "remove file".to_string(),
                    path: path.to_string_lossy().to_string(),
                    message: e.to_string(),
                })?;
            }
        }
    }

    Ok(())
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    if !src.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(src).map_err(|e| WtError::Io {
        operation: "read directory".to_string(),
        path: src.to_string_lossy().to_string(),
        message: e.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| WtError::Io {
            operation: "read directory entry".to_string(),
            path: src.to_string_lossy().to_string(),
            message: e.to_string(),
        })?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dest_path = dest.join(&file_name);

        // Skip .git directory
        if file_name == ".git" {
            continue;
        }

        if path.is_dir() {
            fs::create_dir_all(&dest_path).map_err(|e| WtError::Io {
                operation: "create directory".to_string(),
                path: dest_path.to_string_lossy().to_string(),
                message: e.to_string(),
            })?;
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path).map_err(|e| WtError::Io {
                operation: "copy file".to_string(),
                path: path.to_string_lossy().to_string(),
                message: e.to_string(),
            })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_backup_creates_timestamped_directory() {
        let src_dir = TempDir::new().unwrap();
        let backup_dir = TempDir::new().unwrap();

        // Create some files in source
        fs::write(src_dir.path().join("test.txt"), "content").unwrap();

        let result = backup(
            "test-task",
            src_dir.path().to_str().unwrap(),
            Some(backup_dir.path().to_str().unwrap()),
        );

        assert!(result.is_ok());
        let backup_path = result.unwrap();
        assert!(backup_path.contains("test-task"));
        assert!(Path::new(&backup_path).exists());
        assert!(Path::new(&backup_path).join("test.txt").exists());
    }

    #[test]
    fn test_backup_skips_git_directory() {
        let src_dir = TempDir::new().unwrap();
        let backup_dir = TempDir::new().unwrap();

        // Create .git directory
        fs::create_dir(src_dir.path().join(".git")).unwrap();
        fs::write(src_dir.path().join(".git/config"), "git config").unwrap();
        fs::write(src_dir.path().join("file.txt"), "content").unwrap();

        let result = backup(
            "test",
            src_dir.path().to_str().unwrap(),
            Some(backup_dir.path().to_str().unwrap()),
        );

        assert!(result.is_ok());
        let backup_path = result.unwrap();
        assert!(!Path::new(&backup_path).join(".git").exists());
        assert!(Path::new(&backup_path).join("file.txt").exists());
    }

    #[test]
    fn test_clean_removes_directories() {
        let dir = TempDir::new().unwrap();

        // Create directories to clean
        fs::create_dir(dir.path().join("target")).unwrap();
        fs::write(dir.path().join("target/debug"), "content").unwrap();
        fs::create_dir(dir.path().join("node_modules")).unwrap();
        fs::write(dir.path().join("keep.txt"), "keep").unwrap();

        let result = clean(dir.path().to_str().unwrap(), &["target", "node_modules"]);

        assert!(result.is_ok());
        assert!(!dir.path().join("target").exists());
        assert!(!dir.path().join("node_modules").exists());
        assert!(dir.path().join("keep.txt").exists());
    }

    #[test]
    fn test_clean_nonexistent_patterns() {
        let dir = TempDir::new().unwrap();

        let result = clean(dir.path().to_str().unwrap(), &["nonexistent"]);

        assert!(result.is_ok());
    }

    #[test]
    fn test_copy_dir_recursive() {
        let src = TempDir::new().unwrap();
        let dest = TempDir::new().unwrap();

        // Create nested structure
        fs::create_dir_all(src.path().join("a/b")).unwrap();
        fs::write(src.path().join("a/b/file.txt"), "content").unwrap();
        fs::write(src.path().join("root.txt"), "root").unwrap();

        let result = copy_dir_recursive(src.path(), dest.path());

        assert!(result.is_ok());
        assert!(dest.path().join("a/b/file.txt").exists());
        assert!(dest.path().join("root.txt").exists());
    }
}
