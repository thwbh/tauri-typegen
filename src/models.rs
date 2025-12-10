use heck::{ToLowerCamelCase, ToUpperCamelCase};
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
    pub return_type: String,    // Rust return type (e.g., "Vec<Banana>")
    pub return_type_ts: String, // TypeScript return type (e.g., "Banana[]")
    pub is_async: bool,
    pub channels: Vec<ChannelInfo>,
    /// TypeScript function name (camelCase by convention)
    /// Example: "getUserById" for Rust function "get_user_by_id"
    pub ts_function_name: String,
    /// TypeScript type name prefix (PascalCase by convention)
    /// Example: "GetUserById" used for GetUserByIdParams, GetUserByIdParamsSchema, etc.
    pub ts_type_name: String,
}

impl CommandInfo {
    /// Helper for tests: Create a CommandInfo with computed TypeScript names
    #[doc(hidden)]
    pub fn new_for_test(
        name: impl Into<String>,
        file_path: impl Into<String>,
        line_number: usize,
        parameters: Vec<ParameterInfo>,
        return_type: impl Into<String>,
        return_type_ts: impl Into<String>,
        is_async: bool,
        channels: Vec<ChannelInfo>,
    ) -> Self {
        let name = name.into();
        Self {
            name: name.clone(),
            file_path: file_path.into(),
            line_number,
            parameters,
            return_type: return_type.into(),
            return_type_ts: return_type_ts.into(),
            is_async,
            channels,
            ts_function_name: to_camel_case(&name),
            ts_type_name: to_pascal_case(&name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterInfo {
    pub name: String,
    pub rust_type: String,
    pub typescript_type: String,
    pub is_optional: bool,
    /// Structured representation of the type for generators
    #[serde(default)]
    pub type_structure: TypeStructure,
    /// Serialized name for the parameter (typically camelCase for Tauri)
    pub serialized_name: String,
}

/// Convert snake_case to camelCase using heck
pub(crate) fn to_camel_case(s: &str) -> String {
    s.to_lower_camel_case()
}

/// Convert snake_case to PascalCase using heck
pub(crate) fn to_pascal_case(s: &str) -> String {
    s.to_upper_camel_case()
}

/// Convert event-name to onEventName pattern using heck
/// Examples: "download-started" -> "onDownloadStarted", "user-logged-in" -> "onUserLoggedIn"
pub(crate) fn event_name_to_function(event_name: &str) -> String {
    format!("on{}", event_name.to_upper_camel_case())
}

// New: Struct field information for better type generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    pub file_path: String,
    pub is_enum: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldInfo {
    pub name: String,
    pub rust_type: String,
    pub typescript_type: String,
    pub is_optional: bool,
    pub is_public: bool,
    pub validator_attributes: Option<ValidatorAttributes>,
    /// The serialized name after applying serde rename/rename_all transformations
    pub serialized_name: String,
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
    pub typescript_payload_type: String,
    pub file_path: String,
    pub line_number: usize,
    /// TypeScript listener function name (onEventName pattern)
    /// Example: "download-started" -> "onDownloadStarted"
    pub ts_function_name: String,
}

// Channel information for streaming data from Rust to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelInfo {
    pub parameter_name: String,
    pub message_type: String,
    pub typescript_message_type: String,
    pub command_name: String,
    pub file_path: String,
    pub line_number: usize,
    /// Serialized parameter name for TypeScript (camelCase by convention)
    pub serialized_parameter_name: String,
}

impl ChannelInfo {
    /// Helper for tests: Create a ChannelInfo with computed serialized name
    #[doc(hidden)]
    pub fn new_for_test(
        parameter_name: impl Into<String>,
        message_type: impl Into<String>,
        typescript_message_type: impl Into<String>,
        command_name: impl Into<String>,
        file_path: impl Into<String>,
        line_number: usize,
    ) -> Self {
        let param_name = parameter_name.into();
        Self {
            parameter_name: param_name.clone(),
            message_type: message_type.into(),
            typescript_message_type: typescript_message_type.into(),
            command_name: command_name.into(),
            file_path: file_path.into(),
            line_number,
            serialized_parameter_name: to_camel_case(&param_name),
        }
    }
}
