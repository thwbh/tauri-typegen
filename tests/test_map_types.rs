use std::fs;
use tempfile::TempDir;
use tauri_plugin_typegen::analysis::CommandAnalyzer;
use tauri_plugin_typegen::generators::generator::BindingsGenerator;

#[test]
fn test_map_types_generation() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let output_dir = temp_dir.path().join("generated");
    fs::create_dir_all(&src_dir).unwrap();

    // Create a test file with HashMap and BTreeMap types
    let test_content = r#"
use serde::{Deserialize, Serialize};
use tauri::command;
use std::collections::{HashMap, BTreeMap};

#[derive(Debug, Deserialize, Serialize)]
pub struct DataStore {
    pub string_map: HashMap<String, String>,
    pub number_map: HashMap<String, i32>,
    pub tree_map: BTreeMap<String, f64>,
}

#[command]
pub async fn store_data(data: DataStore) -> Result<String, String> {
    Ok("Data stored".to_string())
}

#[command]
pub async fn get_user_preferences() -> Result<HashMap<String, String>, String> {
    Ok(HashMap::new())
}

#[command]  
pub async fn get_config() -> Result<BTreeMap<String, i32>, String> {
    Ok(BTreeMap::new())
}
"#;

    fs::write(src_dir.join("lib.rs"), test_content).unwrap();

    // Test vanilla TypeScript generation
    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer.analyze_project(temp_dir.path().to_str().unwrap()).unwrap();
    let discovered_structs = analyzer.get_discovered_structs();

    let mut generator = BindingsGenerator::new(None); // vanilla TypeScript
    generator.generate_models(&commands, discovered_structs, output_dir.to_str().unwrap(), &CommandAnalyzer::new()).unwrap();

    let types_content = fs::read_to_string(output_dir.join("types.ts")).unwrap();
    
    println!("Vanilla TypeScript types content:\n{}", types_content);
    
    // Should use Map<K, V> instead of Record<K, V>
    assert!(types_content.contains("Map<string, string>"));
    assert!(types_content.contains("Map<string, number>"));
    
    // Should NOT use Record anymore
    assert!(!types_content.contains("Record<string, string>"));
    
    // Test Zod generation
    let output_dir_zod = temp_dir.path().join("generated_zod");
    let mut generator_zod = BindingsGenerator::new(Some("zod".to_string()));
    generator_zod.generate_models(&commands, discovered_structs, output_dir_zod.to_str().unwrap(), &CommandAnalyzer::new()).unwrap();

    let types_content_zod = fs::read_to_string(output_dir_zod.join("types.ts")).unwrap();
    
    println!("Zod types content:\n{}", types_content_zod);
    
    // For Zod, z.record() should be used
    assert!(types_content_zod.contains("z.record(z.string())"));
}