use std::fs;
use tauri_typegen::analysis::CommandAnalyzer;
use tauri_typegen::generators::create_generator;
use tauri_typegen::GenerateConfig;
use tempfile::TempDir;

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
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();
    let discovered_structs = analyzer.get_discovered_structs();

    let mut generator = create_generator(None); // vanilla TypeScript
    generator
        .generate_models(
            &commands,
            discovered_structs,
            output_dir.to_str().unwrap(),
            &CommandAnalyzer::new(),
            &GenerateConfig::new(),
        )
        .unwrap();

    let types_content = fs::read_to_string(output_dir.join("types.ts")).unwrap();

    println!("Vanilla TypeScript types content:\n{}", types_content);

    // Should use Record<K, V> for JSON-serializable objects with dynamic keys
    assert!(types_content.contains("Record<string, string>"));
    assert!(types_content.contains("Record<string, number>"));

    // Should NOT use Map (Map is not JSON-serializable)
    assert!(!types_content.contains("Map<string, string>"));

    // Test Zod generation
    let output_dir_zod = temp_dir.path().join("generated_zod");
    let mut generator_zod = create_generator(Some("zod".to_string()));
    generator_zod
        .generate_models(
            &commands,
            discovered_structs,
            output_dir_zod.to_str().unwrap(),
            &CommandAnalyzer::new(),
            &GenerateConfig::new(),
        )
        .unwrap();

    let types_content_zod = fs::read_to_string(output_dir_zod.join("types.ts")).unwrap();

    println!("Zod types content:\n{}", types_content_zod);

    // For Zod, z.record() should be used with explicit key and value types
    assert!(types_content_zod.contains("z.record(z.string(), z.string())"));
    assert!(types_content_zod.contains("z.record(z.string(), z.coerce.number())"));
}

#[test]
fn test_map_with_numeric_keys() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let output_dir = temp_dir.path().join("generated");
    fs::create_dir_all(&src_dir).unwrap();

    // Create a test file with HashMap using numeric keys
    let test_content = r#"
use serde::{Deserialize, Serialize};
use tauri::command;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WizardTargetDateResult {
    pub weight_by_rate: HashMap<i32, f32>,
    pub bmi_by_rate: HashMap<i32, f64>,
}

#[command]
pub async fn get_wizard_results() -> Result<WizardTargetDateResult, String> {
    Ok(WizardTargetDateResult {
        weight_by_rate: HashMap::new(),
        bmi_by_rate: HashMap::new(),
    })
}
"#;

    fs::write(src_dir.join("lib.rs"), test_content).unwrap();

    // Test Zod generation
    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();
    let discovered_structs = analyzer.get_discovered_structs();

    let mut generator_zod = create_generator(Some("zod".to_string()));
    generator_zod
        .generate_models(
            &commands,
            discovered_structs,
            output_dir.to_str().unwrap(),
            &CommandAnalyzer::new(),
            &GenerateConfig::new(),
        )
        .unwrap();

    let types_content = fs::read_to_string(output_dir.join("types.ts")).unwrap();

    println!("Generated Zod schema with numeric keys:\n{}", types_content);

    // CRITICAL: Keys must use z.number() NOT z.coerce.number()
    // z.coerce.number() has input type 'unknown' which violates Zod's record key constraint
    assert!(types_content.contains("z.record(z.number(), z.coerce.number())"));

    // Verify that z.coerce.number() is NOT used for keys
    assert!(!types_content.contains("z.record(z.coerce.number()"));
    assert!(types_content_zod.contains("z.record(z.string(), z.coerce.number())"));
}
