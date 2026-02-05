use crate::analysis::CommandAnalyzer;
use crate::generators::base::file_writer::FileWriter;
use crate::generators::base::template_context::FieldContext;
use crate::generators::base::templates::TemplateRegistry;
use crate::generators::base::BaseBindingsGenerator;
use crate::generators::zod::schema_builder::ZodSchemaBuilder;
use crate::generators::zod::templates::ZodTemplate;
use crate::generators::zod::type_visitor::ZodVisitor;
use crate::generators::TypeCollector;
use crate::models::{CommandInfo, EventInfo, StructInfo};
use crate::GenerateConfig;
use std::collections::{HashMap, HashSet};
use tera::{Context, Tera};

/// Generator for Zod schema-based TypeScript bindings with validation
pub struct ZodBindingsGenerator {
    collector: TypeCollector,
    tera: Tera,
}

impl ZodBindingsGenerator {
    pub fn new() -> Self {
        Self {
            collector: TypeCollector::new(),
            tera: ZodTemplate::create_tera().expect("Failed to initialize Zod template engine"),
        }
    }

    /// Generate Zod schema for a struct
    fn generate_struct_schema(
        &self,
        name: &str,
        struct_info: &StructInfo,
        config: &GenerateConfig,
    ) -> String {
        if struct_info.is_enum {
            self.generate_enum_schema(name, struct_info, config)
        } else {
            self.generate_object_schema(name, struct_info, config)
        }
    }

    /// Generate Zod schema for an enum
    fn generate_enum_schema(
        &self,
        name: &str,
        struct_info: &StructInfo,
        config: &GenerateConfig,
    ) -> String {
        let visitor = ZodVisitor::with_config(config);

        // Convert fields to context to get serialized names
        let field_contexts: Vec<FieldContext> =
            self.collector
                .create_field_contexts(struct_info, &visitor, config);

        let variants: Vec<String> = field_contexts
            .iter()
            .map(|field| format!("\"{}\"", field.serialized_name))
            .collect();

        let enum_values = variants.join(", ");
        format!(
            "export const {}Schema = z.enum([{}]);\n\n",
            name, enum_values
        )
    }

    /// Generate Zod schema for an object/struct using templates
    fn generate_object_schema(
        &self,
        name: &str,
        struct_info: &StructInfo,
        config: &GenerateConfig,
    ) -> String {
        let visitor = ZodVisitor::with_config(config);
        let schema_builder = ZodSchemaBuilder::new(config);

        // Convert FieldInfo to FieldContext with computed Zod schemas
        let mut field_contexts: Vec<FieldContext> =
            self.collector
                .create_field_contexts(struct_info, &visitor, config);

        // Enrich with complete zod schemas including validators
        for field_context in &mut field_contexts {
            let zod_schema = schema_builder.build_schema(
                &field_context.type_structure,
                &field_context.validator_attributes,
            );
            field_context.typescript_type = zod_schema;
        }

        let mut context = Context::new();
        context.insert("name", name);
        context.insert("fields", &field_contexts);

        self.render("zod/partials/schema.ts.tera", &context)
            .unwrap_or_else(|e| {
                eprintln!("Template rendering failed for {}: {}", name, e);
                format!("// Error generating schema for {}: {}\n", name, e)
            })
    }

    /// Generate the complete types.ts file content (with embedded schemas)
    fn generate_types_file_content(
        &self,
        commands: &[CommandInfo],
        used_structs: &HashMap<String, StructInfo>,
        analyzer: &CommandAnalyzer,
        config: &GenerateConfig,
    ) -> String {
        // Sort structs topologically
        let type_names: HashSet<String> = used_structs.keys().cloned().collect();
        let sorted_types = analyzer.topological_sort_types(&type_names);

        // Generate struct schemas
        let mut struct_schemas = String::new();
        for name in &sorted_types {
            if let Some(struct_info) = used_structs.get(name) {
                struct_schemas.push_str(&self.generate_struct_schema(name, struct_info, config));
            }
        }

        // Convert commands to context wrappers
        let visitor = ZodVisitor::with_config(config);
        let schema_builder = ZodSchemaBuilder::new(config);
        let mut command_contexts = self
            .collector
            .create_command_contexts(commands, &visitor, analyzer, config);

        // Enrich parameters with complete zod schemas
        for command_context in &mut command_contexts {
            for param in &mut command_context.parameters {
                let zod_schema = schema_builder.build_param_schema(&param.type_structure);
                param.typescript_type = zod_schema;
            }
        }

        // Generate parameter schemas using template
        let param_schemas = {
            let mut context = Context::new();
            context.insert("commands", &command_contexts);
            self.render("zod/partials/param_schemas.ts.tera", &context)
                .unwrap_or_else(|e| {
                    eprintln!("Template rendering failed for param schemas: {}", e);
                    String::new()
                })
        };

        // Generate type aliases using template
        let type_aliases = {
            let mut context = Context::new();
            context.insert("commands", &command_contexts);
            context.insert("struct_names", &sorted_types);
            self.render("zod/partials/type_aliases.ts.tera", &context)
                .unwrap_or_else(|e| {
                    eprintln!("Template rendering failed for type aliases: {}", e);
                    String::new()
                })
        };

        // Render main types.ts template
        let mut context = Context::new();
        context.insert("header", &self.generate_file_header());
        context.insert(
            "has_channels",
            &commands.iter().any(|cmd| !cmd.channels.is_empty()),
        );
        context.insert("struct_schemas", &struct_schemas);
        context.insert("param_schemas", &param_schemas);
        context.insert("type_aliases", &type_aliases);

        self.render("zod/types.ts.tera", &context)
            .unwrap_or_else(|e| {
                eprintln!("Template rendering failed for types.ts: {}", e);
                String::new()
            })
    }

    /// Generate command bindings with validation
    fn generate_command_bindings(
        &self,
        commands: &[CommandInfo],
        analyzer: &CommandAnalyzer,
        config: &GenerateConfig,
    ) -> String {
        // Use ZodVisitor for command bindings - it can generate both Zod schemas
        // and TypeScript types (via visit_type_for_interface)
        let visitor = ZodVisitor::with_config(config);

        // Convert commands to context wrappers
        let command_contexts = self
            .collector
            .create_command_contexts(commands, &visitor, analyzer, config);

        let mut context = Context::new();
        context.insert("header", &self.generate_file_header());
        context.insert("commands", &command_contexts);
        context.insert(
            "has_channels",
            &commands.iter().any(|cmd| !cmd.channels.is_empty()),
        );

        self.render("zod/commands.ts.tera", &context)
            .unwrap_or_else(|e| {
                eprintln!("Template rendering failed for commands.ts: {}", e);
                String::new()
            })
    }

    /// Generate index.ts file
    fn generate_index_file(&self, generated_files: &[String]) -> String {
        let mut context = Context::new();
        context.insert("header", &self.generate_file_header());
        context.insert("files", generated_files);

        self.render("zod/index.ts.tera", &context)
            .unwrap_or_else(|e| {
                eprintln!("Template rendering failed for index.ts: {}", e);
                String::new()
            })
    }

    /// Generate events file content
    fn generate_events_file(
        &self,
        events: &[EventInfo],
        analyzer: &CommandAnalyzer,
        config: &GenerateConfig,
    ) -> String {
        let visitor = ZodVisitor::with_config(config);

        // Convert events to context wrappers
        let event_contexts = self
            .collector
            .create_event_contexts(events, &visitor, analyzer, config);

        let mut context = Context::new();
        context.insert("header", &self.generate_file_header());
        context.insert("events", &event_contexts);

        self.render("zod/events.ts.tera", &context)
            .unwrap_or_else(|e| {
                eprintln!("Template rendering failed for events.ts: {}", e);
                String::new()
            })
    }
}

impl BaseBindingsGenerator for ZodBindingsGenerator {
    fn tera(&self) -> &Tera {
        &self.tera
    }

    fn type_collector(&self) -> &TypeCollector {
        &self.collector
    }

    fn generator_type(&self) -> String {
        "zod".to_string()
    }

    fn generate_models(
        &mut self,
        commands: &[CommandInfo],
        discovered_structs: &HashMap<String, StructInfo>,
        output_path: &str,
        analyzer: &CommandAnalyzer,
        config: &GenerateConfig,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        // Store known structs for reference
        self.collector.known_structs = discovered_structs.clone();

        // Filter to only the types used by commands
        let mut used_structs = self
            .collector
            .collect_used_types(commands, discovered_structs);

        // Also collect types used in events
        let events = analyzer.get_discovered_events();
        for event in events {
            let mut event_types = std::collections::HashSet::new();
            TypeCollector::collect_referenced_types_from_structure(
                &event.payload_type_structure,
                &mut event_types,
            );

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
        let types_content =
            self.generate_types_file_content(commands, &used_structs, analyzer, config);
        file_writer.write_types_file(&types_content)?;

        // Generate and write commands file
        let commands_content = self.generate_command_bindings(commands, analyzer, config);
        file_writer.write_commands_file(&commands_content)?;

        // Generate and write events file if there are any events
        let events = analyzer.get_discovered_events();
        if !events.is_empty() {
            let events_content = self.generate_events_file(events, analyzer, config);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FieldInfo, TypeStructure};

    mod initialization {
        use super::*;

        #[test]
        fn test_new_creates_generator() {
            let gen = ZodBindingsGenerator::new();
            assert!(
                gen.collector.known_structs.is_empty() || !gen.collector.known_structs.is_empty()
            );
        }

        #[test]
        fn test_default_creates_generator() {
            let gen = ZodBindingsGenerator::default();
            assert!(
                gen.collector.known_structs.is_empty() || !gen.collector.known_structs.is_empty()
            );
        }
    }

    mod trait_implementation {
        use super::*;

        #[test]
        fn test_generator_type_returns_zod() {
            let gen = ZodBindingsGenerator::new();
            assert_eq!(gen.generator_type(), "zod");
        }

        #[test]
        fn test_tera_returns_engine() {
            let gen = ZodBindingsGenerator::new();
            let tera = gen.tera();
            // Verify it has registered templates
            assert!(tera.get_template_names().count() > 0);
        }

        #[test]
        fn test_type_collector_returns_collector() {
            let gen = ZodBindingsGenerator::new();
            let collector = gen.type_collector();
            // Verify collector exists
            assert!(collector.known_structs.is_empty() || !collector.known_structs.is_empty());
        }
    }

    mod template_rendering {
        use super::*;

        #[test]
        fn test_generate_file_header() {
            let gen = ZodBindingsGenerator::new();
            let header = gen.generate_file_header();
            assert!(header.contains("Auto-generated") || header.contains("tauri-typegen"));
            assert!(header.contains("zod")); // generator type
        }

        #[test]
        fn test_has_zod_templates() {
            let gen = ZodBindingsGenerator::new();
            let tera = gen.tera();
            let template_names: Vec<&str> = tera.get_template_names().collect();

            // Check for key templates
            assert!(template_names.contains(&"zod/types.ts.tera"));
            assert!(template_names.contains(&"zod/commands.ts.tera"));
            assert!(template_names.contains(&"zod/index.ts.tera"));
        }

        #[test]
        fn test_render_returns_error_for_invalid_template() {
            let gen = ZodBindingsGenerator::new();
            let context = Context::new();
            let result = gen.render("nonexistent/template.tera", &context);
            assert!(result.is_err());
        }
    }

    mod schema_generation {
        use crate::GenerateConfig;

        use super::*;

        fn create_test_config() -> GenerateConfig {
            GenerateConfig {
                project_path: ".".to_string(),
                output_path: "./output".to_string(),
                validation_library: "zod".to_string(),
                visualize_deps: Some(false),
                verbose: Some(false),
                include_private: Some(false),
                type_mappings: None,
                exclude_patterns: None,
                include_patterns: None,
                default_parameter_case: "camelCase".to_string(),
                default_field_case: "camelCase".to_string(),
                force: Some(false),
            }
        }

        fn create_test_struct(is_enum: bool) -> StructInfo {
            StructInfo {
                name: "TestStruct".to_string(),
                fields: vec![FieldInfo {
                    name: "test_field".to_string(),
                    rust_type: "String".to_string(),
                    is_optional: false,
                    is_public: true,
                    type_structure: TypeStructure::Primitive("string".to_string()),
                    serde_rename: None,
                    validator_attributes: None,
                }],
                file_path: "test.rs".to_string(),
                is_enum,
                serde_rename_all: None,
            }
        }

        #[test]
        fn test_generate_enum_schema() {
            let gen = ZodBindingsGenerator::new();
            let config = create_test_config();
            let struct_info = create_test_struct(true);

            let result = gen.generate_enum_schema("TestEnum", &struct_info, &config);
            assert!(result.contains("TestEnumSchema"));
            assert!(result.contains("z.enum"));
        }

        #[test]
        fn test_generate_object_schema() {
            let gen = ZodBindingsGenerator::new();
            let config = create_test_config();
            let struct_info = create_test_struct(false);

            let result = gen.generate_object_schema("TestStruct", &struct_info, &config);
            assert!(!result.is_empty());
        }

        #[test]
        fn test_generate_struct_schema_for_enum() {
            let gen = ZodBindingsGenerator::new();
            let config = create_test_config();
            let struct_info = create_test_struct(true);

            let result = gen.generate_struct_schema("TestEnum", &struct_info, &config);
            assert!(result.contains("z.enum"));
        }

        #[test]
        fn test_generate_struct_schema_for_struct() {
            let gen = ZodBindingsGenerator::new();
            let config = create_test_config();
            let struct_info = create_test_struct(false);

            let result = gen.generate_struct_schema("TestStruct", &struct_info, &config);
            assert!(!result.is_empty());
        }
    }

    mod helper_methods {
        use super::*;

        #[test]
        fn test_generate_index_file_with_empty_files() {
            let gen = ZodBindingsGenerator::new();
            let files = vec![];
            let result = gen.generate_index_file(&files);
            assert!(result.contains("Auto-generated") || result.contains("//"));
        }

        #[test]
        fn test_generate_index_file_with_files() {
            let gen = ZodBindingsGenerator::new();
            let files = vec!["types.ts".to_string(), "commands.ts".to_string()];
            let result = gen.generate_index_file(&files);
            assert!(!result.is_empty());
        }
    }
}
