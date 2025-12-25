use serde::{Deserialize, Serialize};
use serde_rename_rule::RenameRule;

/// Represents the structure of a type for code generation
/// This allows generators to work with parsed type information instead of string parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TypeStructure {
    /// Primitive types: "string", "number", "boolean", "void"
    Primitive(String),

    /// Array/Vec types: `Vec<T>` -> `Array(T)`
    Array(Box<TypeStructure>),

    /// Map types: `HashMap<K, V>`, `BTreeMap<K, V>` -> `Map { key: K, value: V }`
    Map {
        key: Box<TypeStructure>,
        value: Box<TypeStructure>,
    },

    /// Set types: `HashSet<T>`, `BTreeSet<T>` -> `Set(T)`
    Set(Box<TypeStructure>),

    /// Tuple types: `(T, U, V)` -> `Tuple([T, U, V])`
    Tuple(Vec<TypeStructure>),

    /// Optional types: `Option<T>` -> `Optional(T)`
    Optional(Box<TypeStructure>),

    /// Result types: `Result<T, E>` -> `Result(T)` (error type ignored for TS)
    Result(Box<TypeStructure>),

    /// Custom/User-defined types
    Custom(String),
}

impl Default for TypeStructure {
    fn default() -> Self {
        // Default to string for test compatibility
        TypeStructure::Primitive("string".to_string())
    }
}

pub struct CommandInfo {
    pub name: String,
    pub file_path: String,
    pub line_number: usize,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: String, // Rust return type (e.g., "Vec<Banana>")
    /// Structured representation of the return type for generators
    pub return_type_structure: TypeStructure,
    pub is_async: bool,
    pub channels: Vec<ChannelInfo>,
    /// Serde rename_all attribute: #[serde(rename_all = "...")]
    /// Applied to command function, affects parameter/channel serialization
    pub serde_rename_all: Option<RenameRule>,
}

impl CommandInfo {
    /// Helper for tests: Create a CommandInfo
    #[doc(hidden)]
    pub fn new_for_test(
        name: impl Into<String>,
        file_path: impl Into<String>,
        line_number: usize,
        parameters: Vec<ParameterInfo>,
        return_type: impl Into<String>,
        is_async: bool,
        channels: Vec<ChannelInfo>,
    ) -> Self {
        use crate::analysis::type_resolver::TypeResolver;
        let return_type_str = return_type.into();
        let type_resolver = TypeResolver::new();
        let return_type_structure = type_resolver.parse_type_structure(&return_type_str);

        Self {
            name: name.into(),
            file_path: file_path.into(),
            line_number,
            parameters,
            return_type: return_type_str,
            return_type_structure,
            is_async,
            channels,
            serde_rename_all: None,
        }
    }
}

pub struct ParameterInfo {
    pub name: String,
    pub rust_type: String,
    pub is_optional: bool,
    /// Structured representation of the type for generators
    pub type_structure: TypeStructure,
    /// Serde rename attribute (optional, for future extensibility)
    /// Parameters are serialized following Tauri/JS conventions (camelCase)
    pub serde_rename: Option<String>,
}

#[derive(Clone, Debug)]
pub struct StructInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    pub file_path: String,
    pub is_enum: bool,
    /// Serde rename_all attribute: #[serde(rename_all = "...")]
    pub serde_rename_all: Option<RenameRule>,
}

#[derive(Clone, Debug)]
pub struct FieldInfo {
    pub name: String,
    pub rust_type: String,
    pub is_optional: bool,
    pub is_public: bool,
    pub validator_attributes: Option<ValidatorAttributes>,
    /// Serde rename attribute: #[serde(rename = "...")]
    pub serde_rename: Option<String>,
    /// Structured representation of the type for generators
    pub type_structure: TypeStructure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorAttributes {
    pub length: Option<LengthConstraint>,
    pub range: Option<RangeConstraint>,
    pub email: bool,
    pub url: bool,
    pub custom_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LengthConstraint {
    pub min: Option<u64>,
    pub max: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RangeConstraint {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub message: Option<String>,
}

// Event information for frontend event listeners
pub struct EventInfo {
    pub event_name: String,
    pub payload_type: String,
    /// Structured representation of the payload type for generators
    pub payload_type_structure: TypeStructure,
    pub file_path: String,
    pub line_number: usize,
}

// Channel information for streaming data from Rust to frontend
#[derive(Clone)]
pub struct ChannelInfo {
    pub parameter_name: String,
    pub message_type: String,
    pub command_name: String,
    pub file_path: String,
    pub line_number: usize,
    /// Serde rename attribute (optional, for future extensibility)
    /// Channel parameters are serialized following Tauri/JS conventions (camelCase)
    pub serde_rename: Option<String>,
    /// Structured representation of the message type for generators
    pub message_type_structure: TypeStructure,
}

impl ChannelInfo {
    /// Helper for tests: Create a ChannelInfo
    #[doc(hidden)]
    pub fn new_for_test(
        parameter_name: impl Into<String>,
        message_type: impl Into<String>,
        command_name: impl Into<String>,
        file_path: impl Into<String>,
        line_number: usize,
    ) -> Self {
        let message_type_str = message_type.into();
        Self {
            parameter_name: parameter_name.into(),
            message_type: message_type_str.clone(),
            command_name: command_name.into(),
            file_path: file_path.into(),
            line_number,
            serde_rename: None,
            // Parse message_type into TypeStructure for tests
            message_type_structure: crate::analysis::type_resolver::TypeResolver::new()
                .parse_type_structure(&message_type_str),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TypeStructure tests
    mod type_structure {
        use super::*;

        #[test]
        fn test_default_is_string_primitive() {
            let default_type = TypeStructure::default();
            match default_type {
                TypeStructure::Primitive(name) => assert_eq!(name, "string"),
                _ => panic!("Default should be Primitive(\"string\")"),
            }
        }

        #[test]
        fn test_primitive_variants() {
            let types = vec!["string", "number", "boolean", "void"];
            for type_name in types {
                let primitive = TypeStructure::Primitive(type_name.to_string());
                match primitive {
                    TypeStructure::Primitive(name) => assert_eq!(name, type_name),
                    _ => panic!("Should be Primitive variant"),
                }
            }
        }

        #[test]
        fn test_array_wraps_inner_type() {
            let inner = TypeStructure::Primitive("number".to_string());
            let array = TypeStructure::Array(Box::new(inner));

            match array {
                TypeStructure::Array(boxed) => match *boxed {
                    TypeStructure::Primitive(name) => assert_eq!(name, "number"),
                    _ => panic!("Inner should be Primitive"),
                },
                _ => panic!("Should be Array variant"),
            }
        }

        #[test]
        fn test_map_has_key_and_value() {
            let key = TypeStructure::Primitive("string".to_string());
            let value = TypeStructure::Primitive("number".to_string());
            let map = TypeStructure::Map {
                key: Box::new(key),
                value: Box::new(value),
            };

            match map {
                TypeStructure::Map { key, value } => match (*key, *value) {
                    (TypeStructure::Primitive(k), TypeStructure::Primitive(v)) => {
                        assert_eq!(k, "string");
                        assert_eq!(v, "number");
                    }
                    _ => panic!("Key and value should be Primitives"),
                },
                _ => panic!("Should be Map variant"),
            }
        }

        #[test]
        fn test_set_wraps_inner_type() {
            let inner = TypeStructure::Primitive("string".to_string());
            let set = TypeStructure::Set(Box::new(inner));

            match set {
                TypeStructure::Set(boxed) => match *boxed {
                    TypeStructure::Primitive(name) => assert_eq!(name, "string"),
                    _ => panic!("Inner should be Primitive"),
                },
                _ => panic!("Should be Set variant"),
            }
        }

        #[test]
        fn test_tuple_with_multiple_types() {
            let types = vec![
                TypeStructure::Primitive("string".to_string()),
                TypeStructure::Primitive("number".to_string()),
                TypeStructure::Primitive("boolean".to_string()),
            ];
            let tuple = TypeStructure::Tuple(types);

            match tuple {
                TypeStructure::Tuple(types) => {
                    assert_eq!(types.len(), 3);
                    match &types[0] {
                        TypeStructure::Primitive(name) => assert_eq!(name, "string"),
                        _ => panic!("First type should be string"),
                    }
                }
                _ => panic!("Should be Tuple variant"),
            }
        }

        #[test]
        fn test_empty_tuple() {
            let tuple = TypeStructure::Tuple(vec![]);
            match tuple {
                TypeStructure::Tuple(types) => assert_eq!(types.len(), 0),
                _ => panic!("Should be Tuple variant"),
            }
        }

        #[test]
        fn test_optional_wraps_inner_type() {
            let inner = TypeStructure::Custom("User".to_string());
            let optional = TypeStructure::Optional(Box::new(inner));

            match optional {
                TypeStructure::Optional(boxed) => match *boxed {
                    TypeStructure::Custom(name) => assert_eq!(name, "User"),
                    _ => panic!("Inner should be Custom"),
                },
                _ => panic!("Should be Optional variant"),
            }
        }

        #[test]
        fn test_result_wraps_success_type() {
            let success = TypeStructure::Primitive("string".to_string());
            let result = TypeStructure::Result(Box::new(success));

            match result {
                TypeStructure::Result(boxed) => match *boxed {
                    TypeStructure::Primitive(name) => assert_eq!(name, "string"),
                    _ => panic!("Inner should be Primitive"),
                },
                _ => panic!("Should be Result variant"),
            }
        }

        #[test]
        fn test_custom_type() {
            let custom = TypeStructure::Custom("UserProfile".to_string());
            match custom {
                TypeStructure::Custom(name) => assert_eq!(name, "UserProfile"),
                _ => panic!("Should be Custom variant"),
            }
        }

        #[test]
        fn test_nested_structures() {
            // Vec<Option<HashMap<String, User>>>
            let user = TypeStructure::Custom("User".to_string());
            let map = TypeStructure::Map {
                key: Box::new(TypeStructure::Primitive("string".to_string())),
                value: Box::new(user),
            };
            let optional = TypeStructure::Optional(Box::new(map));
            let array = TypeStructure::Array(Box::new(optional));

            match array {
                TypeStructure::Array(arr_inner) => match *arr_inner {
                    TypeStructure::Optional(opt_inner) => match *opt_inner {
                        TypeStructure::Map { key, value } => match (*key, *value) {
                            (TypeStructure::Primitive(k), TypeStructure::Custom(v)) => {
                                assert_eq!(k, "string");
                                assert_eq!(v, "User");
                            }
                            _ => panic!("Map types incorrect"),
                        },
                        _ => panic!("Should be Map"),
                    },
                    _ => panic!("Should be Optional"),
                },
                _ => panic!("Should be Array"),
            }
        }

        #[test]
        fn test_clone_type_structure() {
            let original = TypeStructure::Primitive("string".to_string());
            let cloned = original.clone();

            match (original, cloned) {
                (TypeStructure::Primitive(o), TypeStructure::Primitive(c)) => {
                    assert_eq!(o, c);
                }
                _ => panic!("Clone should maintain variant"),
            }
        }

        #[test]
        fn test_serialize_deserialize_primitive() {
            let primitive = TypeStructure::Primitive("number".to_string());
            let json = serde_json::to_string(&primitive).unwrap();
            let deserialized: TypeStructure = serde_json::from_str(&json).unwrap();

            match deserialized {
                TypeStructure::Primitive(name) => assert_eq!(name, "number"),
                _ => panic!("Should deserialize to Primitive"),
            }
        }

        #[test]
        fn test_serialize_deserialize_complex() {
            let complex = TypeStructure::Array(Box::new(TypeStructure::Optional(Box::new(
                TypeStructure::Custom("User".to_string()),
            ))));

            let json = serde_json::to_string(&complex).unwrap();
            let deserialized: TypeStructure = serde_json::from_str(&json).unwrap();

            match deserialized {
                TypeStructure::Array(arr) => match *arr {
                    TypeStructure::Optional(opt) => match *opt {
                        TypeStructure::Custom(name) => assert_eq!(name, "User"),
                        _ => panic!("Should be Custom"),
                    },
                    _ => panic!("Should be Optional"),
                },
                _ => panic!("Should be Array"),
            }
        }
    }

    // ValidatorAttributes tests
    mod validator_attributes {
        use super::*;

        #[test]
        fn test_length_constraint() {
            let length = LengthConstraint {
                min: Some(5),
                max: Some(100),
                message: Some("Invalid length".to_string()),
            };

            assert_eq!(length.min, Some(5));
            assert_eq!(length.max, Some(100));
            assert_eq!(length.message, Some("Invalid length".to_string()));
        }

        #[test]
        fn test_range_constraint() {
            let range = RangeConstraint {
                min: Some(0.0),
                max: Some(10.5),
                message: Some("Out of range".to_string()),
            };

            assert_eq!(range.min, Some(0.0));
            assert_eq!(range.max, Some(10.5));
            assert_eq!(range.message, Some("Out of range".to_string()));
        }

        #[test]
        fn test_validator_attributes_email() {
            let validator = ValidatorAttributes {
                length: None,
                range: None,
                email: true,
                url: false,
                custom_message: None,
            };

            assert!(validator.email);
            assert!(!validator.url);
        }

        #[test]
        fn test_validator_attributes_with_length() {
            let validator = ValidatorAttributes {
                length: Some(LengthConstraint {
                    min: Some(1),
                    max: Some(50),
                    message: None,
                }),
                range: None,
                email: false,
                url: false,
                custom_message: None,
            };

            assert!(validator.length.is_some());
            let length = validator.length.unwrap();
            assert_eq!(length.min, Some(1));
            assert_eq!(length.max, Some(50));
        }

        #[test]
        fn test_serialize_validator_attributes() {
            let validator = ValidatorAttributes {
                length: Some(LengthConstraint {
                    min: Some(5),
                    max: Some(100),
                    message: None,
                }),
                range: None,
                email: true,
                url: false,
                custom_message: Some("Custom error".to_string()),
            };

            let json = serde_json::to_string(&validator).unwrap();
            let deserialized: ValidatorAttributes = serde_json::from_str(&json).unwrap();

            assert!(deserialized.email);
            assert_eq!(
                deserialized.custom_message,
                Some("Custom error".to_string())
            );
            assert!(deserialized.length.is_some());
        }

        #[test]
        fn test_validator_attributes_clone() {
            let original = ValidatorAttributes {
                length: None,
                range: Some(RangeConstraint {
                    min: Some(0.0),
                    max: Some(1.0),
                    message: None,
                }),
                email: false,
                url: true,
                custom_message: None,
            };

            let cloned = original.clone();
            assert!(cloned.url);
            assert!(cloned.range.is_some());
        }
    }

    // CommandInfo tests
    mod command_info {
        use super::*;

        #[test]
        fn test_new_for_test_creates_valid_command() {
            let params = vec![];
            let channels = vec![];

            let cmd = CommandInfo::new_for_test(
                "greet",
                "src/main.rs",
                10,
                params,
                "String",
                false,
                channels,
            );

            assert_eq!(cmd.name, "greet");
            assert_eq!(cmd.file_path, "src/main.rs");
            assert_eq!(cmd.line_number, 10);
            assert_eq!(cmd.return_type, "String");
            assert!(!cmd.is_async);
            assert!(cmd.serde_rename_all.is_none());
        }

        #[test]
        fn test_new_for_test_parses_return_type_structure() {
            let cmd = CommandInfo::new_for_test(
                "get_users",
                "src/api.rs",
                20,
                vec![],
                "Vec<String>",
                true,
                vec![],
            );

            // Should parse Vec<String> into Array(Primitive("string"))
            match cmd.return_type_structure {
                TypeStructure::Array(inner) => match *inner {
                    TypeStructure::Primitive(name) => assert_eq!(name, "string"),
                    _ => panic!("Should be string primitive"),
                },
                _ => panic!("Should be Array"),
            }
            assert!(cmd.is_async);
        }

        #[test]
        fn test_command_with_parameters() {
            let param = ParameterInfo {
                name: "user_id".to_string(),
                rust_type: "String".to_string(),
                is_optional: false,
                type_structure: TypeStructure::Primitive("string".to_string()),
                serde_rename: None,
            };

            let cmd = CommandInfo::new_for_test(
                "get_user",
                "src/api.rs",
                30,
                vec![param],
                "User",
                false,
                vec![],
            );

            assert_eq!(cmd.parameters.len(), 1);
            assert_eq!(cmd.parameters[0].name, "user_id");
            assert_eq!(cmd.parameters[0].rust_type, "String");
        }

        #[test]
        fn test_command_with_channels() {
            let channel = ChannelInfo::new_for_test(
                "progress",
                "u32",
                "download_file",
                "src/download.rs",
                40,
            );

            let cmd = CommandInfo::new_for_test(
                "download_file",
                "src/download.rs",
                40,
                vec![],
                "Result<(), String>",
                true,
                vec![channel],
            );

            assert_eq!(cmd.channels.len(), 1);
            assert_eq!(cmd.channels[0].parameter_name, "progress");
            assert_eq!(cmd.channels[0].message_type, "u32");
        }
    }

    // ChannelInfo tests
    mod channel_info {
        use super::*;

        #[test]
        fn test_new_for_test_creates_valid_channel() {
            let channel =
                ChannelInfo::new_for_test("updates", "String", "subscribe", "src/events.rs", 50);

            assert_eq!(channel.parameter_name, "updates");
            assert_eq!(channel.message_type, "String");
            assert_eq!(channel.command_name, "subscribe");
            assert_eq!(channel.file_path, "src/events.rs");
            assert_eq!(channel.line_number, 50);
            assert!(channel.serde_rename.is_none());
        }

        #[test]
        fn test_channel_parses_message_type_structure() {
            let channel =
                ChannelInfo::new_for_test("data", "Vec<u32>", "stream_data", "src/stream.rs", 60);

            // Should parse Vec<u32> into Array(Primitive("number"))
            match channel.message_type_structure {
                TypeStructure::Array(inner) => match *inner {
                    TypeStructure::Primitive(name) => assert_eq!(name, "number"),
                    _ => panic!("Should be number primitive"),
                },
                _ => panic!("Should be Array"),
            }
        }

        #[test]
        fn test_channel_clone() {
            let original =
                ChannelInfo::new_for_test("status", "bool", "monitor", "src/monitor.rs", 70);

            let cloned = original.clone();
            assert_eq!(cloned.parameter_name, "status");
            assert_eq!(cloned.message_type, "bool");
            assert_eq!(cloned.command_name, "monitor");
        }
    }

    // ParameterInfo tests
    mod parameter_info {
        use super::*;

        #[test]
        fn test_parameter_with_optional_type() {
            let param = ParameterInfo {
                name: "email".to_string(),
                rust_type: "Option<String>".to_string(),
                is_optional: true,
                type_structure: TypeStructure::Optional(Box::new(TypeStructure::Primitive(
                    "string".to_string(),
                ))),
                serde_rename: None,
            };

            assert!(param.is_optional);
            match param.type_structure {
                TypeStructure::Optional(_) => (),
                _ => panic!("Should be Optional"),
            }
        }

        #[test]
        fn test_parameter_with_serde_rename() {
            let param = ParameterInfo {
                name: "user_id".to_string(),
                rust_type: "String".to_string(),
                is_optional: false,
                type_structure: TypeStructure::Primitive("string".to_string()),
                serde_rename: Some("userId".to_string()),
            };

            assert_eq!(param.serde_rename, Some("userId".to_string()));
        }
    }

    // StructInfo tests
    mod struct_info {
        use super::*;

        #[test]
        fn test_struct_with_fields() {
            let field = FieldInfo {
                name: "name".to_string(),
                rust_type: "String".to_string(),
                is_optional: false,
                is_public: true,
                validator_attributes: None,
                serde_rename: None,
                type_structure: TypeStructure::Primitive("string".to_string()),
            };

            let struct_info = StructInfo {
                name: "User".to_string(),
                fields: vec![field],
                file_path: "src/models.rs".to_string(),
                is_enum: false,
                serde_rename_all: None,
            };

            assert_eq!(struct_info.name, "User");
            assert!(!struct_info.is_enum);
            assert_eq!(struct_info.fields.len(), 1);
        }

        #[test]
        fn test_enum_struct() {
            let struct_info = StructInfo {
                name: "Status".to_string(),
                fields: vec![],
                file_path: "src/types.rs".to_string(),
                is_enum: true,
                serde_rename_all: Some(RenameRule::CamelCase),
            };

            assert!(struct_info.is_enum);
            assert!(struct_info.serde_rename_all.is_some());
        }

        #[test]
        fn test_struct_clone() {
            let original = StructInfo {
                name: "Product".to_string(),
                fields: vec![],
                file_path: "src/product.rs".to_string(),
                is_enum: false,
                serde_rename_all: None,
            };

            let cloned = original.clone();
            assert_eq!(cloned.name, "Product");
            assert!(!cloned.is_enum);
        }
    }

    // FieldInfo tests
    mod field_info {
        use super::*;

        #[test]
        fn test_field_with_validator() {
            let validator = ValidatorAttributes {
                length: Some(LengthConstraint {
                    min: Some(1),
                    max: Some(100),
                    message: None,
                }),
                range: None,
                email: false,
                url: false,
                custom_message: None,
            };

            let field = FieldInfo {
                name: "username".to_string(),
                rust_type: "String".to_string(),
                is_optional: false,
                is_public: true,
                validator_attributes: Some(validator),
                serde_rename: None,
                type_structure: TypeStructure::Primitive("string".to_string()),
            };

            assert!(field.validator_attributes.is_some());
            let attrs = field.validator_attributes.unwrap();
            assert!(attrs.length.is_some());
        }

        #[test]
        fn test_private_field() {
            let field = FieldInfo {
                name: "internal_id".to_string(),
                rust_type: "u64".to_string(),
                is_optional: false,
                is_public: false,
                validator_attributes: None,
                serde_rename: None,
                type_structure: TypeStructure::Primitive("number".to_string()),
            };

            assert!(!field.is_public);
        }

        #[test]
        fn test_field_with_serde_rename() {
            let field = FieldInfo {
                name: "created_at".to_string(),
                rust_type: "String".to_string(),
                is_optional: true,
                is_public: true,
                validator_attributes: None,
                serde_rename: Some("createdAt".to_string()),
                type_structure: TypeStructure::Optional(Box::new(TypeStructure::Primitive(
                    "string".to_string(),
                ))),
            };

            assert_eq!(field.serde_rename, Some("createdAt".to_string()));
            assert!(field.is_optional);
        }

        #[test]
        fn test_field_clone() {
            let original = FieldInfo {
                name: "count".to_string(),
                rust_type: "i32".to_string(),
                is_optional: false,
                is_public: true,
                validator_attributes: None,
                serde_rename: None,
                type_structure: TypeStructure::Primitive("number".to_string()),
            };

            let cloned = original.clone();
            assert_eq!(cloned.name, "count");
            assert_eq!(cloned.rust_type, "i32");
        }
    }

    // EventInfo tests
    mod event_info {
        use super::*;

        #[test]
        fn test_event_info_creation() {
            let event = EventInfo {
                event_name: "user-updated".to_string(),
                payload_type: "User".to_string(),
                payload_type_structure: TypeStructure::Custom("User".to_string()),
                file_path: "src/events.rs".to_string(),
                line_number: 100,
            };

            assert_eq!(event.event_name, "user-updated");
            assert_eq!(event.payload_type, "User");
            match event.payload_type_structure {
                TypeStructure::Custom(name) => assert_eq!(name, "User"),
                _ => panic!("Should be Custom type"),
            }
        }

        #[test]
        fn test_event_with_primitive_payload() {
            let event = EventInfo {
                event_name: "progress".to_string(),
                payload_type: "u32".to_string(),
                payload_type_structure: TypeStructure::Primitive("number".to_string()),
                file_path: "src/progress.rs".to_string(),
                line_number: 50,
            };

            match event.payload_type_structure {
                TypeStructure::Primitive(name) => assert_eq!(name, "number"),
                _ => panic!("Should be Primitive type"),
            }
        }
    }
}
