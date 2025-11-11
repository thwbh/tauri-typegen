use std::fs;
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
fn test_discovers_simple_channel() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::ipc::Channel;
            use serde::Serialize;

            #[derive(Clone, Serialize)]
            pub struct ProgressUpdate {
                pub progress: f32,
            }

            #[tauri::command]
            pub fn my_command(on_progress: Channel<ProgressUpdate>) {
                on_progress.send(ProgressUpdate { progress: 50.0 }).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].channels.len(), 1);
    assert_eq!(commands[0].channels[0].parameter_name, "on_progress");
    assert_eq!(commands[0].channels[0].message_type, "ProgressUpdate");
    assert_eq!(commands[0].channels[0].command_name, "my_command");
}

#[test]
fn test_discovers_multiple_channels() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::ipc::Channel;
            use serde::Serialize;

            #[derive(Clone, Serialize)]
            pub struct Progress {
                pub value: i32,
            }

            #[derive(Clone, Serialize)]
            pub struct Log {
                pub message: String,
            }

            #[tauri::command]
            pub fn my_command(
                on_progress: Channel<Progress>,
                on_log: Channel<Log>
            ) {
                on_progress.send(Progress { value: 42 }).unwrap();
                on_log.send(Log { message: "test".to_string() }).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].channels.len(), 2);

    let channel_names: Vec<&str> = commands[0]
        .channels
        .iter()
        .map(|ch| ch.parameter_name.as_str())
        .collect();
    assert!(channel_names.contains(&"on_progress"));
    assert!(channel_names.contains(&"on_log"));

    let message_types: Vec<&str> = commands[0]
        .channels
        .iter()
        .map(|ch| ch.message_type.as_str())
        .collect();
    assert!(message_types.contains(&"Progress"));
    assert!(message_types.contains(&"Log"));
}

#[test]
fn test_discovers_channel_with_primitive_type() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::ipc::Channel;

            #[tauri::command]
            pub fn stream_numbers(on_number: Channel<i32>) {
                on_number.send(42).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].channels.len(), 1);
    assert_eq!(commands[0].channels[0].parameter_name, "on_number");
    assert_eq!(commands[0].channels[0].message_type, "i32");
}

#[test]
fn test_discovers_channel_with_string_type() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::ipc::Channel;

            #[tauri::command]
            pub fn stream_messages(on_message: Channel<String>) {
                on_message.send("Hello".to_string()).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].channels.len(), 1);
    assert_eq!(commands[0].channels[0].parameter_name, "on_message");
    assert_eq!(commands[0].channels[0].message_type, "String");
}

#[test]
fn test_discovers_channel_with_vec_type() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::ipc::Channel;

            #[tauri::command]
            pub fn stream_batches(on_batch: Channel<Vec<i32>>) {
                on_batch.send(vec![1, 2, 3]).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].channels.len(), 1);
    assert_eq!(commands[0].channels[0].parameter_name, "on_batch");
    assert_eq!(commands[0].channels[0].message_type, "Vec<i32>");
}

#[test]
fn test_discovers_channel_with_option_type() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::ipc::Channel;

            #[tauri::command]
            pub fn optional_stream(on_data: Channel<Option<String>>) {
                on_data.send(Some("test".to_string())).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].channels.len(), 1);
    assert_eq!(commands[0].channels[0].parameter_name, "on_data");
    assert_eq!(commands[0].channels[0].message_type, "Option<String>");
}

#[test]
fn test_discovers_qualified_channel_path() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            #[derive(Clone, serde::Serialize)]
            pub struct Data {
                pub value: i32,
            }

            #[tauri::command]
            pub fn qualified_channel(on_data: tauri::ipc::Channel<Data>) {
                on_data.send(Data { value: 42 }).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].channels.len(), 1);
    assert_eq!(commands[0].channels[0].parameter_name, "on_data");
    assert_eq!(commands[0].channels[0].message_type, "Data");
}

#[test]
fn test_channel_with_regular_parameters() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::ipc::Channel;

            #[tauri::command]
            pub fn mixed_params(
                name: String,
                count: i32,
                on_update: Channel<String>
            ) -> String {
                on_update.send(format!("Processing {}", name)).unwrap();
                format!("Done: {} - {}", name, count)
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);

    // Should have 2 regular parameters
    assert_eq!(commands[0].parameters.len(), 2);
    assert_eq!(commands[0].parameters[0].name, "name");
    assert_eq!(commands[0].parameters[1].name, "count");

    // Should have 1 channel
    assert_eq!(commands[0].channels.len(), 1);
    assert_eq!(commands[0].channels[0].parameter_name, "on_update");
    assert_eq!(commands[0].channels[0].message_type, "String");
}

#[test]
fn test_command_without_channels() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            #[tauri::command]
            pub fn normal_command(data: String) -> String {
                data
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].channels.len(), 0);
    assert_eq!(commands[0].parameters.len(), 1);
}

#[test]
fn test_no_false_positives() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::ipc::Channel;

            // Custom type that happens to be named Channel (should not be detected)
            pub struct Channel<T> {
                data: T,
            }

            // This should NOT be detected as a Tauri channel
            pub fn helper_function(my_channel: Channel<String>) {
                println!("{}", my_channel.data);
            }

            // This SHOULD be detected
            #[tauri::command]
            pub fn real_command(on_data: tauri::ipc::Channel<i32>) {
                on_data.send(42).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    // Should only find the tauri command
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].name, "real_command");
    assert_eq!(commands[0].channels.len(), 1);
}

#[test]
fn test_typescript_type_conversion() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::ipc::Channel;
            use serde::Serialize;

            #[derive(Clone, Serialize)]
            pub struct MyData {
                pub count: i32,
                pub name: String,
                pub active: bool,
            }

            #[tauri::command]
            pub fn my_command(on_data: Channel<MyData>) {
                on_data.send(MyData {
                    count: 42,
                    name: "test".to_string(),
                    active: true,
                }).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].channels.len(), 1);
    assert_eq!(commands[0].channels[0].parameter_name, "on_data");
    assert_eq!(commands[0].channels[0].message_type, "MyData");
    assert_eq!(commands[0].channels[0].typescript_message_type, "MyData");
}

#[test]
fn test_discovers_channels_from_fixture_file() {
    let mut analyzer = CommandAnalyzer::new();
    let fixture_path = std::path::Path::new("tests/fixtures/sample_channels.rs");

    // Just analyze the single fixture file
    let commands = analyzer.analyze_file(fixture_path).unwrap();

    // Should discover multiple commands with channels
    assert!(commands.len() > 0, "Should discover at least one command");

    // Find the download_file command
    let download_cmd = commands.iter().find(|c| c.name == "download_file");
    assert!(download_cmd.is_some(), "Should find download_file command");
    let download_cmd = download_cmd.unwrap();

    assert_eq!(download_cmd.channels.len(), 1);
    assert_eq!(download_cmd.channels[0].parameter_name, "on_progress");
    assert_eq!(download_cmd.channels[0].message_type, "DownloadProgress");

    // Find the complex_operation command
    let complex_cmd = commands.iter().find(|c| c.name == "complex_operation");
    assert!(complex_cmd.is_some(), "Should find complex_operation command");
    let complex_cmd = complex_cmd.unwrap();

    assert_eq!(complex_cmd.channels.len(), 2);
    let channel_names: Vec<&str> = complex_cmd
        .channels
        .iter()
        .map(|ch| ch.parameter_name.as_str())
        .collect();
    assert!(channel_names.contains(&"on_progress"));
    assert!(channel_names.contains(&"on_log"));

    // Find the normal_command (should have no channels)
    let normal_cmd = commands.iter().find(|c| c.name == "normal_command");
    assert!(normal_cmd.is_some(), "Should find normal_command command");
    assert_eq!(normal_cmd.unwrap().channels.len(), 0);
}

#[test]
fn test_channel_file_path_tracking() {
    let temp_dir = create_test_project(&[(
        "src/channels.rs",
        r#"
            use tauri::ipc::Channel;

            #[tauri::command]
            pub fn test_command(on_data: Channel<i32>) {
                on_data.send(42).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].channels.len(), 1);

    // Should track the file path
    assert!(commands[0].channels[0].file_path.ends_with("src/channels.rs"));
    // Should track line number (non-zero)
    assert!(commands[0].channels[0].line_number > 0);
}

#[test]
fn test_multiple_commands_with_channels() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::ipc::Channel;

            #[tauri::command]
            pub fn command_one(on_data: Channel<String>) {
                on_data.send("one".to_string()).unwrap();
            }

            #[tauri::command]
            pub fn command_two(on_data: Channel<i32>) {
                on_data.send(2).unwrap();
            }

            #[tauri::command]
            pub fn command_three(on_first: Channel<bool>, on_second: Channel<f32>) {
                on_first.send(true).unwrap();
                on_second.send(3.14).unwrap();
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 3);

    // Command one: 1 channel
    assert_eq!(commands[0].channels.len(), 1);
    assert_eq!(commands[0].channels[0].message_type, "String");

    // Command two: 1 channel
    assert_eq!(commands[1].channels.len(), 1);
    assert_eq!(commands[1].channels[0].message_type, "i32");

    // Command three: 2 channels
    assert_eq!(commands[2].channels.len(), 2);
    assert_eq!(commands[2].channels[0].message_type, "bool");
    assert_eq!(commands[2].channels[1].message_type, "f32");
}

#[test]
fn test_get_all_discovered_channels() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::ipc::Channel;

            #[tauri::command]
            pub fn cmd1(on_data: Channel<String>) {}

            #[tauri::command]
            pub fn cmd2(on_progress: Channel<i32>, on_log: Channel<String>) {}
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    let all_channels = analyzer.get_all_discovered_channels(&commands);

    // Should have 3 channels total (1 from cmd1, 2 from cmd2)
    assert_eq!(all_channels.len(), 3);

    // Check command names
    assert_eq!(all_channels[0].command_name, "cmd1");
    assert_eq!(all_channels[1].command_name, "cmd2");
    assert_eq!(all_channels[2].command_name, "cmd2");
}
