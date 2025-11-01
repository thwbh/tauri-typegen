use crate::analysis::CommandAnalyzer;
use crate::generators::base::file_writer::FileWriter;
use crate::generators::base::template_helpers::TemplateHelpers;
use crate::generators::base::type_conversion::TypeConverter;
use crate::generators::base::{BaseBindingsGenerator, BaseGenerator};
use crate::models::{CommandInfo, StructInfo};
use std::collections::{HashMap, HashSet};

/// Generator for vanilla TypeScript bindings without validation
pub struct TypeScriptBindingsGenerator {
    base: BaseGenerator,
    type_converter: TypeConverter,
}

impl TypeScriptBindingsGenerator {
    pub fn new() -> Self {
        Self {
            base: BaseGenerator::new(),
            type_converter: TypeConverter::new(),
        }
    }

    /// Generate TypeScript interface definitions from structs
    fn generate_struct_interfaces(&self, used_structs: &HashMap<String, StructInfo>) -> String {
        let mut content = String::new();

        for (name, struct_info) in used_structs {
            if struct_info.is_enum {
                content.push_str(&self.generate_enum_definition(name, struct_info));
            } else {
                content.push_str(&TemplateHelpers::generate_interface(
                    name,
                    &struct_info.fields,
                ));
            }
        }

        content
    }

    /// Generate enum definition (as union type for vanilla TypeScript)
    fn generate_enum_definition(&self, name: &str, struct_info: &StructInfo) -> String {
        let variants: Vec<String> = struct_info
            .fields
            .iter()
            .map(|field| field.name.clone())
            .collect();

        TemplateHelpers::generate_union_type(name, &variants)
    }

    /// Generate parameter interfaces for commands
    fn generate_param_interfaces(&self, commands: &[CommandInfo]) -> String {
        let mut content = String::new();

        for command in commands {
            if !command.parameters.is_empty() {
                if let Some(interface) = TemplateHelpers::generate_params_interface(command) {
                    content.push_str(&interface);
                }
            }
        }

        content
    }

    /// Generate the complete types.ts file content
    fn generate_types_file_content(
        &self,
        commands: &[CommandInfo],
        used_structs: &HashMap<String, StructInfo>,
    ) -> String {
        let mut content = String::new();

        // Add file header
        content.push_str(&self.generate_file_header());

        // Generate parameter interfaces
        content.push_str(&self.generate_param_interfaces(commands));

        // Generate struct interfaces
        content.push_str(&self.generate_struct_interfaces(used_structs));

        content
    }

    /// Generate command bindings
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

        // Generate command functions
        for command in commands {
            content.push_str(&TemplateHelpers::generate_command_function(command));
        }

        content
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

    /// Convert string to camelCase for backward compatibility
    pub fn to_camel_case(&self, s: &str) -> String {
        TemplateHelpers::to_camel_case(s)
    }

    /// Collect referenced types for backward compatibility
    pub fn collect_referenced_types(&self, rust_type: &str, used_types: &mut HashSet<String>) {
        self.type_converter
            .collect_referenced_types(rust_type, used_types);
    }

    /// Check if a type is custom for backward compatibility  
    pub fn is_custom_type(&self, type_name: &str) -> bool {
        // Check if it's in known types or looks like a custom type
        self.type_converter.is_custom_type(type_name) || self.looks_like_custom_type(type_name)
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
}

impl BaseBindingsGenerator for TypeScriptBindingsGenerator {
    fn generate_models(
        &mut self,
        commands: &[CommandInfo],
        discovered_structs: &HashMap<String, StructInfo>,
        output_path: &str,
        _analyzer: &CommandAnalyzer,
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

        // Generate and write types file
        let types_content = self.generate_types_file_content(commands, &used_structs);
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

impl Default for TypeScriptBindingsGenerator {
    fn default() -> Self {
        Self::new()
    }
}
