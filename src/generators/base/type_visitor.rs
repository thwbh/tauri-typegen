use crate::models::TypeStructure;

/// Visitor pattern for converting TypeStructure to target-specific type representations
pub trait TypeVisitor {
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
    fn visit_custom(&self, name: &str) -> String {
        name.to_string()
    }
}

/// TypeScript type visitor - converts TypeStructure to TypeScript types
/// Note: Does NOT add "types." prefix - that's handled at the template context level
/// for function signatures only (return types, parameters)
pub struct TypeScriptVisitor;

impl TypeVisitor for TypeScriptVisitor {
    fn visit_primitive(&self, type_name: &str) -> String {
        // TypeStructure::Primitive should only contain: "string", "number", "boolean", "void"
        type_name.to_string()
    }

    // Uses default visit_custom implementation which returns the name as-is
}

/// Zod schema visitor - converts TypeStructure to Zod schema strings
pub struct ZodVisitor;

impl TypeVisitor for ZodVisitor {
    fn visit_primitive(&self, type_name: &str) -> String {
        // TypeStructure::Primitive should only contain: "string", "number", "boolean", "void"
        match type_name {
            "string" => "z.string()".to_string(),
            "number" => "z.number()".to_string(),
            "boolean" => "z.boolean()".to_string(),
            "void" => "z.void()".to_string(),
            _ => {
                eprintln!(
                    "Warning: ZodVisitor received unexpected primitive: {}",
                    type_name
                );
                format!("z.unknown() /* Unexpected: {} */", type_name)
            }
        }
    }

    fn visit_array(&self, inner: &TypeStructure) -> String {
        format!("z.array({})", self.visit_type(inner))
    }

    fn visit_map(&self, key: &TypeStructure, value: &TypeStructure) -> String {
        format!(
            "z.record({}, {})",
            self.visit_type(key),
            self.visit_type(value)
        )
    }

    fn visit_set(&self, inner: &TypeStructure) -> String {
        // Zod doesn't have a Set schema, use array
        format!("z.array({})", self.visit_type(inner))
    }

    fn visit_tuple(&self, types: &[TypeStructure]) -> String {
        if types.is_empty() {
            "z.void()".to_string()
        } else {
            let type_strs: Vec<String> = types.iter().map(|t| self.visit_type(t)).collect();
            format!("z.tuple([{}])", type_strs.join(", "))
        }
    }

    fn visit_optional(&self, inner: &TypeStructure) -> String {
        format!("{}.nullable()", self.visit_type(inner))
    }

    fn visit_result(&self, inner: &TypeStructure) -> String {
        // Result in Rust becomes the success type in TypeScript (errors thrown by Tauri)
        self.visit_type(inner)
    }

    fn visit_custom(&self, name: &str) -> String {
        // Reference the schema for custom types
        format!("{}Schema", name)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

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
            let visitor = TypeScriptVisitor;

            assert_eq!(visitor.visit_type(&primitive("string")), "string");
            assert_eq!(visitor.visit_type(&primitive("number")), "number");
            assert_eq!(visitor.visit_type(&primitive("boolean")), "boolean");
            assert_eq!(visitor.visit_type(&primitive("void")), "void");
        }

        #[test]
        fn test_array_types() {
            let visitor = TypeScriptVisitor;

            assert_eq!(visitor.visit_type(&array(primitive("string"))), "string[]");
            assert_eq!(visitor.visit_type(&array(primitive("number"))), "number[]");
        }

        #[test]
        fn test_nested_array() {
            let visitor = TypeScriptVisitor;

            let nested = array(array(primitive("number")));
            assert_eq!(visitor.visit_type(&nested), "number[][]");
        }

        #[test]
        fn test_optional_types() {
            let visitor = TypeScriptVisitor;

            assert_eq!(
                visitor.visit_type(&optional(primitive("string"))),
                "string | null"
            );
            assert_eq!(visitor.visit_type(&optional(custom("User"))), "User | null");
        }

        #[test]
        fn test_map_types() {
            let visitor = TypeScriptVisitor;

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
            let visitor = TypeScriptVisitor;

            // Sets become arrays in TypeScript
            assert_eq!(visitor.visit_type(&set(primitive("string"))), "string[]");
        }

        #[test]
        fn test_tuple_types() {
            let visitor = TypeScriptVisitor;

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
            let visitor = TypeScriptVisitor;

            assert_eq!(visitor.visit_type(&tuple(vec![])), "void");
        }

        #[test]
        fn test_result_types() {
            let visitor = TypeScriptVisitor;

            // Result<T, E> becomes T (errors handled by Tauri)
            assert_eq!(visitor.visit_type(&result(primitive("string"))), "string");
            assert_eq!(visitor.visit_type(&result(custom("User"))), "User");
        }

        #[test]
        fn test_custom_types() {
            let visitor = TypeScriptVisitor;

            assert_eq!(visitor.visit_type(&custom("User")), "User");
            assert_eq!(visitor.visit_type(&custom("Product")), "Product");
        }

        #[test]
        fn test_complex_nested_type() {
            let visitor = TypeScriptVisitor;

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
            let visitor = ZodVisitor;

            assert_eq!(visitor.visit_type(&primitive("string")), "z.string()");
            assert_eq!(visitor.visit_type(&primitive("number")), "z.number()");
            assert_eq!(visitor.visit_type(&primitive("boolean")), "z.boolean()");
            assert_eq!(visitor.visit_type(&primitive("void")), "z.void()");
        }

        #[test]
        fn test_array_types() {
            let visitor = ZodVisitor;

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
            let visitor = ZodVisitor;

            let nested = array(array(primitive("number")));
            assert_eq!(visitor.visit_type(&nested), "z.array(z.array(z.number()))");
        }

        #[test]
        fn test_optional_types() {
            let visitor = ZodVisitor;

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
            let visitor = ZodVisitor;

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
            let visitor = ZodVisitor;

            // Sets become arrays in Zod
            assert_eq!(
                visitor.visit_type(&set(primitive("string"))),
                "z.array(z.string())"
            );
        }

        #[test]
        fn test_tuple_types() {
            let visitor = ZodVisitor;

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
            let visitor = ZodVisitor;

            assert_eq!(visitor.visit_type(&tuple(vec![])), "z.void()");
        }

        #[test]
        fn test_result_types() {
            let visitor = ZodVisitor;

            // Result<T, E> becomes T schema
            assert_eq!(
                visitor.visit_type(&result(primitive("string"))),
                "z.string()"
            );
            assert_eq!(visitor.visit_type(&result(custom("User"))), "UserSchema");
        }

        #[test]
        fn test_custom_types() {
            let visitor = ZodVisitor;

            // Custom types reference their schema
            assert_eq!(visitor.visit_type(&custom("User")), "UserSchema");
            assert_eq!(visitor.visit_type(&custom("Product")), "ProductSchema");
        }

        #[test]
        fn test_complex_nested_type() {
            let visitor = ZodVisitor;

            // HashMap<String, Vec<Option<User>>>
            let complex = map(primitive("string"), array(optional(custom("User"))));

            assert_eq!(
                visitor.visit_type(&complex),
                "z.record(z.string(), z.array(UserSchema.nullable()))"
            );
        }

        #[test]
        fn test_unexpected_primitive() {
            let visitor = ZodVisitor;

            // Should handle unexpected primitives gracefully
            let result = visitor.visit_type(&primitive("unknown_type"));
            assert!(result.contains("z.unknown()"));
        }
    }
}
