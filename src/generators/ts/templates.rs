use crate::generators::base::templates::{register_common_filters, register_common_templates};
use std::collections::HashMap;
use tera::{Context, Tera, Value};

/// Create and configure a Tera template engine for TypeScript generator
pub fn create_template_engine() -> Result<Tera, String> {
    let mut tera = Tera::default();

    // Register common templates
    register_common_templates(&mut tera)?;

    // Register typescript-specific templates (if any)
    register_templates(&mut tera)?;

    // Register common filters
    register_common_filters(&mut tera);

    // Register typescript-specific filters
    register_typescript_filters(&mut tera);

    Ok(tera)
}

/// Register TypeScript-specific Tera filters
fn register_typescript_filters(tera: &mut Tera) {
    tera.register_filter("add_types_prefix", add_types_prefix_filter);
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

/// Register typescript-specific templates from embedded strings
fn register_templates(tera: &mut Tera) -> Result<(), String> {
    // Macro to reduce boilerplate for template registration
    macro_rules! template {
        ($name:expr, $path:expr) => {
            tera.add_raw_template($name, include_str!($path))
                .map_err(|e| format!("Failed to register {}: {}", $name, e))?;
        };
    }

    // Main templates
    template!("typescript/types.ts.tera", "templates/types.ts.tera");
    template!("typescript/commands.ts.tera", "templates/commands.ts.tera");
    template!("typescript/events.ts.tera", "templates/events.ts.tera");
    template!("typescript/index.ts.tera", "templates/index.ts.tera");

    // Partial templates
    template!(
        "typescript/partials/interface.tera",
        "templates/partials/interface.tera"
    );
    template!(
        "typescript/partials/enum.tera",
        "templates/partials/enum.tera"
    );
    template!(
        "typescript/partials/param_interface.ts.tera",
        "templates/partials/param_interface.ts.tera"
    );
    template!(
        "typescript/partials/command_function.ts.tera",
        "templates/partials/command_function.ts.tera"
    );
    template!(
        "typescript/partials/event_listener.ts.tera",
        "templates/partials/event_listener.ts.tera"
    );

    Ok(())
}

/// Render a template with the given context
pub fn render(tera: &Tera, template_name: &str, context: &Context) -> Result<String, String> {
    tera.render(template_name, context).map_err(|e| {
        // Get more detailed error information
        let mut error_msg = format!("Failed to render template '{}': {}", template_name, e);

        // Check if there's a source error
        if let Some(source) = std::error::Error::source(&e) {
            error_msg.push_str(&format!("\nSource: {}", source));
        }

        error_msg
    })
}
