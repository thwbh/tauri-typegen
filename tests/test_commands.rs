use std::fs;
use tauri_plugin_typegen::commands::{analyze_commands, generate_models};
use tauri_plugin_typegen::models::*;
use tempfile::TempDir;
use tokio;

fn create_test_project_with_commands() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // Create a simple Rust project with Tauri commands
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let commands_content = r#"
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[command]
pub async fn create_user(request: CreateUserRequest) -> Result<User, String> {
    Ok(User {
        id: 1,
        name: request.name,
        email: request.email,
    })
}

#[command]
pub fn get_users() -> Result<Vec<User>, String> {
    Ok(vec![])
}

#[command]
pub fn calculate(a: i32, b: i32) -> i32 {
    a + b
}
    "#;

    fs::write(src_dir.join("commands.rs"), commands_content).unwrap();

    let lib_content = r#"
pub mod commands;
    "#;

    fs::write(src_dir.join("lib.rs"), lib_content).unwrap();

    temp_dir
}

#[tokio::test]
async fn test_analyze_commands_success() {
    let temp_dir = create_test_project_with_commands();
    let project_path = temp_dir.path().to_str().unwrap().to_string();

    let request = AnalyzeCommandsRequest { project_path };
    let result = analyze_commands(request).await;

    assert!(result.is_ok());
    let response = result.unwrap();

    // Should find 3 commands
    assert_eq!(response.commands.len(), 3);

    let command_names: Vec<String> = response.commands.iter().map(|c| c.name.clone()).collect();
    assert!(command_names.contains(&"create_user".to_string()));
    assert!(command_names.contains(&"get_users".to_string()));
    assert!(command_names.contains(&"calculate".to_string()));
}

#[tokio::test]
async fn test_analyze_commands_invalid_path() {
    let request = AnalyzeCommandsRequest {
        project_path: "/nonexistent/path".to_string(),
    };

    let result = analyze_commands(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_analyze_commands_empty_project() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().to_str().unwrap().to_string();

    // Create empty src directory
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();
    fs::write(temp_dir.path().join("src/lib.rs"), "// Empty file").unwrap();

    let request = AnalyzeCommandsRequest { project_path };
    let result = analyze_commands(request).await;

    assert!(result.is_ok());
    let response = result.unwrap();

    // Should find no commands
    assert_eq!(response.commands.len(), 0);
}

#[tokio::test]
async fn test_generate_models_success() {
    let temp_dir = create_test_project_with_commands();
    let project_path = temp_dir.path().to_str().unwrap().to_string();
    let output_path = temp_dir
        .path()
        .join("generated")
        .to_str()
        .unwrap()
        .to_string();

    let request = GenerateModelsRequest {
        project_path,
        output_path: Some(output_path.clone()),
        validation_library: Some("zod".to_string()),
    };

    let result = generate_models(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should generate files for found commands
    assert!(response.commands_found > 0);
    assert!(response.types_generated > 0);
    assert_eq!(response.generated_files.len(), 3); // types (with zod schemas), commands, index

    // Verify files were created
    let output_dir = temp_dir.path().join("generated");
    assert!(output_dir.join("types.ts").exists());
    // schemas.ts is not generated for zod - schemas are in types.ts
    assert!(output_dir.join("commands.ts").exists());
    assert!(output_dir.join("index.ts").exists());
}

#[tokio::test]
async fn test_generate_models_with_default_output_path() {
    let temp_dir = create_test_project_with_commands();
    let project_path = temp_dir.path().to_str().unwrap().to_string();

    let request = GenerateModelsRequest {
        project_path: project_path.clone(),
        output_path: None,
        validation_library: Some("zod".to_string()),
    };

    let result = generate_models(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.commands_found > 0);

    // Should use default output path: project_path/generated
    let default_output = temp_dir.path().join("generated");
    assert!(default_output.join("types.ts").exists());
}

#[tokio::test]
async fn test_generate_models_without_validation() {
    let temp_dir = create_test_project_with_commands();
    let project_path = temp_dir.path().to_str().unwrap().to_string();
    let output_path = temp_dir
        .path()
        .join("generated")
        .to_str()
        .unwrap()
        .to_string();

    let request = GenerateModelsRequest {
        project_path,
        output_path: Some(output_path),
        validation_library: Some("none".to_string()),
    };

    let result = generate_models(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should generate 3 files (no schemas.ts)
    assert_eq!(response.generated_files.len(), 3);
    assert!(response.generated_files.contains(&"types.ts".to_string()));
    assert!(response
        .generated_files
        .contains(&"commands.ts".to_string()));
    assert!(response.generated_files.contains(&"index.ts".to_string()));
    assert!(!response.generated_files.contains(&"schemas.ts".to_string()));
}

#[tokio::test]
async fn test_generate_models_yup_validation() {
    let temp_dir = create_test_project_with_commands();
    let project_path = temp_dir.path().to_str().unwrap().to_string();
    let output_path = temp_dir
        .path()
        .join("generated")
        .to_str()
        .unwrap()
        .to_string();

    let request = GenerateModelsRequest {
        project_path,
        output_path: Some(output_path.clone()),
        validation_library: Some("yup".to_string()),
    };

    let result = generate_models(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.generated_files.len(), 4);

    // Verify Yup schemas are generated
    let schemas_content = fs::read_to_string(temp_dir.path().join("generated/schemas.ts")).unwrap();
    assert!(schemas_content.contains("import * as yup from 'yup';"));
    assert!(schemas_content.contains("yup.object({"));
}

#[tokio::test]
async fn test_generate_models_invalid_project_path() {
    let request = GenerateModelsRequest {
        project_path: "/nonexistent/path".to_string(),
        output_path: None,
        validation_library: None,
    };

    let result = generate_models(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_generate_models_counts_accurate() {
    let temp_dir = create_test_project_with_commands();
    let project_path = temp_dir.path().to_str().unwrap().to_string();
    let output_path = temp_dir
        .path()
        .join("generated")
        .to_str()
        .unwrap()
        .to_string();

    let request = GenerateModelsRequest {
        project_path,
        output_path: Some(output_path),
        validation_library: Some("zod".to_string()),
    };

    let result = generate_models(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should accurately count commands and types
    assert_eq!(response.commands_found, 3); // create_user, get_users, calculate

    // types_generated should count parameters: create_user(1) + get_users(0) + calculate(2) = 3
    assert_eq!(response.types_generated, 3);
}

#[tokio::test]
async fn test_generate_models_creates_output_directory() {
    let temp_dir = create_test_project_with_commands();
    let project_path = temp_dir.path().to_str().unwrap().to_string();
    let output_path = temp_dir
        .path()
        .join("deep/nested/output")
        .to_str()
        .unwrap()
        .to_string();

    // Output directory doesn't exist initially
    assert!(!temp_dir.path().join("deep").exists());

    let request = GenerateModelsRequest {
        project_path,
        output_path: Some(output_path.clone()),
        validation_library: Some("zod".to_string()),
    };

    let result = generate_models(request).await;
    assert!(result.is_ok());

    // Should create the output directory
    assert!(temp_dir.path().join("deep/nested/output").exists());
    assert!(temp_dir.path().join("deep/nested/output/types.ts").exists());
}

#[tokio::test]
async fn test_command_detail_extraction() {
    let temp_dir = create_test_project_with_commands();
    let project_path = temp_dir.path().to_str().unwrap().to_string();

    let request = AnalyzeCommandsRequest { project_path };
    let result = analyze_commands(request).await;

    assert!(result.is_ok());
    let response = result.unwrap();

    // Find create_user command and verify details
    let create_user = response
        .commands
        .iter()
        .find(|c| c.name == "create_user")
        .expect("create_user command should be found");

    assert!(create_user.is_async);
    assert_eq!(create_user.parameters.len(), 1);
    assert_eq!(create_user.parameters[0].name, "request");
    assert_eq!(create_user.return_type, "User");
    assert!(create_user.file_path.ends_with("commands.rs"));

    // Find calculate command and verify details
    let calculate = response
        .commands
        .iter()
        .find(|c| c.name == "calculate")
        .expect("calculate command should be found");

    assert!(!calculate.is_async);
    assert_eq!(calculate.parameters.len(), 2);
    assert_eq!(calculate.parameters[0].name, "a");
    assert_eq!(calculate.parameters[0].typescript_type, "number");
    assert_eq!(calculate.parameters[1].name, "b");
    assert_eq!(calculate.parameters[1].typescript_type, "number");
    assert_eq!(calculate.return_type, "number");
}

#[tokio::test]
async fn test_generate_models_error_handling() {
    // Test with a project that has syntax errors
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create file with syntax error
    fs::write(src_dir.join("broken.rs"), "invalid rust syntax {{{").unwrap();

    let project_path = temp_dir.path().to_str().unwrap().to_string();
    let output_path = temp_dir
        .path()
        .join("generated")
        .to_str()
        .unwrap()
        .to_string();

    let request = GenerateModelsRequest {
        project_path,
        output_path: Some(output_path),
        validation_library: Some("zod".to_string()),
    };

    let result = generate_models(request).await;

    // Should handle syntax errors gracefully but still succeed for valid files
    // Since we only have a broken file, it should succeed with 0 commands
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.commands_found, 0);
}
