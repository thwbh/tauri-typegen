use crate::generators::{VanillaTypeScriptGenerator, ZodGenerator};
use crate::models::{CommandInfo, StructInfo};
use std::collections::HashMap;

pub struct TypeScriptGenerator {
    validation_library: String,
}

impl TypeScriptGenerator {
    pub fn new(validation_library: Option<String>) -> Self {
        Self {
            validation_library: validation_library.unwrap_or_else(|| "zod".to_string()),
        }
    }

    pub fn generate_models(
        &mut self,
        commands: &[CommandInfo],
        discovered_structs: &HashMap<String, StructInfo>,
        output_path: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        match self.validation_library.as_str() {
            "zod" => {
                let mut generator = ZodGenerator::new();
                generator.generate_models(commands, discovered_structs, output_path)
            }
            "none" => {
                let mut generator = VanillaTypeScriptGenerator::new();
                generator.generate_models(commands, discovered_structs, output_path)
            }
            _ => {
                // For other validation libraries, fall back to vanilla
                let mut generator = VanillaTypeScriptGenerator::new();
                generator.generate_models(commands, discovered_structs, output_path)
            }
        }
    }

    // Helper methods for testing - delegate to appropriate generator
    pub fn to_pascal_case(&self, s: &str) -> String {
        match self.validation_library.as_str() {
            "zod" => {
                let generator = ZodGenerator::new();
                generator.to_pascal_case(s)
            }
            _ => {
                let generator = VanillaTypeScriptGenerator::new();
                generator.to_pascal_case(s)
            }
        }
    }

    pub fn to_camel_case(&self, s: &str) -> String {
        match self.validation_library.as_str() {
            "zod" => {
                let generator = ZodGenerator::new();
                generator.to_camel_case(s)
            }
            _ => {
                let generator = VanillaTypeScriptGenerator::new();
                generator.to_camel_case(s)
            }
        }
    }

    pub fn collect_referenced_types(&self, rust_type: &str, used_types: &mut std::collections::HashSet<String>) {
        match self.validation_library.as_str() {
            "zod" => {
                let generator = ZodGenerator::new();
                generator.collect_referenced_types(rust_type, used_types)
            }
            _ => {
                let generator = VanillaTypeScriptGenerator::new();
                generator.collect_referenced_types(rust_type, used_types)
            }
        }
    }

    pub fn typescript_to_zod_type(&self, ts_type: &str) -> String {
        let generator = ZodGenerator::new();
        generator.typescript_to_zod_type(ts_type)
    }

    pub fn typescript_to_yup_type(&self, ts_type: &str) -> String {
        // Yup support removed
        format!("yup.mixed() /* {} - yup support removed */", ts_type)
    }

    pub fn is_custom_type(&self, ts_type: &str) -> bool {
        match self.validation_library.as_str() {
            "zod" => {
                let generator = ZodGenerator::new();
                generator.is_custom_type(ts_type)
            }
            _ => {
                let generator = VanillaTypeScriptGenerator::new();
                generator.is_custom_type(ts_type)
            }
        }
    }
}
