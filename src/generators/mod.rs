pub mod base;
pub mod ts;
pub mod zod;

use crate::analysis::CommandAnalyzer;
use crate::models::{CommandInfo, EventInfo, StructInfo};
use base::template_context::{CommandContext, EventContext, FieldContext, StructContext};
use base::type_visitor::TypeVisitor;
use std::collections::HashMap;

pub use base::templates::GlobalContext;
pub use base::BaseBindingsGenerator as BindingsGenerator;
pub use ts::generator::TypeScriptBindingsGenerator;
pub use zod::generator::ZodBindingsGenerator;

/// Macro to reduce boilerplate for template registration
#[macro_export]
macro_rules! template {
    ($tera:expr, $name:expr, $path:expr) => {
        $tera
            .add_raw_template($name, include_str!($path))
            .map_err(|e| format!("Failed to register {}: {}", $name, e))?;
    };
}

/// Factory function to create the appropriate bindings generator
/// Returns a boxed trait object for polymorphism
pub fn create_generator(validation_library: Option<String>) -> Box<dyn BindingsGenerator> {
    match validation_library.as_deref().unwrap_or("none") {
        "zod" => Box::new(ZodBindingsGenerator::new()),
        _ => Box::new(TypeScriptBindingsGenerator::new()),
    }
}

/// Utility for collecting and organizing types for bindings generation
///
/// This struct provides filtering and transformation utilities that sit between
/// the analysis phase (which produces TypeStructure) and the generation phase
/// (which consumes filtered types and contexts). It acts as a one-stop-shop for
/// filtering unused code and collecting only the types needed for generation.
pub struct TypeCollector {
    pub known_structs: HashMap<String, StructInfo>,
}

impl TypeCollector {
    pub fn new() -> Self {
        Self {
            known_structs: HashMap::new(),
        }
    }

    /// Filter only the types used by commands
    pub fn collect_used_types(
        &self,
        commands: &[CommandInfo],
        all_structs: &HashMap<String, StructInfo>,
    ) -> HashMap<String, StructInfo> {
        let mut used_types = std::collections::HashSet::new();

        // Collect types from commands using structured TypeStructure
        for command in commands {
            // Add parameter types from type_structure
            for param in &command.parameters {
                self.collect_referenced_types_from_structure(
                    &param.type_structure,
                    &mut used_types,
                );
            }
            // Add return type from return_type_structure
            self.collect_referenced_types_from_structure(
                &command.return_type_structure,
                &mut used_types,
            );
            // Add channel message types from message_type_structure
            for channel in &command.channels {
                self.collect_referenced_types_from_structure(
                    &channel.message_type_structure,
                    &mut used_types,
                );
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
                    // Use type_structure to collect referenced types
                    self.collect_referenced_types_from_structure(
                        &field.type_structure,
                        &mut nested_types,
                    );

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

    /// Recursively collect custom type names from TypeStructure
    /// Works directly with structured type information instead of string parsing
    pub fn collect_referenced_types_from_structure(
        &self,
        type_structure: &crate::TypeStructure,
        used_types: &mut std::collections::HashSet<String>,
    ) {
        use crate::TypeStructure;

        match type_structure {
            TypeStructure::Custom(name) => {
                used_types.insert(name.clone());
            }
            TypeStructure::Array(inner)
            | TypeStructure::Set(inner)
            | TypeStructure::Optional(inner)
            | TypeStructure::Result(inner) => {
                self.collect_referenced_types_from_structure(inner, used_types);
            }
            TypeStructure::Map { key, value } => {
                self.collect_referenced_types_from_structure(key, used_types);
                self.collect_referenced_types_from_structure(value, used_types);
            }
            TypeStructure::Tuple(types) => {
                for t in types {
                    self.collect_referenced_types_from_structure(t, used_types);
                }
            }
            TypeStructure::Primitive(_) => {
                // Primitives are not custom types
            }
        }
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

impl Default for TypeCollector {
    fn default() -> Self {
        Self::new()
    }
}
