//! wt init command - initialize a new wt project.

mod config;
mod templates;

use std::env;
use std::fs;
use std::path::Path;

use crate::constants::TASKS_DIR;
use crate::error::{Result, WtError};
use crate::models::CONFIG_FILE;

use config::generate_config;
use templates::{
    GITIGNORE_ENTRIES, GITIGNORE_MARKER, VERIFY_MD, VERIFY_SETTINGS_JSON, VERIFY_STOP_CJS,
};

fn get_project_name() -> String {
    env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "wt".to_string())
}

fn create_verify_templates(wt_dir: &Path) -> Result<()> {
    // Create .wt/hooks/ directory
    let hooks_dir = wt_dir.join("hooks");
    if !hooks_dir.exists() {
        fs::create_dir_all(&hooks_dir).map_err(|e| WtError::Io {
            operation: "create".to_string(),
            path: hooks_dir.display().to_string(),
            message: e.to_string(),
        })?;
    }

    // Create .wt/templates/ directory
    let templates_dir = wt_dir.join("templates");
    if !templates_dir.exists() {
        fs::create_dir_all(&templates_dir).map_err(|e| WtError::Io {
            operation: "create".to_string(),
            path: templates_dir.display().to_string(),
            message: e.to_string(),
        })?;
    }

    // Create verify.md
    let verify_md_path = wt_dir.join("verify.md");
    fs::write(&verify_md_path, VERIFY_MD).map_err(|e| WtError::Io {
        operation: "create".to_string(),
        path: verify_md_path.display().to_string(),
        message: e.to_string(),
    })?;

    // Create hooks/verify-stop.cjs
    let verify_stop_path = hooks_dir.join("verify-stop.cjs");
    fs::write(&verify_stop_path, VERIFY_STOP_CJS).map_err(|e| WtError::Io {
        operation: "create".to_string(),
        path: verify_stop_path.display().to_string(),
        message: e.to_string(),
    })?;

    // Create templates/verify-settings.json
    let verify_settings_path = templates_dir.join("verify-settings.json");
    fs::write(&verify_settings_path, VERIFY_SETTINGS_JSON).map_err(|e| WtError::Io {
        operation: "create".to_string(),
        path: verify_settings_path.display().to_string(),
        message: e.to_string(),
    })?;

    println!("Created .wt/verify.md");
    println!("Created .wt/hooks/verify-stop.cjs");
    println!("Created .wt/templates/verify-settings.json");

    Ok(())
}

fn update_gitignore() -> Result<bool> {
    let gitignore_path = Path::new(".gitignore");

    if gitignore_path.exists() {
        let content = fs::read_to_string(gitignore_path).map_err(|e| WtError::Io {
            operation: "read".to_string(),
            path: ".gitignore".to_string(),
            message: e.to_string(),
        })?;

        // Check if already has wt entries
        if content.contains(GITIGNORE_MARKER) {
            return Ok(false);
        }

        // Append to existing .gitignore
        let new_content = if content.ends_with('\n') {
            format!("{}\n{}", content, GITIGNORE_ENTRIES)
        } else {
            format!("{}\n\n{}", content, GITIGNORE_ENTRIES)
        };

        fs::write(gitignore_path, new_content).map_err(|e| WtError::Io {
            operation: "write".to_string(),
            path: ".gitignore".to_string(),
            message: e.to_string(),
        })?;
    } else {
        // Create new .gitignore
        fs::write(gitignore_path, GITIGNORE_ENTRIES).map_err(|e| WtError::Io {
            operation: "create".to_string(),
            path: ".gitignore".to_string(),
            message: e.to_string(),
        })?;
    }

    Ok(true)
}

pub fn execute() -> Result<()> {
    let wt_dir = Path::new(".wt");
    let config_path = Path::new(CONFIG_FILE);
    let tasks_dir = Path::new(TASKS_DIR);

    // Check if already initialized
    if config_path.exists() {
        return Err(WtError::Io {
            operation: "init".to_string(),
            path: CONFIG_FILE.to_string(),
            message: "already exists. Remove it first if you want to reinitialize.".to_string(),
        });
    }

    let project_name = get_project_name();

    // Create .wt/ directory
    if !wt_dir.exists() {
        fs::create_dir(wt_dir).map_err(|e| WtError::Io {
            operation: "create".to_string(),
            path: ".wt".to_string(),
            message: e.to_string(),
        })?;
    }

    // Create .wt/config.jsonc
    let config_content = generate_config(&project_name);
    fs::write(config_path, &config_content).map_err(|e| WtError::Io {
        operation: "create".to_string(),
        path: CONFIG_FILE.to_string(),
        message: e.to_string(),
    })?;
    println!("Created {}", CONFIG_FILE);

    // Create .wt/tasks/ directory
    if !tasks_dir.exists() {
        fs::create_dir_all(tasks_dir).map_err(|e| WtError::Io {
            operation: "create".to_string(),
            path: TASKS_DIR.to_string(),
            message: e.to_string(),
        })?;
        println!("Created {}/", TASKS_DIR);
    }

    // Create verify templates for agent self-verification
    create_verify_templates(wt_dir)?;

    // Update .gitignore
    if update_gitignore()? {
        println!("Updated .gitignore");
    } else {
        println!(".gitignore already has wt entries");
    }

    // Install shell completions if not already installed
    if !super::completions::is_installed() {
        println!();
        println!("Installing shell completions...");
        match super::completions::install() {
            Ok(()) => {}
            Err(e) => {
                println!("  Warning: Failed to install completions: {}", e);
                println!("  You can install manually with: wt completions install");
            }
        }
    }

    // Summary
    println!();
    println!("Initialized wt for project '{}'", project_name);
    println!();
    println!("Next steps:");
    println!("  1. Edit {} to customize settings", CONFIG_FILE);
    println!(
        "  2. Create tasks: wt create --json '{{\"name\": \"...\", \"description\": \"...\"}}'"
    );
    println!("  3. Start working: wt next <task>");
    println!();
    println!("Agent self-verification:");
    println!("  Config already enables verify-settings.json in developing phase.");
    println!("  Customize .wt/verify.md to define your quality checklist.");

    Ok(())
}
