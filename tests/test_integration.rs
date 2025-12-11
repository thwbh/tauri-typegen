use serial_test::serial;
use std::fs;
use tauri_typegen::analysis::CommandAnalyzer;
use tauri_typegen::generators::generator::BindingsGenerator;
use tempfile::TempDir;

fn create_complex_test_project() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // Create a complex project structure with multiple files and command types
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(src_dir.join("modules")).unwrap();

    // Main commands file
    let main_commands = r#"
use serde::{Deserialize, Serialize};
use tauri::command;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub created_at: String,
    pub is_active: bool,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    #[validate(length(min = 1, max = 50))]
    pub name: String,
    #[validate(email)]
    pub email: String,
    pub department: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFilter {
    pub search_term: Option<String>,
    pub department: Option<String>,
    pub is_active: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[command]
pub async fn create_user(request: CreateUserRequest) -> Result<User, String> {
    Ok(User {
        id: 1,
        name: request.name,
        email: request.email,
        created_at: "2023-01-01T00:00:00Z".to_string(),
        is_active: true,
    })
}

#[command]
pub async fn get_users(filter: Option<UserFilter>) -> Result<Vec<User>, String> {
    Ok(vec![])
}

#[command]
pub async fn update_user(id: i32, request: CreateUserRequest) -> Result<User, String> {
    Ok(User {
        id,
        name: request.name,
        email: request.email,
        created_at: "2023-01-01T00:00:00Z".to_string(),
        is_active: true,
    })
}

#[command]
pub async fn delete_user(id: i32) -> Result<(), String> {
    Ok(())
}

#[command]
pub fn get_user_count() -> Result<i32, String> {
    Ok(42)
}
    "#;

    fs::write(src_dir.join("user_commands.rs"), main_commands).unwrap();

    // Additional module with more commands
    let analytics_commands = r#"
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyticsData {
    pub total_users: i32,
    pub active_users: i32,
    pub new_users_today: i32,
    pub retention_rate: f64,
}

#[derive(Debug, Deserialize)]
pub struct DateRange {
    pub start_date: String,
    pub end_date: String,
}

#[command]
pub async fn get_analytics(date_range: Option<DateRange>) -> Result<AnalyticsData, String> {
    Ok(AnalyticsData {
        total_users: 1000,
        active_users: 800,
        new_users_today: 50,
        retention_rate: 0.85,
    })
}

#[command]
pub fn export_data(format: String, include_inactive: bool) -> Result<String, String> {
    Ok(format!("Exported data in {} format", format))
}
    "#;

    fs::write(
        src_dir.join("modules").join("analytics.rs"),
        analytics_commands,
    )
    .unwrap();

    // Utility commands
    let utils_commands = r#"
use tauri::command;

#[command]
pub fn calculate_hash(input: String) -> String {
    format!("hash_{}", input)
}

#[command]
pub async fn validate_email(email: String) -> Result<bool, String> {
    Ok(email.contains('@'))
}

// This should NOT be detected - no #[command] attribute
pub fn internal_utility(data: Vec<u8>) -> String {
    "internal".to_string()
}
    "#;

    fs::write(src_dir.join("utils.rs"), utils_commands).unwrap();

    // Lib file
    let lib_content = r#"
pub mod user_commands;
pub mod modules;
pub mod utils;
    "#;

    fs::write(src_dir.join("lib.rs"), lib_content).unwrap();

    // Module index
    fs::write(src_dir.join("modules").join("mod.rs"), "pub mod analytics;").unwrap();

    temp_dir
}

#[test]
#[serial]
fn test_full_pipeline_complex_project() {
    let temp_dir = create_complex_test_project();
    let project_path = temp_dir.path().to_str().unwrap();

    // Step 1: Analyze the project
    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer.analyze_project(project_path).unwrap();

    // Should find all commands from all files
    assert!(commands.len() >= 7); // At least 7 commands expected

    let command_names: Vec<String> = commands.iter().map(|c| c.name.clone()).collect();

    // Verify specific commands are found
    assert!(command_names.contains(&"create_user".to_string()));
    assert!(command_names.contains(&"get_users".to_string()));
    assert!(command_names.contains(&"update_user".to_string()));
    assert!(command_names.contains(&"delete_user".to_string()));
    assert!(command_names.contains(&"get_user_count".to_string()));
    assert!(command_names.contains(&"get_analytics".to_string()));
    assert!(command_names.contains(&"export_data".to_string()));
    assert!(command_names.contains(&"calculate_hash".to_string()));
    assert!(command_names.contains(&"validate_email".to_string()));

    // Should NOT find internal functions
    assert!(!command_names.contains(&"internal_utility".to_string()));

    // Step 2: Generate TypeScript models
    let output_path = temp_dir.path().join("generated");
    let mut generator = BindingsGenerator::new(Some("zod".to_string()));

    let generated_files = generator
        .generate_models(
            &commands,
            analyzer.get_discovered_structs(),
            output_path.to_str().unwrap(),
            &analyzer,
        )
        .unwrap();

    assert_eq!(generated_files.len(), 3);
    assert!(generated_files.contains(&"types.ts".to_string()));
    assert!(generated_files.contains(&"commands.ts".to_string()));
    assert!(generated_files.contains(&"index.ts".to_string()));

    // Step 3: Verify generated content quality
    let types_content = fs::read_to_string(output_path.join("types.ts")).unwrap();
    let commands_content = fs::read_to_string(output_path.join("commands.ts")).unwrap();
    let index_content = fs::read_to_string(output_path.join("index.ts")).unwrap();

    // Verify types.ts - should have zod schemas and inferred types
    assert!(types_content.contains("export type CreateUserParams"));
    assert!(types_content.contains("export type UpdateUserParams"));
    assert!(types_content.contains("export type GetUsersParams"));
    assert!(types_content.contains("export type ExportDataParams"));

    // Verify zod schemas are generated for parameters
    assert!(types_content.contains("= z.object({"));
    assert!(types_content.contains("z.string()") || types_content.contains("z.coerce.number()"));
    // Verify custom type schemas are generated (zod format)
    assert!(types_content.contains("UserSchema") || types_content.contains("export type User"));
    assert!(
        types_content.contains("CreateUserRequestSchema")
            || types_content.contains("export type CreateUserRequest")
    );
    assert!(
        types_content.contains("AnalyticsDataSchema")
            || types_content.contains("export type AnalyticsData")
    );

    // Verify schemas are embedded in types.ts (Zod)
    assert!(types_content.contains("import { z } from 'zod';"));
    assert!(types_content.contains("CreateUserParamsSchema"));
    assert!(types_content.contains("z.object({"));
    // Verify that parameter schemas exist (flexible check)
    assert!(types_content.contains("ExportDataParamsSchema"));
    assert!(
        types_content.contains("z.string()")
            || types_content.contains("z.coerce.number()")
            || types_content.contains("z.boolean()")
    );

    // Verify commands.ts
    assert!(commands_content.contains("import { invoke } from '@tauri-apps/api/core';"));
    assert!(commands_content.contains("import * as types from './types';"));

    // Check specific command functions
    assert!(commands_content.contains("export async function createUser"));
    assert!(commands_content.contains("params: types.CreateUserParams"));
    assert!(commands_content.contains("Promise<types.User>"));
    assert!(commands_content.contains("types.CreateUserParamsSchema.safeParse(params)"));
    assert!(commands_content.contains("invoke") && commands_content.contains("'create_user'"));

    assert!(commands_content.contains("export async function getUsers"));
    assert!(commands_content.contains("export async function deleteUser"));
    assert!(commands_content.contains("export async function getUserCount"));
    // Check for void return type handling
    assert!(
        commands_content.contains("Promise<void>") || commands_content.contains("Promise<unknown>")
    );

    // Check that function without parameters works correctly
    assert!(commands_content.contains("export async function getUserCount"));
    assert!(commands_content.contains("invoke") && commands_content.contains("'get_user_count'"));

    // Verify index.ts (zod generator doesn't have separate schemas file)
    assert!(index_content.contains("export * from './types';"));
    assert!(index_content.contains("export * from './commands';"));
    // Schemas are embedded in types.ts for zod generator
}

#[test]
#[serial]
fn test_full_pipeline_with_yup() {
    let temp_dir = create_complex_test_project();
    let project_path = temp_dir.path().to_str().unwrap();

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer.analyze_project(project_path).unwrap();

    let output_path = temp_dir.path().join("generated_yup");
    let mut generator = BindingsGenerator::new(Some("yup".to_string()));

    let generated_files = generator
        .generate_models(
            &commands,
            analyzer.get_discovered_structs(),
            output_path.to_str().unwrap(),
            &analyzer,
        )
        .unwrap();

    assert_eq!(generated_files.len(), 3);

    let types_content = fs::read_to_string(output_path.join("types.ts")).unwrap();

    // Yup support has been removed, should fall back to vanilla TypeScript
    assert!(!types_content.contains("import * as yup from 'yup';"));
    assert!(!types_content.contains("yup.object({"));
    assert!(!types_content.contains("yup.string()"));
    assert!(!types_content.contains("z.string()"));
}

#[test]
#[serial]
fn test_full_pipeline_without_validation() {
    let temp_dir = create_complex_test_project();
    let project_path = temp_dir.path().to_str().unwrap();

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer.analyze_project(project_path).unwrap();

    let output_path = temp_dir.path().join("generated_no_validation");
    let mut generator = BindingsGenerator::new(Some("none".to_string()));

    let generated_files = generator
        .generate_models(
            &commands,
            analyzer.get_discovered_structs(),
            output_path.to_str().unwrap(),
            &analyzer,
        )
        .unwrap();

    // Should generate 3 files (no schemas.ts)
    assert_eq!(generated_files.len(), 3);
    assert!(!generated_files.contains(&"schemas.ts".to_string()));
    assert!(!output_path.join("schemas.ts").exists());

    let commands_content = fs::read_to_string(output_path.join("commands.ts")).unwrap();

    // Should not import schemas
    assert!(!commands_content.contains("import * as schemas"));
    assert!(commands_content.contains("return invoke('create_user', params);"));
    assert!(!commands_content.contains("parse(params)"));
}

#[test]
#[serial]
fn test_type_mapping_accuracy() {
    let temp_dir = create_complex_test_project();
    let project_path = temp_dir.path().to_str().unwrap();

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer.analyze_project(project_path).unwrap();

    // Find create_user command and verify parameter types
    let create_user = commands
        .iter()
        .find(|c| c.name == "create_user")
        .expect("create_user should be found");

    assert_eq!(create_user.parameters.len(), 1);
    let request_param = &create_user.parameters[0];
    assert_eq!(request_param.name, "request");
    assert_eq!(request_param.rust_type, "CreateUserRequest");
    assert_eq!(request_param.typescript_type(), "CreateUserRequest");

    // Find get_users command and verify optional parameter
    let get_users = commands
        .iter()
        .find(|c| c.name == "get_users")
        .expect("get_users should be found");

    assert_eq!(get_users.parameters.len(), 1);
    let filter_param = &get_users.parameters[0];
    assert_eq!(filter_param.name, "filter");
    assert!(filter_param.is_optional);
    assert!(filter_param.typescript_type().contains("| null"));

    // Find export_data command and verify multiple parameters
    let export_data = commands
        .iter()
        .find(|c| c.name == "export_data")
        .expect("export_data should be found");

    assert_eq!(export_data.parameters.len(), 2);
    assert_eq!(export_data.parameters[0].name, "format");
    assert_eq!(export_data.parameters[0].typescript_type(), "string");
    assert_eq!(export_data.parameters[1].name, "include_inactive");
    assert_eq!(export_data.parameters[1].typescript_type(), "boolean");
}

#[test]
#[serial]
fn test_generated_content_syntax_valid() {
    let temp_dir = create_complex_test_project();
    let project_path = temp_dir.path().to_str().unwrap();

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer.analyze_project(project_path).unwrap();

    let output_path = temp_dir.path().join("generated");
    let mut generator = BindingsGenerator::new(Some("zod".to_string()));

    generator
        .generate_models(
            &commands,
            analyzer.get_discovered_structs(),
            output_path.to_str().unwrap(),
            &analyzer,
        )
        .unwrap();

    let types_content = fs::read_to_string(output_path.join("types.ts")).unwrap();
    let commands_content = fs::read_to_string(output_path.join("commands.ts")).unwrap();

    // Basic syntax validation - check for proper TypeScript syntax (zod types use 'export type')
    assert!(types_content.contains("export type") || types_content.contains("export const"));
    assert!(types_content.matches('{').count() == types_content.matches('}').count());

    assert!(types_content.contains("export const"));
    assert!(types_content.contains("= z.object"));

    assert!(commands_content.contains("export async function"));
    assert!(commands_content.contains("invoke(") || commands_content.contains("invoke<"));

    // Check that all functions return Promise types for async consistency
    let async_functions = commands_content.matches("export async function").count();
    let promise_returns = commands_content.matches("Promise<").count();
    assert_eq!(async_functions, promise_returns);
}

#[test]
#[serial]
fn test_file_path_tracking_accuracy() {
    let temp_dir = create_complex_test_project();
    let project_path = temp_dir.path().to_str().unwrap();

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer.analyze_project(project_path).unwrap();

    // Verify file paths are tracked correctly
    let create_user = commands
        .iter()
        .find(|c| c.name == "create_user")
        .expect("create_user should be found");
    assert!(create_user.file_path.ends_with("user_commands.rs"));

    let get_analytics = commands
        .iter()
        .find(|c| c.name == "get_analytics")
        .expect("get_analytics should be found");
    assert!(
        get_analytics.file_path.contains("modules")
            && get_analytics.file_path.ends_with("analytics.rs")
    );

    let calculate_hash = commands
        .iter()
        .find(|c| c.name == "calculate_hash")
        .expect("calculate_hash should be found");
    assert!(calculate_hash.file_path.ends_with("utils.rs"));
}

#[test]
#[serial]
fn test_async_detection_accuracy() {
    let temp_dir = create_complex_test_project();
    let project_path = temp_dir.path().to_str().unwrap();

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer.analyze_project(project_path).unwrap();

    // Verify async detection
    let create_user = commands.iter().find(|c| c.name == "create_user").unwrap();
    assert!(create_user.is_async);

    let get_user_count = commands
        .iter()
        .find(|c| c.name == "get_user_count")
        .unwrap();
    assert!(!get_user_count.is_async);

    let validate_email = commands
        .iter()
        .find(|c| c.name == "validate_email")
        .unwrap();
    assert!(validate_email.is_async);

    let calculate_hash = commands
        .iter()
        .find(|c| c.name == "calculate_hash")
        .unwrap();
    assert!(!calculate_hash.is_async);
}
