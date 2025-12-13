pub mod file_writer;
pub mod template_context;
pub mod templates;
pub mod type_conversion;
pub mod type_visitor;

use crate::analysis::CommandAnalyzer;
use crate::generators::base::template_context::FieldContext;
use crate::generators::base::type_conversion::TypeConverter;
use crate::models::{CommandInfo, EventInfo, StructInfo};
use std::collections::HashMap;

use template_context::{CommandContext, EventContext, StructContext};
use type_visitor::TypeVisitor;

/// Common trait for all generators
pub trait BaseBindingsGenerator {
    /// Generate models from Rust commands and structs
    fn generate_models(
        &mut self,
        commands: &[CommandInfo],
        discovered_structs: &HashMap<String, StructInfo>,
        output_path: &str,
        analyzer: &CommandAnalyzer,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>>;
}

/// Base generator with common functionality
pub struct BaseGenerator {
    pub known_structs: HashMap<String, StructInfo>,
    type_converter: TypeConverter,
}

impl BaseGenerator {
    pub fn new() -> Self {
        Self {
            known_structs: HashMap::new(),
            type_converter: TypeConverter::new(),
        }
    }

    /// Filter only the types used by commands
    pub fn collect_used_types(
        &self,
        commands: &[CommandInfo],
        all_structs: &HashMap<String, StructInfo>,
    ) -> HashMap<String, StructInfo> {
        let mut used_types = std::collections::HashSet::new();

        // Collect types from commands
        for command in commands {
            // Add parameter types
            for param in &command.parameters {
                self.collect_referenced_types(&param.rust_type, &mut used_types);
            }
            // Add return type
            self.collect_referenced_types(&command.return_type, &mut used_types);
            // Add channel message types
            for channel in &command.channels {
                self.collect_referenced_types(&channel.message_type, &mut used_types);
            }
        }

        // Clone to avoid borrow checker issues
        let initial_types = used_types.clone();

        // Discover nested dependencies (types referenced by the collected types)
        self.discover_nested_dependencies(&initial_types, all_structs, &mut used_types);

        // Filter to only include used types
        all_structs
            .iter()
            .filter(|(name, _)| used_types.contains(*name))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Recursively discover nested dependencies
    fn discover_nested_dependencies(
        &self,
        initial_types: &std::collections::HashSet<String>,
        all_structs: &HashMap<String, StructInfo>,
        all_types: &mut std::collections::HashSet<String>,
    ) {
        let mut to_process: Vec<String> = initial_types.iter().cloned().collect();
        let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();

        while let Some(type_name) = to_process.pop() {
            if processed.contains(&type_name) {
                continue;
            }
            processed.insert(type_name.clone());

            if let Some(struct_info) = all_structs.get(&type_name) {
                for field in &struct_info.fields {
                    let mut nested_types = std::collections::HashSet::new();
                    // Use rust_type to collect referenced types
                    self.collect_referenced_types(&field.rust_type, &mut nested_types);

                    for nested_type in nested_types {
                        if !all_types.contains(&nested_type)
                            && all_structs.contains_key(&nested_type)
                        {
                            all_types.insert(nested_type.clone());
                            to_process.push(nested_type);
                        }
                    }
                }
            }
        }
    }

    /// Recursively collect type names from complex types
    /// Delegates to TypeConverter for comprehensive type extraction
    pub fn collect_referenced_types(
        &self,
        type_str: &str,
        used_types: &mut std::collections::HashSet<String>,
    ) {
        self.type_converter
            .collect_referenced_types(type_str, used_types);
    }

    /// Create CommandContext instances from CommandInfo using the provided visitor
    pub fn create_command_contexts<V: TypeVisitor>(
        &self,
        commands: &[CommandInfo],
        visitor: &V,
        analyzer: &CommandAnalyzer,
    ) -> Vec<CommandContext> {
        let type_resolver = analyzer.get_type_resolver();

        commands
            .iter()
            .map(|cmd| {
                CommandContext::from_command_info(cmd, visitor, &|rust_type: &str| {
                    type_resolver.borrow_mut().parse_type_structure(rust_type)
                })
            })
            .collect()
    }

    /// Create EventContext instances from EventInfo using the provided visitor
    pub fn create_event_contexts<V: TypeVisitor>(
        &self,
        events: &[EventInfo],
        visitor: &V,
        analyzer: &CommandAnalyzer,
    ) -> Vec<EventContext> {
        let type_resolver = analyzer.get_type_resolver();

        events
            .iter()
            .map(|event| {
                EventContext::from_event_info(event, visitor, &|rust_type: &str| {
                    type_resolver.borrow_mut().parse_type_structure(rust_type)
                })
            })
            .collect()
    }

    /// Create StructContext instances from StructInfo using the provided visitor
    pub fn create_struct_contexts<V: TypeVisitor>(
        &self,
        used_structs: &HashMap<String, StructInfo>,
        visitor: &V,
    ) -> Vec<StructContext> {
        used_structs
            .iter()
            .map(|(name, struct_info)| StructContext::from_struct_info(name, struct_info, visitor))
            .collect()
    }

    /// Create FieldContext instances from StructInfo using the provided visitor
    pub fn create_field_contexts<V: TypeVisitor>(
        &self,
        struct_info: &StructInfo,
        visitor: &V,
    ) -> Vec<FieldContext> {
        struct_info
            .fields
            .iter()
            .map(|field| {
                FieldContext::from_field_info(field, &struct_info.serde_rename_all, visitor)
            })
            .collect()
    }
}

impl Default for BaseGenerator {
    fn default() -> Self {
        Self::new()
    }
}
