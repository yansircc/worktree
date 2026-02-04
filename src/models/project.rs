//! Project configuration for Phases system.
//!
//! Contains configuration structures used by WtConfig:
//! - PhasesConfig: Phase sequence and definitions
//! - ConcurrencyConfig: Concurrent task limits
//! - ProjectObserve: Observation settings

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::phase::Phase;

// ============================================================================
// Concurrency Config
// ============================================================================

/// Project concurrency configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConcurrencyConfig {
    /// Maximum concurrent active tasks
    #[serde(default = "default_max_active_tasks")]
    pub max_active_tasks: usize,
    /// Maximum concurrent agents
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,
}

fn default_max_active_tasks() -> usize {
    5
}

fn default_max_agents() -> usize {
    3
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_active_tasks: default_max_active_tasks(),
            max_agents: default_max_agents(),
        }
    }
}

// ============================================================================
// Phases Config
// ============================================================================

/// Project phases configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct PhasesConfig {
    /// Phase sequence (e.g., ["pending", "developing", "reviewing", "completed"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sequence: Vec<String>,
    /// Phase definitions (override defaults)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub definitions: HashMap<String, Phase>,
}

impl PhasesConfig {
    /// Get phase sequence (empty if not configured) - test only
    #[cfg(test)]
    pub fn sequence(&self) -> &[String] {
        &self.sequence
    }
}

// ============================================================================
// Observe Config
// ============================================================================

/// Project observation configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ProjectObserve {
    /// Enable dashboard
    #[serde(default)]
    pub dashboard: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concurrency_defaults() {
        let config = ConcurrencyConfig::default();
        assert_eq!(config.max_active_tasks, 5);
        assert_eq!(config.max_agents, 3);
    }

    #[test]
    fn test_phases_config_empty_sequence() {
        let config = PhasesConfig::default();
        assert!(config.sequence().is_empty());
    }

    #[test]
    fn test_phases_config_custom_sequence() {
        let config = PhasesConfig {
            sequence: vec!["a".to_string(), "b".to_string()],
            definitions: HashMap::new(),
        };
        assert_eq!(config.sequence(), &["a", "b"]);
    }
}
