//! wt validate command - validate tasks and configuration.

use std::fs;
use std::path::Path;

use crate::constants::TASKS_DIR;
use crate::error::{Result, WtError};
use crate::models::{TaskStore, WtConfig, CONFIG_FILE};

/// Schema file path
const SCHEMA_FILE: &str = ".wt/config.schema.json";

/// Validate that all phases in sequence have definitions
fn validate_phase_definitions(config: &WtConfig) -> Vec<String> {
    let seq = match config.phase_sequence() {
        Ok(s) => s,
        Err(_) => return vec!["No phases configured. Run 'wt init' to create config.".to_string()],
    };
    let mut errors = vec![];

    for phase_id in &seq {
        if config.get_phase(phase_id).is_none() {
            errors.push(format!(
                "Phase '{}' is in sequence but not defined in phases.definitions. \
                 All phases must have explicit definitions.",
                phase_id
            ));
        }
    }

    errors
}

/// Validate config.jsonc against the JSON Schema
fn validate_config_schema() -> Result<Vec<String>> {
    let schema_path = Path::new(SCHEMA_FILE);
    let config_path = Path::new(CONFIG_FILE);

    // If schema doesn't exist, skip validation (not initialized with schema support)
    if !schema_path.exists() {
        return Ok(vec![]);
    }

    // If config doesn't exist, skip (will be caught elsewhere)
    if !config_path.exists() {
        return Ok(vec![]);
    }

    // Load schema
    let schema_content = fs::read_to_string(schema_path).map_err(|e| WtError::Io {
        operation: "read".to_string(),
        path: SCHEMA_FILE.to_string(),
        message: e.to_string(),
    })?;

    let schema: serde_json::Value =
        serde_json::from_str(&schema_content).map_err(|e| WtError::ConfigRead(e.to_string()))?;

    // Load config (strip comments first)
    let config_content = fs::read_to_string(config_path).map_err(|e| WtError::Io {
        operation: "read".to_string(),
        path: CONFIG_FILE.to_string(),
        message: e.to_string(),
    })?;

    // Strip JSONC comments
    let stripped = json_comments::StripComments::new(config_content.as_bytes());
    let config: serde_json::Value =
        serde_json::from_reader(stripped).map_err(|e| WtError::ConfigRead(e.to_string()))?;

    // Create validator
    let validator = jsonschema::validator_for(&schema)
        .map_err(|e| WtError::ConfigRead(format!("Invalid schema: {}", e)))?;

    // Collect errors
    let errors: Vec<String> = validator
        .iter_errors(&config)
        .map(|e| {
            let path = e.instance_path.to_string();
            if path.is_empty() {
                e.to_string()
            } else {
                format!("{} at {}", e, path)
            }
        })
        .collect();

    Ok(errors)
}

pub fn execute(task_ref: Option<String>) -> Result<()> {
    let mut has_errors = false;

    // First validate config schema
    let schema_errors = validate_config_schema()?;
    if !schema_errors.is_empty() {
        println!("Config validation errors:");
        for error in &schema_errors {
            println!("  ✗ {}", error);
        }
        println!();
        has_errors = true;
    }

    // Validate phase definitions
    let config = WtConfig::load()?;
    let phase_errors = validate_phase_definitions(&config);
    if !phase_errors.is_empty() {
        println!("Phase definition errors:");
        for error in &phase_errors {
            println!("  ✗ {}", error);
        }
        println!();
        has_errors = true;
    }

    // Then validate tasks
    let store = TaskStore::load()?;

    // Resolve task reference to name if provided
    let name = match task_ref {
        Some(ref r) => Some(store.resolve_task_ref(r)?),
        None => None,
    };

    // Task existence already checked by resolve_task_ref

    if store.tasks.is_empty() {
        if !has_errors {
            println!("No tasks found in {}/", TASKS_DIR);
        }
        return Ok(());
    }

    let errors = store.validate();

    // Filter by name if specified
    let errors: Vec<_> = if let Some(ref name) = name {
        errors
            .into_iter()
            .filter(|(n, _)| n == name || n.contains(name))
            .collect()
    } else {
        errors
    };

    if errors.is_empty() {
        let count = if name.is_some() { 1 } else { store.tasks.len() };
        if !has_errors {
            println!("✓ Config valid.");
        }
        println!("✓ All {} task(s) valid.", count);
    } else {
        println!("Task validation errors:");
        for (task, error) in &errors {
            println!("  ✗ {}: {}", task, error);
        }
        println!();
        println!("{} task error(s) found.", errors.len());
        has_errors = true;
    }

    if has_errors {
        println!();
        println!(
            "Total: {} config error(s), {} phase error(s), {} task error(s).",
            schema_errors.len(),
            phase_errors.len(),
            errors.len()
        );
    }

    Ok(())
}
