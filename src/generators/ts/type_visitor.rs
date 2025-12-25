use crate::generators::base::type_visitor::TypeVisitor;
use crate::GenerateConfig;

/// TypeScript type visitor - converts TypeStructure to TypeScript types
/// Note: Does NOT add "types." prefix - that's handled at the template context level
/// for function signatures only (return types, parameters)
pub struct TypeScriptVisitor<'a> {
    config: Option<&'a GenerateConfig>,
}

impl<'a> Default for TypeScriptVisitor<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> TypeScriptVisitor<'a> {
    pub fn new() -> Self {
        Self { config: None }
    }

    pub fn with_config(config: &'a GenerateConfig) -> Self {
        Self {
            config: Some(config),
        }
    }
}

impl<'a> TypeVisitor for TypeScriptVisitor<'a> {
    fn get_config(&self) -> Option<&GenerateConfig> {
        self.config
    }

    fn visit_primitive(&self, type_name: &str) -> String {
        // TypeStructure::Primitive should only contain: "string", "number", "boolean", "void"
        type_name.to_string()
    }

    // Uses default visit_custom implementation which checks type_mappings
}
