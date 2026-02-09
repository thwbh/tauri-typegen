pub mod base;
pub mod ts;
pub mod zod;

use crate::analysis::CommandAnalyzer;
use crate::models::{CommandInfo, EventInfo, StructInfo};
use crate::GenerateConfig;
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
                Self::collect_referenced_types_from_structure(
                    &param.type_structure,
                    &mut used_types,
                );
            }
            // Add return type from return_type_structure
            Self::collect_referenced_types_from_structure(
                &command.return_type_structure,
                &mut used_types,
            );
            // Add channel message types from message_type_structure
            for channel in &command.channels {
                Self::collect_referenced_types_from_structure(
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
                    Self::collect_referenced_types_from_structure(
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
                Self::collect_referenced_types_from_structure(inner, used_types);
            }
            TypeStructure::Map { key, value } => {
                Self::collect_referenced_types_from_structure(key, used_types);
                Self::collect_referenced_types_from_structure(value, used_types);
            }
            TypeStructure::Tuple(types) => {
                for t in types {
                    Self::collect_referenced_types_from_structure(t, used_types);
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
        config: &GenerateConfig,
    ) -> Vec<CommandContext> {
        let type_resolver = analyzer.get_type_resolver();

        commands
            .iter()
            .map(|cmd| {
                CommandContext::new(config).from_command_info(cmd, visitor, &|rust_type: &str| {
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
        config: &GenerateConfig,
    ) -> Vec<EventContext> {
        let type_resolver = analyzer.get_type_resolver();

        events
            .iter()
            .map(|event| {
                EventContext::new(config).from_event_info(event, visitor, &|rust_type: &str| {
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
        config: &GenerateConfig,
    ) -> Vec<StructContext> {
        used_structs
            .iter()
            .map(|(name, struct_info)| {
                StructContext::new(config).from_struct_info(name, struct_info, visitor)
            })
            .collect()
    }

    /// Create FieldContext instances from StructInfo using the provided visitor
    pub fn create_field_contexts<V: TypeVisitor>(
        &self,
        struct_info: &StructInfo,
        visitor: &V,
        config: &GenerateConfig,
    ) -> Vec<FieldContext> {
        struct_info
            .fields
            .iter()
            .map(|field| {
                FieldContext::new(config).from_field_info(
                    field,
                    &struct_info.serde_rename_all,
                    visitor,
                )
            })
            .collect()
    }
}

impl Default for TypeCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TypeStructure;
    use std::collections::HashSet;

    mod factory {
        use super::*;

        #[test]
        fn test_create_generator_zod() {
            let gen = create_generator(Some("zod".to_string()));
            // Just verify it creates without panic - we can't easily inspect trait objects
            assert!(std::any::type_name_of_val(&gen).contains("Box"));
        }

        #[test]
        fn test_create_generator_none() {
            let gen = create_generator(Some("none".to_string()));
            assert!(std::any::type_name_of_val(&gen).contains("Box"));
        }

        #[test]
        fn test_create_generator_default() {
            let gen = create_generator(None);
            assert!(std::any::type_name_of_val(&gen).contains("Box"));
        }

        #[test]
        fn test_create_generator_unknown_fallback() {
            let gen = create_generator(Some("unknown".to_string()));
            assert!(std::any::type_name_of_val(&gen).contains("Box"));
        }
    }

    mod type_collector {
        use super::*;

        #[test]
        fn test_new_creates_empty_collector() {
            let collector = TypeCollector::new();
            assert!(collector.known_structs.is_empty());
        }

        #[test]
        fn test_default_creates_empty_collector() {
            let collector = TypeCollector::default();
            assert!(collector.known_structs.is_empty());
        }
    }

    mod collect_referenced_types {
        use super::*;

        #[test]
        fn test_collect_primitive() {
            let mut used = HashSet::new();
            let ts = TypeStructure::Primitive("string".to_string());
            TypeCollector::collect_referenced_types_from_structure(&ts, &mut used);
            assert!(used.is_empty());
        }

        #[test]
        fn test_collect_custom() {
            let mut used = HashSet::new();
            let ts = TypeStructure::Custom("User".to_string());
            TypeCollector::collect_referenced_types_from_structure(&ts, &mut used);
            assert_eq!(used.len(), 1);
            assert!(used.contains("User"));
        }

        #[test]
        fn test_collect_array() {
            let mut used = HashSet::new();
            let ts = TypeStructure::Array(Box::new(TypeStructure::Custom("User".to_string())));
            TypeCollector::collect_referenced_types_from_structure(&ts, &mut used);
            assert_eq!(used.len(), 1);
            assert!(used.contains("User"));
        }

        #[test]
        fn test_collect_optional() {
            let mut used = HashSet::new();
            let ts = TypeStructure::Optional(Box::new(TypeStructure::Custom("User".to_string())));
            TypeCollector::collect_referenced_types_from_structure(&ts, &mut used);
            assert_eq!(used.len(), 1);
            assert!(used.contains("User"));
        }

        #[test]
        fn test_collect_result() {
            let mut used = HashSet::new();
            let ts = TypeStructure::Result(Box::new(TypeStructure::Custom("User".to_string())));
            TypeCollector::collect_referenced_types_from_structure(&ts, &mut used);
            assert_eq!(used.len(), 1);
            assert!(used.contains("User"));
        }

        #[test]
        fn test_collect_set() {
            let mut used = HashSet::new();
            let ts = TypeStructure::Set(Box::new(TypeStructure::Custom("User".to_string())));
            TypeCollector::collect_referenced_types_from_structure(&ts, &mut used);
            assert_eq!(used.len(), 1);
            assert!(used.contains("User"));
        }

        #[test]
        fn test_collect_map() {
            let mut used = HashSet::new();
            let ts = TypeStructure::Map {
                key: Box::new(TypeStructure::Primitive("string".to_string())),
                value: Box::new(TypeStructure::Custom("User".to_string())),
            };
            TypeCollector::collect_referenced_types_from_structure(&ts, &mut used);
            assert_eq!(used.len(), 1);
            assert!(used.contains("User"));
        }

        #[test]
        fn test_collect_map_both_custom() {
            let mut used = HashSet::new();
            let ts = TypeStructure::Map {
                key: Box::new(TypeStructure::Custom("UserId".to_string())),
                value: Box::new(TypeStructure::Custom("User".to_string())),
            };
            TypeCollector::collect_referenced_types_from_structure(&ts, &mut used);
            assert_eq!(used.len(), 2);
            assert!(used.contains("User"));
            assert!(used.contains("UserId"));
        }

        #[test]
        fn test_collect_tuple() {
            let mut used = HashSet::new();
            let ts = TypeStructure::Tuple(vec![
                TypeStructure::Custom("User".to_string()),
                TypeStructure::Custom("Product".to_string()),
            ]);
            TypeCollector::collect_referenced_types_from_structure(&ts, &mut used);
            assert_eq!(used.len(), 2);
            assert!(used.contains("User"));
            assert!(used.contains("Product"));
        }

        #[test]
        fn test_collect_nested() {
            let mut used = HashSet::new();
            let ts = TypeStructure::Array(Box::new(TypeStructure::Optional(Box::new(
                TypeStructure::Custom("User".to_string()),
            ))));
            TypeCollector::collect_referenced_types_from_structure(&ts, &mut used);
            assert_eq!(used.len(), 1);
            assert!(used.contains("User"));
        }

        #[test]
        fn test_collect_multiple_calls_accumulate() {
            let mut used = HashSet::new();
            let ts1 = TypeStructure::Custom("User".to_string());
            let ts2 = TypeStructure::Custom("Product".to_string());
            TypeCollector::collect_referenced_types_from_structure(&ts1, &mut used);
            TypeCollector::collect_referenced_types_from_structure(&ts2, &mut used);
            assert_eq!(used.len(), 2);
        }

        #[test]
        fn test_collect_duplicates_deduped() {
            let mut used = HashSet::new();
            let ts = TypeStructure::Custom("User".to_string());
            TypeCollector::collect_referenced_types_from_structure(&ts, &mut used);
            TypeCollector::collect_referenced_types_from_structure(&ts, &mut used);
            assert_eq!(used.len(), 1);
        }
    }

    mod collect_used_types {
        use super::*;
        use crate::models::{CommandInfo, ParameterInfo, StructInfo};

        fn create_struct(name: &str) -> StructInfo {
            StructInfo {
                name: name.to_string(),
                fields: vec![],
                file_path: "test.rs".to_string(),
                is_enum: false,
                serde_rename_all: None,
                serde_tag: None,
                enum_variants: None,
            }
        }

        fn create_param(
            name: &str,
            rust_type: &str,
            type_structure: TypeStructure,
        ) -> ParameterInfo {
            ParameterInfo {
                name: name.to_string(),
                rust_type: rust_type.to_string(),
                is_optional: false,
                type_structure,
                serde_rename: None,
            }
        }

        #[test]
        fn test_collect_from_empty_commands() {
            let collector = TypeCollector::new();
            let commands = vec![];
            let all_structs = HashMap::new();
            let used = collector.collect_used_types(&commands, &all_structs);
            assert!(used.is_empty());
        }

        #[test]
        fn test_collect_from_command_parameters() {
            let collector = TypeCollector::new();
            let mut all_structs = HashMap::new();
            let user_struct = create_struct("User");
            all_structs.insert("User".to_string(), user_struct.clone());

            let param = create_param("user", "User", TypeStructure::Custom("User".to_string()));
            let command = CommandInfo::new_for_test(
                "greet",
                "test.rs",
                1,
                vec![param],
                "string",
                false,
                vec![],
            );

            let used = collector.collect_used_types(&[command], &all_structs);
            assert_eq!(used.len(), 1);
            assert!(used.contains_key("User"));
        }

        #[test]
        fn test_collect_from_command_return_type() {
            let collector = TypeCollector::new();
            let mut all_structs = HashMap::new();
            let result_struct = create_struct("ApiResult");
            all_structs.insert("ApiResult".to_string(), result_struct.clone());

            // Create command that returns ApiResult
            let mut command = CommandInfo::new_for_test(
                "fetch_data",
                "test.rs",
                1,
                vec![],
                "ApiResult",
                false,
                vec![],
            );
            // Set the return_type_structure
            command.return_type_structure = TypeStructure::Custom("ApiResult".to_string());

            let used = collector.collect_used_types(&[command], &all_structs);
            assert_eq!(used.len(), 1);
            assert!(used.contains_key("ApiResult"));
        }

        #[test]
        fn test_filters_unused_types() {
            let collector = TypeCollector::new();
            let mut all_structs = HashMap::new();

            // Add two structs but only use one
            let user_struct = create_struct("User");
            let product_struct = create_struct("Product");
            all_structs.insert("User".to_string(), user_struct);
            all_structs.insert("Product".to_string(), product_struct);

            let param = create_param("user", "User", TypeStructure::Custom("User".to_string()));
            let command = CommandInfo::new_for_test(
                "greet",
                "test.rs",
                1,
                vec![param],
                "string",
                false,
                vec![],
            );

            let used = collector.collect_used_types(&[command], &all_structs);
            assert_eq!(used.len(), 1);
            assert!(used.contains_key("User"));
            assert!(!used.contains_key("Product"));
        }
    }

    mod nested_dependencies {
        use super::*;
        use crate::models::{CommandInfo, FieldInfo, ParameterInfo, StructInfo};

        fn create_field(name: &str, rust_type: &str, type_structure: TypeStructure) -> FieldInfo {
            FieldInfo {
                name: name.to_string(),
                rust_type: rust_type.to_string(),
                is_optional: false,
                is_public: true,
                validator_attributes: None,
                serde_rename: None,
                type_structure,
            }
        }

        fn create_struct_with_fields(name: &str, fields: Vec<FieldInfo>) -> StructInfo {
            StructInfo {
                name: name.to_string(),
                fields,
                file_path: "test.rs".to_string(),
                is_enum: false,
                serde_rename_all: None,
                serde_tag: None,
                enum_variants: None,
            }
        }

        fn create_param(
            name: &str,
            rust_type: &str,
            type_structure: TypeStructure,
        ) -> ParameterInfo {
            ParameterInfo {
                name: name.to_string(),
                rust_type: rust_type.to_string(),
                is_optional: false,
                type_structure,
                serde_rename: None,
            }
        }

        #[test]
        fn test_discovers_nested_dependencies() {
            let collector = TypeCollector::new();
            let mut all_structs = HashMap::new();

            // User has a field of type Address
            let address_field = create_field(
                "address",
                "Address",
                TypeStructure::Custom("Address".to_string()),
            );
            let user_struct = create_struct_with_fields("User", vec![address_field]);
            let address_struct = create_struct_with_fields("Address", vec![]);

            all_structs.insert("User".to_string(), user_struct);
            all_structs.insert("Address".to_string(), address_struct);

            // Command only uses User directly
            let param = create_param("user", "User", TypeStructure::Custom("User".to_string()));
            let command = CommandInfo::new_for_test(
                "greet",
                "test.rs",
                1,
                vec![param],
                "string",
                false,
                vec![],
            );

            let used = collector.collect_used_types(&[command], &all_structs);

            // Should include both User and Address
            assert_eq!(used.len(), 2);
            assert!(used.contains_key("User"));
            assert!(used.contains_key("Address"));
        }

        #[test]
        fn test_handles_deep_nesting() {
            let collector = TypeCollector::new();
            let mut all_structs = HashMap::new();

            // A -> B -> C chain
            let c_struct = create_struct_with_fields("C", vec![]);
            let b_field = create_field("c", "C", TypeStructure::Custom("C".to_string()));
            let b_struct = create_struct_with_fields("B", vec![b_field]);
            let a_field = create_field("b", "B", TypeStructure::Custom("B".to_string()));
            let a_struct = create_struct_with_fields("A", vec![a_field]);

            all_structs.insert("A".to_string(), a_struct);
            all_structs.insert("B".to_string(), b_struct);
            all_structs.insert("C".to_string(), c_struct);

            let param = create_param("data", "A", TypeStructure::Custom("A".to_string()));
            let command = CommandInfo::new_for_test(
                "process",
                "test.rs",
                1,
                vec![param],
                "void",
                false,
                vec![],
            );

            let used = collector.collect_used_types(&[command], &all_structs);

            // Should include A, B, and C
            assert_eq!(used.len(), 3);
            assert!(used.contains_key("A"));
            assert!(used.contains_key("B"));
            assert!(used.contains_key("C"));
        }
    }
}
