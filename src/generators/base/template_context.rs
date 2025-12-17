use crate::generators::base::type_visitor::TypeVisitor;
use crate::models::{ChannelInfo, CommandInfo, EventInfo, FieldInfo, ParameterInfo};
use crate::TypeStructure;
use serde::{Deserialize, Serialize};
use serde_rename_rule::RenameRule;

/// Trait for contexts that provide naming convention functionality
pub trait NamingContext {
    /// Get the config reference
    fn config(&self) -> &crate::GenerateConfig;

    /// Convert an event name to a TypeScript event listener function name
    /// Example: "user_login" -> "onUserLogin"
    fn event_name_to_function(&self, event_name: &str) -> String {
        format!(
            "on{}",
            self.apply_naming_convention(event_name, RenameRule::PascalCase)
        )
    }

    /// Apply serde naming convention transformations
    fn apply_naming_convention(&self, field_name: &str, convention: RenameRule) -> String {
        match RenameRule::from_rename_all_str(&convention.to_string()) {
            Ok(rule) => rule.apply_to_field(field_name),
            Err(_) => field_name.to_string(),
        }
    }

    /// Compute the serialized name for a field based on serde attributes
    ///
    /// Priority:
    /// 1. Field-level `#[serde(rename = "...")]` takes precedence
    /// 2. Struct-level `#[serde(rename_all = "...")]` applies naming convention
    /// 3. Otherwise, apply default_field_case from config
    fn compute_field_name(
        &self,
        field_name: &str,
        field_rename: &Option<String>,
        struct_rename_all: &Option<RenameRule>,
    ) -> String {
        if let Some(rename) = field_rename {
            // Explicit field-level rename takes precedence
            rename.to_string()
        } else if let Some(convention) = struct_rename_all {
            // Apply struct-level naming convention
            self.apply_naming_convention(field_name, *convention)
        } else {
            // No serde attributes, apply default from config
            let default_case = RenameRule::from_rename_all_str(&self.config().default_field_case)
                .unwrap_or(RenameRule::CamelCase);
            self.apply_naming_convention(field_name, default_case)
        }
    }

    /// Compute the serialized name for a parameter based on serde attributes
    ///
    /// Priority:
    /// 1. Parameter-level `#[serde(rename = "...")]` takes precedence
    /// 2. Command-level `#[serde(rename_all = "...")]` applies naming convention
    /// 3. Otherwise, apply default_parameter_case from config
    fn compute_parameter_name(
        &self,
        param_name: &str,
        param_rename: &Option<String>,
        command_rename_all: &Option<RenameRule>,
    ) -> String {
        if let Some(rename) = param_rename {
            // Explicit parameter-level rename takes precedence
            rename.to_string()
        } else if let Some(convention) = command_rename_all {
            // Apply command-level naming convention
            self.apply_naming_convention(param_name, *convention)
        } else {
            // No serde attributes, apply default from config
            let default_case =
                RenameRule::from_rename_all_str(&self.config().default_parameter_case)
                    .unwrap_or(RenameRule::CamelCase);
            self.apply_naming_convention(param_name, default_case)
        }
    }

    /// Compute the TypeScript name for a function
    ///
    /// Note: Command-level #[serde(rename_all = "...")] affects parameters/channels,
    /// NOT the function name itself. Function names always use TypeScript conventions.
    fn compute_function_name(&self, name: &str, _rename_all: &Option<RenameRule>) -> String {
        // Always use TypeScript conventions (camelCase for functions)
        // Command-level rename_all doesn't affect the function name
        self.apply_naming_convention(name, RenameRule::CamelCase)
    }

    /// Compute the TypeScript type name (PascalCase)
    ///
    /// Note: Command-level #[serde(rename_all = "...")] affects parameters/channels,
    /// NOT the type name itself. Type names always use TypeScript conventions.
    fn compute_type_name(&self, name: &str, _rename_all: &Option<RenameRule>) -> String {
        // Always use TypeScript conventions (PascalCase for types)
        // Command-level rename_all doesn't affect the type name
        self.apply_naming_convention(name, RenameRule::PascalCase)
    }
}

/// Template context wrapper for CommandInfo with computed TypeScript-specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandContext {
    pub name: String,
    pub file_path: String,
    pub line_number: usize,
    pub parameters: Vec<ParameterContext>,
    pub return_type: String,
    pub return_type_ts: String, // Computed field
    pub is_async: bool,
    pub channels: Vec<ChannelContext>,
    pub ts_function_name: String, // Computed field
    pub ts_type_name: String,     // Computed field
    #[serde(skip)]
    config: crate::GenerateConfig,
}

impl NamingContext for CommandContext {
    fn config(&self) -> &crate::GenerateConfig {
        &self.config
    }
}

impl CommandContext {
    /// Create a new CommandContext with the given config
    pub fn new(config: &crate::GenerateConfig) -> Self {
        Self {
            name: String::new(),
            file_path: String::new(),
            line_number: 0,
            parameters: Vec::new(),
            return_type: String::new(),
            return_type_ts: String::new(),
            is_async: false,
            channels: Vec::new(),
            ts_function_name: String::new(),
            ts_type_name: String::new(),
            config: config.clone(),
        }
    }

    /// Populate this context from a CommandInfo
    pub fn from_command_info<V: TypeVisitor>(
        mut self,
        cmd: &CommandInfo,
        visitor: &V,
        type_resolver: &dyn Fn(&str) -> TypeStructure,
    ) -> Self {
        // Use pre-parsed type structure from CommandInfo
        let return_type_ts = visitor.visit_type(&cmd.return_type_structure);

        // Compute TypeScript names using NamingContext trait methods
        let ts_function_name = self.compute_function_name(&cmd.name, &cmd.serde_rename_all);
        let ts_type_name = self.compute_type_name(&cmd.name, &cmd.serde_rename_all);

        // Populate parameters
        let parameters: Vec<ParameterContext> = cmd
            .parameters
            .iter()
            .map(|p| {
                let serialized_name =
                    self.compute_parameter_name(&p.name, &p.serde_rename, &cmd.serde_rename_all);
                ParameterContext::new(&self.config).from_parameter_info(
                    p,
                    &cmd.serde_rename_all,
                    visitor,
                    &serialized_name,
                )
            })
            .collect();

        // Populate channels
        let channels: Vec<ChannelContext> = cmd
            .channels
            .iter()
            .map(|c| {
                let serialized_name = self.compute_parameter_name(
                    &c.parameter_name,
                    &c.serde_rename,
                    &cmd.serde_rename_all,
                );
                ChannelContext::new(&self.config).from_channel_info(
                    c,
                    &cmd.serde_rename_all,
                    visitor,
                    type_resolver,
                    &serialized_name,
                )
            })
            .collect();

        // Update all fields
        self.name = cmd.name.clone();
        self.file_path = cmd.file_path.clone();
        self.line_number = cmd.line_number;
        self.parameters = parameters;
        self.return_type = cmd.return_type.clone();
        self.return_type_ts = return_type_ts;
        self.is_async = cmd.is_async;
        self.channels = channels;
        self.ts_function_name = ts_function_name;
        self.ts_type_name = ts_type_name;

        self
    }
}

/// Template context wrapper for ParameterInfo with computed TypeScript-specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterContext {
    pub name: String,
    pub rust_type: String,
    pub typescript_type: String, // Computed field
    pub is_optional: bool,
    pub serialized_name: String, // Computed field
    pub type_structure: TypeStructure,
    #[serde(skip)]
    config: crate::GenerateConfig,
}

impl NamingContext for ParameterContext {
    fn config(&self) -> &crate::GenerateConfig {
        &self.config
    }
}

impl ParameterContext {
    /// Create a new ParameterContext with the given config
    pub fn new(config: &crate::GenerateConfig) -> Self {
        Self {
            name: String::new(),
            rust_type: String::new(),
            typescript_type: String::new(),
            is_optional: false,
            serialized_name: String::new(),
            type_structure: TypeStructure::default(),
            config: config.clone(),
        }
    }

    /// Populate this context from a ParameterInfo
    pub fn from_parameter_info<V: TypeVisitor>(
        mut self,
        param: &ParameterInfo,
        _command_rename_all: &Option<RenameRule>,
        visitor: &V,
        serialized_name: &str,
    ) -> Self {
        // NO prefix - this is used in type definitions (Params interfaces in types.ts)
        // Prefix is added in command templates for function signatures
        let typescript_type = visitor.visit_type(&param.type_structure);

        self.name = param.name.clone();
        self.rust_type = param.rust_type.clone();
        self.typescript_type = typescript_type;
        self.is_optional = param.is_optional;
        self.serialized_name = serialized_name.to_string();
        self.type_structure = param.type_structure.clone();

        self
    }
}

/// Template context wrapper for FieldInfo with computed TypeScript-specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldContext {
    pub name: String,
    pub rust_type: String,
    pub typescript_type: String, // Computed field
    pub is_optional: bool,
    pub serialized_name: String,
    pub validator_attributes: Option<crate::models::ValidatorAttributes>,
    pub type_structure: TypeStructure,
    #[serde(skip)]
    config: crate::GenerateConfig,
}

impl NamingContext for FieldContext {
    fn config(&self) -> &crate::GenerateConfig {
        &self.config
    }
}

impl FieldContext {
    /// Create a new FieldContext with the given config
    pub fn new(config: &crate::GenerateConfig) -> Self {
        Self {
            name: String::new(),
            rust_type: String::new(),
            typescript_type: String::new(),
            is_optional: false,
            serialized_name: String::new(),
            validator_attributes: None,
            type_structure: TypeStructure::default(),
            config: config.clone(),
        }
    }

    /// Populate this context from a FieldInfo
    pub fn from_field_info<V: TypeVisitor>(
        mut self,
        field: &FieldInfo,
        struct_rename_all: &Option<RenameRule>,
        visitor: &V,
    ) -> Self {
        let typescript_type = visitor.visit_type(&field.type_structure);

        // Compute serialized name from serde attributes using NamingContext trait
        let serialized_name =
            self.compute_field_name(&field.name, &field.serde_rename, struct_rename_all);

        self.name = field.name.clone();
        self.rust_type = field.rust_type.clone();
        self.typescript_type = typescript_type;
        self.is_optional = field.is_optional;
        self.serialized_name = serialized_name;
        self.validator_attributes = field.validator_attributes.clone();
        self.type_structure = field.type_structure.clone();

        self
    }
}

/// Template context wrapper for StructInfo with computed TypeScript-specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructContext {
    pub name: String,
    pub fields: Vec<FieldContext>,
    pub is_enum: bool,
    #[serde(skip)]
    config: crate::GenerateConfig,
}

impl NamingContext for StructContext {
    fn config(&self) -> &crate::GenerateConfig {
        &self.config
    }
}

impl StructContext {
    /// Create a new StructContext with the given config
    pub fn new(config: &crate::GenerateConfig) -> Self {
        Self {
            name: String::new(),
            fields: Vec::new(),
            is_enum: false,
            config: config.clone(),
        }
    }

    /// Populate this context from a StructInfo
    pub fn from_struct_info<V: TypeVisitor>(
        mut self,
        name: &str,
        struct_info: &crate::models::StructInfo,
        visitor: &V,
    ) -> Self {
        let field_contexts: Vec<FieldContext> = struct_info
            .fields
            .iter()
            .map(|field| {
                FieldContext::new(&self.config).from_field_info(
                    field,
                    &struct_info.serde_rename_all,
                    visitor,
                )
            })
            .collect();

        self.name = name.to_string();
        self.fields = field_contexts;
        self.is_enum = struct_info.is_enum;

        self
    }
}

/// Template context wrapper for ChannelInfo with computed TypeScript-specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelContext {
    pub parameter_name: String,
    pub message_type: String,
    pub typescript_message_type: String, // Computed field
    pub command_name: String,
    pub file_path: String,
    pub line_number: usize,
    pub serialized_parameter_name: String, // Computed field
    #[serde(skip)]
    config: crate::GenerateConfig,
}

impl NamingContext for ChannelContext {
    fn config(&self) -> &crate::GenerateConfig {
        &self.config
    }
}

impl ChannelContext {
    /// Create a new ChannelContext with the given config
    pub fn new(config: &crate::GenerateConfig) -> Self {
        Self {
            parameter_name: String::new(),
            message_type: String::new(),
            typescript_message_type: String::new(),
            command_name: String::new(),
            file_path: String::new(),
            line_number: 0,
            serialized_parameter_name: String::new(),
            config: config.clone(),
        }
    }

    /// Populate this context from a ChannelInfo
    pub fn from_channel_info<V: TypeVisitor>(
        mut self,
        channel: &ChannelInfo,
        _command_rename_all: &Option<RenameRule>,
        visitor: &V,
        type_resolver: &dyn Fn(&str) -> TypeStructure,
        serialized_parameter_name: &str,
    ) -> Self {
        let message_type_structure = type_resolver(&channel.message_type);
        let typescript_message_type = visitor.visit_type(&message_type_structure);

        self.parameter_name = channel.parameter_name.clone();
        self.message_type = channel.message_type.clone();
        self.typescript_message_type = typescript_message_type;
        self.command_name = channel.command_name.clone();
        self.file_path = channel.file_path.clone();
        self.line_number = channel.line_number;
        self.serialized_parameter_name = serialized_parameter_name.to_string();

        self
    }
}

/// Template context wrapper for EventInfo with computed TypeScript-specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventContext {
    pub event_name: String,
    pub payload_type: String,
    pub typescript_payload_type: String, // Computed field
    pub file_path: String,
    pub line_number: usize,
    pub ts_function_name: String, // Computed field
    #[serde(skip)]
    config: crate::GenerateConfig,
}

impl NamingContext for EventContext {
    fn config(&self) -> &crate::GenerateConfig {
        &self.config
    }
}

impl EventContext {
    /// Create a new EventContext with the given config
    pub fn new(config: &crate::GenerateConfig) -> Self {
        Self {
            event_name: String::new(),
            payload_type: String::new(),
            typescript_payload_type: String::new(),
            file_path: String::new(),
            line_number: 0,
            ts_function_name: String::new(),
            config: config.clone(),
        }
    }

    /// Populate this context from an EventInfo
    pub fn from_event_info<V: TypeVisitor>(
        mut self,
        event: &EventInfo,
        visitor: &V,
        type_resolver: &dyn Fn(&str) -> TypeStructure,
    ) -> Self {
        let payload_type_structure = type_resolver(&event.payload_type);
        let typescript_payload_type = visitor.visit_type(&payload_type_structure);

        // Use NamingContext trait method
        let ts_function_name = self.event_name_to_function(&event.event_name);

        self.event_name = event.event_name.clone();
        self.payload_type = event.payload_type.clone();
        self.typescript_payload_type = typescript_payload_type;
        self.file_path = event.file_path.clone();
        self.line_number = event.line_number;
        self.ts_function_name = ts_function_name;

        self
    }
}
