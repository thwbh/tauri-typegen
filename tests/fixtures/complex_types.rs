use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap, HashSet, BTreeSet};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub metadata: HashMap<String, String>,
    pub tags: HashSet<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub price: f64,
    pub categories: Vec<String>,
    pub attributes: BTreeMap<String, AttributeValue>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeValue {
    pub value: String,
    pub data_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Processing { estimated_completion: String },
    Shipped(String), // tracking number
    Delivered { delivery_date: String, signed_by: Option<String> },
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub id: i32,
    pub user_id: i32,
    pub status: OrderStatus,
    pub items: Vec<(i32, i32)>, // (product_id, quantity) tuples
    pub shipping_info: Option<(String, String, String)>, // (address, city, zip) tuple
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NestedContainer {
    pub deep_map: HashMap<String, Vec<Option<User>>>,
    pub complex_result: Result<HashMap<i32, Product>, String>,
    pub tuple_data: (String, i32, Option<f64>),
    pub set_of_maps: BTreeSet<HashMap<String, i32>>,
}

// Commands using complex types
#[command]
pub async fn create_user_with_metadata(
    name: String,
    metadata: HashMap<String, String>,
    tags: HashSet<String>
) -> Result<User, String> {
    Ok(User {
        id: 1,
        name,
        metadata,
        tags,
    })
}

#[command]
pub async fn get_user_products(
    user_id: i32,
    filters: Option<BTreeMap<String, Vec<String>>>
) -> Result<Vec<Product>, String> {
    Ok(vec![])
}

#[command]
pub async fn update_order_status(
    order_id: i32,
    status: OrderStatus
) -> Result<Order, String> {
    Ok(Order {
        id: order_id,
        user_id: 1,
        status,
        items: vec![],
        shipping_info: None,
    })
}

#[command]
pub fn process_complex_data(
    data: NestedContainer
) -> Result<HashMap<String, i32>, String> {
    Ok(HashMap::new())
}

#[command]
pub fn get_tuple_data() -> (String, i32, Option<f64>) {
    ("test".to_string(), 42, Some(3.14))
}

#[command]
pub async fn bulk_update_attributes(
    product_ids: Vec<i32>,
    attribute_updates: HashMap<String, AttributeValue>
) -> Result<Vec<Product>, String> {
    Ok(vec![])
}