use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub bytes_downloaded: usize,
    pub total_bytes: usize,
    pub percentage: f32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogMessage {
    pub level: String,
    pub message: String,
    pub timestamp: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProcessOutput {
    pub line: String,
    pub is_error: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StreamData {
    pub sequence: i32,
    pub data: String,
}

// Simple channel command
#[tauri::command]
pub async fn download_file(
    url: String,
    on_progress: Channel<DownloadProgress>
) -> Result<String, String> {
    // Simulated download with progress updates
    for i in 1..=10 {
        on_progress.send(DownloadProgress {
            bytes_downloaded: i * 1024,
            total_bytes: 10240,
            percentage: (i as f32 / 10.0) * 100.0,
        }).unwrap();
    }

    Ok("Download complete".to_string())
}

// Multiple channels in one command
#[tauri::command]
pub async fn complex_operation(
    config: String,
    on_progress: Channel<DownloadProgress>,
    on_log: Channel<LogMessage>
) -> Result<(), String> {
    on_log.send(LogMessage {
        level: "info".to_string(),
        message: format!("Starting operation with config: {}", config),
        timestamp: 1234567890,
    }).unwrap();

    for i in 1..=5 {
        on_progress.send(DownloadProgress {
            bytes_downloaded: i * 2048,
            total_bytes: 10240,
            percentage: (i as f32 / 5.0) * 100.0,
        }).unwrap();

        on_log.send(LogMessage {
            level: "debug".to_string(),
            message: format!("Progress: {}%", (i as f32 / 5.0) * 100.0),
            timestamp: 1234567890 + i as i64,
        }).unwrap();
    }

    on_log.send(LogMessage {
        level: "info".to_string(),
        message: "Operation complete".to_string(),
        timestamp: 1234567900,
    }).unwrap();

    Ok(())
}

// Channel with primitive type
#[tauri::command]
pub fn stream_numbers(on_number: Channel<i32>) -> Result<(), String> {
    for i in 0..100 {
        on_number.send(i).unwrap();
    }
    Ok(())
}

// Channel with string type
#[tauri::command]
pub fn stream_messages(on_message: Channel<String>) -> Result<(), String> {
    let messages = vec!["Hello", "World", "From", "Tauri"];
    for msg in messages {
        on_message.send(msg.to_string()).unwrap();
    }
    Ok(())
}

// Command with channels and regular parameters
#[tauri::command]
pub async fn run_process(
    command: String,
    args: Vec<String>,
    on_stdout: Channel<ProcessOutput>,
    on_stderr: Channel<ProcessOutput>,
) -> Result<i32, String> {
    // Simulate process execution
    on_stdout.send(ProcessOutput {
        line: format!("Running: {} {:?}", command, args),
        is_error: false,
    }).unwrap();

    on_stderr.send(ProcessOutput {
        line: "Warning: This is a simulated process".to_string(),
        is_error: true,
    }).unwrap();

    on_stdout.send(ProcessOutput {
        line: "Process completed successfully".to_string(),
        is_error: false,
    }).unwrap();

    Ok(0)
}

// Channel with fully qualified path
#[tauri::command]
pub fn qualified_channel(data: tauri::ipc::Channel<StreamData>) -> Result<(), String> {
    for i in 0..5 {
        data.send(StreamData {
            sequence: i,
            data: format!("Item {}", i),
        }).unwrap();
    }
    Ok(())
}

// Command without channels (control case)
#[tauri::command]
pub fn normal_command(data: String) -> String {
    format!("Received: {}", data)
}

// Channel with Option wrapper
#[tauri::command]
pub fn optional_stream(
    required: String,
    on_data: Channel<Option<String>>
) -> Result<(), String> {
    on_data.send(Some(required.clone())).unwrap();
    on_data.send(None).unwrap();
    on_data.send(Some("Done".to_string())).unwrap();
    Ok(())
}

// Channel with Vec type
#[tauri::command]
pub fn batch_stream(on_batch: Channel<Vec<i32>>) -> Result<(), String> {
    on_batch.send(vec![1, 2, 3]).unwrap();
    on_batch.send(vec![4, 5, 6]).unwrap();
    on_batch.send(vec![7, 8, 9]).unwrap();
    Ok(())
}
