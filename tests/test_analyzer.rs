use std::fs;
use std::path::Path;
use tauri_typegen::analysis::CommandAnalyzer;
use tempfile::TempDir;

fn create_test_project(files: &[(&str, &str)]) -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    for (file_path, content) in files {
        let full_path = temp_dir.path().join(file_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full_path, content).unwrap();
    }

    temp_dir
}

#[test]
fn test_analyzer_finds_commands_in_sample_file() {
    let mut analyzer = CommandAnalyzer::new();
    let fixture_path = Path::new("tests/fixtures/sample_commands.rs");

    let commands = analyzer.analyze_file(fixture_path).unwrap();

    // Should find 5 commands from sample_commands.rs
    assert_eq!(commands.len(), 5);

    // Check specific commands
    let command_names: Vec<String> = commands.iter().map(|c| c.name.clone()).collect();
    assert!(command_names.contains(&"create_user".to_string()));
    assert!(command_names.contains(&"get_users".to_string()));
    assert!(command_names.contains(&"get_server_info".to_string()));
    assert!(command_names.contains(&"calculate_sum".to_string()));
    assert!(command_names.contains(&"process_data".to_string()));

    // Should NOT find internal_helper (no #[command] attribute)
    assert!(!command_names.contains(&"internal_helper".to_string()));
}

#[test]
fn test_analyzer_extracts_command_details() {
    let mut analyzer = CommandAnalyzer::new();
    let fixture_path = Path::new("tests/fixtures/sample_commands.rs");

    let commands = analyzer.analyze_file(fixture_path).unwrap();

    // Find the create_user command
    let create_user = commands
        .iter()
        .find(|c| c.name == "create_user")
        .expect("create_user command should be found");

    assert!(create_user.is_async);
    assert_eq!(create_user.parameters.len(), 1);
    assert_eq!(create_user.parameters[0].name, "request");
    assert_eq!(create_user.parameters[0].rust_type, "CreateUserRequest");
    assert_eq!(create_user.return_type, "Result<User, String>");
    assert_eq!(create_user.return_type_ts, "User");

    // Find the calculate_sum command
    let calculate_sum = commands
        .iter()
        .find(|c| c.name == "calculate_sum")
        .expect("calculate_sum command should be found");

    assert!(!calculate_sum.is_async);
    assert_eq!(calculate_sum.parameters.len(), 2);
    assert_eq!(calculate_sum.parameters[0].name, "a");
    assert_eq!(calculate_sum.parameters[0].typescript_type, "number");
    assert_eq!(calculate_sum.parameters[1].name, "b");
    assert_eq!(calculate_sum.parameters[1].typescript_type, "number");
    assert_eq!(calculate_sum.return_type, "i32");
    assert_eq!(calculate_sum.return_type_ts, "number");
}

#[test]
fn test_analyzer_handles_complex_parameters() {
    let mut analyzer = CommandAnalyzer::new();
    let fixture_path = Path::new("tests/fixtures/sample_commands.rs");

    let commands = analyzer.analyze_file(fixture_path).unwrap();

    // Find the get_users command
    let get_users = commands
        .iter()
        .find(|c| c.name == "get_users")
        .expect("get_users command should be found");

    assert_eq!(get_users.parameters.len(), 3);

    // Check limit parameter
    assert_eq!(get_users.parameters[0].name, "limit");
    assert_eq!(get_users.parameters[0].typescript_type, "number");
    assert!(!get_users.parameters[0].is_optional);

    // Check offset parameter (Optional)
    assert_eq!(get_users.parameters[1].name, "offset");
    assert_eq!(get_users.parameters[1].typescript_type, "number | null");
    assert!(get_users.parameters[1].is_optional);

    // Check search parameter
    assert_eq!(get_users.parameters[2].name, "search");
    assert_eq!(get_users.parameters[2].typescript_type, "string");
    assert!(!get_users.parameters[2].is_optional);
}

#[test]
fn test_analyzer_handles_no_parameters() {
    let mut analyzer = CommandAnalyzer::new();
    let fixture_path = Path::new("tests/fixtures/sample_commands.rs");

    let commands = analyzer.analyze_file(fixture_path).unwrap();

    // Find the get_server_info command (no parameters)
    let get_server_info = commands
        .iter()
        .find(|c| c.name == "get_server_info")
        .expect("get_server_info command should be found");

    assert_eq!(get_server_info.parameters.len(), 0);
    assert_eq!(
        get_server_info.return_type,
        "Result<SimpleResponse, String>"
    );
    assert_eq!(get_server_info.return_type_ts, "SimpleResponse");
}

#[test]
fn test_analyzer_handles_empty_file() {
    let mut analyzer = CommandAnalyzer::new();
    let fixture_path = Path::new("tests/fixtures/empty_file.rs");

    let commands = analyzer.analyze_file(fixture_path).unwrap();
    assert_eq!(commands.len(), 0);
}

#[test]
fn test_analyzer_handles_invalid_syntax() {
    let mut analyzer = CommandAnalyzer::new();
    let fixture_path = Path::new("tests/fixtures/invalid_syntax.rs");

    // Should handle syntax errors gracefully and return empty results
    let result = analyzer.analyze_file(fixture_path);
    assert!(result.is_ok()); // Now returns Ok(vec![]) instead of Err
    assert_eq!(result.unwrap().len(), 0);
}

#[test]
fn test_analyzer_project_analysis() {
    let temp_dir = create_test_project(&[
        (
            "src/commands.rs",
            r#"
            use tauri::command;
            
            #[command]
            pub fn test_command(value: String) -> String {
                value
            }
        "#,
        ),
        (
            "src/lib.rs",
            r#"
            pub mod commands;
        "#,
        ),
        (
            "src/other/nested.rs",
            r#"
            use tauri::command;
            
            #[command]
            pub fn nested_command() -> i32 {
                42
            }
        "#,
        ),
        ("ignored.txt", "This should be ignored"),
    ]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    // Should find commands from both .rs files
    assert_eq!(commands.len(), 2);

    let command_names: Vec<String> = commands.iter().map(|c| c.name.clone()).collect();
    assert!(command_names.contains(&"test_command".to_string()));
    assert!(command_names.contains(&"nested_command".to_string()));
}

#[test]
fn test_type_mappings() {
    let mut analyzer = CommandAnalyzer::new();

    // Test basic type mappings
    assert_eq!(analyzer.map_rust_type_to_typescript("String"), "string");
    assert_eq!(analyzer.map_rust_type_to_typescript("str"), "string");
    assert_eq!(analyzer.map_rust_type_to_typescript("i32"), "number");
    assert_eq!(analyzer.map_rust_type_to_typescript("f64"), "number");
    assert_eq!(analyzer.map_rust_type_to_typescript("bool"), "boolean");

    // Test Option types
    assert_eq!(
        analyzer.map_rust_type_to_typescript("Option<String>"),
        "string | null"
    );
    assert_eq!(
        analyzer.map_rust_type_to_typescript("Option<i32>"),
        "number | null"
    );

    // Test Vec types
    assert_eq!(
        analyzer.map_rust_type_to_typescript("Vec<String>"),
        "string[]"
    );
    assert_eq!(analyzer.map_rust_type_to_typescript("Vec<User>"), "User[]");

    // Test Result types
    assert_eq!(
        analyzer.map_rust_type_to_typescript("Result<User, String>"),
        "User"
    );
    assert_eq!(
        analyzer.map_rust_type_to_typescript("Result<Vec<User>, String>"),
        "User[]"
    );

    // Test references
    assert_eq!(analyzer.map_rust_type_to_typescript("&str"), "string");
    assert_eq!(analyzer.map_rust_type_to_typescript("&String"), "string");

    // Test custom types
    assert_eq!(
        analyzer.map_rust_type_to_typescript("CustomType"),
        "CustomType"
    );
}

#[test]
fn test_analyzer_skips_tauri_parameters() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{command, AppHandle, Runtime, Window};
            
            #[tauri::command]
            pub async fn command_with_tauri_params<R: Runtime>(
                app: AppHandle<R>,
                window: Window<R>,
                user_param: String,
            ) -> String {
                user_param
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    let command = &commands[0];

    // Should only have user_param, not app or window
    assert_eq!(command.parameters.len(), 1);
    assert_eq!(command.parameters[0].name, "user_param");
    assert_eq!(command.parameters[0].typescript_type, "string");
}

#[test]
fn test_analyzer_nonexistent_path() {
    let mut analyzer = CommandAnalyzer::new();
    let result = analyzer.analyze_project("/nonexistent/path");

    // Should handle nonexistent paths gracefully
    assert!(result.is_err());
}

#[test]
fn test_analyzer_file_path_tracking() {
    let temp_dir = create_test_project(&[(
        "src/commands.rs",
        r#"
            use tauri::command;
            
            #[command]
            pub fn test_command() -> String {
                "test".to_string()
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    let command = &commands[0];

    // Should track the file path
    assert!(command.file_path.ends_with("src/commands.rs"));
}
