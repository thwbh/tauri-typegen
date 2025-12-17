/// Fixture: Commands with complex Rust types

pub const NESTED_STRUCT: &str = r#"
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub zip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub address: Address,
}

#[tauri::command]
pub fn get_user(id: String) -> User {
    User {
        id,
        name: "John".to_string(),
        address: Address {
            street: "123 Main St".to_string(),
            city: "Springfield".to_string(),
            zip: "12345".to_string(),
        },
    }
}
"#;

pub const COLLECTIONS: &str = r#"
use std::collections::{HashMap, HashSet};

#[tauri::command]
pub fn get_tags() -> Vec<String> {
    vec!["tag1".to_string(), "tag2".to_string()]
}

#[tauri::command]
pub fn get_metadata() -> HashMap<String, String> {
    HashMap::new()
}

#[tauri::command]
pub fn get_unique_ids() -> HashSet<String> {
    HashSet::new()
}
"#;

pub const TUPLES: &str = r#"
#[tauri::command]
pub fn get_coordinates() -> (f64, f64) {
    (40.7128, -74.0060)
}

#[tauri::command]
pub fn parse_response() -> (String, u32, bool) {
    ("OK".to_string(), 200, true)
}
"#;

pub const GENERIC_RESULT: &str = r#"
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: u32,
    pub message: String,
}

#[tauri::command]
pub fn fetch_data(id: String) -> Result<String, ApiError> {
    Ok("data".to_string())
}
"#;

pub const NESTED_COLLECTIONS: &str = r#"
use std::collections::HashMap;

#[tauri::command]
pub fn get_user_permissions() -> HashMap<String, Vec<String>> {
    HashMap::new()
}

#[tauri::command]
pub fn get_matrix() -> Vec<Vec<i32>> {
    vec![vec![1, 2], vec![3, 4]]
}
"#;

pub const OPTION_TYPES: &str = r#"
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub email: Option<String>,
    pub age: Option<u32>,
}

#[tauri::command]
pub fn get_profile(id: String) -> Option<Profile> {
    Some(Profile {
        name: "John".to_string(),
        email: None,
        age: Some(30),
    })
}
"#;
