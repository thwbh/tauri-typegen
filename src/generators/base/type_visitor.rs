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
