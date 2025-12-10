use crate::models::{CommandInfo, EventInfo, FieldInfo};

/// Template helper functions for generating common TypeScript patterns
pub struct TemplateHelpers;

impl TemplateHelpers {
    /// Check if a TypeScript type is a primitive or compound primitive type
    /// (e.g., "string", "number", "string[]", "string | null", "boolean[]")
    fn is_primitive_type(ts_type: &str) -> bool {
        let primitives = ["string", "number", "boolean", "void", "null", "undefined"];

        // Strip array suffix and optional suffix for checking
        let base_type = ts_type
            .replace("[]", "")
            .replace(" | null", "")
            .replace(" | undefined", "");
        let base_type = base_type.trim();

        primitives.contains(&base_type)
    }

    /// Generate TypeScript interface definition
    pub fn generate_interface(name: &str, fields: &[FieldInfo]) -> String {
        Self::generate_interface_with_options(name, fields, false)
    }

    /// Generate TypeScript interface with optional index signature
    pub fn generate_interface_with_options(
        name: &str,
        fields: &[FieldInfo],
        add_index_signature: bool,
    ) -> String {
        let mut result = format!("export interface {} {{\n", name);

        for field in fields {
            let optional_marker = if field.is_optional { "?" } else { "" };
            // Use get_serialized_name() which respects serde rename/rename_all attributes
            // and falls back to the field name
            result.push_str(&format!(
                "  {}{}: {};\n",
                field.serialized_name, optional_marker, field.typescript_type
            ));
        }

        // Add index signature for parameter interfaces to satisfy InvokeArgs
        if add_index_signature {
            result.push_str("  [key: string]: unknown;\n");
        }

        result.push_str("}\n\n");
        result
    }

    /// Generate TypeScript type alias
    pub fn generate_type_alias(name: &str, target_type: &str) -> String {
        format!("export type {} = {};\n\n", name, target_type)
    }

    /// Generate command function signature for vanilla TypeScript
    pub fn generate_command_function(command: &CommandInfo) -> String {
        let camel_name = Self::to_camel_case(&command.name);

        let param_type = if command.parameters.is_empty() {
            String::new()
        } else {
            format!(
                "params: types.{}Params",
                Self::to_pascal_case(&command.name)
            )
        };

        let return_type = if command.return_type_ts == "void" || command.return_type_ts == "()" {
            "void".to_string()
        } else if command.return_type_ts == "string" || command.return_type_ts == "String" {
            "string".to_string()
        } else if command.return_type_ts == "number"
            || command.return_type_ts.starts_with("i")
            || command.return_type_ts.starts_with("u")
            || command.return_type_ts.starts_with("f")
        {
            "number".to_string()
        } else if command.return_type_ts == "boolean" || command.return_type_ts == "bool" {
            "boolean".to_string()
        } else if Self::is_primitive_type(&command.return_type_ts) {
            // Handle compound primitive types like "string[]", "string | null", "number[]", etc.
            command.return_type_ts.clone()
        } else {
            format!("types.{}", command.return_type_ts)
        };

        // Tauri commands are always async from the frontend perspective
        let async_keyword = "async ";
        let return_promise = format!("Promise<{}>", return_type);

        let invoke_params = if command.parameters.is_empty() {
            String::new()
        } else {
            ", params".to_string()
        };

        format!(
            "export {}function {}({}): {} {{\n  return invoke('{}'{});\n}}\n\n",
            async_keyword, camel_name, param_type, return_promise, command.name, invoke_params
        )
    }

    /// Generate command function with validation for zod
    pub fn generate_command_function_with_validation(command: &CommandInfo) -> String {
        let param_type = if command.parameters.is_empty() {
            String::new()
        } else {
            format!(
                "params: types.{}Params",
                Self::to_pascal_case(&command.name)
            )
        };

        // command.return_type_ts is already in TypeScript format, just need to add types. prefix for custom types
        let return_type = Self::format_typescript_return_type(&command.return_type_ts);

        // Tauri commands are always async from the frontend perspective
        let async_keyword = "async ";
        let return_promise = format!("Promise<{}>", return_type);

        let camel_case_name = Self::to_camel_case(&command.name);

        if command.parameters.is_empty() {
            // Commands without parameters
            format!(
                "export {}function {}(hooks?: CommandHooks<{}>): {} {{\n  try {{\n    const data = await invoke<{}>('{}');\n    hooks?.onSuccess?.(data);\n    return data;\n  }} catch (error) {{\n    hooks?.onInvokeError?.(error);\n    throw error;\n  }} finally {{\n    hooks?.onSettled?.();\n  }}\n}}\n\n",
                async_keyword, camel_case_name, return_type, return_promise, return_type, command.name
            )
        } else {
            let pascal_name = Self::to_pascal_case(&command.name);
            // Commands with parameters
            format!(
                "export {}function {}({}, hooks?: CommandHooks<{}>): {} {{\n  try {{\n    const result = types.{}ParamsSchema.safeParse(params);\n    \n    if (!result.success) {{\n      hooks?.onValidationError?.(result.error);\n      throw result.error;\n    }}\n    \n    const data = await invoke<{}>('{}', result.data);\n    hooks?.onSuccess?.(data);\n    return data;\n  }} catch (error) {{\n    if (!(error instanceof ZodError)) {{\n      hooks?.onInvokeError?.(error);\n    }}\n    throw error;\n  }} finally {{\n    hooks?.onSettled?.();\n  }}\n}}\n\n",
                async_keyword,
                camel_case_name,
                param_type,
                return_type,
                return_promise,
                pascal_name,
                return_type,
                command.name
            )
        }
    }

    /// Format a TypeScript type that's already been converted from Rust, adding types. prefix where needed
    pub fn format_typescript_return_type(ts_type: &str) -> String {
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
                    let formatted_base = Self::format_typescript_return_type(base_type);
                    return format!("{}[]", formatted_base);
                }
            }
        }

        // Handle Record<K, V>
        if ts_type.starts_with("Record<") && ts_type.ends_with(">") {
            return ts_type.to_string(); // Record types are fine as-is
        }

        // Handle Map<K, V>
        if ts_type.starts_with("Map<") && ts_type.ends_with(">") {
            return ts_type.to_string(); // Map types are fine as-is
        }

        // Handle union types with null: Type | null
        if ts_type.contains(" | null") {
            let base_type = ts_type.replace(" | null", "");
            let formatted_base = Self::format_typescript_return_type(&base_type);
            return format!("{} | null", formatted_base);
        }

        // Handle union types with undefined: Type | undefined
        if ts_type.contains(" | undefined") {
            let base_type = ts_type.replace(" | undefined", "");
            let formatted_base = Self::format_typescript_return_type(&base_type);
            return format!("{} | undefined", formatted_base);
        }

        // Handle tuple types [T, U, ...]
        if ts_type.starts_with("[") && ts_type.ends_with("]") {
            return ts_type.to_string(); // Tuples are fine as-is
        }

        // Custom type - add types. prefix if not already present
        if ts_type.starts_with("types.") {
            ts_type.to_string()
        } else {
            format!("types.{}", ts_type)
        }
    }

    /// Convert Rust type to TypeScript type
    pub fn convert_rust_type_to_typescript(rust_type: &str) -> String {
        // Handle basic types
        match rust_type {
            "void" | "()" => "void".to_string(),
            "String" | "str" | "&str" | "&String" | "string" => "string".to_string(),
            "bool" | "boolean" => "boolean".to_string(),
            t if t.starts_with("i")
                || t.starts_with("u")
                || t.starts_with("f")
                || t == "number" =>
            {
                "number".to_string()
            }
            _ => {
                // Handle complex types
                if rust_type.starts_with("Vec<") && rust_type.ends_with(">") {
                    let inner = &rust_type[4..rust_type.len() - 1];
                    let inner_ts = Self::convert_rust_type_to_typescript(inner);
                    format!("{}[]", inner_ts)
                } else if rust_type.starts_with("Result<") && rust_type.ends_with(">") {
                    let inner = &rust_type[7..rust_type.len() - 1];
                    if let Some(comma_pos) = inner.find(", ") {
                        let ok_type = inner[..comma_pos].trim();
                        Self::convert_rust_type_to_typescript(ok_type)
                    } else {
                        format!("types.{}", rust_type)
                    }
                } else if rust_type.starts_with("Option<") && rust_type.ends_with(">") {
                    let inner = &rust_type[7..rust_type.len() - 1];
                    let inner_ts = Self::convert_rust_type_to_typescript(inner);
                    format!("{} | null", inner_ts)
                } else if rust_type.starts_with("HashMap<") || rust_type.starts_with("BTreeMap<") {
                    let prefix_len = if rust_type.starts_with("HashMap<") {
                        8
                    } else {
                        9
                    };
                    let inner = &rust_type[prefix_len..rust_type.len() - 1];
                    if let Some(comma_pos) = inner.find(", ") {
                        let key_type = inner[..comma_pos].trim();
                        let value_type = inner[comma_pos + 2..].trim();
                        let key_ts = Self::convert_rust_type_to_typescript(key_type);
                        let value_ts = Self::convert_rust_type_to_typescript(value_type);
                        format!("Record<{}, {}>", key_ts, value_ts)
                    } else {
                        format!("types.{}", rust_type)
                    }
                } else if rust_type.starts_with("(")
                    && rust_type.ends_with(")")
                    && rust_type.contains(",")
                {
                    // Handle tuple types
                    let inner = &rust_type[1..rust_type.len() - 1];
                    let tuple_types: Vec<&str> = inner.split(", ").collect();
                    let ts_types: Vec<String> = tuple_types
                        .iter()
                        .map(|t| Self::convert_rust_type_to_typescript(t.trim()))
                        .collect();
                    format!("[{}]", ts_types.join(", "))
                } else {
                    // Custom type - but don't prefix with 'types.' if it's already a complete TS type
                    if rust_type.starts_with("[")
                        || rust_type.contains("Record<")
                        || rust_type.contains(" | ")
                    {
                        rust_type.to_string()
                    } else {
                        format!("types.{}", rust_type)
                    }
                }
            }
        }
    }

    /// Generate parameter interface from command parameters
    pub fn generate_params_interface(command: &CommandInfo) -> Option<String> {
        if command.parameters.is_empty() {
            return None;
        }

        let interface_name = format!("{}Params", Self::to_pascal_case(&command.name));
        let fields: Vec<_> = command
            .parameters
            .iter()
            .map(|param| FieldInfo {
                name: param.name.clone(),
                rust_type: param.rust_type.clone(),
                typescript_type: param.typescript_type.clone(),
                is_optional: param.is_optional,
                is_public: true,
                validator_attributes: None,
                // For command parameters (not struct fields), use camelCase by default
                serialized_name: Self::to_camel_case(&param.name),
                type_structure: param.type_structure.clone(),
            })
            .collect();

        // Parameter interfaces need index signature for Tauri InvokeArgs compatibility
        Some(Self::generate_interface_with_options(
            &interface_name,
            &fields,
            true,
        ))
    }

    /// Check if a command has channels
    pub fn command_has_channels(command: &CommandInfo) -> bool {
        !command.channels.is_empty()
    }

    /// Generate parameter interface that includes channels for commands with Channel parameters
    pub fn generate_params_interface_with_channels(command: &CommandInfo) -> Option<String> {
        // If no parameters and no channels, return None
        if command.parameters.is_empty() && command.channels.is_empty() {
            return None;
        }

        let interface_name = format!("{}Params", Self::to_pascal_case(&command.name));
        let mut fields: Vec<FieldInfo> = command
            .parameters
            .iter()
            .map(|param| FieldInfo {
                name: param.name.clone(),
                rust_type: param.rust_type.clone(),
                typescript_type: param.typescript_type.clone(),
                is_optional: param.is_optional,
                is_public: true,
                validator_attributes: None,
                // For command parameters (not struct fields), use camelCase by default
                serialized_name: Self::to_camel_case(&param.name),
                type_structure: param.type_structure.clone(),
            })
            .collect();

        // Add channel parameters
        for channel in &command.channels {
            let param_name = channel.parameter_name.clone();
            fields.push(FieldInfo {
                name: param_name.clone(),
                rust_type: format!("Channel<{}>", channel.message_type),
                typescript_type: format!("Channel<{}>", channel.typescript_message_type),
                is_optional: false,
                is_public: true,
                validator_attributes: None,
                // For channel parameters, use camelCase by default
                serialized_name: Self::to_camel_case(&param_name),
                // Channels are custom types from @tauri-apps/api/core
                type_structure: crate::models::TypeStructure::Custom("Channel".to_string()),
            });
        }

        // Parameter interfaces need index signature for Tauri InvokeArgs compatibility
        Some(Self::generate_interface_with_options(
            &interface_name,
            &fields,
            true,
        ))
    }

    /// Generate command function for commands with channels (vanilla TypeScript)
    pub fn generate_command_function_with_channels(command: &CommandInfo) -> String {
        let pascal_name = Self::to_pascal_case(&command.name);
        let camel_name = Self::to_camel_case(&command.name);

        // Build parameter type
        let param_type = if command.parameters.is_empty() && command.channels.is_empty() {
            String::new()
        } else {
            format!("params: types.{}Params", pascal_name)
        };

        // Format return type
        let return_type = Self::format_typescript_return_type(&command.return_type_ts);
        let return_promise = format!("Promise<{}>", return_type);

        // Build invoke parameters - pass the params object directly
        let invoke_params = if command.parameters.is_empty() && command.channels.is_empty() {
            String::new()
        } else {
            ", params".to_string()
        };

        format!(
            "export async function {}({}): {} {{\n  return invoke('{}'{});\n}}\n\n",
            camel_name, param_type, return_promise, command.name, invoke_params
        )
    }

    /// Generate command function with channels for Zod validation
    pub fn generate_command_function_with_channels_and_validation(command: &CommandInfo) -> String {
        let pascal_name = Self::to_pascal_case(&command.name);
        let camel_name = Self::to_camel_case(&command.name);

        // Build parameter type
        let param_type = if command.parameters.is_empty() && command.channels.is_empty() {
            String::new()
        } else {
            format!("params: types.{}Params", pascal_name)
        };

        // Format return type
        let return_type = Self::format_typescript_return_type(&command.return_type_ts);
        let return_promise = format!("Promise<{}>", return_type);

        // If command has no parameters and no channels
        if command.parameters.is_empty() && command.channels.is_empty() {
            return format!(
                "export async function {}(hooks?: CommandHooks<{}>): {} {{\n  try {{\n    const data = await invoke<{}>('{}');\n    hooks?.onSuccess?.(data);\n    return data;\n  }} catch (error) {{\n    hooks?.onInvokeError?.(error);\n    throw error;\n  }} finally {{\n    hooks?.onSettled?.();\n  }}\n}}\n\n",
                camel_name, return_type, return_promise, return_type, command.name
            );
        }

        // Build validation logic - only validate non-channel parameters
        let validation_code = if command.parameters.is_empty() {
            // Only channels, no validation needed
            String::from("    // No validation needed for channel-only parameters\n")
        } else {
            // Has regular parameters that need validation
            format!(
                "    const result = types.{}ParamsSchema.safeParse(params);\n    \n    if (!result.success) {{\n      hooks?.onValidationError?.(result.error);\n      throw result.error;\n    }}\n    \n",
                pascal_name
            )
        };

        // Determine what to pass to invoke
        let invoke_data = if command.parameters.is_empty() {
            "params".to_string() // Pass channels directly
        } else {
            // Build explicit channel references from CommandInfo
            let channel_refs: Vec<String> = command
                .channels
                .iter()
                .map(|ch| {
                    let camel_name = Self::to_camel_case(&ch.parameter_name);
                    format!("{}: params.{}", camel_name, camel_name)
                })
                .collect();

            if channel_refs.is_empty() {
                "result.data".to_string()
            } else {
                format!("{{ ...result.data, {} }}", channel_refs.join(", "))
            }
        };

        format!(
            "export async function {}({}, hooks?: CommandHooks<{}>): {} {{\n  try {{\n{}    const data = await invoke<{}>('{}', {});\n    hooks?.onSuccess?.(data);\n    return data;\n  }} catch (error) {{\n    if (!(error instanceof ZodError)) {{\n      hooks?.onInvokeError?.(error);\n    }}\n    throw error;\n  }} finally {{\n    hooks?.onSettled?.();\n  }}\n}}\n\n",
            camel_name,
            param_type,
            return_type,
            return_promise,
            validation_code,
            return_type,
            command.name,
            invoke_data
        )
    }
    /// Generate enum definition
    pub fn generate_enum(name: &str, variants: &[String]) -> String {
        let variants_str = variants
            .iter()
            .enumerate()
            .map(|(i, variant)| {
                let comma = if i == variants.len() - 1 { "" } else { "," };
                format!("  {} = \"{}\"{}", variant, variant, comma)
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!("export enum {} {{\n{}\n}}\n\n", name, variants_str)
    }

    /// Generate union type from enum variants
    pub fn generate_union_type(name: &str, variants: &[String]) -> String {
        let union_values = variants
            .iter()
            .map(|v| format!("\"{}\"", v))
            .collect::<Vec<_>>()
            .join(" | ");

        format!("export type {} = {};\n\n", name, union_values)
    }

    /// Generate import statements
    pub fn generate_imports(imports: &[(&str, &str)]) -> String {
        let mut result = String::new();

        for (import_path, items) in imports {
            result.push_str(&format!("import {} from '{}';\n", items, import_path));
        }

        if !imports.is_empty() {
            result.push('\n');
        }

        result
    }

    /// Generate named imports
    pub fn generate_named_imports(imports: &[(&str, &[&str])]) -> String {
        let mut result = String::new();

        for (import_path, items) in imports {
            let items_str = items.join(", ");
            result.push_str(&format!(
                "import {{ {} }} from '{}';\n",
                items_str, import_path
            ));
        }

        if !imports.is_empty() {
            result.push('\n');
        }

        result
    }

    /// Generate type-only imports
    pub fn generate_type_imports(imports: &[(&str, &str)]) -> String {
        let mut result = String::new();

        for (import_path, items) in imports {
            result.push_str(&format!("import {} from '{}';\n", items, import_path));
        }

        if !imports.is_empty() {
            result.push('\n');
        }

        result
    }

    /// Convert string to PascalCase
    pub fn to_pascal_case(s: &str) -> String {
        s.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                    }
                }
            })
            .collect()
    }

    /// Convert string to camelCase
    pub fn to_camel_case(s: &str) -> String {
        let pascal = Self::to_pascal_case(s);
        let mut chars = pascal.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        }
    }

    /// Convert string to snake_case
    pub fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        let mut prev_was_lower = false;

        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() && i > 0 && prev_was_lower {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap_or(c));
            prev_was_lower = c.is_lowercase();
        }

        result
    }

    /// Generate a comment block
    pub fn generate_comment_block(lines: &[&str]) -> String {
        let mut result = String::from("/**\n");
        for line in lines {
            result.push_str(&format!(" * {}\n", line));
        }
        result.push_str(" */\n\n");
        result
    }

    /// Generate single-line comment
    pub fn generate_comment(text: &str) -> String {
        format!("// {}\n", text)
    }

    /// Wrap content in namespace
    pub fn wrap_in_namespace(namespace: &str, content: &str) -> String {
        format!(
            "export namespace {} {{\n{}\n}}\n",
            namespace,
            Self::indent_content(content, 1)
        )
    }

    /// Indent content by specified number of levels (2 spaces per level)
    pub fn indent_content(content: &str, levels: usize) -> String {
        let indent = "  ".repeat(levels);
        content
            .lines()
            .map(|line| {
                if line.trim().is_empty() {
                    line.to_string()
                } else {
                    format!("{}{}", indent, line)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Generate JSDoc comment for a function
    pub fn generate_jsdoc(
        description: &str,
        params: &[(&str, &str)],
        returns: Option<&str>,
    ) -> String {
        let mut result = String::from("/**\n");
        result.push_str(&format!(" * {}\n", description));

        if !params.is_empty() {
            result.push_str(" *\n");
            for (name, desc) in params {
                result.push_str(&format!(" * @param {} {}\n", name, desc));
            }
        }

        if let Some(return_desc) = returns {
            result.push_str(&format!(" * @returns {}\n", return_desc));
        }

        result.push_str(" */\n");
        result
    }

    /// Generate event listener helper function
    /// Creates a type-safe wrapper around the Tauri listen() function
    pub fn generate_event_listener_function(event: &EventInfo) -> String {
        // Convert event-name to EventName for function naming
        let function_name = Self::event_name_to_function_name(&event.event_name);
        let payload_type = &event.typescript_payload_type;

        format!(
            r#"/**
 * Listen for '{}' events
 * @param handler - Callback function to handle the event
 * @returns Promise that resolves to an unlisten function
 */
export async function {}(
  handler: (payload: {}) => void
): Promise<UnlistenFn> {{
  return listen<{}>('{}', (event) => {{
    handler(event.payload);
  }});
}}

"#,
            event.event_name, function_name, payload_type, payload_type, event.event_name
        )
    }

    /// Convert event-name to eventName function name
    /// Examples: "download-started" -> "onDownloadStarted", "user-logged-in" -> "onUserLoggedIn"
    fn event_name_to_function_name(event_name: &str) -> String {
        let pascal = event_name
            .split('-')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<String>();

        format!("on{}", pascal)
    }

    /// Generate event constants for consistent event name usage
    pub fn generate_event_constants(events: &[EventInfo]) -> String {
        let mut result = String::from("// Event name constants\n");

        for event in events {
            let constant_name = event.event_name.to_uppercase().replace('-', "_");
            result.push_str(&format!(
                "export const {} = '{}';\n",
                constant_name, event.event_name
            ));
        }

        result.push('\n');
        result
    }

    /// Generate all event listener functions
    pub fn generate_all_event_listeners(events: &[EventInfo]) -> String {
        let mut result = String::new();

        // Add header comment
        result.push_str(&Self::generate_comment_block(&[
            "Event Listeners",
            "Type-safe event listener helpers for Tauri events",
        ]));

        // Generate imports
        result.push_str(
            "import { listen, type UnlistenFn, type Event } from '@tauri-apps/api/event';\n",
        );

        // Collect unique payload types for imports
        let mut payload_types: std::collections::HashSet<String> = std::collections::HashSet::new();
        for event in events {
            // Extract the TypeScript type name (without generics, arrays, etc.)
            let type_name = event
                .typescript_payload_type
                .split('<')
                .next()
                .unwrap_or(&event.typescript_payload_type);
            let type_name = type_name.split('[').next().unwrap_or(type_name);
            let type_name = type_name.trim();

            // Only import if it's not a primitive type
            if !matches!(
                type_name,
                "string" | "number" | "boolean" | "void" | "null" | "undefined"
            ) {
                payload_types.insert(type_name.to_string());
            }
        }

        // Add imports from types file if needed
        if !payload_types.is_empty() {
            let types_list: Vec<String> = payload_types.into_iter().collect();
            result.push_str(&format!(
                "import type {{ {} }} from './types';\n\n",
                types_list.join(", ")
            ));
        } else {
            result.push('\n');
        }

        // Generate each listener function
        for event in events {
            result.push_str(&Self::generate_event_listener_function(event));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(TemplateHelpers::to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(TemplateHelpers::to_pascal_case("test"), "Test");
        assert_eq!(
            TemplateHelpers::to_pascal_case("already_pascal"),
            "AlreadyPascal"
        );
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(TemplateHelpers::to_camel_case("hello_world"), "helloWorld");
        assert_eq!(TemplateHelpers::to_camel_case("test"), "test");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(TemplateHelpers::to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(TemplateHelpers::to_snake_case("Test"), "test");
        assert_eq!(
            TemplateHelpers::to_snake_case("alreadySnake"),
            "already_snake"
        );
    }
}
