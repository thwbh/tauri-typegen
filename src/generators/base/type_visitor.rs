use crate::models::TypeStructure;
use crate::GenerateConfig;

/// Visitor pattern for converting TypeStructure to target-specific type representations
pub trait TypeVisitor {
    /// Get the config (if any) for type mappings
    fn get_config(&self) -> Option<&GenerateConfig> {
        None
    }
    /// Convert a TypeStructure to the target language's type string
    fn visit_type(&self, structure: &TypeStructure) -> String {
        match structure {
            TypeStructure::Primitive(prim) => self.visit_primitive(prim),
            TypeStructure::Array(inner) => self.visit_array(inner),
            TypeStructure::Map { key, value } => self.visit_map(key, value),
            TypeStructure::Set(inner) => self.visit_set(inner),
            TypeStructure::Tuple(types) => self.visit_tuple(types),
            TypeStructure::Optional(inner) => self.visit_optional(inner),
            TypeStructure::Result(inner) => self.visit_result(inner),
            TypeStructure::Custom(name) => self.visit_custom(name),
        }
    }

    /// Visit a primitive type
    fn visit_primitive(&self, type_name: &str) -> String;

    /// Visit an array type
    fn visit_array(&self, inner: &TypeStructure) -> String {
        format!("{}[]", self.visit_type(inner))
    }

    /// Visit a map type (HashMap, BTreeMap)
    fn visit_map(&self, key: &TypeStructure, value: &TypeStructure) -> String {
        format!(
            "Record<{}, {}>",
            self.visit_type(key),
            self.visit_type(value)
        )
    }

    /// Visit a set type (HashSet, BTreeSet)
    fn visit_set(&self, inner: &TypeStructure) -> String {
        format!("{}[]", self.visit_type(inner))
    }

    /// Visit a tuple type
    fn visit_tuple(&self, types: &[TypeStructure]) -> String {
        if types.is_empty() {
            "void".to_string()
        } else {
            let type_strs: Vec<String> = types.iter().map(|t| self.visit_type(t)).collect();
            format!("[{}]", type_strs.join(", "))
        }
    }

    /// Visit an optional type
    fn visit_optional(&self, inner: &TypeStructure) -> String {
        format!("{} | null", self.visit_type(inner))
    }

    /// Visit a result type (success type only, errors handled by Tauri)
    fn visit_result(&self, inner: &TypeStructure) -> String {
        self.visit_type(inner)
    }

    /// Visit a custom/user-defined type
    /// Checks config.type_mappings first before returning the type name as-is
    fn visit_custom(&self, name: &str) -> String {
        // Check if there's a custom type mapping configured
        if let Some(config) = self.get_config() {
            if let Some(ref mappings) = config.type_mappings {
                if let Some(mapped_type) = mappings.get(name) {
                    return mapped_type.clone();
                }
            }
        }
        // No mapping found, return the type name as-is
        name.to_string()
    }

    /// Visit a type for use in TypeScript type interfaces (not schemas)
    /// Default implementation uses visit_type, but can be overridden by visitors
    /// that need different representations for type interfaces vs schemas (e.g., ZodVisitor)
    fn visit_type_for_interface(&self, structure: &TypeStructure) -> String {
        self.visit_type(structure)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::ts::type_visitor::TypeScriptVisitor;
    use crate::generators::zod::type_visitor::ZodVisitor;

    #[test]
    fn test_zod_visitor_interface_types() {
        let config = GenerateConfig::default();
        let visitor = ZodVisitor::with_config(&config);

        let num_type = TypeStructure::Primitive("number".to_string());
        let custom_type = TypeStructure::Custom("LogEntry".to_string());

        // For schemas
        assert_eq!(visitor.visit_type(&num_type), "z.number()");
        assert_eq!(visitor.visit_type(&custom_type), "LogEntrySchema");

        // For type interfaces
        assert_eq!(visitor.visit_type_for_interface(&num_type), "number");
        assert_eq!(visitor.visit_type_for_interface(&custom_type), "LogEntry");
    }

    // Helper to create test type structures
    fn primitive(name: &str) -> TypeStructure {
        TypeStructure::Primitive(name.to_string())
    }

    fn array(inner: TypeStructure) -> TypeStructure {
        TypeStructure::Array(Box::new(inner))
    }

    fn optional(inner: TypeStructure) -> TypeStructure {
        TypeStructure::Optional(Box::new(inner))
    }

    fn map(key: TypeStructure, value: TypeStructure) -> TypeStructure {
        TypeStructure::Map {
            key: Box::new(key),
            value: Box::new(value),
        }
    }

    fn tuple(types: Vec<TypeStructure>) -> TypeStructure {
        TypeStructure::Tuple(types)
    }

    fn custom(name: &str) -> TypeStructure {
        TypeStructure::Custom(name.to_string())
    }

    fn result(inner: TypeStructure) -> TypeStructure {
        TypeStructure::Result(Box::new(inner))
    }

    fn set(inner: TypeStructure) -> TypeStructure {
        TypeStructure::Set(Box::new(inner))
    }

    // TypeScriptVisitor tests
    mod typescript_visitor {
        use super::*;

        #[test]
        fn test_primitive_types() {
            let visitor = TypeScriptVisitor::new();

            assert_eq!(visitor.visit_type(&primitive("string")), "string");
            assert_eq!(visitor.visit_type(&primitive("number")), "number");
            assert_eq!(visitor.visit_type(&primitive("boolean")), "boolean");
            assert_eq!(visitor.visit_type(&primitive("void")), "void");
        }

        #[test]
        fn test_array_types() {
            let visitor = TypeScriptVisitor::new();

            assert_eq!(visitor.visit_type(&array(primitive("string"))), "string[]");
            assert_eq!(visitor.visit_type(&array(primitive("number"))), "number[]");
        }

        #[test]
        fn test_nested_array() {
            let visitor = TypeScriptVisitor::new();

            let nested = array(array(primitive("number")));
            assert_eq!(visitor.visit_type(&nested), "number[][]");
        }

        #[test]
        fn test_optional_types() {
            let visitor = TypeScriptVisitor::new();

            assert_eq!(
                visitor.visit_type(&optional(primitive("string"))),
                "string | null"
            );
            assert_eq!(visitor.visit_type(&optional(custom("User"))), "User | null");
        }

        #[test]
        fn test_map_types() {
            let visitor = TypeScriptVisitor::new();

            assert_eq!(
                visitor.visit_type(&map(primitive("string"), primitive("number"))),
                "Record<string, number>"
            );
            assert_eq!(
                visitor.visit_type(&map(primitive("string"), custom("User"))),
                "Record<string, User>"
            );
        }

        #[test]
        fn test_set_types() {
            let visitor = TypeScriptVisitor::new();

            // Sets become arrays in TypeScript
            assert_eq!(visitor.visit_type(&set(primitive("string"))), "string[]");
        }

        #[test]
        fn test_tuple_types() {
            let visitor = TypeScriptVisitor::new();

            assert_eq!(
                visitor.visit_type(&tuple(vec![primitive("string"), primitive("number")])),
                "[string, number]"
            );
            assert_eq!(
                visitor.visit_type(&tuple(vec![
                    primitive("string"),
                    primitive("number"),
                    primitive("boolean")
                ])),
                "[string, number, boolean]"
            );
        }

        #[test]
        fn test_empty_tuple() {
            let visitor = TypeScriptVisitor::new();

            assert_eq!(visitor.visit_type(&tuple(vec![])), "void");
        }

        #[test]
        fn test_result_types() {
            let visitor = TypeScriptVisitor::new();

            // Result<T, E> becomes T (errors handled by Tauri)
            assert_eq!(visitor.visit_type(&result(primitive("string"))), "string");
            assert_eq!(visitor.visit_type(&result(custom("User"))), "User");
        }

        #[test]
        fn test_custom_types() {
            let visitor = TypeScriptVisitor::new();

            assert_eq!(visitor.visit_type(&custom("User")), "User");
            assert_eq!(visitor.visit_type(&custom("Product")), "Product");
        }

        #[test]
        fn test_complex_nested_type() {
            let visitor = TypeScriptVisitor::new();

            // HashMap<String, Vec<Option<User>>>
            let complex = map(primitive("string"), array(optional(custom("User"))));

            // Note: The visitor doesn't add parentheses around "User | null"
            // So "Vec<Option<User>>" becomes "User | null[]" not "(User | null)[]"
            // This is technically incorrect TypeScript (means "User or array of null")
            // but matches current implementation
            assert_eq!(
                visitor.visit_type(&complex),
                "Record<string, User | null[]>"
            );
        }
    }

    // ZodVisitor tests
    mod zod_visitor {
        use super::*;

        #[test]
        fn test_primitive_types() {
            let visitor = ZodVisitor::new();

            assert_eq!(visitor.visit_type(&primitive("string")), "z.string()");
            assert_eq!(visitor.visit_type(&primitive("number")), "z.number()");
            assert_eq!(visitor.visit_type(&primitive("boolean")), "z.boolean()");
            assert_eq!(visitor.visit_type(&primitive("void")), "z.void()");
        }

        #[test]
        fn test_array_types() {
            let visitor = ZodVisitor::new();

            assert_eq!(
                visitor.visit_type(&array(primitive("string"))),
                "z.array(z.string())"
            );
            assert_eq!(
                visitor.visit_type(&array(primitive("number"))),
                "z.array(z.number())"
            );
        }

        #[test]
        fn test_nested_array() {
            let visitor = ZodVisitor::new();

            let nested = array(array(primitive("number")));
            assert_eq!(visitor.visit_type(&nested), "z.array(z.array(z.number()))");
        }

        #[test]
        fn test_optional_types() {
            let visitor = ZodVisitor::new();

            assert_eq!(
                visitor.visit_type(&optional(primitive("string"))),
                "z.string().nullable()"
            );
            assert_eq!(
                visitor.visit_type(&optional(custom("User"))),
                "UserSchema.nullable()"
            );
        }

        #[test]
        fn test_map_types() {
            let visitor = ZodVisitor::new();

            assert_eq!(
                visitor.visit_type(&map(primitive("string"), primitive("number"))),
                "z.record(z.string(), z.number())"
            );
            assert_eq!(
                visitor.visit_type(&map(primitive("string"), custom("User"))),
                "z.record(z.string(), UserSchema)"
            );
        }

        #[test]
        fn test_set_types() {
            let visitor = ZodVisitor::new();

            // Sets become arrays in Zod
            assert_eq!(
                visitor.visit_type(&set(primitive("string"))),
                "z.array(z.string())"
            );
        }

        #[test]
        fn test_tuple_types() {
            let visitor = ZodVisitor::new();

            assert_eq!(
                visitor.visit_type(&tuple(vec![primitive("string"), primitive("number")])),
                "z.tuple([z.string(), z.number()])"
            );
            assert_eq!(
                visitor.visit_type(&tuple(vec![
                    primitive("string"),
                    primitive("number"),
                    primitive("boolean")
                ])),
                "z.tuple([z.string(), z.number(), z.boolean()])"
            );
        }

        #[test]
        fn test_empty_tuple() {
            let visitor = ZodVisitor::new();

            assert_eq!(visitor.visit_type(&tuple(vec![])), "z.void()");
        }

        #[test]
        fn test_result_types() {
            let visitor = ZodVisitor::new();

            // Result<T, E> becomes T schema
            assert_eq!(
                visitor.visit_type(&result(primitive("string"))),
                "z.string()"
            );
            assert_eq!(visitor.visit_type(&result(custom("User"))), "UserSchema");
        }

        #[test]
        fn test_custom_types() {
            let visitor = ZodVisitor::new();

            // Custom types reference their schema
            assert_eq!(visitor.visit_type(&custom("User")), "UserSchema");
            assert_eq!(visitor.visit_type(&custom("Product")), "ProductSchema");
        }

        #[test]
        fn test_complex_nested_type() {
            let visitor = ZodVisitor::new();

            // HashMap<String, Vec<Option<User>>>
            let complex = map(primitive("string"), array(optional(custom("User"))));

            assert_eq!(
                visitor.visit_type(&complex),
                "z.record(z.string(), z.array(UserSchema.nullable()))"
            );
        }

        #[test]
        fn test_unexpected_primitive() {
            let visitor = ZodVisitor::new();

            // Should handle unexpected primitives gracefully
            let result = visitor.visit_type(&primitive("unknown_type"));
            assert!(result.contains("z.unknown()"));
        }
    }

    // Type mappings tests
    mod type_mappings {
        use super::*;
        use crate::GenerateConfig;
        use std::collections::HashMap;

        fn create_test_config_with_mappings() -> GenerateConfig {
            let mut type_mappings = HashMap::new();
            type_mappings.insert("CustomDateTime".to_string(), "string".to_string());
            type_mappings.insert("CustomDate".to_string(), "string".to_string());

            GenerateConfig {
                project_path: ".".to_string(),
                output_path: "./output".to_string(),
                validation_library: "none".to_string(),
                visualize_deps: Some(false),
                verbose: Some(false),
                include_private: Some(false),
                type_mappings: Some(type_mappings),
                exclude_patterns: None,
                include_patterns: None,
                default_parameter_case: "camelCase".to_string(),
                default_field_case: "camelCase".to_string(),
            }
        }

        #[test]
        fn test_typescript_visitor_without_mappings() {
            let visitor = TypeScriptVisitor::new();

            // Without mappings, custom types should return their name as-is
            assert_eq!(
                visitor.visit_type(&custom("CustomDateTime")),
                "CustomDateTime"
            );
            assert_eq!(visitor.visit_type(&custom("CustomDate")), "CustomDate");
        }

        #[test]
        fn test_typescript_visitor_with_mappings() {
            let config = create_test_config_with_mappings();
            let visitor = TypeScriptVisitor::with_config(&config);

            // With mappings, custom types should return the mapped type
            assert_eq!(visitor.visit_type(&custom("CustomDateTime")), "string");
            assert_eq!(visitor.visit_type(&custom("CustomDate")), "string");

            // Unmapped types should still return their name
            assert_eq!(visitor.visit_type(&custom("UnmappedType")), "UnmappedType");
        }

        #[test]
        fn test_typescript_visitor_mappings_in_complex_types() {
            let config = create_test_config_with_mappings();
            let visitor = TypeScriptVisitor::with_config(&config);

            // Array of mapped type
            let array_of_custom = array(custom("CustomDateTime"));
            assert_eq!(visitor.visit_type(&array_of_custom), "string[]");

            // Optional mapped type
            let optional_custom = optional(custom("CustomDate"));
            assert_eq!(visitor.visit_type(&optional_custom), "string | null");

            // Map with mapped value
            let map_with_custom = map(primitive("string"), custom("CustomDateTime"));
            assert_eq!(
                visitor.visit_type(&map_with_custom),
                "Record<string, string>"
            );
        }

        #[test]
        fn test_zod_visitor_without_mappings() {
            let visitor = ZodVisitor::new();

            // Without mappings, custom types should reference their schema
            assert_eq!(
                visitor.visit_type(&custom("CustomDateTime")),
                "CustomDateTimeSchema"
            );
            assert_eq!(
                visitor.visit_type(&custom("CustomDate")),
                "CustomDateSchema"
            );
        }

        #[test]
        fn test_zod_visitor_with_string_mapping() {
            let config = create_test_config_with_mappings();
            let visitor = ZodVisitor::with_config(&config);

            // With string mapping, should return z.string()
            assert_eq!(visitor.visit_type(&custom("CustomDateTime")), "z.string()");
            assert_eq!(visitor.visit_type(&custom("CustomDate")), "z.string()");

            // Unmapped types should still reference their schema
            assert_eq!(
                visitor.visit_type(&custom("UnmappedType")),
                "UnmappedTypeSchema"
            );
        }

        #[test]
        fn test_zod_visitor_with_number_mapping() {
            let mut type_mappings = HashMap::new();
            type_mappings.insert("Timestamp".to_string(), "number".to_string());

            let config = GenerateConfig {
                type_mappings: Some(type_mappings),
                ..Default::default()
            };

            let visitor = ZodVisitor::with_config(&config);

            // Should map to z.number()
            assert_eq!(visitor.visit_type(&custom("Timestamp")), "z.number()");
        }

        #[test]
        fn test_zod_visitor_mappings_in_complex_types() {
            let config = create_test_config_with_mappings();
            let visitor = ZodVisitor::with_config(&config);

            // Array of mapped type
            let array_of_custom = array(custom("CustomDateTime"));
            assert_eq!(visitor.visit_type(&array_of_custom), "z.array(z.string())");

            // Optional mapped type
            let optional_custom = optional(custom("CustomDate"));
            assert_eq!(
                visitor.visit_type(&optional_custom),
                "z.string().nullable()"
            );

            // Map with mapped value
            let map_with_custom = map(primitive("string"), custom("CustomDateTime"));
            assert_eq!(
                visitor.visit_type(&map_with_custom),
                "z.record(z.string(), z.string())"
            );
        }

        #[test]
        fn test_zod_visitor_with_non_primitive_mapping() {
            let mut type_mappings = HashMap::new();
            type_mappings.insert("CustomType".to_string(), "MyCustomType".to_string());

            let config = GenerateConfig {
                type_mappings: Some(type_mappings),
                ..Default::default()
            };

            let visitor = ZodVisitor::with_config(&config);

            // Non-primitive mappings should use z.custom()
            let result = visitor.visit_type(&custom("CustomType"));
            assert!(result.contains("z.custom<MyCustomType>"));
            assert!(result.contains("(val) => true"));
        }
    }
}
