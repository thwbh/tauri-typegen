use crate::generators::base::type_visitor::TypeVisitor;
use crate::models::TypeStructure;
use crate::GenerateConfig;

/// Zod schema visitor - converts TypeStructure to Zod schema strings
pub struct ZodVisitor<'a> {
    config: Option<&'a GenerateConfig>,
}

impl<'a> Default for ZodVisitor<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ZodVisitor<'a> {
    pub fn new() -> Self {
        Self { config: None }
    }

    pub fn with_config(config: &'a GenerateConfig) -> Self {
        Self {
            config: Some(config),
        }
    }
}

impl<'a> TypeVisitor for ZodVisitor<'a> {
    fn get_config(&self) -> Option<&GenerateConfig> {
        self.config
    }
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
        // Check if there's a custom type mapping configured
        if let Some(config) = self.get_config() {
            if let Some(ref mappings) = config.type_mappings {
                if let Some(mapped_type) = mappings.get(name) {
                    // Type is mapped to a primitive TypeScript type
                    // Convert to appropriate Zod schema
                    return match mapped_type.as_str() {
                        "string" => "z.string()".to_string(),
                        "number" => "z.number()".to_string(),
                        "boolean" => "z.boolean()".to_string(),
                        "void" => "z.void()".to_string(),
                        _ => {
                            // For non-primitive mappings, use z.custom()
                            format!("z.custom<{}>((val) => true)", mapped_type)
                        }
                    };
                }
            }
        }
        // No mapping found, reference the schema for custom types
        format!("{}Schema", name)
    }

    /// Override to return TypeScript types (not zod schemas) for type interfaces
    /// This uses the default trait implementations which return proper TypeScript types
    fn visit_type_for_interface(&self, structure: &TypeStructure) -> String {
        // Use the default trait implementations by matching on the structure
        // and calling the trait's default methods
        match structure {
            TypeStructure::Primitive(prim) => prim.clone(),
            TypeStructure::Array(inner) => {
                format!("{}[]", self.visit_type_for_interface(inner))
            }
            TypeStructure::Map { key, value } => {
                format!(
                    "Record<{}, {}>",
                    self.visit_type_for_interface(key),
                    self.visit_type_for_interface(value)
                )
            }
            TypeStructure::Set(inner) => {
                format!("{}[]", self.visit_type_for_interface(inner))
            }
            TypeStructure::Tuple(types) => {
                if types.is_empty() {
                    "void".to_string()
                } else {
                    let type_strs: Vec<String> = types
                        .iter()
                        .map(|t| self.visit_type_for_interface(t))
                        .collect();
                    format!("[{}]", type_strs.join(", "))
                }
            }
            TypeStructure::Optional(inner) => {
                format!("{} | null", self.visit_type_for_interface(inner))
            }
            TypeStructure::Result(inner) => self.visit_type_for_interface(inner),
            TypeStructure::Custom(name) => {
                // Apply custom type mappings
                if let Some(config) = self.get_config() {
                    if let Some(ref mappings) = config.type_mappings {
                        if let Some(mapped_type) = mappings.get(name) {
                            return mapped_type.clone();
                        }
                    }
                }
                // Return the type name (not schema name)
                name.clone()
            }
        }
    }
}
