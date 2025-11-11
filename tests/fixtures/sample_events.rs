use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStarted {
    pub url: String,
    pub download_id: usize,
    pub content_length: usize,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub download_id: usize,
    pub bytes_downloaded: usize,
    pub total_bytes: usize,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadComplete {
    pub download_id: usize,
    pub file_path: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserLoggedIn {
    pub user_id: i32,
    pub username: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    pub message: String,
    pub timestamp: i64,
}

// Command that emits events
#[tauri::command]
pub async fn download_file(app: AppHandle, url: String) -> Result<String, String> {
    let download_id = 123;

    // Emit download started event
    app.emit("download-started", DownloadStarted {
        url: url.clone(),
        download_id,
        content_length: 1024,
    }).unwrap();

    // Simulate download progress
    for i in 0..5 {
        app.emit("download-progress", DownloadProgress {
            download_id,
            bytes_downloaded: i * 256,
            total_bytes: 1024,
        }).unwrap();
    }

    // Emit completion event
    app.emit("download-complete", DownloadComplete {
        download_id,
        file_path: "/downloads/file.txt".to_string(),
    }).unwrap();

    Ok("Download complete".to_string())
}

// Command with emit_to for targeted events
#[tauri::command]
pub fn login_user(app: AppHandle, username: String, password: String) -> Result<(), String> {
    // Simulate login
    let user_id = 42;

    // Emit to specific webview
    app.emit_to("main", "user-logged-in", UserLoggedIn {
        user_id,
        username: username.clone(),
    }).unwrap();

    Ok(())
}

// Function that emits events without being a command
pub fn notify_status_change(app: &AppHandle, message: String) {
    app.emit("status-update", StatusUpdate {
        message,
        timestamp: 1234567890,
    }).unwrap();
}

// Command with conditional event emission
#[tauri::command]
pub fn process_data(app: AppHandle, data: Vec<String>) -> Result<i32, String> {
    if data.is_empty() {
        app.emit("error-occurred", "No data provided").unwrap();
        return Err("No data provided".to_string());
    }

    let count = data.len() as i32;
    app.emit("processing-complete", count).unwrap();

    Ok(count)
}

// Command with event emission in different control flow structures
#[tauri::command]
pub fn complex_workflow(app: AppHandle, step: i32) -> Result<(), String> {
    match step {
        1 => {
            app.emit("workflow-step-1", "Starting").unwrap();
        }
        2 => {
            app.emit("workflow-step-2", "Processing").unwrap();
        }
        3 => {
            app.emit("workflow-step-3", "Finalizing").unwrap();
        }
        _ => {
            app.emit("workflow-error", "Invalid step").unwrap();
        }
    }

    // Emit in a loop
    for i in 0..step {
        app.emit("workflow-iteration", i).unwrap();
    }

    Ok(())
}
