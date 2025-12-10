use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tera::{Tera, Value};

/// Global context available to all templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalContext {
    pub version: String,
    pub timestamp: String,
    pub generator_name: String,
}

impl GlobalContext {
    pub fn new(generator_name: &str) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            generator_name: generator_name.to_string(),
        }
    }
}

/// Register common templates used across all generators
pub fn register_common_templates(tera: &mut Tera) -> Result<(), String> {
    tera.add_raw_template("common/header.tera", include_str!("templates/header.tera"))
        .map_err(|e| format!("Failed to register common/header.tera: {}", e))?;

    Ok(())
}

/// Register common string escaping filters
pub fn register_common_filters(tera: &mut Tera) {
    tera.register_filter("escape_js", escape_js_filter);
}

// === Common Filters ===

fn escape_js_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    if let Some(s) = value.as_str() {
        let escaped = s
            .replace('\\', "\\\\") // Backslash must be first
            .replace('"', "\\\"") // Escape double quotes
            .replace('\n', "\\n") // Escape newlines
            .replace('\r', "\\r") // Escape carriage returns
            .replace('\t', "\\t"); // Escape tabs
        Ok(Value::String(escaped))
    } else {
        Err("escape_js filter expects a string".into())
    }
}

/// Helper function to escape strings for JavaScript
pub fn escape_for_js(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
