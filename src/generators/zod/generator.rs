use crate::analysis::CommandAnalyzer;
use crate::generators::base::file_writer::FileWriter;
use crate::generators::base::template_context::FieldContext;
use crate::generators::base::templates::BaseTemplate;
use crate::generators::base::type_visitor::{TypeScriptVisitor, ZodVisitor};
use crate::generators::base::BaseBindingsGenerator;
use crate::generators::zod::templates::ZodTemplate;
use crate::generators::TypeCollector;
use crate::models::{CommandInfo, EventInfo, StructInfo};
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
            tera: ZodTemplate
                .create()
                .expect("Failed to initialize Zod template engine"),
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
        let visitor = ZodVisitor;

        // Convert fields to context to get serialized names
        let field_contexts: Vec<FieldContext> =
            self.collector.create_field_contexts(struct_info, &visitor);

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
    fn generate_object_schema(&self, name: &str, struct_info: &StructInfo) -> String {
        let visitor = ZodVisitor;

        // Convert FieldInfo to FieldContext with computed Zod schemas
        let field_contexts: Vec<FieldContext> =
            self.collector.create_field_contexts(struct_info, &visitor);

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
    ) -> String {
        // Sort structs topologically
        let type_names: HashSet<String> = used_structs.keys().cloned().collect();
        let sorted_types = analyzer.topological_sort_types(&type_names);

        // Generate struct schemas
        let mut struct_schemas = String::new();
        for name in &sorted_types {
            if let Some(struct_info) = used_structs.get(name) {
                struct_schemas.push_str(&self.generate_struct_schema(name, struct_info));
            }
        }

        // Convert commands to context wrappers
        let visitor = ZodVisitor;
        let command_contexts = self
            .collector
            .create_command_contexts(commands, &visitor, analyzer);

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
    ) -> String {
        // Use TypeScriptVisitor for command bindings to get proper TS types
        // (not Zod schemas) in function signatures
        let visitor = TypeScriptVisitor;

        // Convert commands to context wrappers
        let command_contexts = self
            .collector
            .create_command_contexts(commands, &visitor, analyzer);

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
    fn generate_events_file(&self, events: &[EventInfo], analyzer: &CommandAnalyzer) -> String {
        let visitor = ZodVisitor;

        // Convert events to context wrappers
        let event_contexts = self
            .collector
            .create_event_contexts(events, &visitor, analyzer);

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
            self.collector.collect_referenced_types_from_structure(
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
        let types_content = self.generate_types_file_content(commands, &used_structs, analyzer);
        file_writer.write_types_file(&types_content)?;

        // Generate and write commands file
        let commands_content = self.generate_command_bindings(commands, analyzer);
        file_writer.write_commands_file(&commands_content)?;

        // Generate and write events file if there are any events
        let events = analyzer.get_discovered_events();
        if !events.is_empty() {
            let events_content = self.generate_events_file(events, analyzer);
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
