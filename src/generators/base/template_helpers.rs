use crate::models::{CommandInfo, FieldInfo};

/// Template helper functions for generating common TypeScript patterns
pub struct TemplateHelpers;

impl TemplateHelpers {
    /// Generate TypeScript interface definition
    pub fn generate_interface(name: &str, fields: &[FieldInfo]) -> String {
        let mut result = format!("export interface {} {{\n", name);
        
        for field in fields {
            let optional_marker = if field.is_optional { "?" } else { "" };
            result.push_str(&format!(
                "  {}{}: {};\n", 
                field.name, 
                optional_marker, 
                field.typescript_type
            ));
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
        let param_type = if command.parameters.is_empty() {
            String::new()
        } else {
            format!("params: types.{}Params", Self::to_pascal_case(&command.name))
        };

        let return_type = if command.return_type == "void" || command.return_type == "()" {
            "void".to_string()
        } else if command.return_type == "string" || command.return_type == "String" {
            "string".to_string()
        } else if command.return_type == "number" || command.return_type.starts_with("i") || command.return_type.starts_with("u") || command.return_type.starts_with("f") {
            "number".to_string()
        } else if command.return_type == "boolean" || command.return_type == "bool" {
            "boolean".to_string()
        } else {
            format!("types.{}", command.return_type)
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
            async_keyword,
            command.name,
            param_type,
            return_promise,
            command.name,
            invoke_params
        )
    }

    /// Generate command function with validation for zod
    pub fn generate_command_function_with_validation(command: &CommandInfo) -> String {
        let param_type = if command.parameters.is_empty() {
            String::new()
        } else {
            format!("params: types.{}Params", Self::to_pascal_case(&command.name))
        };

        let return_type = Self::convert_rust_type_to_typescript(&command.return_type);

        // Tauri commands are always async from the frontend perspective
        let async_keyword = "async ";
        let return_promise = format!("Promise<{}>", return_type);

        let camel_case_name = Self::to_camel_case(&command.name);
        
        if command.parameters.is_empty() {
            format!(
                "export {}function {}(): {} {{\n  return invoke('{}');\n}}\n\n",
                async_keyword,
                camel_case_name,
                return_promise,
                command.name
            )
        } else {
            let pascal_name = Self::to_pascal_case(&command.name);
            format!(
                "export {}function {}({}): {} {{\n  const validatedParams = types.{}ParamsSchema.parse(params);\n  return invoke('{}', validatedParams);\n}}\n\n",
                async_keyword,
                camel_case_name,
                param_type,
                return_promise,
                pascal_name,
                command.name
            )
        }
    }

    /// Convert Rust type to TypeScript type  
    pub fn convert_rust_type_to_typescript(rust_type: &str) -> String {
        // Handle basic types
        match rust_type {
            "void" | "()" => "void".to_string(),
            "String" | "str" | "&str" | "&String" | "string" => "string".to_string(),
            "bool" | "boolean" => "boolean".to_string(),
            t if t.starts_with("i") || t.starts_with("u") || t.starts_with("f") || t == "number" => "number".to_string(),
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
                    let prefix_len = if rust_type.starts_with("HashMap<") { 8 } else { 9 };
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
                } else if rust_type.starts_with("(") && rust_type.ends_with(")") && rust_type.contains(",") {
                    // Handle tuple types
                    let inner = &rust_type[1..rust_type.len() - 1];
                    let tuple_types: Vec<&str> = inner.split(", ").collect();
                    let ts_types: Vec<String> = tuple_types.iter()
                        .map(|t| Self::convert_rust_type_to_typescript(t.trim()))
                        .collect();
                    format!("[{}]", ts_types.join(", "))
                } else {
                    // Custom type - but don't prefix with 'types.' if it's already a complete TS type
                    if rust_type.starts_with("[") || rust_type.contains("Record<") || rust_type.contains(" | ") {
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
        let mut fields = Vec::new();

        for param in &command.parameters {
            fields.push(FieldInfo {
                name: param.name.clone(),
                rust_type: param.rust_type.clone(),
                typescript_type: param.typescript_type.clone(),
                is_optional: param.is_optional,
                is_public: true,
                validator_attributes: None,
            });
        }

        Some(Self::generate_interface(&interface_name, &fields))
    }

    /// Generate enum definition
    pub fn generate_enum(name: &str, variants: &[String]) -> String {
        let mut result = format!("export enum {} {{\n", name);
        
        for (i, variant) in variants.iter().enumerate() {
            let comma = if i == variants.len() - 1 { "" } else { "," };
            result.push_str(&format!("  {} = \"{}\"{}\n", variant, variant, comma));
        }
        
        result.push_str("}\n\n");
        result
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
            result.push_str(&format!("import {{ {} }} from '{}';\n", items_str, import_path));
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
                    Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
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
        format!("export namespace {} {{\n{}\n}}\n", namespace, Self::indent_content(content, 1))
    }

    /// Indent content by specified number of levels (2 spaces per level)
    pub fn indent_content(content: &str, levels: usize) -> String {
        let indent = "  ".repeat(levels);
        content
            .lines()
            .map(|line| if line.trim().is_empty() { line.to_string() } else { format!("{}{}", indent, line) })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Generate JSDoc comment for a function
    pub fn generate_jsdoc(description: &str, params: &[(&str, &str)], returns: Option<&str>) -> String {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(TemplateHelpers::to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(TemplateHelpers::to_pascal_case("test"), "Test");
        assert_eq!(TemplateHelpers::to_pascal_case("already_pascal"), "AlreadyPascal");
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
        assert_eq!(TemplateHelpers::to_snake_case("alreadySnake"), "already_snake");
    }
}