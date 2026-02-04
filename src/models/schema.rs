//! JSON Schema generation for wt configuration.
//!
//! Uses schemars to generate JSON Schema from Rust types.
//! Generates Draft 7 schema for better VS Code compatibility.

use schemars::generate::SchemaSettings;

use crate::models::WtConfig;

/// Generate JSON Schema for WtConfig (Draft 7 for VS Code compatibility)
pub fn generate_config_schema() -> String {
    let settings = SchemaSettings::draft07();
    let generator = settings.into_generator();
    let schema = generator.into_root_schema_for::<WtConfig>();
    serde_json::to_string_pretty(&schema).expect("schema serialization should not fail")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_config_schema() {
        let schema = generate_config_schema();
        assert!(schema.contains("\"$schema\""));
        assert!(schema.contains("\"type\": \"object\""));
        assert!(schema.contains("multiplexer"));
        assert!(schema.contains("session_name"));
        assert!(schema.contains("phases"));
    }

    #[test]
    fn test_schema_is_valid_json() {
        let schema = generate_config_schema();
        let parsed: serde_json::Value = serde_json::from_str(&schema).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn test_schema_uses_draft07() {
        let schema = generate_config_schema();
        assert!(schema.contains("draft-07"));
    }
}
