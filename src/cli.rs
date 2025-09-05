use crate::analyzer::CommandAnalyzer;
use crate::generator::TypeScriptGenerator;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateConfig {
    pub project_path: String,
    pub output_path: String,
    pub validation_library: String,
    pub verbose: Option<bool>,
}

impl Default for GenerateConfig {
    fn default() -> Self {
        Self {
            project_path: "./src-tauri".to_string(),
            output_path: "./src/generated".to_string(),
            validation_library: "zod".to_string(),
            verbose: Some(false),
        }
    }
}

pub fn generate_from_config(
    config: &GenerateConfig,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let verbose = config.verbose.unwrap_or(false);

    if verbose {
        println!("🔍 Analyzing Tauri commands in: {}", config.project_path);
    }

    // Validate paths
    let project_path = Path::new(&config.project_path);
    if !project_path.exists() {
        return Err(format!("Project path does not exist: {}", config.project_path).into());
    }

    // Analyze commands with struct discovery
    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer.analyze_project(&config.project_path)?;

    if verbose {
        println!("📋 Found {} Tauri commands:", commands.len());
        for cmd in &commands {
            println!("  - {} ({})", cmd.name, cmd.file_path);
        }

        let discovered_structs = analyzer.get_discovered_structs();
        println!("🏗️  Found {} struct definitions:", discovered_structs.len());
        for (name, struct_info) in discovered_structs {
            let struct_type = if struct_info.is_enum {
                "enum"
            } else {
                "struct"
            };
            println!(
                "  - {} ({}) with {} fields",
                name,
                struct_type,
                struct_info.fields.len()
            );
            for field in &struct_info.fields {
                let visibility = if field.is_public { "pub" } else { "private" };
                let optional = if field.is_optional { "?" } else { "" };
                println!(
                    "    • {}{}: {} ({})",
                    field.name, optional, field.typescript_type, visibility
                );
            }
        }

        if discovered_structs.is_empty() {
            println!("  ℹ️  No custom struct definitions found in the project");
        }
    }

    if commands.is_empty() {
        if verbose {
            println!("⚠️  No Tauri commands found. Make sure your project contains functions with #[tauri::command] attributes.");
        }
        return Ok(vec![]);
    }

    // Validate validation library
    let validation = match config.validation_library.as_str() {
        "zod" | "none" => Some(config.validation_library.clone()),
        _ => {
            return Err("Invalid validation library. Use 'zod' or 'none'".into());
        }
    };

    if verbose {
        println!(
            "🚀 Generating TypeScript models with {} validation...",
            validation.as_ref().unwrap()
        );
    }

    // Generate TypeScript models with discovered structs
    let mut generator = TypeScriptGenerator::new(validation);
    let generated_files = generator.generate_models(
        &commands,
        analyzer.get_discovered_structs(),
        &config.output_path,
    )?;

    if verbose {
        println!(
            "✅ Successfully generated {} files for {} commands:",
            generated_files.len(),
            commands.len()
        );
        for file in &generated_files {
            println!("  📄 {}/{}", config.output_path, file);
        }
    }

    Ok(generated_files)
}
