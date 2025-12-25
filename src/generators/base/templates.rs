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

#[cfg(test)]
mod tests {
    use super::*;

    mod global_context {
        use super::*;

        #[test]
        fn test_new_creates_context() {
            let ctx = GlobalContext::new("test-generator");
            assert_eq!(ctx.generator_name, "test-generator");
            assert_eq!(ctx.version, env!("CARGO_PKG_VERSION"));
            assert!(!ctx.timestamp.is_empty());
        }

        #[test]
        fn test_timestamp_format() {
            let ctx = GlobalContext::new("test");
            // RFC3339 format should contain T and Z
            assert!(ctx.timestamp.contains('T'));
            assert!(ctx.timestamp.ends_with('Z') || ctx.timestamp.contains('+'));
        }

        #[test]
        fn test_clone_works() {
            let ctx1 = GlobalContext::new("test");
            let ctx2 = ctx1.clone();
            assert_eq!(ctx1.generator_name, ctx2.generator_name);
            assert_eq!(ctx1.version, ctx2.version);
            assert_eq!(ctx1.timestamp, ctx2.timestamp);
        }
    }

    mod escape_js_filter {
        use super::*;

        #[test]
        fn test_escapes_backslash() {
            let value = Value::String("test\\path".to_string());
            let result = escape_js_filter(&value, &HashMap::new()).unwrap();
            assert_eq!(result.as_str().unwrap(), "test\\\\path");
        }

        #[test]
        fn test_escapes_double_quotes() {
            let value = Value::String("test\"quoted\"".to_string());
            let result = escape_js_filter(&value, &HashMap::new()).unwrap();
            assert_eq!(result.as_str().unwrap(), "test\\\"quoted\\\"");
        }

        #[test]
        fn test_escapes_newline() {
            let value = Value::String("line1\nline2".to_string());
            let result = escape_js_filter(&value, &HashMap::new()).unwrap();
            assert_eq!(result.as_str().unwrap(), "line1\\nline2");
        }

        #[test]
        fn test_escapes_carriage_return() {
            let value = Value::String("test\rvalue".to_string());
            let result = escape_js_filter(&value, &HashMap::new()).unwrap();
            assert_eq!(result.as_str().unwrap(), "test\\rvalue");
        }

        #[test]
        fn test_escapes_tab() {
            let value = Value::String("test\tvalue".to_string());
            let result = escape_js_filter(&value, &HashMap::new()).unwrap();
            assert_eq!(result.as_str().unwrap(), "test\\tvalue");
        }

        #[test]
        fn test_escapes_multiple_special_chars() {
            let value = Value::String("test\\\"line\n\ttab".to_string());
            let result = escape_js_filter(&value, &HashMap::new()).unwrap();
            assert_eq!(result.as_str().unwrap(), "test\\\\\\\"line\\n\\ttab");
        }

        #[test]
        fn test_empty_string() {
            let value = Value::String("".to_string());
            let result = escape_js_filter(&value, &HashMap::new()).unwrap();
            assert_eq!(result.as_str().unwrap(), "");
        }

        #[test]
        fn test_no_special_chars() {
            let value = Value::String("hello world".to_string());
            let result = escape_js_filter(&value, &HashMap::new()).unwrap();
            assert_eq!(result.as_str().unwrap(), "hello world");
        }

        #[test]
        fn test_non_string_value_errors() {
            let value = Value::Number(42.into());
            let result = escape_js_filter(&value, &HashMap::new());
            assert!(result.is_err());
        }
    }

    mod add_types_prefix_func {
        use super::*;

        #[test]
        fn test_primitives_unchanged() {
            assert_eq!(add_types_prefix("void"), "void");
            assert_eq!(add_types_prefix("string"), "string");
            assert_eq!(add_types_prefix("number"), "number");
            assert_eq!(add_types_prefix("boolean"), "boolean");
            assert_eq!(add_types_prefix("any"), "any");
            assert_eq!(add_types_prefix("unknown"), "unknown");
            assert_eq!(add_types_prefix("null"), "null");
            assert_eq!(add_types_prefix("undefined"), "undefined");
        }

        #[test]
        fn test_custom_type_gets_prefix() {
            assert_eq!(add_types_prefix("User"), "types.User");
            assert_eq!(add_types_prefix("Product"), "types.Product");
        }

        #[test]
        fn test_already_prefixed_unchanged() {
            assert_eq!(add_types_prefix("types.User"), "types.User");
            assert_eq!(add_types_prefix("types.Product"), "types.Product");
        }

        #[test]
        fn test_primitive_arrays_unchanged() {
            assert_eq!(add_types_prefix("string[]"), "string[]");
            assert_eq!(add_types_prefix("number[]"), "number[]");
            assert_eq!(add_types_prefix("boolean[]"), "boolean[]");
        }

        #[test]
        fn test_custom_type_arrays_get_prefix() {
            assert_eq!(add_types_prefix("User[]"), "types.User[]");
            assert_eq!(add_types_prefix("Product[]"), "types.Product[]");
        }

        #[test]
        fn test_union_with_null() {
            assert_eq!(add_types_prefix("User | null"), "types.User | null");
            assert_eq!(add_types_prefix("string | null"), "string | null");
        }

        #[test]
        fn test_union_with_undefined() {
            assert_eq!(
                add_types_prefix("User | undefined"),
                "types.User | undefined"
            );
            assert_eq!(add_types_prefix("number | undefined"), "number | undefined");
        }

        #[test]
        fn test_record_types_unchanged() {
            assert_eq!(
                add_types_prefix("Record<string, number>"),
                "Record<string, number>"
            );
            assert_eq!(
                add_types_prefix("Record<string, User>"),
                "Record<string, User>"
            );
        }

        #[test]
        fn test_map_types_unchanged() {
            assert_eq!(
                add_types_prefix("Map<string, number>"),
                "Map<string, number>"
            );
        }

        #[test]
        fn test_tuple_types_unchanged() {
            assert_eq!(add_types_prefix("[string, number]"), "[string, number]");
            assert_eq!(add_types_prefix("[User, Product]"), "[User, Product]");
        }

        #[test]
        fn test_empty_array_syntax() {
            // "[]" by itself is treated as a custom type name, gets prefix
            // This is an edge case - in practice arrays are always "SomeType[]"
            assert_eq!(add_types_prefix("[]"), "types.[]");
        }
    }

    mod add_types_prefix_filter_func {
        use super::*;

        #[test]
        fn test_filter_with_custom_type() {
            let value = Value::String("User".to_string());
            let result = add_types_prefix_filter(&value, &HashMap::new()).unwrap();
            assert_eq!(result.as_str().unwrap(), "types.User");
        }

        #[test]
        fn test_filter_with_primitive() {
            let value = Value::String("string".to_string());
            let result = add_types_prefix_filter(&value, &HashMap::new()).unwrap();
            assert_eq!(result.as_str().unwrap(), "string");
        }

        #[test]
        fn test_filter_with_array() {
            let value = Value::String("User[]".to_string());
            let result = add_types_prefix_filter(&value, &HashMap::new()).unwrap();
            assert_eq!(result.as_str().unwrap(), "types.User[]");
        }

        #[test]
        fn test_filter_non_string_errors() {
            let value = Value::Bool(true);
            let result = add_types_prefix_filter(&value, &HashMap::new());
            assert!(result.is_err());
        }
    }
}
