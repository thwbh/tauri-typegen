use std::fs;
use tauri_typegen::analysis::CommandAnalyzer;
use tempfile::TempDir;

#[test]
fn test_parameter_serde_rename_attribute() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");

    fs::write(
        &test_file,
        r#"
        use serde::Deserialize;
        
        #[tauri::command]
        pub fn update_order(
            #[serde(rename = "id")] order_id: String,
            status: String,
        ) -> Result<String, String> {
            Ok(format!("Order {} updated", order_id))
        }
    "#,
    )
    .unwrap();

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    let cmd = &commands[0];
    assert_eq!(cmd.parameters.len(), 2);

    // First parameter should have serde_rename = Some("id")
    assert_eq!(cmd.parameters[0].name, "order_id");
    assert_eq!(cmd.parameters[0].serde_rename, Some("id".to_string()));

    // Second parameter should have no serde_rename
    assert_eq!(cmd.parameters[1].name, "status");
    assert_eq!(cmd.parameters[1].serde_rename, None);
}
