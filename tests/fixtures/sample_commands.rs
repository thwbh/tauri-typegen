use serde::{Deserialize, Serialize};
use tauri::command;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    #[validate(length(min = 1, max = 50))]
    pub name: String,
    #[validate(email)]
    pub email: String,
    pub age: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub age: Option<i32>,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleResponse {
    pub message: String,
    pub count: i32,
}

// Basic command with parameters
#[command]
pub async fn create_user(request: CreateUserRequest) -> Result<User, String> {
    Ok(User {
        id: 1,
        name: request.name,
        email: request.email,
        age: request.age,
        is_active: true,
    })
}

// Command with multiple parameters
#[command]
pub fn get_users(limit: i32, offset: Option<i32>, search: String) -> Result<Vec<User>, String> {
    Ok(vec![])
}

// Command without parameters
#[command]
pub fn get_server_info() -> Result<SimpleResponse, String> {
    Ok(SimpleResponse {
        message: "Server running".to_string(),
        count: 42,
    })
}

// Synchronous command
#[command]
pub fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}

// Command with complex types
#[command]
pub async fn process_data(
    data: Vec<String>,
    options: Option<ProcessingOptions>,
) -> Result<ProcessingResult, String> {
    Ok(ProcessingResult {
        processed_count: data.len() as i32,
        success: true,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessingOptions {
    pub timeout: f32,
    pub retry_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessingResult {
    pub processed_count: i32,
    pub success: bool,
}

// Function that should NOT be detected (no #[command] attribute)
pub fn internal_helper(data: String) -> String {
    data
}