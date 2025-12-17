use std::collections::HashMap;
use std::fs;
use tauri_typegen::analysis::CommandAnalyzer;
use tauri_typegen::generators::create_generator;
use tempfile::TempDir;

#[test]
fn test_backward_compatibility_default_camel_case() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");

    // Command WITHOUT any serde attributes
    fs::write(
        &test_file,
        r#"
        #[tauri::command]
        pub fn update_order(
            order_id: String,
            new_status: String,
        ) -> Result<String, String> {
            Ok("Updated".to_string())
        }
    "#,
    )
    .unwrap();

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let output_dir = TempDir::new().unwrap();
    let mut generator = create_generator(Some("none".to_string()));
    generator
        .generate_models(
            &commands,
            &HashMap::new(),
            output_dir.path().to_str().unwrap(),
            &analyzer,
            &tauri_typegen::GenerateConfig::default(),
        )
        .unwrap();

    let types_content = fs::read_to_string(output_dir.path().join("types.ts")).unwrap();

    // Should apply camelCase by default (backward compatibility)
    assert!(
        types_content.contains("orderId: string"),
        "Default should be camelCase for backward compatibility. Found:\n{}",
        types_content
    );
    assert!(
        types_content.contains("newStatus: string"),
        "Default should be camelCase for backward compatibility. Found:\n{}",
        types_content
    );
}
