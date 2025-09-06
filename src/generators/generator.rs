

use crate::analysis::CommandAnalyzer;
use crate::generators::base::BaseBindingsGenerator as GeneratorTrait;
use crate::models::{CommandInfo, StructInfo};
use std::collections::HashMap;
use crate::generators::TypeScriptBindingsGenerator;
use crate::generators::ZodBindingsGenerator;

pub struct BindingsGenerator {
    validation_library: String,
}

impl BindingsGenerator {
    pub fn new(validation_library: Option<String>) -> Self {
        Self {
            validation_library: validation_library.unwrap_or_else(|| "none".to_string()),
        }
    }

    pub fn generate_models(
        &mut self,
        commands: &[CommandInfo],
        discovered_structs: &HashMap<String, StructInfo>,
        output_path: &str,
        analyzer: &CommandAnalyzer,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        match self.validation_library.as_str() {
            "zod" => {
                let mut generator = ZodBindingsGenerator::new();
                generator.generate_models(commands, discovered_structs, output_path, analyzer)
            }
            "none" => {
                let mut generator = TypeScriptBindingsGenerator::new();
                generator.generate_models(commands, discovered_structs, output_path, analyzer)
            }
            _ => {
                // For other validation libraries, fall back to vanilla
                let mut generator = TypeScriptBindingsGenerator::new();
                generator.generate_models(commands, discovered_structs, output_path, analyzer)
            }
        }
    }

    // Helper methods for testing - delegate to appropriate generator
    pub fn to_pascal_case(&self, s: &str) -> String {
        match self.validation_library.as_str() {
            "zod" => {
                let generator = ZodBindingsGenerator::new();
                generator.to_pascal_case(s)
            }
            _ => {
                let generator = TypeScriptBindingsGenerator::new();
                generator.to_pascal_case(s)
            }
        }
    }

    pub fn to_camel_case(&self, s: &str) -> String {
        match self.validation_library.as_str() {
            "zod" => {
                let generator = ZodBindingsGenerator::new();
                generator.to_camel_case(s)
            }
            _ => {
                let generator = TypeScriptBindingsGenerator::new();
                generator.to_camel_case(s)
            }
        }
    }

    pub fn collect_referenced_types(&self, rust_type: &str, used_types: &mut std::collections::HashSet<String>) {
        match self.validation_library.as_str() {
            "zod" => {
                let generator = ZodBindingsGenerator::new();
                generator.collect_referenced_types(rust_type, used_types)
            }
            _ => {
                let generator = TypeScriptBindingsGenerator::new();
                generator.collect_referenced_types(rust_type, used_types)
            }
        }
    }

    pub fn typescript_to_zod_type(&self, ts_type: &str) -> String {
        let generator = ZodBindingsGenerator::new();
        generator.typescript_to_zod_type(ts_type)
    }

    pub fn typescript_to_yup_type(&self, ts_type: &str) -> String {
        // Yup support removed
        format!("yup.mixed() /* {} - yup support removed */", ts_type)
    }

    pub fn is_custom_type(&self, ts_type: &str) -> bool {
        match self.validation_library.as_str() {
            "zod" => {
                let generator = ZodBindingsGenerator::new();
                generator.is_custom_type(ts_type)
            }
            _ => {
                let generator = TypeScriptBindingsGenerator::new();
                generator.is_custom_type(ts_type)
            }
        }
    }
}
