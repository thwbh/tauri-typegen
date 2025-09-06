use serde::{Deserialize, Serialize};

// Learn more about Tauri commands at https://v2.tauri.app/develop/calling-rust/#commands

#[derive(Debug, Serialize, Deserialize)]
pub struct GreetingResponse {
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Deserialize)]
pub struct GreetRequest {
    pub name: String,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn greet_advanced(request: GreetRequest) -> Result<GreetingResponse, String> {
    if request.name.trim().is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    
    Ok(GreetingResponse {
        message: format!("Hello, {}! You've been greeted from Rust!", request.name),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, greet_advanced])
        .plugin(tauri_plugin_typegen::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
