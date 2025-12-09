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

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PingRequest {
    pub value: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PingResponse {
    pub value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateModelsRequest {
    pub project_path: String,
    pub output_path: Option<String>,
    pub validation_library: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateModelsResponse {
    pub generated_files: Vec<String>,
    pub commands_found: i32,
    pub types_generated: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeCommandsRequest {
    pub project_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeCommandsResponse {
    pub commands: Vec<CommandInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
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
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterInfo {
    pub name: String,
    pub rust_type: String,
    pub typescript_type: String,
    pub is_optional: bool,
    /// Structured representation of the type for generators
    #[serde(default)]
    pub type_structure: TypeStructure,
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
    /// The serialized name after applying serde rename/rename_all transformations.
    /// If None, the field name will be used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serialized_name: Option<String>,
    /// Structured representation of the type for generators
    #[serde(default)]
    pub type_structure: TypeStructure,
}

impl FieldInfo {
    /// Get the effective serialized name, falling back to name if serialized_name is None
    pub fn get_serialized_name(&self) -> &str {
        self.serialized_name.as_deref().unwrap_or(&self.name)
    }
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
}
