use serde::{Deserialize, Serialize};

/// Represents the structure of a type for code generation
/// This allows generators to work with parsed type information instead of string parsing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TypeStructure {
    /// Primitive types: "string", "number", "boolean", "void"
    Primitive(String),

    /// Array/Vec types: Vec<T> -> Array(T)
    Array(Box<TypeStructure>),

    /// Map types: HashMap<K, V>, BTreeMap<K, V> -> Map { key: K, value: V }
    Map {
        key: Box<TypeStructure>,
        value: Box<TypeStructure>,
    },

    /// Set types: HashSet<T>, BTreeSet<T> -> Set(T)
    Set(Box<TypeStructure>),

    /// Tuple types: (T, U, V) -> Tuple([T, U, V])
    Tuple(Vec<TypeStructure>),

    /// Optional types: Option<T> -> Optional(T)
    Optional(Box<TypeStructure>),

    /// Result types: Result<T, E> -> Result(T) (error type ignored for TS)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandInfo {
    pub name: String,
    pub file_path: String,
    pub line_number: usize,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: String, // Rust return type (e.g., "Vec<Banana>")
    /// Structured representation of the return type for generators
    #[serde(default)]
    pub return_type_structure: TypeStructure,
    pub is_async: bool,
    pub channels: Vec<ChannelInfo>,
    /// Serde rename_all attribute: #[serde(rename_all = "...")]
    /// Applied to command function, affects parameter/channel serialization
    #[serde(default)]
    pub serde_rename_all: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterInfo {
    pub name: String,
    pub rust_type: String,
    pub is_optional: bool,
    /// Structured representation of the type for generators
    #[serde(default)]
    pub type_structure: TypeStructure,
    /// Serde rename attribute (optional, for future extensibility)
    /// Parameters are serialized following Tauri/JS conventions (camelCase)
    #[serde(default)]
    pub serde_rename: Option<String>,
}

// New: Struct field information for better type generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    pub file_path: String,
    pub is_enum: bool,
    /// Serde rename_all attribute: #[serde(rename_all = "...")]
    pub serde_rename_all: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldInfo {
    pub name: String,
    pub rust_type: String,
    pub is_optional: bool,
    pub is_public: bool,
    pub validator_attributes: Option<ValidatorAttributes>,
    /// Serde rename attribute: #[serde(rename = "...")]
    pub serde_rename: Option<String>,
    /// Structured representation of the type for generators
    #[serde(default)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventInfo {
    pub event_name: String,
    pub payload_type: String,
    /// Structured representation of the payload type for generators
    #[serde(default)]
    pub payload_type_structure: TypeStructure,
    pub file_path: String,
    pub line_number: usize,
}

// Channel information for streaming data from Rust to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelInfo {
    pub parameter_name: String,
    pub message_type: String,
    pub command_name: String,
    pub file_path: String,
    pub line_number: usize,
    /// Serde rename attribute (optional, for future extensibility)
    /// Channel parameters are serialized following Tauri/JS conventions (camelCase)
    #[serde(default)]
    pub serde_rename: Option<String>,
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
        Self {
            parameter_name: parameter_name.into(),
            message_type: message_type.into(),
            command_name: command_name.into(),
            file_path: file_path.into(),
            line_number,
            serde_rename: None,
        }
    }
}
