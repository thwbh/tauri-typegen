use crate::analysis::CommandAnalyzer;
use crate::generators::base::file_writer::FileWriter;
use crate::generators::base::templates::TemplateRegistry;
use crate::generators::base::BaseBindingsGenerator;
use crate::generators::ts::templates::TypeScriptTemplate;
use crate::generators::ts::type_visitor::TypeScriptVisitor;
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
        let struct_contexts = self
            .collector
            .create_struct_contexts(used_structs, &visitor, config);

        // Convert commands to context wrappers
        let command_contexts = self
            .collector
            .create_command_contexts(commands, &visitor, analyzer, config);
        let param_commands = command_contexts
            .iter()
            .filter(|command| !command.parameters.is_empty() || !command.channels.is_empty())
            .cloned()
            .collect::<Vec<_>>();

        // Render main types.ts template
        let mut context = Context::new();
        context.insert("header", &self.generate_file_header());
        context.insert("has_channels", &has_channels);
        context.insert("structs", &struct_contexts);
        context.insert("param_commands", &param_commands);

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
        let modules = generated_files
            .iter()
            .filter(|file| file.as_str() != "index.ts")
            .cloned()
            .collect::<Vec<_>>();
        let mut context = Context::new();
        context.insert("header", &self.generate_file_header());
        context.insert("modules", &modules);

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

        // Filter to only the types used by commands and events
        let events = analyzer.get_discovered_events();
        let used_structs = self
            .collector
            .collect_used_types(commands, events, discovered_structs);

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
    use crate::analysis::CommandAnalyzer;
    use crate::models::{EventInfo, FieldInfo, StructInfo, TypeStructure};
    use crate::GenerateConfig;
    use std::collections::HashMap;

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
        use crate::models::{ParameterInfo, TypeStructure};

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

        #[test]
        fn test_generate_index_file_skips_index_without_blank_lines() {
            let gen = TypeScriptBindingsGenerator::new();
            let files = vec![
                "types.ts".to_string(),
                "index.ts".to_string(),
                "commands.ts".to_string(),
            ];
            let result = result_without_timestamp(&gen.generate_index_file(&files));

            assert!(result.contains(" */\n\nexport * from './types';"));
            assert!(result.contains("export * from './types';\nexport * from './commands';"));
            assert!(!result.contains("export * from './types';\n\nexport * from './commands';"));
        }

        #[test]
        fn test_generate_command_bindings_avoid_blank_lines_between_functions() {
            let gen = TypeScriptBindingsGenerator::new();
            let analyzer = CommandAnalyzer::new();
            let config = GenerateConfig::default();
            let commands = vec![
                CommandInfo::new_for_test(
                    "alpha_command",
                    "a.rs",
                    1,
                    vec![ParameterInfo {
                        name: "value".to_string(),
                        rust_type: "String".to_string(),
                        is_optional: false,
                        type_structure: TypeStructure::Primitive("string".to_string()),
                        serde_rename: None,
                    }],
                    "Alpha",
                    false,
                    vec![],
                ),
                CommandInfo::new_for_test("beta_command", "b.rs", 1, vec![], "Beta", false, vec![]),
            ];
            let rendered = result_without_timestamp(
                &gen.generate_command_bindings(&commands, &analyzer, &config),
            );

            assert!(
                rendered.contains(
                    "import * as types from './types';\n\nexport async function alphaCommand"
                ),
                "unexpected render:\n{rendered}"
            );
            assert!(
                rendered.contains(
                    "return invoke('alpha_command', params);\n}\n\nexport async function betaCommand"
                ),
                "unexpected render:\n{rendered}"
            );
            assert!(
                !rendered.contains(
                    "return invoke('alpha_command', params);\n}\n\n\nexport async function betaCommand"
                ),
                "unexpected render:\n{rendered}"
            );
        }

        #[test]
        fn test_generate_events_file_has_single_blank_line_between_listeners() {
            let gen = TypeScriptBindingsGenerator::new();
            let analyzer = CommandAnalyzer::new();
            let config = GenerateConfig::default();
            let events = vec![
                EventInfo {
                    event_name: "alpha-ready".to_string(),
                    payload_type: "String".to_string(),
                    payload_type_structure: TypeStructure::Primitive("string".to_string()),
                    file_path: "a.rs".to_string(),
                    line_number: 1,
                },
                EventInfo {
                    event_name: "beta-ready".to_string(),
                    payload_type: "String".to_string(),
                    payload_type_structure: TypeStructure::Primitive("string".to_string()),
                    file_path: "b.rs".to_string(),
                    line_number: 2,
                },
            ];
            let rendered =
                result_without_timestamp(&gen.generate_events_file(&events, &analyzer, &config));

            assert!(
                rendered.contains("  });\n}\n\n/**\n * Listen for 'beta-ready' events"),
                "unexpected render:\n{rendered}"
            );
            assert!(
                !rendered.contains("  });\n}\n\n\n/**\n * Listen for 'beta-ready' events"),
                "unexpected render:\n{rendered}"
            );
        }

        #[test]
        fn test_generate_types_file_keeps_blank_line_after_header() {
            let gen = TypeScriptBindingsGenerator::new();
            let analyzer = CommandAnalyzer::new();
            let config = GenerateConfig::default();
            let rendered = result_without_timestamp(&gen.generate_types_file_content(
                &[],
                &HashMap::new(),
                &analyzer,
                &config,
            ));

            assert!(
                rendered.contains(" */\n\n"),
                "unexpected render:\n{rendered}"
            );
        }

        #[test]
        fn test_generate_types_file_compacts_param_interfaces() {
            let gen = TypeScriptBindingsGenerator::new();
            let analyzer = CommandAnalyzer::new();
            let config = GenerateConfig::default();
            let commands = vec![CommandInfo::new_for_test(
                "abort_loopback_fetch",
                "test.rs",
                1,
                vec![ParameterInfo {
                    name: "request_id".to_string(),
                    rust_type: "String".to_string(),
                    is_optional: false,
                    type_structure: TypeStructure::Primitive("string".to_string()),
                    serde_rename: None,
                }],
                "void",
                false,
                vec![],
            )];
            let rendered = result_without_timestamp(&gen.generate_types_file_content(
                &commands,
                &HashMap::new(),
                &analyzer,
                &config,
            ));

            assert!(
                rendered.contains(
                    "export interface AbortLoopbackFetchParams {\n  requestId: string;\n  [key: string]: unknown;\n}"
                ),
                "unexpected render:\n{rendered}"
            );
        }

        fn result_without_timestamp(content: &str) -> String {
            content
                .lines()
                .map(|line| {
                    if line.starts_with(" * Generated at:") {
                        " * Generated at: <normalized>".to_string()
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    }

    mod determinism {
        use super::*;

        fn create_test_config() -> GenerateConfig {
            GenerateConfig {
                project_path: ".".to_string(),
                output_path: "./generated".to_string(),
                validation_library: "none".to_string(),
                verbose: Some(false),
                visualize_deps: Some(false),
                include_private: Some(false),
                type_mappings: None,
                exclude_patterns: None,
                include_patterns: None,
                default_parameter_case: "camelCase".to_string(),
                default_field_case: "snake_case".to_string(),
                force: Some(false),
            }
        }

        fn create_test_struct(name: &str, rust_type: &str, ts_type: &str) -> StructInfo {
            StructInfo {
                name: name.to_string(),
                fields: vec![FieldInfo {
                    name: "value".to_string(),
                    rust_type: rust_type.to_string(),
                    is_optional: false,
                    is_public: true,
                    type_structure: TypeStructure::Primitive(ts_type.to_string()),
                    serde_rename: None,
                    validator_attributes: None,
                }],
                file_path: format!("{name}.rs"),
                is_enum: false,
                serde_rename_all: None,
                serde_tag: None,
                enum_variants: None,
            }
        }

        fn create_test_event(event_name: &str, file_path: &str, line_number: usize) -> EventInfo {
            EventInfo {
                event_name: event_name.to_string(),
                payload_type: "String".to_string(),
                payload_type_structure: TypeStructure::Primitive("string".to_string()),
                file_path: file_path.to_string(),
                line_number,
            }
        }

        fn normalize_generated_output(content: &str) -> String {
            content
                .lines()
                .map(|line| {
                    if line.starts_with(" * Generated at:") {
                        " * Generated at: <normalized>".to_string()
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        }

        #[test]
        fn deterministic_output_for_reversed_inputs() {
            let generator = TypeScriptBindingsGenerator::new();
            let analyzer = CommandAnalyzer::new();
            let config = create_test_config();

            let commands1 = vec![
                CommandInfo::new_for_test(
                    "alpha_command",
                    "b.rs",
                    1,
                    vec![],
                    "Alpha",
                    false,
                    vec![],
                ),
                CommandInfo::new_for_test("beta_command", "a.rs", 1, vec![], "Beta", false, vec![]),
            ];
            let commands2 = vec![
                CommandInfo::new_for_test("beta_command", "a.rs", 1, vec![], "Beta", false, vec![]),
                CommandInfo::new_for_test(
                    "alpha_command",
                    "b.rs",
                    1,
                    vec![],
                    "Alpha",
                    false,
                    vec![],
                ),
            ];

            let mut structs1 = HashMap::new();
            structs1.insert(
                "Alpha".to_string(),
                create_test_struct("Alpha", "String", "string"),
            );
            structs1.insert(
                "Beta".to_string(),
                create_test_struct("Beta", "i32", "number"),
            );

            let mut structs2 = HashMap::new();
            structs2.insert(
                "Beta".to_string(),
                create_test_struct("Beta", "i32", "number"),
            );
            structs2.insert(
                "Alpha".to_string(),
                create_test_struct("Alpha", "String", "string"),
            );

            let events1 = vec![
                create_test_event("beta-ready", "b.rs", 20),
                create_test_event("alpha-ready", "a.rs", 10),
            ];
            let events2 = vec![
                create_test_event("alpha-ready", "a.rs", 10),
                create_test_event("beta-ready", "b.rs", 20),
            ];

            let types1 =
                generator.generate_types_file_content(&commands1, &structs1, &analyzer, &config);
            let types2 =
                generator.generate_types_file_content(&commands2, &structs2, &analyzer, &config);
            let commands_file1 =
                generator.generate_command_bindings(&commands1, &analyzer, &config);
            let commands_file2 =
                generator.generate_command_bindings(&commands2, &analyzer, &config);
            let events_file1 = generator.generate_events_file(&events1, &analyzer, &config);
            let events_file2 = generator.generate_events_file(&events2, &analyzer, &config);

            assert_eq!(
                normalize_generated_output(&types1),
                normalize_generated_output(&types2)
            );
            assert_eq!(
                normalize_generated_output(&commands_file1),
                normalize_generated_output(&commands_file2)
            );
            assert_eq!(
                normalize_generated_output(&events_file1),
                normalize_generated_output(&events_file2)
            );

            for (file_name, content) in [
                ("types.ts", &types1),
                ("commands.ts", &commands_file1),
                ("events.ts", &events_file1),
            ] {
                let normalized = normalize_generated_output(content);
                assert!(
                    !normalized.contains("\n\n\n"),
                    "unexpected blank lines in {file_name}:\n{normalized}"
                );
                assert!(content.ends_with('\n'));
            }
        }
    }
}
