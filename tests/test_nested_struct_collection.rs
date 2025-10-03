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
    
    // Create a command that uses PetShop struct
    let commands = vec![create_command_with_params("get_petshop", vec![("petshop", "PetShop")])];
    
    // Create PetShop struct with a pets field
    let mut structs = HashMap::new();
    structs.insert("PetShop".to_string(), create_struct_with_fields("PetShop", vec![
        ("name", "String"),
        ("pets", "Vec<Pet>"),
    ]));
    
    // Create Pet struct
    structs.insert("Pet".to_string(), create_struct_with_fields("Pet", vec![
        ("name", "String"),
        ("species", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Both PetShop and Pet should be collected
    assert!(used_types.contains_key("PetShop"));
    assert!(used_types.contains_key("Pet"));
    assert_eq!(used_types.len(), 2);
}

#[test]
fn test_collect_used_types_deeply_nested() {
    let generator = BaseGenerator::new();
    
    // Create a command that uses PetStore struct
    let commands = vec![create_command_with_return("get_petstore", "PetStore")];
    
    let mut structs = HashMap::new();
    
    // PetStore -> PetShop -> Pet -> Owner
    structs.insert("PetStore".to_string(), create_struct_with_fields("PetStore", vec![
        ("name", "String"),
        ("shops", "Vec<PetShop>"),
    ]));
    
    structs.insert("PetShop".to_string(), create_struct_with_fields("PetShop", vec![
        ("id", "i32"),
        ("pets", "Vec<Pet>"),
    ]));
    
    structs.insert("Pet".to_string(), create_struct_with_fields("Pet", vec![
        ("name", "String"),
        ("owners", "Vec<Owner>"),
    ]));
    
    structs.insert("Owner".to_string(), create_struct_with_fields("Owner", vec![
        ("name", "String"),
        ("age", "i32"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // All structs should be collected due to nesting
    assert!(used_types.contains_key("PetStore"));
    assert!(used_types.contains_key("PetShop"));
    assert!(used_types.contains_key("Pet"));
    assert!(used_types.contains_key("Owner"));
    assert_eq!(used_types.len(), 4);
}

#[test]
fn test_collect_used_types_option_wrapper() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_params("update_petshop", vec![("petshop", "Option<PetShop>")])];
    
    let mut structs = HashMap::new();
    structs.insert("PetShop".to_string(), create_struct_with_fields("PetShop", vec![
        ("pets", "Vec<Pet>"),
    ]));
    
    structs.insert("Pet".to_string(), create_struct_with_fields("Pet", vec![
        ("name", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Both PetShop and Pet should be collected even with Option wrapper
    assert!(used_types.contains_key("PetShop"));
    assert!(used_types.contains_key("Pet"));
    assert_eq!(used_types.len(), 2);
}

#[test]
fn test_collect_used_types_result_wrapper() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_return("get_petshop_result", "Result<PetShop, AppError>")];
    
    let mut structs = HashMap::new();
    structs.insert("PetShop".to_string(), create_struct_with_fields("PetShop", vec![
        ("pets", "Vec<Pet>"),
    ]));
    
    structs.insert("Pet".to_string(), create_struct_with_fields("Pet", vec![
        ("name", "String"),
    ]));
    
    structs.insert("AppError".to_string(), create_struct_with_fields("AppError", vec![
        ("message", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // PetShop, Pet, and AppError should all be collected
    assert!(used_types.contains_key("PetShop"));
    assert!(used_types.contains_key("Pet"));
    assert!(used_types.contains_key("AppError"));
    assert_eq!(used_types.len(), 3);
}

#[test]
fn test_collect_used_types_multiple_commands() {
    let generator = BaseGenerator::new();
    
    let commands = vec![
        create_command_with_params("create_petshop", vec![("petshop", "PetShop")]),
        create_command_with_return("get_veterinarian", "Veterinarian"),
    ];
    
    let mut structs = HashMap::new();
    
    // PetShop uses Pet
    structs.insert("PetShop".to_string(), create_struct_with_fields("PetShop", vec![
        ("pets", "Vec<Pet>"),
    ]));
    
    structs.insert("Pet".to_string(), create_struct_with_fields("Pet", vec![
        ("name", "String"),
    ]));
    
    // Veterinarian uses Treatment (separate hierarchy)
    structs.insert("Veterinarian".to_string(), create_struct_with_fields("Veterinarian", vec![
        ("treatments", "Vec<Treatment>"),
    ]));
    
    structs.insert("Treatment".to_string(), create_struct_with_fields("Treatment", vec![
        ("name", "String"),
        ("cost", "f64"),
    ]));
    
    // Unused struct that shouldn't be collected
    structs.insert("UnusedStruct".to_string(), create_struct_with_fields("UnusedStruct", vec![
        ("data", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Should collect PetShop->Pet and Veterinarian->Treatment, but not UnusedStruct
    assert!(used_types.contains_key("PetShop"));
    assert!(used_types.contains_key("Pet"));
    assert!(used_types.contains_key("Veterinarian"));
    assert!(used_types.contains_key("Treatment"));
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
    
    let commands = vec![create_command_with_params("process_data", vec![("petshop", "&PetShop")])];
    
    let mut structs = HashMap::new();
    structs.insert("PetShop".to_string(), create_struct_with_fields("PetShop", vec![
        ("pets", "Vec<Pet>"),
    ]));
    
    structs.insert("Pet".to_string(), create_struct_with_fields("Pet", vec![
        ("name", "&str"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Should handle reference types and collect nested dependencies
    assert!(used_types.contains_key("PetShop"));
    assert!(used_types.contains_key("Pet"));
    assert_eq!(used_types.len(), 2);
}

#[test]
fn test_collect_used_types_complex_nested_wrappers() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_return("complex_return", "Option<Vec<Result<PetShop, String>>>")];
    
    let mut structs = HashMap::new();
    structs.insert("PetShop".to_string(), create_struct_with_fields("PetShop", vec![
        ("pets", "HashMap<String, Pet>"),
    ]));
    
    structs.insert("Pet".to_string(), create_struct_with_fields("Pet", vec![
        ("caretakers", "Vec<Caretaker>"),
    ]));
    
    structs.insert("Caretaker".to_string(), create_struct_with_fields("Caretaker", vec![
        ("name", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Should unwrap complex nested types and collect all dependencies
    assert!(used_types.contains_key("PetShop"));
    assert!(used_types.contains_key("Pet"));
    assert!(used_types.contains_key("Caretaker"));
    assert_eq!(used_types.len(), 3);
}

#[test]
fn test_collect_used_types_preserves_struct_data() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_params("get_petshop", vec![("petshop", "PetShop")])];
    
    let mut structs = HashMap::new();
    let original_petshop = create_struct_with_fields("PetShop", vec![
        ("name", "String"),
        ("pets", "Vec<Pet>"),
        ("capacity", "i32"),
    ]);
    
    let original_pet = create_struct_with_fields("Pet", vec![
        ("name", "String"),
        ("age", "i32"),
    ]);
    
    structs.insert("PetShop".to_string(), original_petshop.clone());
    structs.insert("Pet".to_string(), original_pet.clone());
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Verify that the collected structs maintain their original data
    let collected_petshop = used_types.get("PetShop").unwrap();
    let collected_pet = used_types.get("Pet").unwrap();
    
    assert_eq!(collected_petshop.name, original_petshop.name);
    assert_eq!(collected_petshop.fields.len(), original_petshop.fields.len());
    assert_eq!(collected_petshop.fields[0].name, "name");
    assert_eq!(collected_petshop.fields[1].name, "pets");
    assert_eq!(collected_petshop.fields[2].name, "capacity");
    
    assert_eq!(collected_pet.name, original_pet.name);
    assert_eq!(collected_pet.fields.len(), original_pet.fields.len());
    assert_eq!(collected_pet.fields[0].name, "name");
    assert_eq!(collected_pet.fields[1].name, "age");
}

#[test]
fn test_collect_used_types_hashmap_values() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_params("process_data", vec![("data", "HashMap<String, PetShop>")])];
    
    let mut structs = HashMap::new();
    structs.insert("PetShop".to_string(), create_struct_with_fields("PetShop", vec![
        ("pets", "Vec<Pet>"),
    ]));
    
    structs.insert("Pet".to_string(), create_struct_with_fields("Pet", vec![
        ("name", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Both PetShop and Pet should be collected from HashMap value type
    assert!(used_types.contains_key("PetShop"));
    assert!(used_types.contains_key("Pet"));
    assert_eq!(used_types.len(), 2);
}

#[test]
fn test_collect_used_types_hashmap_keys_and_values() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_params("process_mapping", vec![("mapping", "HashMap<PetId, PetProfile>")])];
    
    let mut structs = HashMap::new();
    structs.insert("PetId".to_string(), create_struct_with_fields("PetId", vec![
        ("id", "i32"),
    ]));
    
    structs.insert("PetProfile".to_string(), create_struct_with_fields("PetProfile", vec![
        ("name", "String"),
        ("attributes", "Vec<PetAttribute>"),
    ]));
    
    structs.insert("PetAttribute".to_string(), create_struct_with_fields("PetAttribute", vec![
        ("key", "String"),
        ("value", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Should collect PetId, PetProfile, and PetAttribute
    assert!(used_types.contains_key("PetId"));
    assert!(used_types.contains_key("PetProfile"));
    assert!(used_types.contains_key("PetAttribute"));
    assert_eq!(used_types.len(), 3);
}

#[test]
fn test_collect_used_types_tuple_parameters() {
    let generator = BaseGenerator::new();
    
    let commands = vec![create_command_with_params("process_tuple", vec![("data", "(Owner, Pet, Appointment)")])];
    
    let mut structs = HashMap::new();
    structs.insert("Owner".to_string(), create_struct_with_fields("Owner", vec![
        ("name", "String"),
    ]));
    
    structs.insert("Pet".to_string(), create_struct_with_fields("Pet", vec![
        ("name", "String"),
        ("breed", "Breed"),
    ]));
    
    structs.insert("Appointment".to_string(), create_struct_with_fields("Appointment", vec![
        ("id", "i32"),
    ]));
    
    structs.insert("Breed".to_string(), create_struct_with_fields("Breed", vec![
        ("name", "String"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Should collect Owner, Pet, Appointment, and Breed (nested from Pet)
    assert!(used_types.contains_key("Owner"));
    assert!(used_types.contains_key("Pet"));
    assert!(used_types.contains_key("Appointment"));
    assert!(used_types.contains_key("Breed"));
    assert_eq!(used_types.len(), 4);
}

#[test]
fn test_collect_used_types_mixed_wrappers() {
    let generator = BaseGenerator::new();
    
    // Test complex nested structure with multiple wrapper types
    let commands = vec![create_command_with_return("complex_operation", "Result<Option<Vec<HashMap<String, PetShop>>>, AppError>")];
    
    let mut structs = HashMap::new();
    structs.insert("PetShop".to_string(), create_struct_with_fields("PetShop", vec![
        ("pets", "Vec<Pet>"),
    ]));
    
    structs.insert("Pet".to_string(), create_struct_with_fields("Pet", vec![
        ("name", "String"),
    ]));
    
    structs.insert("AppError".to_string(), create_struct_with_fields("AppError", vec![
        ("message", "String"),
        ("code", "i32"),
    ]));
    
    let used_types = generator.collect_used_types(&commands, &structs);
    
    // Should unwrap all nested wrappers and collect PetShop, Pet, and AppError
    assert!(used_types.contains_key("PetShop"));
    assert!(used_types.contains_key("Pet"));
    assert!(used_types.contains_key("AppError"));
    assert_eq!(used_types.len(), 3);
}
