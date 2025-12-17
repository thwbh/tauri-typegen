/// Fixture: Basic Tauri commands without complex types or attributes

pub const SIMPLE_COMMAND: &str = r#"
#[tauri::command]
pub fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}
"#;

pub const MULTIPLE_PARAMETERS: &str = r#"
#[tauri::command]
pub fn calculate(x: i32, y: i32, operation: String) -> Result<i32, String> {
    match operation.as_str() {
        "add" => Ok(x + y),
        "subtract" => Ok(x - y),
        _ => Err("Invalid operation".to_string()),
    }
}
"#;

pub const NO_PARAMETERS: &str = r#"
#[tauri::command]
pub fn get_version() -> String {
    "1.0.0".to_string()
}
"#;

pub const ASYNC_COMMAND: &str = r#"
#[tauri::command]
pub async fn fetch_data(url: String) -> Result<String, String> {
    // Simulated async operation
    Ok("data".to_string())
}
"#;

pub const OPTIONAL_PARAMETERS: &str = r#"
#[tauri::command]
pub fn search(query: String, limit: Option<u32>) -> Vec<String> {
    vec![]
}
"#;
