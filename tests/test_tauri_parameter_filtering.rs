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
fn test_filters_app_handle() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{command, AppHandle};

            #[command]
            pub fn my_command(app: AppHandle, user_input: String) -> String {
                user_input
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    let command = &commands[0];

    // Should only have user_input, not app
    assert_eq!(command.parameters.len(), 1);
    assert_eq!(command.parameters[0].name, "user_input");
    assert_eq!(command.parameters[0].typescript_type, "string");
}

#[test]
fn test_filters_window() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{command, Window, Runtime};

            #[command]
            pub fn my_command<R: Runtime>(window: Window<R>, count: i32) -> i32 {
                count
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    let command = &commands[0];

    // Should only have count, not window
    assert_eq!(command.parameters.len(), 1);
    assert_eq!(command.parameters[0].name, "count");
    assert_eq!(command.parameters[0].typescript_type, "number");
}

#[test]
fn test_filters_webview_window() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{command, WebviewWindow};

            #[command]
            pub fn my_command(webview: WebviewWindow, enabled: bool) -> bool {
                enabled
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    let command = &commands[0];

    // Should only have enabled, not webview
    assert_eq!(command.parameters.len(), 1);
    assert_eq!(command.parameters[0].name, "enabled");
    assert_eq!(command.parameters[0].typescript_type, "boolean");
}

#[test]
fn test_filters_state() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{command, State};

            pub struct AppState {
                count: i32,
            }

            #[command]
            pub fn my_command(state: State<AppState>, name: String) -> String {
                name
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    let command = &commands[0];

    // Should only have name, not state
    assert_eq!(command.parameters.len(), 1);
    assert_eq!(command.parameters[0].name, "name");
    assert_eq!(command.parameters[0].typescript_type, "string");
}

#[test]
fn test_filters_ipc_request() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            #[tauri::command]
            pub fn my_command(request: tauri::ipc::Request, data: String) -> String {
                data
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    let command = &commands[0];

    // Should only have data, not request
    assert_eq!(command.parameters.len(), 1);
    assert_eq!(command.parameters[0].name, "data");
    assert_eq!(command.parameters[0].typescript_type, "string");
}

#[test]
fn test_filters_ipc_channel() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            #[tauri::command]
            pub fn my_command(channel: tauri::ipc::Channel<String>, initial_value: i32) -> i32 {
                initial_value
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    let command = &commands[0];

    // Should only have initial_value, not channel
    assert_eq!(command.parameters.len(), 1);
    assert_eq!(command.parameters[0].name, "initial_value");
    assert_eq!(command.parameters[0].typescript_type, "number");
}

#[test]
fn test_filters_manager_trait() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{command, Manager, AppHandle};

            #[command]
            pub fn my_command(app: AppHandle, value: f64) -> f64 {
                value
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    let command = &commands[0];

    // Should only have value, not app
    assert_eq!(command.parameters.len(), 1);
    assert_eq!(command.parameters[0].name, "value");
    assert_eq!(command.parameters[0].typescript_type, "number");
}

#[test]
fn test_filters_multiple_tauri_params() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{command, AppHandle, State, WebviewWindow};

            pub struct MyState {
                value: String,
            }

            #[command]
            pub fn my_command(
                app: AppHandle,
                window: WebviewWindow,
                state: State<MyState>,
                user_name: String,
                user_age: i32,
            ) -> String {
                user_name
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    let command = &commands[0];

    // Should only have user_name and user_age
    assert_eq!(command.parameters.len(), 2);
    assert_eq!(command.parameters[0].name, "user_name");
    assert_eq!(command.parameters[0].typescript_type, "string");
    assert_eq!(command.parameters[1].name, "user_age");
    assert_eq!(command.parameters[1].typescript_type, "number");
}

#[test]
fn test_tauri_params_in_different_order() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::{command, AppHandle, State};

            pub struct AppState {}

            #[command]
            pub fn my_command(
                first_user_param: String,
                app: AppHandle,
                second_user_param: i32,
                state: State<AppState>,
                third_user_param: bool,
            ) -> String {
                first_user_param
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    let command = &commands[0];

    // Should have all three user params in order
    assert_eq!(command.parameters.len(), 3);
    assert_eq!(command.parameters[0].name, "first_user_param");
    assert_eq!(command.parameters[0].typescript_type, "string");
    assert_eq!(command.parameters[1].name, "second_user_param");
    assert_eq!(command.parameters[1].typescript_type, "number");
    assert_eq!(command.parameters[2].name, "third_user_param");
    assert_eq!(command.parameters[2].typescript_type, "boolean");
}

#[test]
fn test_does_not_filter_user_types_with_similar_names() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            use tauri::command;

            // User-defined types that happen to have similar names
            pub struct MyWindow {
                title: String,
            }

            pub struct MyState {
                value: i32,
            }

            #[command]
            pub fn my_command(
                window: MyWindow,
                state: MyState,
            ) -> String {
                "test".to_string()
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    let command = &commands[0];

    // Should keep both parameters since they're user-defined types
    assert_eq!(command.parameters.len(), 2);
    assert_eq!(command.parameters[0].name, "window");
    assert_eq!(command.parameters[0].typescript_type, "MyWindow");
    assert_eq!(command.parameters[1].name, "state");
    assert_eq!(command.parameters[1].typescript_type, "MyState");
}

#[test]
fn test_fully_qualified_tauri_types() {
    let temp_dir = create_test_project(&[(
        "src/test.rs",
        r#"
            #[tauri::command]
            pub fn my_command(
                app: tauri::AppHandle,
                window: tauri::WebviewWindow,
                request: tauri::ipc::Request,
                user_data: String,
            ) -> String {
                user_data
            }
        "#,
    )]);

    let mut analyzer = CommandAnalyzer::new();
    let commands = analyzer
        .analyze_project(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert_eq!(commands.len(), 1);
    let command = &commands[0];

    // Should only have user_data
    assert_eq!(command.parameters.len(), 1);
    assert_eq!(command.parameters[0].name, "user_data");
    assert_eq!(command.parameters[0].typescript_type, "string");
}
