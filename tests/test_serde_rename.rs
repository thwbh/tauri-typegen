use tauri_typegen::analysis::struct_parser::StructParser;
use tauri_typegen::analysis::type_resolver::TypeResolver;

#[test]
fn test_serde_rename_all_camel_case() {
    let code = r#"
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct User {
            pub user_id: i32,
            pub user_name: String,
            pub is_active: bool,
        }
    "#;

    let parsed_file = syn::parse_file(code).unwrap();
    let parser = StructParser::new();
    let mut type_resolver = TypeResolver::new();

    // Find the struct
    let item_struct = parsed_file.items.iter().find_map(|item| {
        if let syn::Item::Struct(s) = item {
            Some(s)
        } else {
            None
        }
    });

    assert!(item_struct.is_some());
    let item_struct = item_struct.unwrap();

    let struct_info = parser
        .parse_struct(
            item_struct,
            std::path::Path::new("test.rs"),
            &mut type_resolver,
        )
        .unwrap();

    // Check that rename_all="camelCase" was applied
    assert_eq!(struct_info.fields.len(), 3);

    let user_id_field = struct_info
        .fields
        .iter()
        .find(|f| f.name == "user_id")
        .unwrap();
    assert_eq!(user_id_field.serialized_name, "userId");

    let user_name_field = struct_info
        .fields
        .iter()
        .find(|f| f.name == "user_name")
        .unwrap();
    assert_eq!(user_name_field.serialized_name, "userName");

    let is_active_field = struct_info
        .fields
        .iter()
        .find(|f| f.name == "is_active")
        .unwrap();
    assert_eq!(is_active_field.serialized_name, "isActive");
}

#[test]
fn test_serde_field_rename() {
    let code = r#"
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        pub struct User {
            #[serde(rename = "userId")]
            pub user_id: i32,
            pub name: String,
        }
    "#;

    let parsed_file = syn::parse_file(code).unwrap();
    let parser = StructParser::new();
    let mut type_resolver = TypeResolver::new();

    let item_struct = parsed_file.items.iter().find_map(|item| {
        if let syn::Item::Struct(s) = item {
            Some(s)
        } else {
            None
        }
    });

    assert!(item_struct.is_some());
    let item_struct = item_struct.unwrap();

    let struct_info = parser
        .parse_struct(
            item_struct,
            std::path::Path::new("test.rs"),
            &mut type_resolver,
        )
        .unwrap();

    assert_eq!(struct_info.fields.len(), 2);

    // Field with explicit rename
    let user_id_field = struct_info
        .fields
        .iter()
        .find(|f| f.name == "user_id")
        .unwrap();
    assert_eq!(user_id_field.serialized_name, "userId");

    // Field without rename and no rename_all should use the field name
    let name_field = struct_info
        .fields
        .iter()
        .find(|f| f.name == "name")
        .unwrap();
    assert_eq!(name_field.serialized_name, "name");
}

#[test]
fn test_serde_rename_override() {
    let code = r#"
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct User {
            pub user_id: i32,
            #[serde(rename = "customName")]
            pub user_name: String,
        }
    "#;

    let parsed_file = syn::parse_file(code).unwrap();
    let parser = StructParser::new();
    let mut type_resolver = TypeResolver::new();

    let item_struct = parsed_file.items.iter().find_map(|item| {
        if let syn::Item::Struct(s) = item {
            Some(s)
        } else {
            None
        }
    });

    assert!(item_struct.is_some());
    let item_struct = item_struct.unwrap();

    let struct_info = parser
        .parse_struct(
            item_struct,
            std::path::Path::new("test.rs"),
            &mut type_resolver,
        )
        .unwrap();

    assert_eq!(struct_info.fields.len(), 2);

    // Field without explicit rename uses rename_all
    let user_id_field = struct_info
        .fields
        .iter()
        .find(|f| f.name == "user_id")
        .unwrap();
    assert_eq!(user_id_field.serialized_name, "userId");

    // Field with explicit rename overrides rename_all
    let user_name_field = struct_info
        .fields
        .iter()
        .find(|f| f.name == "user_name")
        .unwrap();
    assert_eq!(user_name_field.serialized_name, "customName");
}

#[test]
fn test_serde_skip() {
    let code = r#"
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        pub struct User {
            pub user_id: i32,
            #[serde(skip)]
            pub internal_field: String,
            pub name: String,
        }
    "#;

    let parsed_file = syn::parse_file(code).unwrap();
    let parser = StructParser::new();
    let mut type_resolver = TypeResolver::new();

    let item_struct = parsed_file.items.iter().find_map(|item| {
        if let syn::Item::Struct(s) = item {
            Some(s)
        } else {
            None
        }
    });

    assert!(item_struct.is_some());
    let item_struct = item_struct.unwrap();

    let struct_info = parser
        .parse_struct(
            item_struct,
            std::path::Path::new("test.rs"),
            &mut type_resolver,
        )
        .unwrap();

    // internal_field should be skipped
    assert_eq!(struct_info.fields.len(), 2);
    assert!(struct_info
        .fields
        .iter()
        .all(|f| f.name != "internal_field"));

    // Other fields should be present
    assert!(struct_info.fields.iter().any(|f| f.name == "user_id"));
    assert!(struct_info.fields.iter().any(|f| f.name == "name"));
}

#[test]
fn test_serde_rename_all_snake_case() {
    let code = r#"
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub struct User {
            pub userId: i32,
            pub userName: String,
        }
    "#;

    let parsed_file = syn::parse_file(code).unwrap();
    let parser = StructParser::new();
    let mut type_resolver = TypeResolver::new();

    let item_struct = parsed_file.items.iter().find_map(|item| {
        if let syn::Item::Struct(s) = item {
            Some(s)
        } else {
            None
        }
    });

    assert!(item_struct.is_some());
    let item_struct = item_struct.unwrap();

    let struct_info = parser
        .parse_struct(
            item_struct,
            std::path::Path::new("test.rs"),
            &mut type_resolver,
        )
        .unwrap();

    assert_eq!(struct_info.fields.len(), 2);

    let user_id_field = struct_info
        .fields
        .iter()
        .find(|f| f.name == "userId")
        .unwrap();
    assert_eq!(user_id_field.serialized_name, "user_id");

    let user_name_field = struct_info
        .fields
        .iter()
        .find(|f| f.name == "userName")
        .unwrap();
    assert_eq!(user_name_field.serialized_name, "user_name");
}

#[test]
fn test_serde_rename_all_pascal_case() {
    let code = r#"
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        pub struct User {
            pub user_id: i32,
            pub user_name: String,
        }
    "#;

    let parsed_file = syn::parse_file(code).unwrap();
    let parser = StructParser::new();
    let mut type_resolver = TypeResolver::new();

    let item_struct = parsed_file.items.iter().find_map(|item| {
        if let syn::Item::Struct(s) = item {
            Some(s)
        } else {
            None
        }
    });

    assert!(item_struct.is_some());
    let item_struct = item_struct.unwrap();

    let struct_info = parser
        .parse_struct(
            item_struct,
            std::path::Path::new("test.rs"),
            &mut type_resolver,
        )
        .unwrap();

    assert_eq!(struct_info.fields.len(), 2);

    let user_id_field = struct_info
        .fields
        .iter()
        .find(|f| f.name == "user_id")
        .unwrap();
    assert_eq!(user_id_field.serialized_name, "UserId");

    let user_name_field = struct_info
        .fields
        .iter()
        .find(|f| f.name == "user_name")
        .unwrap();
    assert_eq!(user_name_field.serialized_name, "UserName");
}
