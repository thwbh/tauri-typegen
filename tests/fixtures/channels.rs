/// Fixture: Commands with Tauri IPC channels

pub const SIMPLE_CHANNEL: &str = r#"
use tauri::ipc::Channel;

#[tauri::command]
pub fn stream_data(channel: Channel<String>) {
    channel.send("data".to_string()).ok();
}
"#;

pub const CHANNEL_WITH_PARAMETERS: &str = r#"
use tauri::ipc::Channel;

#[tauri::command]
pub fn download_file(
    url: String,
    progress: Channel<u32>,
) -> Result<String, String> {
    progress.send(50).ok();
    progress.send(100).ok();
    Ok("Complete".to_string())
}
"#;

pub const MULTIPLE_CHANNELS: &str = r#"
use tauri::ipc::Channel;

#[tauri::command]
pub fn process_data(
    data: Vec<String>,
    progress: Channel<u32>,
    logs: Channel<String>,
) -> Result<(), String> {
    logs.send("Starting".to_string()).ok();
    progress.send(50).ok();
    logs.send("Finished".to_string()).ok();
    Ok(())
}
"#;

pub const CHANNEL_WITH_CUSTOM_TYPE: &str = r#"
use tauri::ipc::Channel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub current: u32,
    pub total: u32,
    pub message: String,
}

#[tauri::command]
pub fn long_running_task(progress: Channel<ProgressUpdate>) -> Result<(), String> {
    progress.send(ProgressUpdate {
        current: 50,
        total: 100,
        message: "Processing...".to_string(),
    }).ok();
    Ok(())
}
"#;

pub const CHANNEL_WITH_SERDE_RENAME: &str = r#"
use tauri::ipc::Channel;

#[tauri::command]
#[serde(rename_all = "camelCase")]
pub fn update_progress(
    task_id: String,
    #[serde(rename = "progressChannel")]
    progress: Channel<u32>,
) -> Result<(), String> {
    progress.send(100).ok();
    Ok(())
}
"#;
