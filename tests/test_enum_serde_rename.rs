use tauri_typegen::analysis::struct_parser::StructParser;
use tauri_typegen::analysis::type_resolver::TypeResolver;

#[test]
fn test_enum_serde_rename_all_screaming_snake_case() {
    let code = r#"
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum MyEnum {
            HelloWorld,
            ByeWorld,
        }
    "#;

    let parsed_file = syn::parse_file(code).unwrap();
    let parser = StructParser::new();
    let mut type_resolver = TypeResolver::new();

    let item_enum = parsed_file.items.iter().find_map(|item| {
        if let syn::Item::Enum(e) = item {
            Some(e)
        } else {
            None
        }
    });

    assert!(item_enum.is_some());
    let item_enum = item_enum.unwrap();

    let struct_info = parser
        .parse_enum(
            item_enum,
            std::path::Path::new("test.rs"),
            &mut type_resolver,
        )
        .unwrap();

    assert_eq!(struct_info.fields.len(), 2);
    assert!(struct_info.is_enum);

    let hello_variant = struct_info
        .fields
        .iter()
        .find(|f| f.name == "HelloWorld")
        .unwrap();
    assert_eq!(
        hello_variant.serialized_name,
        Some("HELLO_WORLD".to_string())
    );

    let bye_variant = struct_info
        .fields
        .iter()
        .find(|f| f.name == "ByeWorld")
        .unwrap();
    assert_eq!(bye_variant.serialized_name, Some("BYE_WORLD".to_string()));
}

#[test]
fn test_enum_serde_rename_all_camel_case() {
    let code = r#"
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub enum Status {
            InProgress,
            NotStarted,
            Completed,
        }
    "#;

    let parsed_file = syn::parse_file(code).unwrap();
    let parser = StructParser::new();
    let mut type_resolver = TypeResolver::new();

    let item_enum = parsed_file.items.iter().find_map(|item| {
        if let syn::Item::Enum(e) = item {
            Some(e)
        } else {
            None
        }
    });

    assert!(item_enum.is_some());
    let item_enum = item_enum.unwrap();

    let struct_info = parser
        .parse_enum(
            item_enum,
            std::path::Path::new("test.rs"),
            &mut type_resolver,
        )
        .unwrap();

    assert_eq!(struct_info.fields.len(), 3);

    let in_progress = struct_info
        .fields
        .iter()
        .find(|f| f.name == "InProgress")
        .unwrap();
    assert_eq!(in_progress.serialized_name, Some("inProgress".to_string()));

    let not_started = struct_info
        .fields
        .iter()
        .find(|f| f.name == "NotStarted")
        .unwrap();
    assert_eq!(not_started.serialized_name, Some("notStarted".to_string()));

    let completed = struct_info
        .fields
        .iter()
        .find(|f| f.name == "Completed")
        .unwrap();
    assert_eq!(completed.serialized_name, Some("completed".to_string()));
}

#[test]
fn test_enum_variant_level_rename() {
    let code = r#"
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        pub enum MyEnum {
            #[serde(rename = "hello")]
            HelloWorld,
            ByeWorld,
        }
    "#;

    let parsed_file = syn::parse_file(code).unwrap();
    let parser = StructParser::new();
    let mut type_resolver = TypeResolver::new();

    let item_enum = parsed_file.items.iter().find_map(|item| {
        if let syn::Item::Enum(e) = item {
            Some(e)
        } else {
            None
        }
    });

    assert!(item_enum.is_some());
    let item_enum = item_enum.unwrap();

    let struct_info = parser
        .parse_enum(
            item_enum,
            std::path::Path::new("test.rs"),
            &mut type_resolver,
        )
        .unwrap();

    assert_eq!(struct_info.fields.len(), 2);

    // Variant with explicit rename
    let hello_variant = struct_info
        .fields
        .iter()
        .find(|f| f.name == "HelloWorld")
        .unwrap();
    assert_eq!(hello_variant.serialized_name, Some("hello".to_string()));

    // Variant without rename should be None
    let bye_variant = struct_info
        .fields
        .iter()
        .find(|f| f.name == "ByeWorld")
        .unwrap();
    assert_eq!(bye_variant.serialized_name, None);
}

#[test]
fn test_enum_variant_rename_overrides_rename_all() {
    let code = r#"
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum MyEnum {
            HelloWorld,
            #[serde(rename = "custom_bye")]
            ByeWorld,
        }
    "#;

    let parsed_file = syn::parse_file(code).unwrap();
    let parser = StructParser::new();
    let mut type_resolver = TypeResolver::new();

    let item_enum = parsed_file.items.iter().find_map(|item| {
        if let syn::Item::Enum(e) = item {
            Some(e)
        } else {
            None
        }
    });

    assert!(item_enum.is_some());
    let item_enum = item_enum.unwrap();

    let struct_info = parser
        .parse_enum(
            item_enum,
            std::path::Path::new("test.rs"),
            &mut type_resolver,
        )
        .unwrap();

    assert_eq!(struct_info.fields.len(), 2);

    // Variant without explicit rename uses rename_all
    let hello_variant = struct_info
        .fields
        .iter()
        .find(|f| f.name == "HelloWorld")
        .unwrap();
    assert_eq!(
        hello_variant.serialized_name,
        Some("HELLO_WORLD".to_string())
    );

    // Variant with explicit rename overrides rename_all
    let bye_variant = struct_info
        .fields
        .iter()
        .find(|f| f.name == "ByeWorld")
        .unwrap();
    assert_eq!(bye_variant.serialized_name, Some("custom_bye".to_string()));
}

#[test]
fn test_enum_without_serde_attributes() {
    let code = r#"
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        pub enum SimpleEnum {
            Foo,
            Bar,
        }
    "#;

    let parsed_file = syn::parse_file(code).unwrap();
    let parser = StructParser::new();
    let mut type_resolver = TypeResolver::new();

    let item_enum = parsed_file.items.iter().find_map(|item| {
        if let syn::Item::Enum(e) = item {
            Some(e)
        } else {
            None
        }
    });

    assert!(item_enum.is_some());
    let item_enum = item_enum.unwrap();

    let struct_info = parser
        .parse_enum(
            item_enum,
            std::path::Path::new("test.rs"),
            &mut type_resolver,
        )
        .unwrap();

    assert_eq!(struct_info.fields.len(), 2);

    // Without serde attributes, serialized_name should be None
    let foo_variant = struct_info.fields.iter().find(|f| f.name == "Foo").unwrap();
    assert_eq!(foo_variant.serialized_name, None);

    let bar_variant = struct_info.fields.iter().find(|f| f.name == "Bar").unwrap();
    assert_eq!(bar_variant.serialized_name, None);
}

#[test]
fn test_enum_snake_case_to_pascal_case() {
    let code = r#"
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        pub enum MyEnum {
            hello_world,
            bye_world,
        }
    "#;

    let parsed_file = syn::parse_file(code).unwrap();
    let parser = StructParser::new();
    let mut type_resolver = TypeResolver::new();

    let item_enum = parsed_file.items.iter().find_map(|item| {
        if let syn::Item::Enum(e) = item {
            Some(e)
        } else {
            None
        }
    });

    assert!(item_enum.is_some());
    let item_enum = item_enum.unwrap();

    let struct_info = parser
        .parse_enum(
            item_enum,
            std::path::Path::new("test.rs"),
            &mut type_resolver,
        )
        .unwrap();

    assert_eq!(struct_info.fields.len(), 2);

    let hello_variant = struct_info
        .fields
        .iter()
        .find(|f| f.name == "hello_world")
        .unwrap();
    assert_eq!(
        hello_variant.serialized_name,
        Some("HelloWorld".to_string())
    );

    let bye_variant = struct_info
        .fields
        .iter()
        .find(|f| f.name == "bye_world")
        .unwrap();
    assert_eq!(bye_variant.serialized_name, Some("ByeWorld".to_string()));
}
