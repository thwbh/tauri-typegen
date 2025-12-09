use crate::analysis::CommandAnalyzer;
use crate::generators::base::file_writer::FileWriter;
use crate::generators::base::template_helpers::TemplateHelpers;
use crate::generators::base::type_conversion::TypeConverter;
use crate::generators::base::{BaseBindingsGenerator, BaseGenerator};
use crate::models::{
    CommandInfo, EventInfo, FieldInfo, LengthConstraint, RangeConstraint, StructInfo,
    ValidatorAttributes,
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
            .map(|field| format!("\"{}\"", field.get_serialized_name()))
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
            // Use get_serialized_name() which respects serde rename/rename_all attributes
            content.push_str(&format!(
                "  {}: {},\n",
                field.get_serialized_name(),
                field_schema
            ));
        }

        content.push_str("});\n\n");
        content
    }

    /// Generate Zod schema for a field
    fn generate_field_schema(&self, field: &FieldInfo) -> String {
        let base_schema = self.type_structure_to_zod_schema(&field.type_structure, false);
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

    /// Convert TypeStructure to Zod schema
    ///
    /// # Arguments
    /// * `type_structure` - The parsed type structure from analysis
    /// * `is_record_key` - Whether this type is being used as a record/map key
    fn type_structure_to_zod_schema(
        &self,
        type_structure: &crate::models::TypeStructure,
        is_record_key: bool,
    ) -> String {
        use crate::models::TypeStructure;

        match type_structure {
            TypeStructure::Primitive(ts_type) => {
                // Map TypeScript primitive to Zod type
                match ts_type.as_str() {
                    "string" => "z.string()".to_string(),
                    "number" => {
                        // Record keys must not use coerce
                        if is_record_key {
                            "z.number()".to_string()
                        } else {
                            "z.coerce.number()".to_string()
                        }
                    }
                    "boolean" => "z.boolean()".to_string(),
                    "void" => "z.void()".to_string(),
                    _ => "z.string()".to_string(), // Fallback
                }
            }

            TypeStructure::Array(inner) => {
                let inner_schema = self.type_structure_to_zod_schema(inner, false);
                format!("z.array({})", inner_schema)
            }

            TypeStructure::Map { key, value } => {
                // Keys must not use coerce - pass true for is_record_key
                let key_schema = self.type_structure_to_zod_schema(key, true);
                let value_schema = self.type_structure_to_zod_schema(value, false);
                format!("z.record({}, {})", key_schema, value_schema)
            }

            TypeStructure::Set(inner) => {
                // Sets are represented as arrays in TypeScript/Zod
                let inner_schema = self.type_structure_to_zod_schema(inner, false);
                format!("z.array({})", inner_schema)
            }

            TypeStructure::Tuple(types) => {
                if types.is_empty() {
                    return "z.void()".to_string();
                }
                let tuple_schemas: Vec<String> = types
                    .iter()
                    .map(|t| self.type_structure_to_zod_schema(t, false))
                    .collect();
                format!("z.tuple([{}])", tuple_schemas.join(", "))
            }

            TypeStructure::Optional(inner) => {
                // Optional is handled by the caller adding .optional()
                // Just return the inner type
                self.type_structure_to_zod_schema(inner, is_record_key)
            }

            TypeStructure::Result(ok_type) => {
                // Result<T, E> -> just use the Ok type (ignore error for TS)
                self.type_structure_to_zod_schema(ok_type, is_record_key)
            }

            TypeStructure::Custom(type_name) => {
                // Custom types reference their schema
                format!("{}Schema", type_name)
            }
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
                schema = format!(
                    "z.string().min({}{}).max({}{})",
                    min, min_error, max, max_error
                );
            } else if schema.contains("z.array(") {
                let min_error = format_error(&length.message);
                let max_error = format_error(&length.message);
                schema = format!(
                    "{}.min({}{}).max({}{})",
                    schema, min, min_error, max, max_error
                );
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
            if schema == "z.coerce.number()" {
                let min_error = format_error(&range.message);
                let max_error = format_error(&range.message);
                schema = format!(
                    "z.coerce.number().min({}{}).max({}{})",
                    min, min_error, max, max_error
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

    /// Generate parameter schemas for commands
    fn generate_param_schemas(&self, commands: &[CommandInfo]) -> String {
        let mut content = String::new();

        for command in commands {
            // Only generate schema if command has regular parameters (not just channels)
            // Channels are runtime objects and should not be validated
            if !command.parameters.is_empty() {
                let schema_name = format!(
                    "{}ParamsSchema",
                    TemplateHelpers::to_pascal_case(&command.name)
                );
                content.push_str(&format!("export const {} = z.object({{\n", schema_name));

                for param in &command.parameters {
                    let param_schema =
                        self.type_structure_to_zod_schema(&param.type_structure, false);
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
            // If command has only channels (no regular params), generate interface manually
            if command.parameters.is_empty() && !command.channels.is_empty() {
                if let Some(interface) =
                    TemplateHelpers::generate_params_interface_with_channels(command)
                {
                    content.push_str(&interface);
                }
            } else if !command.parameters.is_empty() && command.channels.is_empty() {
                // Only regular params, generate from schema
                let type_name = format!("{}Params", TemplateHelpers::to_pascal_case(&command.name));
                let schema_name = format!(
                    "{}ParamsSchema",
                    TemplateHelpers::to_pascal_case(&command.name)
                );
                content.push_str(&TemplateHelpers::generate_type_alias(
                    &type_name,
                    &format!("z.infer<typeof {}>", schema_name),
                ));
            } else if !command.parameters.is_empty() && !command.channels.is_empty() {
                // Both regular params and channels: generate interface with both
                if let Some(interface) =
                    TemplateHelpers::generate_params_interface_with_channels(command)
                {
                    content.push_str(&interface);
                }
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

        // Import Channel if any command has channels
        let has_channels = commands.iter().any(|cmd| !cmd.channels.is_empty());
        if has_channels {
            content.push_str(&TemplateHelpers::generate_type_imports(&[(
                "@tauri-apps/api/core",
                "{ Channel }",
            )]));
        }
        content.push('\n');

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

        // Check if any command has channels
        let has_channels = commands.iter().any(|cmd| !cmd.channels.is_empty());

        // Add imports - include Channel if needed
        if has_channels {
            content.push_str(&TemplateHelpers::generate_named_imports(&[(
                "@tauri-apps/api/core",
                &["invoke", "Channel"],
            )]));
        } else {
            content.push_str(&TemplateHelpers::generate_named_imports(&[(
                "@tauri-apps/api/core",
                &["invoke"],
            )]));
        }
        content.push_str(
            TemplateHelpers::generate_type_imports(&[("zod", "{ ZodError }")]).trim_end(),
        );
        content.push('\n');
        content.push_str(
            TemplateHelpers::generate_type_imports(&[("./types", "* as types")]).trim_end(),
        );
        content.push_str("\n\n");

        // Generate CommandHooks interface
        content.push_str(&Self::generate_command_hooks_interface());

        // Generate command functions with validation
        for command in commands {
            if !command.channels.is_empty() {
                content.push_str(
                    &TemplateHelpers::generate_command_function_with_channels_and_validation(
                        command,
                    ),
                );
            } else {
                content.push_str(&TemplateHelpers::generate_command_function_with_validation(
                    command,
                ));
            }
        }

        content
    }

    /// Generate the CommandHooks interface
    fn generate_command_hooks_interface() -> String {
        r#"export interface CommandHooks<T> {
  /** Called when Zod schema validation fails */
  onValidationError?: (error: ZodError) => void;

  /** Called when Tauri invoke fails (Rust error, serialization, etc.) */
  onInvokeError?: (error: unknown) => void;

  /** Called when command succeeds */
  onSuccess?: (result: T) => void;

  /** Called after command settles (success or error) */
  onSettled?: () => void;
}

"#
        .to_string()
    }

    /// Generate index.ts file
    fn generate_index_file(&self, generated_files: &[String]) -> String {
        // Export from all generated files except index.ts
        let files_to_export: Vec<&str> = generated_files
            .iter()
            .filter(|f| *f != "index.ts")
            .map(|s| s.as_str())
            .collect();

        // generate_standard_index already includes the header
        FileWriter::new("")
            .unwrap()
            .generate_standard_index(&files_to_export)
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

    /// Generate events file content
    fn generate_events_file(&self, events: &[EventInfo]) -> String {
        let mut content = String::new();

        // Add file header
        content.push_str(&self.generate_file_header());

        // Generate event listeners
        content.push_str(&TemplateHelpers::generate_all_event_listeners(events));

        content
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
        let mut used_structs = self.base.collect_used_types(commands, discovered_structs);

        // Also collect types used in events
        let events = analyzer.get_discovered_events();
        for event in events {
            let mut event_types = std::collections::HashSet::new();
            self.base
                .collect_referenced_types(&event.payload_type, &mut event_types);

            // Add event payload types to used_structs
            for type_name in event_types {
                if let Some(struct_info) = discovered_structs.get(&type_name) {
                    used_structs.insert(type_name.clone(), struct_info.clone());
                }
            }
        }

        // Create file writer
        let mut file_writer = FileWriter::new(output_path)?;

        // Generate and write types file (with embedded schemas)
        let types_content = self.generate_types_file_content(commands, &used_structs, analyzer);
        file_writer.write_types_file(&types_content)?;

        // Generate and write commands file
        let commands_content = self.generate_command_bindings(commands);
        file_writer.write_commands_file(&commands_content)?;

        // Generate and write events file if there are any events
        let events = analyzer.get_discovered_events();
        if !events.is_empty() {
            let events_content = self.generate_events_file(events);
            file_writer.write_events_file(&events_content)?;
        }

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
