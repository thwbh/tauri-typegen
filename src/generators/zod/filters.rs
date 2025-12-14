use crate::models::{TypeStructure, ValidatorAttributes};
use std::collections::HashMap;
use tera::Value;

/// Format TypeScript return type with types. prefix for custom types
pub fn format_return_type_filter(
    value: &Value,
    _args: &HashMap<String, Value>,
) -> tera::Result<Value> {
    if let Some(ts_type) = value.as_str() {
        let formatted = format_typescript_return_type(ts_type);
        Ok(Value::String(formatted))
    } else {
        Err("format_return_type filter expects a string".into())
    }
}

fn format_typescript_return_type(ts_type: &str) -> String {
    // Handle primitives that don't need types. prefix
    match ts_type {
        "void" | "string" | "number" | "boolean" => return ts_type.to_string(),
        _ => {}
    }

    // Handle arrays of primitives: string[], number[], boolean[]
    if let Some(base_type) = ts_type.strip_suffix("[]") {
        match base_type {
            "string" | "number" | "boolean" | "void" => return ts_type.to_string(),
            _ => {
                // Arrays of custom types: CustomType[] -> types.CustomType[]
                let formatted_base = format_typescript_return_type(base_type);
                return format!("{}[]", formatted_base);
            }
        }
    }

    // Handle Record<K, V> and Map<K, V>
    if (ts_type.starts_with("Record<") || ts_type.starts_with("Map<")) && ts_type.ends_with(">") {
        return ts_type.to_string();
    }

    // Handle union types with null: Type | null
    if ts_type.contains(" | null") {
        let base_type = ts_type.replace(" | null", "");
        let formatted_base = format_typescript_return_type(&base_type);
        return format!("{} | null", formatted_base);
    }

    // Handle union types with undefined: Type | undefined
    if ts_type.contains(" | undefined") {
        let base_type = ts_type.replace(" | undefined", "");
        let formatted_base = format_typescript_return_type(&base_type);
        return format!("{} | undefined", formatted_base);
    }

    // Handle tuple types [T, U, ...]
    if ts_type.starts_with("[") && ts_type.ends_with("]") {
        return ts_type.to_string();
    }

    // Custom type - add types. prefix if not already present
    if ts_type.starts_with("types.") {
        ts_type.to_string()
    } else {
        format!("types.{}", ts_type)
    }
}

/// Convert TypeStructure to Zod schema with optional validation
pub fn to_zod_schema_filter(value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
    let is_record_key = args
        .get("is_record_key")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Deserialize the TypeStructure from the value
    let type_structure: TypeStructure = serde_json::from_value(
        serde_json::to_value(value).map_err(|e| format!("Serialization error: {}", e))?,
    )
    .map_err(|e| format!("Failed to deserialize TypeStructure: {}", e))?;

    // Check if we need to apply validation
    let validator_opt = if let Some(validator_value) = args.get("validator") {
        if !validator_value.is_null() {
            Some(
                serde_json::from_value::<ValidatorAttributes>(
                    serde_json::to_value(validator_value)
                        .map_err(|e| format!("Validator serialization error: {}", e))?,
                )
                .map_err(|e| format!("Failed to deserialize ValidatorAttributes: {}", e))?,
            )
        } else {
            None
        }
    } else {
        None
    };

    // Generate schema with validation applied at the right level
    let schema = type_structure_to_zod_schema_with_validation(
        &type_structure,
        is_record_key,
        validator_opt.as_ref(),
    );

    Ok(Value::String(schema))
}

/// Convert TypeStructure to Zod schema string with optional validation
fn type_structure_to_zod_schema_with_validation(
    type_structure: &TypeStructure,
    is_record_key: bool,
    validator: Option<&ValidatorAttributes>,
) -> String {
    match type_structure {
        TypeStructure::Optional(inner) => {
            // For optional types, apply validation to the inner type, then add .optional()
            let inner_schema =
                type_structure_to_zod_schema_with_validation(inner, is_record_key, validator);
            format!("{}.optional()", inner_schema)
        }
        _ => {
            // For non-optional types, generate base schema and apply validation
            let mut schema = type_structure_to_zod_schema(type_structure, is_record_key);
            if let Some(v) = validator {
                schema = apply_validation_to_schema(schema, v);
            }
            schema
        }
    }
}

/// Convert TypeStructure to Zod schema string (without validation)
fn type_structure_to_zod_schema(type_structure: &TypeStructure, is_record_key: bool) -> String {
    match type_structure {
        TypeStructure::Primitive(ts_type) => primitive_to_zod_schema(ts_type, is_record_key),
        TypeStructure::Array(inner) => {
            let inner_schema = type_structure_to_zod_schema(inner, false);
            format!("z.array({})", inner_schema)
        }
        TypeStructure::Map { key, value } => {
            let key_schema = type_structure_to_zod_schema(key, true);
            let value_schema = type_structure_to_zod_schema(value, false);
            format!("z.record({}, {})", key_schema, value_schema)
        }
        TypeStructure::Set(inner) => {
            let inner_schema = type_structure_to_zod_schema(inner, false);
            format!("z.set({})", inner_schema)
        }
        TypeStructure::Tuple(elements) => {
            let schemas: Vec<String> = elements
                .iter()
                .map(|t| type_structure_to_zod_schema(t, false))
                .collect();
            format!("z.tuple([{}])", schemas.join(", "))
        }
        TypeStructure::Optional(inner) => {
            let inner_schema = type_structure_to_zod_schema(inner, is_record_key);
            format!("{}.optional()", inner_schema)
        }
        TypeStructure::Result(inner) => {
            // Result<T, E> maps to union of T and error
            let inner_schema = type_structure_to_zod_schema(inner, false);
            format!(
                "z.union([{}, z.object({{ error: z.string() }})])",
                inner_schema
            )
        }
        TypeStructure::Custom(name) => {
            // Reference to a custom type schema
            format!("{}Schema", name)
        }
    }
}

fn primitive_to_zod_schema(target_primitive: &str, is_record_key: bool) -> String {
    // TypeStructure::Primitive should only contain: "string", "number", "boolean", "void"
    match target_primitive {
        "number" => {
            if is_record_key {
                "z.number()".to_string()
            } else {
                "z.coerce.number()".to_string()
            }
        }
        "string" => "z.string()".to_string(),
        "boolean" => "z.coerce.boolean()".to_string(),
        "void" => "z.void()".to_string(),
        "null" => "z.null()".to_string(),
        "any" => "z.any()".to_string(),
        "unknown" => "z.unknown()".to_string(),
        _ => {
            eprintln!(
                "Warning: Zod filter received unexpected primitive: {}",
                target_primitive
            );
            format!("z.unknown() /* Unexpected: {} */", target_primitive)
        }
    }
}

/// Apply validation constraints to a Zod schema
fn apply_validation_to_schema(mut schema: String, validator: &ValidatorAttributes) -> String {
    // Apply length constraints
    if let Some(ref length) = validator.length {
        schema = apply_length_constraint(schema, length);
    }

    // Apply range constraints
    if let Some(ref range) = validator.range {
        schema = apply_range_constraint(schema, range);
    }

    // Apply email validation
    if validator.email && schema.starts_with("z.string()") {
        schema = schema.replace("z.string()", "z.string().email()");
    }

    // Apply URL validation
    if validator.url && schema.starts_with("z.string()") {
        schema = schema.replace("z.string()", "z.string().url()");
    }

    schema
}

/// Apply length constraints to Zod schema
fn apply_length_constraint(mut schema: String, length: &crate::models::LengthConstraint) -> String {
    let format_error = |msg: &Option<String>| -> String {
        msg.as_ref()
            .map(|m| format!(", {{ message: \"{}\" }}", escape_for_js(m)))
            .unwrap_or_default()
    };

    if let (Some(min), Some(max)) = (length.min, length.max) {
        if schema.starts_with("z.string()") {
            let error = format_error(&length.message);
            schema = format!("z.string().min({}{}).max({}{}))", min, error, max, error);
        } else if schema.contains("z.array(") {
            let error = format_error(&length.message);
            schema = format!("{}.min({}{}).max({}{}))", schema, min, error, max, error);
        }
    } else if let Some(min) = length.min {
        let error = format_error(&length.message);
        if schema.starts_with("z.string()") {
            schema = format!("z.string().min({}{})", min, error);
        } else if schema.contains("z.array(") {
            schema = format!("{}.min({}{})", schema, min, error);
        }
    } else if let Some(max) = length.max {
        let error = format_error(&length.message);
        if schema.starts_with("z.string()") {
            schema = format!("z.string().max({}{})", max, error);
        } else if schema.contains("z.array(") {
            schema = format!("{}.max({}{})", schema, max, error);
        }
    }

    schema
}

/// Apply range constraints to Zod schema
fn apply_range_constraint(mut schema: String, range: &crate::models::RangeConstraint) -> String {
    let format_error = |msg: &Option<String>| -> String {
        msg.as_ref()
            .map(|m| format!(", {{ message: \"{}\" }}", escape_for_js(m)))
            .unwrap_or_default()
    };

    if let (Some(min), Some(max)) = (range.min, range.max) {
        if schema == "z.coerce.number()" {
            let error = format_error(&range.message);
            schema = format!(
                "z.coerce.number().min({}{}).max({}{})",
                min, error, max, error
            );
        }
    } else if let Some(min) = range.min {
        if schema == "z.coerce.number()" {
            let error = format_error(&range.message);
            schema = format!("z.coerce.number().min({}{})", min, error);
        }
    } else if let Some(max) = range.max {
        if schema == "z.coerce.number()" {
            let error = format_error(&range.message);
            schema = format!("z.coerce.number().max({}{})", max, error);
        }
    }

    schema
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_structure_to_zod_schema() {
        // Test primitive with target type names
        let ts = TypeStructure::Primitive("string".to_string());
        assert_eq!(type_structure_to_zod_schema(&ts, false), "z.string()");

        // Test array with target numeric type
        let ts = TypeStructure::Array(Box::new(TypeStructure::Primitive("number".to_string())));
        assert_eq!(
            type_structure_to_zod_schema(&ts, false),
            "z.array(z.coerce.number())"
        );

        // Test map with number key (record key should not use coerce)
        let ts = TypeStructure::Map {
            key: Box::new(TypeStructure::Primitive("number".to_string())),
            value: Box::new(TypeStructure::Primitive("string".to_string())),
        };
        assert_eq!(
            type_structure_to_zod_schema(&ts, false),
            "z.record(z.number(), z.string())"
        );

        // Test tuple with target types
        let ts = TypeStructure::Tuple(vec![
            TypeStructure::Primitive("string".to_string()),
            TypeStructure::Primitive("number".to_string()),
        ]);
        assert_eq!(
            type_structure_to_zod_schema(&ts, false),
            "z.tuple([z.string(), z.coerce.number()])"
        );

        // Test optional with target type
        let ts = TypeStructure::Optional(Box::new(TypeStructure::Primitive("string".to_string())));
        assert_eq!(
            type_structure_to_zod_schema(&ts, false),
            "z.string().optional()"
        );

        // Test custom type
        let ts = TypeStructure::Custom("User".to_string());
        assert_eq!(type_structure_to_zod_schema(&ts, false), "UserSchema");
    }

    #[test]
    fn test_format_return_type() {
        // Primitives should not get types. prefix
        assert_eq!(format_typescript_return_type("string"), "string");
        assert_eq!(format_typescript_return_type("number"), "number");
        assert_eq!(format_typescript_return_type("void"), "void");

        // Custom types should get types. prefix
        assert_eq!(format_typescript_return_type("User"), "types.User");
        assert_eq!(
            format_typescript_return_type("CustomType"),
            "types.CustomType"
        );

        // Arrays
        assert_eq!(format_typescript_return_type("string[]"), "string[]");
        assert_eq!(format_typescript_return_type("User[]"), "types.User[]");

        // Union with null
        assert_eq!(
            format_typescript_return_type("User | null"),
            "types.User | null"
        );
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
