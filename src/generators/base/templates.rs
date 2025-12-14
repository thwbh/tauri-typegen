use crate::template;
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

pub trait TemplateRegistry: Sized {
    fn create_tera() -> Result<Tera, String> {
        let mut tera = Tera::default();

        // register common template
        template!(tera, "common/header.tera", "templates/header.tera");

        // register common filters
        tera.register_filter("escape_js", escape_js_filter);
        tera.register_filter("add_types_prefix", add_types_prefix_filter);

        // register registry specific templates
        Self::register_templates(&mut tera)?;

        // register registry specific filters
        Self::register_filters(&mut tera);

        Ok(tera)
    }

    /// Register specific templates used in the associated generator
    fn register_templates(tera: &mut Tera) -> Result<(), String>;

    /// Register spefific filters used in the associated generator
    fn register_filters(tera: &mut Tera);
}

/// Filter to escape problematic JS characters
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

/// Filter to add "types." prefix to custom types for namespace imports
/// Usage: {{ some_type | add_types_prefix }}
///
/// Examples:
/// - "User" -> "types.User"
/// - "string" -> "string" (primitives unchanged)
/// - "User[]" -> "types.User[]"
/// - "User | null" -> "types.User | null"
fn add_types_prefix_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    if let Some(ts_type) = value.as_str() {
        let prefixed = add_types_prefix(ts_type);
        Ok(Value::String(prefixed))
    } else {
        Err("add_types_prefix filter expects a string".into())
    }
}

/// Add "types." prefix to custom types for use in function signatures
fn add_types_prefix(ts_type: &str) -> String {
    // Handle primitives - no prefix needed
    if matches!(
        ts_type,
        "void" | "string" | "number" | "boolean" | "any" | "unknown" | "null" | "undefined"
    ) {
        return ts_type.to_string();
    }

    // Handle arrays: CustomType[] -> types.CustomType[]
    if let Some(base_type) = ts_type.strip_suffix("[]") {
        if matches!(base_type, "string" | "number" | "boolean" | "void") {
            return ts_type.to_string();
        }
        return format!("types.{}[]", base_type);
    }

    // Handle Record/Map - they contain types but the structure itself doesn't need prefix
    if ts_type.starts_with("Record<") || ts_type.starts_with("Map<") {
        return ts_type.to_string();
    }

    // Handle union with null: CustomType | null -> types.CustomType | null
    if ts_type.ends_with(" | null") {
        let base = ts_type.strip_suffix(" | null").unwrap();
        return format!("{} | null", add_types_prefix(base));
    }

    // Handle union with undefined: CustomType | undefined -> types.CustomType | undefined
    if ts_type.ends_with(" | undefined") {
        let base = ts_type.strip_suffix(" | undefined").unwrap();
        return format!("{} | undefined", add_types_prefix(base));
    }

    // Handle tuples [T, U, ...] - keep as is since they're inline
    if ts_type.starts_with('[') && ts_type.ends_with(']') {
        return ts_type.to_string();
    }

    // Custom type - add prefix if not already present
    if ts_type.starts_with("types.") {
        ts_type.to_string()
    } else {
        format!("types.{}", ts_type)
    }
}
