use std::collections::HashMap;
use std::fs;
use tauri_typegen::analysis::CommandAnalyzer;
use tauri_typegen::generators::create_generator;
use tauri_typegen::GenerateConfig;
use tempfile::TempDir;

#[test]
fn test_command_rename_all_only_affects_parameters() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");

    // Command with rename_all on the function
    fs::write(
        &test_file,
        r#"
        use serde::Deserialize;
        
        #[tauri::command]
        #[serde(rename_all = "snake_case")]
        pub fn updateOrderStatus(
            order_id: String,
            new_status: String,
        ) -> Result<String, String> {
            Ok(format!("Updated"))
        }
    "#,
    )
    .unwrap();

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);

    // Generate TypeScript bindings
    let output_dir = TempDir::new().unwrap();
    let mut generator = create_generator(Some("none".to_string()));
    generator
        .generate_models(
            &commands,
            &HashMap::new(),
            output_dir.path().to_str().unwrap(),
            &analyzer,
            &GenerateConfig::default(),
        )
        .unwrap();

    let commands_content = fs::read_to_string(output_dir.path().join("commands.ts")).unwrap();

    // Function name should be camelCase (TypeScript convention), NOT snake_case
    assert!(
        commands_content.contains("export async function updateOrderStatus"),
        "Function name should use TypeScript camelCase convention, not serde rename_all"
    );

    // Parameter names should use snake_case (from rename_all)
    // Check in the Params interface
    let types_content = fs::read_to_string(output_dir.path().join("types.ts")).unwrap();
    assert!(
        types_content.contains("order_id: string"),
        "Parameters should use snake_case from command-level rename_all"
    );
    assert!(
        types_content.contains("new_status: string"),
        "Parameters should use snake_case from command-level rename_all"
    );
}
