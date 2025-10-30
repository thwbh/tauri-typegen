use crate::analysis::CommandAnalyzer;
use crate::generators::base::file_writer::FileWriter;
use crate::generators::base::template_helpers::TemplateHelpers;
use crate::generators::base::type_conversion::TypeConverter;
use crate::generators::base::{BaseBindingsGenerator, BaseGenerator};
use crate::models::{
    CommandInfo, FieldInfo, LengthConstraint, RangeConstraint, StructInfo, ValidatorAttributes,
};
use std::collections::{HashMap, HashSet};

/// Generator for Zod schema-based TypeScript bindings with validation
pub struct ZodBindingsGenerator {
    base: BaseGenerator,
    type_converter: TypeConverter,
}

impl ZodBindingsGenerator {
    pub fn new() -> Self {
        Self {
            base: BaseGenerator::new(),
            type_converter: TypeConverter::new(),
        }
    }

    /// Generate Zod schema for a struct
    fn generate_struct_schema(&self, name: &str, struct_info: &StructInfo) -> String {
        if struct_info.is_enum {
            self.generate_enum_schema(name, struct_info)
        } else {
            self.generate_object_schema(name, struct_info)
        }
    }

    /// Generate Zod schema for an enum
    fn generate_enum_schema(&self, name: &str, struct_info: &StructInfo) -> String {
        let variants: Vec<String> = struct_info
            .fields
            .iter()
            .map(|field| format!("\"{}\"", field.name))
            .collect();

        let enum_values = variants.join(", ");
        format!(
            "export const {}Schema = z.enum([{}]);\n\n",
            name, enum_values
        )
    }

    /// Generate Zod schema for an object/struct
    fn generate_object_schema(&self, name: &str, struct_info: &StructInfo) -> String {
        let mut content = format!("export const {}Schema = z.object({{\n", name);

        for field in &struct_info.fields {
            let field_schema = self.generate_field_schema(field);
            // Convert to camelCase to match serde serialization
            let field_name = self.to_camel_case(&field.name);
            content.push_str(&format!("  {}: {},\n", field_name, field_schema));
        }

        content.push_str("});\n\n");
        content
    }

    /// Generate Zod schema for a field
    fn generate_field_schema(&self, field: &FieldInfo) -> String {
        let base_schema = self.rust_type_to_zod_schema(&field.rust_type);
        let mut schema = base_schema;

        // Apply validation attributes if present
        if let Some(ref validator) = field.validator_attributes {
            schema = self.apply_validator_constraints(schema, validator);
        }

        // Make optional if needed
        if field.is_optional {
            schema = format!("{}.optional()", schema);
        }

        schema
    }

    /// Convert Rust type to Zod schema
    fn rust_type_to_zod_schema(&self, rust_type: &str) -> String {
        let cleaned = self.type_converter.strip_reference(rust_type);

        // Handle Option<T> -> T (will be made optional later)
        if let Some(inner) = self.type_converter.extract_option_inner_type(&cleaned) {
            return self.rust_type_to_zod_schema(&inner);
        }

        // Handle Result<T, E> -> T
        if let Some(ok_type) = self.type_converter.extract_result_ok_type(&cleaned) {
            return self.rust_type_to_zod_schema(&ok_type);
        }

        // Handle Vec<T> -> z.array(T)
        if let Some(inner) = self.type_converter.extract_vec_inner_type(&cleaned) {
            let inner_schema = self.rust_type_to_zod_schema(&inner);
            return format!("z.array({})", inner_schema);
        }

        // Handle HashMap<K, V> and BTreeMap<K, V> -> z.record(key, value)
        if let Some((key_type, value_type)) = self
            .type_converter
            .extract_hashmap_types(&cleaned)
            .or_else(|| self.type_converter.extract_btreemap_types(&cleaned))
        {
            let key_schema = self.rust_type_to_zod_schema(&key_type);
            let value_schema = self.rust_type_to_zod_schema(&value_type);
            return format!("z.record({}, {})", key_schema, value_schema);
        }

        // Handle HashSet<T> and BTreeSet<T> -> z.array(T)
        if let Some(inner) = self
            .type_converter
            .extract_hashset_inner_type(&cleaned)
            .or_else(|| self.type_converter.extract_btreeset_inner_type(&cleaned))
        {
            let inner_schema = self.rust_type_to_zod_schema(&inner);
            return format!("z.array({})", inner_schema);
        }

        // Handle tuple types -> z.tuple([...])
        if let Some(tuple_types) = self.type_converter.extract_tuple_types(&cleaned) {
            if tuple_types.is_empty() {
                return "z.void()".to_string();
            }
            let tuple_schemas: Vec<String> = tuple_types
                .iter()
                .map(|t| self.rust_type_to_zod_schema(t))
                .collect();
            return format!("z.tuple([{}])", tuple_schemas.join(", "));
        }

        // Handle primitive types
        if let Some(zod_type) = self.map_primitive_to_zod(&cleaned) {
            return zod_type;
        }

        // Custom types - reference the schema
        if self.type_converter.is_custom_type(&cleaned) {
            return format!("{}Schema", cleaned);
        }

        // Fallback to string for unknown types
        "z.string()".to_string()
    }

    /// Map primitive Rust types to Zod schemas
    fn map_primitive_to_zod(&self, rust_type: &str) -> Option<String> {
        match rust_type {
            "String" | "str" | "&str" | "&String" => Some("z.string()".to_string()),
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
            | "u128" | "usize" | "f32" | "f64" => Some("z.number()".to_string()),
            "bool" => Some("z.boolean()".to_string()),
            "()" => Some("z.void()".to_string()),
            _ => None,
        }
    }

    /// Apply validator constraints to a Zod schema
    fn apply_validator_constraints(
        &self,
        mut schema: String,
        validator: &ValidatorAttributes,
    ) -> String {
        // Apply length constraints
        if let Some(ref length) = validator.length {
            schema = self.apply_length_constraint(schema, length);
        }

        // Apply range constraints
        if let Some(ref range) = validator.range {
            schema = self.apply_range_constraint(schema, range);
        }

        // Apply email validation
        if validator.email && schema == "z.string()" {
            schema = "z.string().email()".to_string();
        }

        // Apply URL validation
        if validator.url && schema == "z.string()" {
            schema = "z.string().url()".to_string();
        }

        schema
    }

    /// Apply length constraints to Zod schema
    fn apply_length_constraint(&self, mut schema: String, length: &LengthConstraint) -> String {
        // Helper function to format error message parameter
        let format_error = |msg: &Option<String>| -> String {
            msg.as_ref()
                .map(|m| format!(", {{ message: \"{}\" }}", Self::escape_for_js(m)))
                .unwrap_or_default()
        };

        if let (Some(min), Some(max)) = (length.min, length.max) {
            if schema.starts_with("z.string()") {
                let min_error = format_error(&length.message);
                let max_error = format_error(&length.message);
                schema = format!("z.string().min({}{}).max({}{})", min, min_error, max, max_error);
            } else if schema.contains("z.array(") {
                let min_error = format_error(&length.message);
                let max_error = format_error(&length.message);
                schema = format!("{}.min({}{}).max({}{})", schema, min, min_error, max, max_error);
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
    fn apply_range_constraint(&self, mut schema: String, range: &RangeConstraint) -> String {
        // Helper function to format error message parameter
        let format_error = |msg: &Option<String>| -> String {
            msg.as_ref()
                .map(|m| format!(", {{ message: \"{}\" }}", Self::escape_for_js(m)))
                .unwrap_or_default()
        };

        if let (Some(min), Some(max)) = (range.min, range.max) {
            if schema == "z.number()" {
                let min_error = format_error(&range.message);
                let max_error = format_error(&range.message);
                schema = format!("z.number().min({}{}).max({}{})", min, min_error, max, max_error);
            }
        } else if let Some(min) = range.min {
            if schema == "z.number()" {
                let error = format_error(&range.message);
                schema = format!("z.number().min({}{})", min, error);
            }
        } else if let Some(max) = range.max {
            if schema == "z.number()" {
                let error = format_error(&range.message);
                schema = format!("z.number().max({}{})", max, error);
            }
        }

        schema
    }

    /// Generate parameter schemas for commands
    fn generate_param_schemas(&self, commands: &[CommandInfo]) -> String {
        let mut content = String::new();

        for command in commands {
            if !command.parameters.is_empty() {
                let schema_name = format!(
                    "{}ParamsSchema",
                    TemplateHelpers::to_pascal_case(&command.name)
                );
                content.push_str(&format!("export const {} = z.object({{\n", schema_name));

                for param in &command.parameters {
                    let param_schema = self.rust_type_to_zod_schema(&param.rust_type);
                    let final_schema = if param.is_optional {
                        format!("{}.optional()", param_schema)
                    } else {
                        param_schema
                    };
                    // Convert param name to camelCase to match Tauri's serialization
                    let param_name = self.to_camel_case(&param.name);
                    content.push_str(&format!("  {}: {},\n", param_name, final_schema));
                }

                content.push_str("});\n\n");
            }
        }

        content
    }

    /// Generate TypeScript types from Zod schemas
    fn generate_types_from_schemas(
        &self,
        commands: &[CommandInfo],
        used_structs: &HashMap<String, StructInfo>,
    ) -> String {
        let mut content = String::new();

        // Generate parameter types
        for command in commands {
            if !command.parameters.is_empty() {
                let type_name = format!("{}Params", TemplateHelpers::to_pascal_case(&command.name));
                let schema_name = format!(
                    "{}ParamsSchema",
                    TemplateHelpers::to_pascal_case(&command.name)
                );
                content.push_str(&TemplateHelpers::generate_type_alias(
                    &type_name,
                    &format!("z.infer<typeof {}>", schema_name),
                ));
            }
        }

        // Generate struct types
        for name in used_structs.keys() {
            content.push_str(&TemplateHelpers::generate_type_alias(
                name,
                &format!("z.infer<typeof {}Schema>", name),
            ));
        }

        content
    }

    /// Generate the complete types.ts file content (with embedded schemas)
    fn generate_types_file_content(
        &self,
        commands: &[CommandInfo],
        used_structs: &HashMap<String, StructInfo>,
        analyzer: &CommandAnalyzer,
    ) -> String {
        let mut content = String::new();

        // Add file header
        content.push_str(&self.generate_file_header());

        // Import Zod
        content.push_str(&TemplateHelpers::generate_named_imports(&[("zod", &["z"])]));

        // Sort structs topologically to ensure dependencies are defined before use
        let type_names: HashSet<String> = used_structs.keys().cloned().collect();
        let sorted_types = analyzer.topological_sort_types(&type_names);

        // Generate struct schemas in topological order
        for name in &sorted_types {
            if let Some(struct_info) = used_structs.get(name) {
                content.push_str(&self.generate_struct_schema(name, struct_info));
            }
        }

        // Generate parameter schemas
        content.push_str(&self.generate_param_schemas(commands));

        // Generate TypeScript types from schemas
        content.push_str(&self.generate_types_from_schemas(commands, used_structs));

        content
    }

    /// Generate command bindings with validation
    fn generate_command_bindings(&self, commands: &[CommandInfo]) -> String {
        let mut content = String::new();

        // Add file header
        content.push_str(&self.generate_command_file_header());

        // Add imports
        content.push_str(&TemplateHelpers::generate_named_imports(&[(
            "@tauri-apps/api/core",
            &["invoke"],
        )]));
        content.push_str(
            TemplateHelpers::generate_type_imports(&[("./types", "* as types")]).trim_end(),
        );
        content.push_str("\n\n");

        // Generate command functions with validation
        for command in commands {
            content.push_str(&TemplateHelpers::generate_command_function_with_validation(
                command,
            ));
        }

        content
    }

    /// Generate index.ts file
    fn generate_index_file(&self, generated_files: &[String]) -> String {
        let mut content = String::new();
        content.push_str(&self.generate_index_file_header());

        // Export from all generated files except index.ts
        let files_to_export: Vec<&str> = generated_files
            .iter()
            .filter(|f| *f != "index.ts")
            .map(|s| s.as_str())
            .collect();

        content.push_str(
            &FileWriter::new("")
                .unwrap()
                .generate_standard_index(&files_to_export),
        );
        content
    }

    /// Escape a string for use in JavaScript/TypeScript string literals
    fn escape_for_js(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    /// Backward compatibility methods
    pub fn to_camel_case(&self, s: &str) -> String {
        TemplateHelpers::to_camel_case(s)
    }

    pub fn collect_referenced_types(&self, rust_type: &str, used_types: &mut HashSet<String>) {
        self.type_converter
            .collect_referenced_types(rust_type, used_types);
    }

    pub fn is_custom_type(&self, type_name: &str) -> bool {
        self.type_converter.is_custom_type(type_name)
    }

    /// Check if a type looks like a custom type (starts with capital letter, not a primitive)
    fn looks_like_custom_type(&self, ts_type: &str) -> bool {
        // Must start with a capital letter
        if !ts_type
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
        {
            return false;
        }

        // Must not be a primitive type
        !self.type_converter.is_primitive_type(ts_type)
    }

    /// Convert TypeScript type to Zod schema for backward compatibility
    pub fn typescript_to_zod_type(&self, ts_type: &str) -> String {
        match ts_type {
            "string" => "z.string()".to_string(),
            "number" => "z.number()".to_string(),
            "boolean" => "z.boolean()".to_string(),
            "void" => "z.void()".to_string(),
            _ if ts_type.ends_with("[]") => {
                let inner_type = &ts_type[..ts_type.len() - 2];
                let inner_schema = self.typescript_to_zod_type(inner_type);
                format!("z.array({})", inner_schema)
            }
            // Handle Record<K, V> types
            _ if ts_type.starts_with("Record<") && ts_type.ends_with(">") => {
                let inner = &ts_type[7..ts_type.len() - 1]; // Remove "Record<" and ">"
                if let Some(comma_pos) = inner.find(", ") {
                    let key_type = inner[..comma_pos].trim();
                    let value_type = inner[comma_pos + 2..].trim();
                    let key_schema = self.typescript_to_zod_type(key_type);
                    let value_schema = self.typescript_to_zod_type(value_type);
                    format!("z.record({}, {})", key_schema, value_schema)
                } else {
                    "z.string()".to_string() // Fallback for malformed Record type
                }
            }
            // Handle tuple types [T, U, ...]
            _ if ts_type.starts_with("[") && ts_type.ends_with("]") => {
                let inner = &ts_type[1..ts_type.len() - 1]; // Remove brackets
                if inner.is_empty() {
                    return "z.void()".to_string();
                }

                let tuple_types: Vec<&str> = inner.split(", ").collect();
                let tuple_schemas: Vec<String> = tuple_types
                    .iter()
                    .map(|t| self.typescript_to_zod_type(t.trim()))
                    .collect();
                format!("z.tuple([{}])", tuple_schemas.join(", "))
            }
            _ if ts_type.contains(" | null") => {
                let base_type = ts_type.replace(" | null", "");
                let base_schema = self.typescript_to_zod_type(&base_type);
                format!("{}.nullable()", base_schema)
            }
            _ => {
                // Custom type - use lazy for complex types as test expects
                // Check if it's a custom type (in known_types) OR if it looks like a custom type (capitalized)
                if self.type_converter.is_custom_type(ts_type)
                    || self.looks_like_custom_type(ts_type)
                {
                    format!(
                        "z.lazy(() => z.any()) /* {} - define schema separately if needed */",
                        ts_type
                    )
                } else {
                    "z.string()".to_string() // Fallback
                }
            }
        }
    }
}

impl BaseBindingsGenerator for ZodBindingsGenerator {
    fn generate_models(
        &mut self,
        commands: &[CommandInfo],
        discovered_structs: &HashMap<String, StructInfo>,
        output_path: &str,
        analyzer: &CommandAnalyzer,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        // Set up the type converter with known structs
        self.type_converter
            .set_known_types(discovered_structs.clone());

        // Store known structs for reference
        self.base.known_structs = discovered_structs.clone();

        // Filter to only the types used by commands
        let used_structs = self.base.collect_used_types(commands, discovered_structs);

        // Create file writer
        let mut file_writer = FileWriter::new(output_path)?;

        // Generate and write types file (with embedded schemas)
        let types_content = self.generate_types_file_content(commands, &used_structs, analyzer);
        file_writer.write_types_file(&types_content)?;

        // Generate and write commands file
        let commands_content = self.generate_command_bindings(commands);
        file_writer.write_commands_file(&commands_content)?;

        // Generate and write index file
        let index_content = self.generate_index_file(file_writer.get_generated_files());
        file_writer.write_index_file(&index_content)?;

        Ok(file_writer.get_generated_files().to_vec())
    }
}

impl Default for ZodBindingsGenerator {
    fn default() -> Self {
        Self::new()
    }
}
