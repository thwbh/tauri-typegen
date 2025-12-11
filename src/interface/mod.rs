pub mod cli;
pub mod config;
pub mod output;

use crate::analysis::CommandAnalyzer;
use crate::generators::generator::BindingsGenerator;

pub use cli::*;
pub use config::*;
pub use output::*;

/// Generate TypeScript bindings from configuration (backward compatibility function)
pub fn generate_from_config(
    config: &config::GenerateConfig,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let logger = output::Logger::new(config.is_verbose(), false);

    if config.is_verbose() {
        logger.info(&format!(
            "🔍 Analyzing Tauri commands in: {}",
            config.project_path
        ));
    }

    // Validate configuration
    config
        .validate()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    // Analyze commands with struct discovery
    let mut analyzer = CommandAnalyzer::new();

    // Apply custom type mappings from configuration
    if let Some(ref mappings) = config.type_mappings {
        analyzer.add_type_mappings(mappings);
        if config.is_verbose() {
            logger.info(&format!(
                "📝 Applied {} custom type mappings",
                mappings.len()
            ));
            for (rust_type, ts_type) in mappings {
                logger.verbose(&format!("  {} → {}", rust_type, ts_type));
            }
        }
    }

    let commands = analyzer.analyze_project(&config.project_path)?;

    if config.is_verbose() {
        logger.info(&format!("📋 Found {} Tauri commands:", commands.len()));
        for cmd in &commands {
            logger.verbose(&format!("  - {} ({})", cmd.name, cmd.file_path));
        }

        let discovered_structs = analyzer.get_discovered_structs();
        logger.info(&format!(
            "🏗️  Found {} struct definitions:",
            discovered_structs.len()
        ));
        for (name, struct_info) in discovered_structs {
            let struct_type = if struct_info.is_enum {
                "enum"
            } else {
                "struct"
            };
            logger.verbose(&format!(
                "  - {} ({}) with {} fields",
                name,
                struct_type,
                struct_info.fields.len()
            ));
            for field in &struct_info.fields {
                let visibility = if field.is_public { "pub" } else { "private" };
                let optional = if field.is_optional { "?" } else { "" };
                logger.verbose(&format!(
                    "    • {}{}: {} ({})",
                    field.name, optional, field.rust_type, visibility
                ));
            }
        }

        if discovered_structs.is_empty() {
            logger.info("  ℹ️  No custom struct definitions found in the project");
        }
    }

    if commands.is_empty() {
        if config.is_verbose() {
            logger.warning("⚠️  No Tauri commands found. Make sure your project contains functions with #[tauri::command] attributes.");
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

    if config.is_verbose() {
        logger.info(&format!(
            "🚀 Generating TypeScript models with {} validation...",
            validation.as_ref().unwrap()
        ));
    }

    // Generate TypeScript models with discovered structs
    let mut generator = BindingsGenerator::new(validation);
    let generated_files = generator.generate_models(
        &commands,
        analyzer.get_discovered_structs(),
        &config.output_path,
        &analyzer,
    )?;

    if config.is_verbose() {
        logger.info(&format!(
            "✅ Successfully generated {} files for {} commands:",
            generated_files.len(),
            commands.len()
        ));
        for file in &generated_files {
            logger.verbose(&format!("  📄 {}/{}", config.output_path, file));
        }
    }

    Ok(generated_files)
}
