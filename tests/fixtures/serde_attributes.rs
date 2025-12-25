#![allow(dead_code)]
/// Fixture: Commands and structs with serde attributes
pub const COMMAND_WITH_RENAME_ALL: &str = r#"
#[tauri::command]
#[serde(rename_all = "snake_case")]
pub fn update_user_profile(user_id: String, full_name: String) -> Result<String, String> {
    Ok("Updated".to_string())
}
"#;

pub const PARAMETER_WITH_RENAME: &str = r#"
#[tauri::command]
pub fn create_order(
    #[serde(rename = "id")] order_id: String,
    customer_name: String,
) -> Result<String, String> {
    Ok("Created".to_string())
}
"#;

pub const STRUCT_WITH_RENAME_ALL: &str = r#"
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub user_id: String,
    pub first_name: String,
    pub last_name: String,
    pub email_address: String,
}

#[tauri::command]
pub fn get_user(id: String) -> UserProfile {
    UserProfile {
        user_id: id,
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        email_address: "john@example.com".to_string(),
    }
}
"#;

pub const FIELD_WITH_RENAME: &str = r#"
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    #[serde(rename = "productName")]
    pub name: String,
    pub price: f64,
}

#[tauri::command]
pub fn get_product(id: String) -> Product {
    Product {
        id,
        name: "Widget".to_string(),
        price: 19.99,
    }
}
"#;

pub const ENUM_WITH_RENAME: &str = r#"
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Active,
    Pending,
    Completed,
    Cancelled,
}

#[tauri::command]
pub fn get_status() -> Status {
    Status::Active
}
"#;

pub const ENUM_VARIANT_WITH_RENAME: &str = r#"
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderStatus {
    #[serde(rename = "new")]
    New,
    #[serde(rename = "processing")]
    InProgress,
    #[serde(rename = "shipped")]
    Shipped,
}

#[tauri::command]
pub fn get_order_status() -> OrderStatus {
    OrderStatus::New
}
"#;
