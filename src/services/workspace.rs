//! Workspace initialization utilities for worktree setup.

use std::path::{Path, PathBuf};

use crate::error::{Result, WtError};

/// Helper for initializing a worktree workspace.
pub struct WorkspaceInitializer<'a> {
    worktree_path: &'a str,
    source_dir: &'a Path,
}

impl<'a> WorkspaceInitializer<'a> {
    /// Create a new workspace initializer.
    pub fn new(worktree_path: &'a str, source_dir: &'a Path) -> Self {
        Self {
            worktree_path,
            source_dir,
        }
    }

    /// Copy files from source directory to worktree.
    ///
    /// Returns list of successfully copied files.
    pub fn copy_files(&self, files: &[String]) -> Result<Vec<String>> {
        let mut copied = Vec::new();

        for file in files {
            let src = self.source_dir.join(file);
            let dest = PathBuf::from(self.worktree_path).join(file);

            if src.exists() {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| WtError::Io {
                        operation: "create directory".to_string(),
                        path: parent.to_string_lossy().to_string(),
                        message: e.to_string(),
                    })?;
                }
                std::fs::copy(&src, &dest).map_err(|e| WtError::Io {
                    operation: "copy file".to_string(),
                    path: file.clone(),
                    message: e.to_string(),
                })?;
                copied.push(file.clone());
            }
        }

        Ok(copied)
    }

    /// Create symlink for status.json so wt commands work from worktree.
    pub fn link_status_file(&self) -> Result<()> {
        use std::os::unix::fs::symlink;

        let wt_dir = PathBuf::from(self.worktree_path).join(".wt");
        let link_path = wt_dir.join("status.json");
        let target = self.source_dir.join(".wt/status.json");

        // Ensure .wt directory exists in worktree
        std::fs::create_dir_all(&wt_dir).map_err(|e| WtError::Io {
            operation: "create .wt directory".to_string(),
            path: wt_dir.to_string_lossy().to_string(),
            message: e.to_string(),
        })?;

        // Remove existing file/link if present
        let _ = std::fs::remove_file(&link_path);

        // Create symlink
        symlink(&target, &link_path).map_err(|e| WtError::Io {
            operation: "create symlink".to_string(),
            path: link_path.to_string_lossy().to_string(),
            message: e.to_string(),
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_dirs() -> (TempDir, TempDir) {
        let src = TempDir::new().unwrap();
        let dest = TempDir::new().unwrap();
        (src, dest)
    }

    #[test]
    fn test_copy_files_existing() {
        let (src_dir, dest_dir) = setup_test_dirs();

        // Create source file
        let src_file = src_dir.path().join("test.txt");
        std::fs::write(&src_file, "content").unwrap();

        let init = WorkspaceInitializer::new(dest_dir.path().to_str().unwrap(), src_dir.path());

        let copied = init.copy_files(&["test.txt".to_string()]).unwrap();

        assert_eq!(copied, vec!["test.txt"]);
        assert!(dest_dir.path().join("test.txt").exists());
    }

    #[test]
    fn test_copy_files_nonexistent() {
        let (src_dir, dest_dir) = setup_test_dirs();

        let init = WorkspaceInitializer::new(dest_dir.path().to_str().unwrap(), src_dir.path());

        let copied = init.copy_files(&["nonexistent.txt".to_string()]).unwrap();

        assert!(copied.is_empty());
    }

    #[test]
    fn test_copy_files_nested() {
        let (src_dir, dest_dir) = setup_test_dirs();

        // Create nested source file
        let nested_dir = src_dir.path().join("config");
        std::fs::create_dir(&nested_dir).unwrap();
        std::fs::write(nested_dir.join("app.json"), "{}").unwrap();

        let init = WorkspaceInitializer::new(dest_dir.path().to_str().unwrap(), src_dir.path());

        let copied = init.copy_files(&["config/app.json".to_string()]).unwrap();

        assert_eq!(copied, vec!["config/app.json"]);
        assert!(dest_dir.path().join("config/app.json").exists());
    }

}
