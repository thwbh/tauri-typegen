use std::collections::HashMap;
use tauri_plugin_typegen::generators::base::BaseGenerator;
use tauri_plugin_typegen::models::{CommandInfo, ParameterInfo, StructInfo, FieldInfo};

/// Helper function to create a CommandInfo with parameters
fn create_command_with_params(name: &str, param_types: Vec<(&str, &str)>) -> CommandInfo {
    CommandInfo {
        name: name.to_string(),
        file_path: "test_file.rs".to_string(),
        line_number: 10,
        parameters: param_types
            .into_iter()
            .enumerate()
            .map(|(_i, (param_name, rust_type))| ParameterInfo {
                name: param_name.to_string(),
                rust_type: rust_type.to_string(),
                typescript_type: "any".to_string(), // Not important for this test
                is_optional: false,
            })
            .collect(),
        return_type: "void".to_string(),
        is_async: true,
    }
}

/// Helper function to create a CommandInfo with return type
fn create_command_with_return(name: &str, return_type: &str) -> CommandInfo {
    CommandInfo {
        name: name.to_string(),
        file_path: "test_file.rs".to_string(),
        line_number: 10,
        parameters: vec![],
        return_type: return_type.to_string(),
        is_async: true,
    }
}

/// Helper function to create a StructInfo with fields
fn create_struct_with_fields(name: &str, fields: Vec<(&str, &str)>) -> StructInfo {
    StructInfo {
        name: name.to_string(),
        fields: fields
            .into_iter()
            .map(|(field_name, rust_type)| FieldInfo {
                name: field_name.to_string(),
                rust_type: rust_type.to_string(),
                typescript_type: "any".to_string(), // Not important for this test
                is_optional: false,
                is_public: true,
                validator_attributes: None,
            })
            .collect(),
        file_path: "test_file.rs".to_string(),
        is_enum: false,
    }
}

#[test]
fn test_collect_used_types_basic_struct() {
    let generator = BaseGenerator::new();
    
    // Create a command that uses Cinema struct
    let commands = vec![create_command_with_params("get_cinema", vec![("cinema", "Cinema")])];
    
    // Create Cinema struct with a movies field
    let mut structs = HashMap::new();
    structs.insert("Cinema".to_string(), create_struct_with_fields("Cinema", vec![
        ("name", "String"),
        ("movies", "Vec<Movie>"),
    ]));
    
    // Create Movie struct
    structs.insert("Movie".to_string(), create_struct_with_fields("Movie", vec![
        ("title", "String"),
        ("year", "i32"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Both Cinema and Movie should be collected
    assert!(used_types.contains_key("Cinema"));
    assert!(used_types.contains_key("Movie"));
    assert_eq!(used_types.len(), 2);
}

#[test]
fn test_collect_used_types_deeply_nested() {
    let generator = BaseGenerator::new();
    
    // Create a command that uses Theater struct
    let commands = vec![create_command_with_return("get_theater", "Theater")];
    
    let mut structs = HashMap::new();
    
    // Theater -> Cinema -> Movie -> Actor
    structs.insert("Theater".to_string(), create_struct_with_fields("Theater", vec![
        ("name", "String"),
        ("cinemas", "Vec<Cinema>"),
    ]));
    
    structs.insert("Cinema".to_string(), create_struct_with_fields("Cinema", vec![
        ("id", "i32"),
        ("movies", "Vec<Movie>"),
    ]));
    
    structs.insert("Movie".to_string(), create_struct_with_fields("Movie", vec![
        ("title", "String"),
        ("actors", "Vec<Actor>"),
    ]));
    
    structs.insert("Actor".to_string(), create_struct_with_fields("Actor", vec![
        ("name", "String"),
        ("age", "i32"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // All structs should be collected due to nesting
    assert!(used_types.contains_key("Theater"));
    assert!(used_types.contains_key("Cinema"));
    assert!(used_types.contains_key("Movie"));
    assert!(used_types.contains_key("Actor"));
    assert_eq!(used_types.len(), 4);
}

#[test]
fn test_collect_used_types_option_wrapper() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_params("update_cinema", vec![("cinema", "Option<Cinema>")])];
    
    let mut structs = HashMap::new();
    structs.insert("Cinema".to_string(), create_struct_with_fields("Cinema", vec![
        ("movies", "Vec<Movie>"),
    ]));
    
    structs.insert("Movie".to_string(), create_struct_with_fields("Movie", vec![
        ("title", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Both Cinema and Movie should be collected even with Option wrapper
    assert!(used_types.contains_key("Cinema"));
    assert!(used_types.contains_key("Movie"));
    assert_eq!(used_types.len(), 2);
}

#[test]
fn test_collect_used_types_result_wrapper() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_return("get_cinema_result", "Result<Cinema, AppError>")];
    
    let mut structs = HashMap::new();
    structs.insert("Cinema".to_string(), create_struct_with_fields("Cinema", vec![
        ("movies", "Vec<Movie>"),
    ]));
    
    structs.insert("Movie".to_string(), create_struct_with_fields("Movie", vec![
        ("title", "String"),
    ]));
    
    structs.insert("AppError".to_string(), create_struct_with_fields("AppError", vec![
        ("message", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Cinema, Movie, and AppError should all be collected
    assert!(used_types.contains_key("Cinema"));
    assert!(used_types.contains_key("Movie"));
    assert!(used_types.contains_key("AppError"));
    assert_eq!(used_types.len(), 3);
}

#[test]
fn test_collect_used_types_multiple_commands() {
    let generator = BaseGenerator::new();
    
    let commands = vec![
        create_command_with_params("create_cinema", vec![("cinema", "Cinema")]),
        create_command_with_return("get_restaurant", "Restaurant"),
    ];
    
    let mut structs = HashMap::new();
    
    // Cinema uses Movie
    structs.insert("Cinema".to_string(), create_struct_with_fields("Cinema", vec![
        ("movies", "Vec<Movie>"),
    ]));
    
    structs.insert("Movie".to_string(), create_struct_with_fields("Movie", vec![
        ("title", "String"),
    ]));
    
    // Restaurant uses Food (separate hierarchy)
    structs.insert("Restaurant".to_string(), create_struct_with_fields("Restaurant", vec![
        ("menu", "Vec<Food>"),
    ]));
    
    structs.insert("Food".to_string(), create_struct_with_fields("Food", vec![
        ("name", "String"),
        ("price", "f64"),
    ]));
    
    // Unused struct that shouldn't be collected
    structs.insert("UnusedStruct".to_string(), create_struct_with_fields("UnusedStruct", vec![
        ("data", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Should collect Cinema->Movie and Restaurant->Food, but not UnusedStruct
    assert!(used_types.contains_key("Cinema"));
    assert!(used_types.contains_key("Movie"));
    assert!(used_types.contains_key("Restaurant"));
    assert!(used_types.contains_key("Food"));
    assert!(!used_types.contains_key("UnusedStruct"));
    assert_eq!(used_types.len(), 4);
}

#[test]
fn test_collect_used_types_circular_reference() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_params("get_user", vec![("user", "User")])];
    
    let mut structs = HashMap::new();
    
    // Create circular reference: User -> Profile -> User
    structs.insert("User".to_string(), create_struct_with_fields("User", vec![
        ("name", "String"),
        ("profile", "Option<Profile>"),
    ]));
    
    structs.insert("Profile".to_string(), create_struct_with_fields("Profile", vec![
        ("bio", "String"),
        ("user", "User"), // Circular reference
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Should handle circular references without infinite loop
    assert!(used_types.contains_key("User"));
    assert!(used_types.contains_key("Profile"));
    assert_eq!(used_types.len(), 2);
}

#[test]
fn test_collect_used_types_no_nested_dependencies() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_params("simple_command", vec![("data", "SimpleStruct")])];
    
    let mut structs = HashMap::new();
    
    // Struct with only primitive fields
    structs.insert("SimpleStruct".to_string(), create_struct_with_fields("SimpleStruct", vec![
        ("name", "String"),
        ("count", "i32"),
        ("active", "bool"),
    ]));
    
    // Other struct that should not be collected
    structs.insert("UnusedStruct".to_string(), create_struct_with_fields("UnusedStruct", vec![
        ("data", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Should only collect SimpleStruct since it has no nested struct dependencies
    assert!(used_types.contains_key("SimpleStruct"));
    assert!(!used_types.contains_key("UnusedStruct"));
    assert_eq!(used_types.len(), 1);
}

#[test]
fn test_collect_used_types_reference_types() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_params("process_data", vec![("cinema", "&Cinema")])];
    
    let mut structs = HashMap::new();
    structs.insert("Cinema".to_string(), create_struct_with_fields("Cinema", vec![
        ("movies", "Vec<Movie>"),
    ]));
    
    structs.insert("Movie".to_string(), create_struct_with_fields("Movie", vec![
        ("title", "&str"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Should handle reference types and collect nested dependencies
    assert!(used_types.contains_key("Cinema"));
    assert!(used_types.contains_key("Movie"));
    assert_eq!(used_types.len(), 2);
}

#[test]
fn test_collect_used_types_complex_nested_wrappers() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_return("complex_return", "Option<Vec<Result<Cinema, String>>>")];
    
    let mut structs = HashMap::new();
    structs.insert("Cinema".to_string(), create_struct_with_fields("Cinema", vec![
        ("movies", "HashMap<String, Movie>"),
    ]));
    
    structs.insert("Movie".to_string(), create_struct_with_fields("Movie", vec![
        ("actors", "Vec<Actor>"),
    ]));
    
    structs.insert("Actor".to_string(), create_struct_with_fields("Actor", vec![
        ("name", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Should unwrap complex nested types and collect all dependencies
    assert!(used_types.contains_key("Cinema"));
    assert!(used_types.contains_key("Movie"));
    assert!(used_types.contains_key("Actor"));
    assert_eq!(used_types.len(), 3);
}

#[test]
fn test_collect_used_types_preserves_struct_data() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_params("get_cinema", vec![("cinema", "Cinema")])];
    
    let mut structs = HashMap::new();
    let original_cinema = create_struct_with_fields("Cinema", vec![
        ("name", "String"),
        ("movies", "Vec<Movie>"),
        ("capacity", "i32"),
    ]);
    
    let original_movie = create_struct_with_fields("Movie", vec![
        ("title", "String"),
        ("year", "i32"),
    ]);
    
    structs.insert("Cinema".to_string(), original_cinema.clone());
    structs.insert("Movie".to_string(), original_movie.clone());
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Verify that the collected structs maintain their original data
    let collected_cinema = used_types.get("Cinema").unwrap();
    let collected_movie = used_types.get("Movie").unwrap();
    
    assert_eq!(collected_cinema.name, original_cinema.name);
    assert_eq!(collected_cinema.fields.len(), original_cinema.fields.len());
    assert_eq!(collected_cinema.fields[0].name, "name");
    assert_eq!(collected_cinema.fields[1].name, "movies");
    assert_eq!(collected_cinema.fields[2].name, "capacity");
    
    assert_eq!(collected_movie.name, original_movie.name);
    assert_eq!(collected_movie.fields.len(), original_movie.fields.len());
    assert_eq!(collected_movie.fields[0].name, "title");
    assert_eq!(collected_movie.fields[1].name, "year");
}

#[test]
fn test_collect_used_types_hashmap_values() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_params("process_data", vec![("data", "HashMap<String, Cinema>")])];
    
    let mut structs = HashMap::new();
    structs.insert("Cinema".to_string(), create_struct_with_fields("Cinema", vec![
        ("movies", "Vec<Movie>"),
    ]));
    
    structs.insert("Movie".to_string(), create_struct_with_fields("Movie", vec![
        ("title", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Both Cinema and Movie should be collected from HashMap value type
    assert!(used_types.contains_key("Cinema"));
    assert!(used_types.contains_key("Movie"));
    assert_eq!(used_types.len(), 2);
}

#[test]
fn test_collect_used_types_hashmap_keys_and_values() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_params("process_mapping", vec![("mapping", "HashMap<UserId, UserProfile>")])];
    
    let mut structs = HashMap::new();
    structs.insert("UserId".to_string(), create_struct_with_fields("UserId", vec![
        ("id", "i32"),
    ]));
    
    structs.insert("UserProfile".to_string(), create_struct_with_fields("UserProfile", vec![
        ("name", "String"),
        ("preferences", "Vec<Preference>"),
    ]));
    
    structs.insert("Preference".to_string(), create_struct_with_fields("Preference", vec![
        ("key", "String"),
        ("value", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Should collect UserId, UserProfile, and Preference
    assert!(used_types.contains_key("UserId"));
    assert!(used_types.contains_key("UserProfile"));
    assert!(used_types.contains_key("Preference"));
    assert_eq!(used_types.len(), 3);
}

#[test]
fn test_collect_used_types_tuple_parameters() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_params("process_tuple", vec![("data", "(User, Product, Order)")])];
    
    let mut structs = HashMap::new();
    structs.insert("User".to_string(), create_struct_with_fields("User", vec![
        ("name", "String"),
    ]));
    
    structs.insert("Product".to_string(), create_struct_with_fields("Product", vec![
        ("name", "String"),
        ("category", "Category"),
    ]));
    
    structs.insert("Order".to_string(), create_struct_with_fields("Order", vec![
        ("id", "i32"),
    ]));
    
    structs.insert("Category".to_string(), create_struct_with_fields("Category", vec![
        ("name", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Should collect User, Product, Order, and Category (nested from Product)
    assert!(used_types.contains_key("User"));
    assert!(used_types.contains_key("Product"));
    assert!(used_types.contains_key("Order"));
    assert!(used_types.contains_key("Category"));
    assert_eq!(used_types.len(), 4);
}

#[test]
fn test_collect_used_types_mixed_wrappers() {
    let generator = BaseGenerator::new();
    
    // Test complex nested structure with multiple wrapper types
    let commands = vec![create_command_with_return("complex_operation", "Result<Option<Vec<HashMap<String, Cinema>>>, AppError>")];
    
    let mut structs = HashMap::new();
    structs.insert("Cinema".to_string(), create_struct_with_fields("Cinema", vec![
        ("movies", "Vec<Movie>"),
    ]));
    
    structs.insert("Movie".to_string(), create_struct_with_fields("Movie", vec![
        ("title", "String"),
    ]));
    
    structs.insert("AppError".to_string(), create_struct_with_fields("AppError", vec![
        ("message", "String"),
        ("code", "i32"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Should unwrap all nested wrappers and collect Cinema, Movie, and AppError
    assert!(used_types.contains_key("Cinema"));
    assert!(used_types.contains_key("Movie"));
    assert!(used_types.contains_key("AppError"));
    assert_eq!(used_types.len(), 3);
}
