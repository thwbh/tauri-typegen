use crate::cli::{GenerateConfig, generate_from_config};
use std::path::Path;

/// Build-time code generation function that can be called from build.rs
pub fn generate_at_build_time() -> Result<(), Box<dyn std::error::Error>> {
    // Check if we're in a Tauri project
    let tauri_conf = Path::new("tauri.conf.json");
    let src_tauri = Path::new("src-tauri");
    
    if !tauri_conf.exists() && !src_tauri.exists() {
        // Not in a Tauri project, skip generation
        return Ok(());
    }
    
    // Try to find tauri.conf.json to read configuration
    let config = if tauri_conf.exists() {
        read_config_from_tauri_conf(tauri_conf)?
    } else {
        // Use defaults
        GenerateConfig::default()
    };
    
    println!("cargo:rerun-if-changed={}", config.project_path);
    
    match generate_from_config(&config) {
        Ok(files) => {
            if !files.is_empty() {
                println!("Generated {} TypeScript files during build", files.len());
            }
        }
        Err(e) => {
            println!("cargo:warning=Failed to generate TypeScript bindings: {}", e);
        }
    }
    
    Ok(())
}

fn read_config_from_tauri_conf(path: &Path) -> Result<GenerateConfig, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let tauri_config: serde_json::Value = serde_json::from_str(&content)?;
    
    // Look for typegen plugin configuration
    let mut config = GenerateConfig::default();
    
    if let Some(plugins) = tauri_config.get("plugins") {
        if let Some(typegen) = plugins.get("typegen") {
            if let Some(output_path) = typegen.get("outputPath").and_then(|v| v.as_str()) {
                config.output_path = output_path.to_string();
            }
            if let Some(validation) = typegen.get("validationLibrary").and_then(|v| v.as_str()) {
                config.validation_library = validation.to_string();
            }
        }
    }
    
    Ok(config)
}