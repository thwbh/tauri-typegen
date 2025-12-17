/// Fixture: Tauri event emissions

pub const SIMPLE_EVENT: &str = r#"
use tauri::Manager;

pub fn emit_update(app: &tauri::AppHandle) {
    app.emit("update", "new data").ok();
}
"#;

pub const EVENT_WITH_CUSTOM_TYPE: &str = r#"
use tauri::Manager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    pub status: String,
    pub progress: u32,
}

pub fn emit_status(app: &tauri::AppHandle) {
    app.emit("status-update", StatusUpdate {
        status: "processing".to_string(),
        progress: 50,
    }).ok();
}
"#;

pub const MULTIPLE_EVENTS: &str = r#"
use tauri::Manager;

pub fn emit_events(app: &tauri::AppHandle) {
    app.emit("started", ()).ok();
    app.emit("progress", 50u32).ok();
    app.emit("completed", "done".to_string()).ok();
}
"#;

pub const EVENT_WITH_SERDE_RENAME: &str = r#"
use tauri::Manager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserEvent {
    pub user_id: String,
    pub event_type: String,
}

pub fn emit_user_event(app: &tauri::AppHandle) {
    app.emit("user-event", UserEvent {
        user_id: "123".to_string(),
        event_type: "login".to_string(),
    }).ok();
}
"#;
