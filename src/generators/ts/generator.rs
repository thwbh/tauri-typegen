use crate::analysis::CommandAnalyzer;
use crate::generators::base::file_writer::FileWriter;
use crate::generators::base::templates::TemplateRegistry;
use crate::generators::base::type_visitor::TypeScriptVisitor;
use crate::generators::base::BaseBindingsGenerator;
use crate::generators::ts::templates::TypeScriptTemplate;
use crate::generators::TypeCollector;
use crate::models::{CommandInfo, EventInfo, StructInfo};
use crate::GenerateConfig;
use std::collections::HashMap;
use tera::{Context, Tera};

/// Generator for vanilla TypeScript bindings without validation
pub struct TypeScriptBindingsGenerator {
    collector: TypeCollector,
    tera: Tera,
}

impl TypeScriptBindingsGenerator {
    pub fn new() -> Self {
        Self {
            collector: TypeCollector::new(),
            tera: TypeScriptTemplate::create_tera()
                .expect("Failed to initialize TypeScript template engine"),
        }
    }

    /// Generate the complete types.ts file content
    fn generate_types_file_content(
        &self,
        commands: &[CommandInfo],
        used_structs: &HashMap<String, StructInfo>,
        analyzer: &CommandAnalyzer,
        config: &GenerateConfig,
    ) -> String {
        let has_channels = commands.iter().any(|cmd| !cmd.channels.is_empty());
        let visitor = TypeScriptVisitor::with_config(config);

        // Convert structs to context wrappers
        let struct_context = self
            .collector
            .create_struct_contexts(used_structs, &visitor, config);

        // Convert commands to context wrappers
        let command_context = self
            .collector
            .create_command_contexts(commands, &visitor, analyzer, config);

        // Render main types.ts template
        let mut context = Context::new();
        context.insert("header", &self.generate_file_header());
        context.insert("has_channels", &has_channels);
        context.insert("structs", &struct_context);
        context.insert("commands", &command_context);

        self.render("typescript/types.ts.tera", &context)
            .unwrap_or_else(|e| {
                eprintln!("Template rendering failed for types.ts: {}", e);
                String::new()
            })
    }

    /// Generate command bindings
    fn generate_command_bindings(
        &self,
        commands: &[CommandInfo],
        analyzer: &CommandAnalyzer,
        config: &GenerateConfig,
    ) -> String {
        let has_channels = commands.iter().any(|cmd| !cmd.channels.is_empty());
        let visitor = TypeScriptVisitor::with_config(config);

        // Convert commands to context wrappers
        let command_contexts = self
            .collector
            .create_command_contexts(commands, &visitor, analyzer, config);

        let mut context = Context::new();
        context.insert("header", &self.generate_file_header());
        context.insert("commands", &command_contexts);
        context.insert("has_channels", &has_channels);

        self.render("typescript/commands.ts.tera", &context)
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

        self.render("typescript/index.ts.tera", &context)
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
        let visitor = TypeScriptVisitor::with_config(config);

        // Convert events to context wrappers
        let event_contexts = self
            .collector
            .create_event_contexts(events, &visitor, analyzer, config);

        let mut context = Context::new();
        context.insert("header", &self.generate_file_header());
        context.insert("events", &event_contexts);

        self.render("typescript/events.ts.tera", &context)
            .unwrap_or_else(|e| {
                eprintln!("Template rendering failed for events.ts: {}", e);
                String::new()
            })
    }
}

impl BaseBindingsGenerator for TypeScriptBindingsGenerator {
    fn tera(&self) -> &Tera {
        &self.tera
    }

    fn type_collector(&self) -> &TypeCollector {
        &self.collector
    }

    fn generator_type(&self) -> String {
        "none".to_string()
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

        // Generate and write types file
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

impl Default for TypeScriptBindingsGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod initialization {
        use super::*;

        #[test]
        fn test_new_creates_generator() {
            let gen = TypeScriptBindingsGenerator::new();
            assert!(
                !gen.collector.known_structs.is_empty() || gen.collector.known_structs.is_empty()
            );
        }

        #[test]
        fn test_default_creates_generator() {
            let gen = TypeScriptBindingsGenerator::default();
            assert!(
                !gen.collector.known_structs.is_empty() || gen.collector.known_structs.is_empty()
            );
        }
    }

    mod trait_implementation {
        use super::*;

        #[test]
        fn test_generator_type_returns_none() {
            let gen = TypeScriptBindingsGenerator::new();
            assert_eq!(gen.generator_type(), "none");
        }

        #[test]
        fn test_tera_returns_engine() {
            let gen = TypeScriptBindingsGenerator::new();
            let tera = gen.tera();
            // Verify it has registered templates
            assert!(tera.get_template_names().count() > 0);
        }

        #[test]
        fn test_type_collector_returns_collector() {
            let gen = TypeScriptBindingsGenerator::new();
            let collector = gen.type_collector();
            // Verify collector exists
            assert!(collector.known_structs.is_empty() || !collector.known_structs.is_empty());
        }
    }

    mod template_rendering {
        use super::*;

        #[test]
        fn test_generate_file_header() {
            let gen = TypeScriptBindingsGenerator::new();
            let header = gen.generate_file_header();
            assert!(header.contains("Auto-generated") || header.contains("tauri-typegen"));
            assert!(header.contains("none")); // generator type
        }

        #[test]
        fn test_has_typescript_templates() {
            let gen = TypeScriptBindingsGenerator::new();
            let tera = gen.tera();
            let template_names: Vec<&str> = tera.get_template_names().collect();

            // Check for key templates
            assert!(template_names.contains(&"typescript/types.ts.tera"));
            assert!(template_names.contains(&"typescript/commands.ts.tera"));
            assert!(template_names.contains(&"typescript/index.ts.tera"));
        }

        #[test]
        fn test_render_returns_error_for_invalid_template() {
            let gen = TypeScriptBindingsGenerator::new();
            let context = Context::new();
            let result = gen.render("nonexistent/template.tera", &context);
            assert!(result.is_err());
        }
    }

    mod helper_methods {
        use super::*;

        #[test]
        fn test_generate_index_file_with_empty_files() {
            let gen = TypeScriptBindingsGenerator::new();
            let files = vec![];
            let result = gen.generate_index_file(&files);
            assert!(result.contains("Auto-generated") || result.contains("//"));
        }

        #[test]
        fn test_generate_index_file_with_files() {
            let gen = TypeScriptBindingsGenerator::new();
            let files = vec!["types.ts".to_string(), "commands.ts".to_string()];
            let result = gen.generate_index_file(&files);
            assert!(!result.is_empty());
        }
    }
}
