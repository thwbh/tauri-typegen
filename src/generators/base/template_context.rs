use crate::generators::base::type_visitor::TypeVisitor;
use crate::models::{
    ChannelInfo, CommandInfo, EnumVariantInfo, EnumVariantKind, EventInfo, FieldInfo, ParameterInfo,
};
use crate::{GenerateConfig, TypeStructure};
use serde::{Deserialize, Serialize};
use serde_rename_rule::RenameRule;

/// Trait for contexts that provide naming convention functionality
pub trait NamingContext {
    /// Get the config reference
    fn config(&self) -> &GenerateConfig;

    /// Convert an event name to a TypeScript event listener function name
    /// Example: "user_login" -> "onUserLogin", "user-login" -> "onUserLogin"
    fn event_name_to_function(&self, event_name: &str) -> String {
        // Normalize kebab-case to snake_case since serde_rename_rule expects snake_case
        let normalized = event_name.replace('-', "_");
        format!(
            "on{}",
            self.apply_naming_convention(&normalized, RenameRule::PascalCase)
        )
    }

    /// Apply serde naming convention transformations
    fn apply_naming_convention(&self, field_name: &str, convention: RenameRule) -> String {
        convention.apply_to_field(field_name)
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
    config: GenerateConfig,
}

impl NamingContext for CommandContext {
    fn config(&self) -> &GenerateConfig {
        &self.config
    }
}

impl CommandContext {
    /// Create a new CommandContext with the given config
    pub fn new(config: &GenerateConfig) -> Self {
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
        // Use visit_type_for_interface to get TypeScript types (not Zod schemas)
        let return_type_ts = visitor.visit_type_for_interface(&cmd.return_type_structure);

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
    config: GenerateConfig,
}

impl NamingContext for ParameterContext {
    fn config(&self) -> &GenerateConfig {
        &self.config
    }
}

impl ParameterContext {
    /// Create a new ParameterContext with the given config
    pub fn new(config: &GenerateConfig) -> Self {
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
    pub typescript_type: String, // Computed field (for vanilla TS or zod schemas)
    pub is_optional: bool,
    pub serialized_name: String,
    pub validator_attributes: Option<crate::models::ValidatorAttributes>,
    #[serde(skip_serializing)]
    pub type_structure: TypeStructure, // Keep for internal use but don't expose to templates
    #[serde(skip)]
    config: GenerateConfig,
}

impl NamingContext for FieldContext {
    fn config(&self) -> &GenerateConfig {
        &self.config
    }
}

impl FieldContext {
    /// Create a new FieldContext with the given config
    pub fn new(config: &GenerateConfig) -> Self {
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

/// Template context wrapper for enum variants with computed TypeScript-specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnumVariantContext {
    /// Original variant name
    pub name: String,
    /// Serialized name (after applying serde rename rules)
    pub serialized_name: String,
    /// Variant kind: "unit", "tuple", or "struct"
    pub kind: String,
    /// TypeScript types for tuple variant fields (e.g., ["number", "number"] for Move(i32, i32))
    pub tuple_types: Vec<String>,
    /// Zod schemas for tuple variant fields (e.g., ["z.number()", "z.number()"])
    pub tuple_zod_types: Vec<String>,
    /// Field contexts for struct variant fields
    pub struct_fields: Vec<FieldContext>,
    #[serde(skip)]
    config: GenerateConfig,
}

impl NamingContext for EnumVariantContext {
    fn config(&self) -> &GenerateConfig {
        &self.config
    }
}

impl EnumVariantContext {
    /// Create a new EnumVariantContext with the given config
    pub fn new(config: &GenerateConfig) -> Self {
        Self {
            name: String::new(),
            serialized_name: String::new(),
            kind: String::new(),
            tuple_types: Vec::new(),
            tuple_zod_types: Vec::new(),
            struct_fields: Vec::new(),
            config: config.clone(),
        }
    }

    /// Populate this context from an EnumVariantInfo
    pub fn from_variant_info<V: TypeVisitor>(
        mut self,
        variant: &EnumVariantInfo,
        enum_rename_all: &Option<RenameRule>,
        visitor: &V,
    ) -> Self {
        // Compute serialized name from serde attributes
        let serialized_name =
            self.compute_field_name(&variant.name, &variant.serde_rename, enum_rename_all);

        self.name = variant.name.clone();
        self.serialized_name = serialized_name;

        match &variant.kind {
            EnumVariantKind::Unit => {
                self.kind = "unit".to_string();
            }
            EnumVariantKind::Tuple(types) => {
                self.kind = "tuple".to_string();
                self.tuple_types = types
                    .iter()
                    .map(|t| visitor.visit_type_for_interface(t))
                    .collect();
                self.tuple_zod_types = types.iter().map(|t| visitor.visit_type(t)).collect();
            }
            EnumVariantKind::Struct(fields) => {
                self.kind = "struct".to_string();
                self.struct_fields = fields
                    .iter()
                    .map(|field| {
                        // Struct variant fields don't inherit enum's rename_all
                        // They use their own serde attributes
                        FieldContext::new(&self.config).from_field_info(field, &None, visitor)
                    })
                    .collect();
            }
        }

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
    /// Whether this is a simple enum (all unit variants) - can use string literal union
    pub is_simple_enum: bool,
    /// The discriminator tag name for complex enums (default: "type")
    pub discriminator_tag: String,
    /// Enum variants with full type information (only for enums)
    pub enum_variants: Vec<EnumVariantContext>,
    #[serde(skip)]
    config: GenerateConfig,
}

impl NamingContext for StructContext {
    fn config(&self) -> &GenerateConfig {
        &self.config
    }
}

impl StructContext {
    /// Create a new StructContext with the given config
    pub fn new(config: &GenerateConfig) -> Self {
        Self {
            name: String::new(),
            fields: Vec::new(),
            is_enum: false,
            is_simple_enum: false,
            discriminator_tag: "type".to_string(),
            enum_variants: Vec::new(),
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

        // Build enum variant contexts if this is an enum with enum_variants
        let enum_variants: Vec<EnumVariantContext> = struct_info
            .enum_variants
            .as_ref()
            .map(|variants| {
                variants
                    .iter()
                    .map(|v| {
                        EnumVariantContext::new(&self.config).from_variant_info(
                            v,
                            &struct_info.serde_rename_all,
                            visitor,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        self.name = name.to_string();
        self.fields = field_contexts;
        self.is_enum = struct_info.is_enum;
        self.is_simple_enum = struct_info.is_simple_enum();
        self.discriminator_tag = struct_info.discriminator_tag().to_string();
        self.enum_variants = enum_variants;

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
    config: GenerateConfig,
}

impl NamingContext for ChannelContext {
    fn config(&self) -> &GenerateConfig {
        &self.config
    }
}

impl ChannelContext {
    /// Create a new ChannelContext with the given config
    pub fn new(config: &GenerateConfig) -> Self {
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
        let typescript_message_type = visitor.visit_type_for_interface(&message_type_structure);

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
    config: GenerateConfig,
}

impl NamingContext for EventContext {
    fn config(&self) -> &GenerateConfig {
        &self.config
    }
}

impl EventContext {
    /// Create a new EventContext with the given config
    pub fn new(config: &GenerateConfig) -> Self {
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
        let typescript_payload_type = visitor.visit_type_for_interface(&payload_type_structure);

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

#[cfg(test)]
mod tests {
    use GenerateConfig;

    use super::*;
    use serde_rename_rule::RenameRule;

    // Mock config for testing
    fn mock_config() -> GenerateConfig {
        GenerateConfig::default()
    }

    fn mock_config_with_snake_case() -> GenerateConfig {
        GenerateConfig {
            default_parameter_case: "snake_case".to_string(),
            default_field_case: "snake_case".to_string(),
            ..Default::default()
        }
    }

    // Mock NamingContext for testing
    struct MockContext {
        config: GenerateConfig,
    }

    impl NamingContext for MockContext {
        fn config(&self) -> &GenerateConfig {
            &self.config
        }
    }

    #[test]
    fn test_apply_naming_convention_camel_case() {
        let ctx = MockContext {
            config: mock_config(),
        };

        assert_eq!(
            ctx.apply_naming_convention("user_name", RenameRule::CamelCase),
            "userName"
        );
        assert_eq!(
            ctx.apply_naming_convention("first_last_name", RenameRule::CamelCase),
            "firstLastName"
        );
    }

    #[test]
    fn test_apply_naming_convention_snake_case() {
        let ctx = MockContext {
            config: mock_config(),
        };

        // SnakeCase rule assumes input is in Rust format (snake_case)
        // and keeps it as snake_case for serialization
        assert_eq!(
            ctx.apply_naming_convention("user_name", RenameRule::SnakeCase),
            "user_name"
        );
        assert_eq!(
            ctx.apply_naming_convention("first_last_name", RenameRule::SnakeCase),
            "first_last_name"
        );
    }

    #[test]
    fn test_apply_naming_convention_pascal_case() {
        let ctx = MockContext {
            config: mock_config(),
        };

        assert_eq!(
            ctx.apply_naming_convention("user_name", RenameRule::PascalCase),
            "UserName"
        );
        assert_eq!(
            ctx.apply_naming_convention("userName", RenameRule::PascalCase),
            "UserName"
        );
    }

    #[test]
    fn test_apply_naming_convention_screaming_snake_case() {
        let ctx = MockContext {
            config: mock_config(),
        };

        // ScreamingSnakeCase rule assumes input is in Rust format (snake_case)
        assert_eq!(
            ctx.apply_naming_convention("user_name", RenameRule::ScreamingSnakeCase),
            "USER_NAME"
        );
        assert_eq!(
            ctx.apply_naming_convention("first_last_name", RenameRule::ScreamingSnakeCase),
            "FIRST_LAST_NAME"
        );
    }

    #[test]
    fn test_compute_field_name_with_explicit_rename() {
        let ctx = MockContext {
            config: mock_config(),
        };

        // Field-level rename takes precedence
        let result = ctx.compute_field_name(
            "user_id",
            &Some("id".to_string()),
            &Some(RenameRule::CamelCase),
        );
        assert_eq!(result, "id");
    }

    #[test]
    fn test_compute_field_name_with_struct_rename_all() {
        let ctx = MockContext {
            config: mock_config(),
        };

        // Struct-level rename_all applies
        let result = ctx.compute_field_name("user_id", &None, &Some(RenameRule::CamelCase));
        assert_eq!(result, "userId");
    }

    #[test]
    fn test_compute_field_name_with_config_default() {
        let ctx = MockContext {
            config: mock_config(),
        };

        // No serde attributes, use config default (snake_case to match serde's default)
        let result = ctx.compute_field_name("user_id", &None, &None);
        assert_eq!(result, "user_id");
    }

    #[test]
    fn test_compute_field_name_with_snake_case_default() {
        let ctx = MockContext {
            config: mock_config_with_snake_case(),
        };

        // Config default is snake_case - input should be Rust field name (snake_case)
        let result = ctx.compute_field_name("user_id", &None, &None);
        assert_eq!(result, "user_id");
    }

    #[test]
    fn test_compute_parameter_name_with_explicit_rename() {
        let ctx = MockContext {
            config: mock_config(),
        };

        // Parameter-level rename takes precedence
        let result = ctx.compute_parameter_name(
            "order_id",
            &Some("id".to_string()),
            &Some(RenameRule::SnakeCase),
        );
        assert_eq!(result, "id");
    }

    #[test]
    fn test_compute_parameter_name_with_command_rename_all() {
        let ctx = MockContext {
            config: mock_config(),
        };

        // Command-level rename_all applies
        let result = ctx.compute_parameter_name("order_id", &None, &Some(RenameRule::SnakeCase));
        assert_eq!(result, "order_id");
    }

    #[test]
    fn test_compute_parameter_name_with_config_default() {
        let ctx = MockContext {
            config: mock_config(),
        };

        // No serde attributes, use config default
        let result = ctx.compute_parameter_name("order_id", &None, &None);
        assert_eq!(result, "orderId");
    }

    #[test]
    fn test_compute_function_name_ignores_rename_all() {
        let ctx = MockContext {
            config: mock_config(),
        };

        // Function names always use camelCase, ignore rename_all
        let result = ctx.compute_function_name("get_user", &Some(RenameRule::SnakeCase));
        assert_eq!(result, "getUser");

        let result = ctx.compute_function_name("get_user", &None);
        assert_eq!(result, "getUser");
    }

    #[test]
    fn test_compute_type_name_ignores_rename_all() {
        let ctx = MockContext {
            config: mock_config(),
        };

        // Type names always use PascalCase, ignore rename_all
        let result = ctx.compute_type_name("user_profile", &Some(RenameRule::SnakeCase));
        assert_eq!(result, "UserProfile");

        let result = ctx.compute_type_name("user_profile", &None);
        assert_eq!(result, "UserProfile");
    }

    #[test]
    fn test_event_name_to_function() {
        let ctx = MockContext {
            config: mock_config(),
        };

        // Event names convert to "on" + PascalCase
        // Handles both snake_case and kebab-case inputs
        assert_eq!(ctx.event_name_to_function("user_login"), "onUserLogin");
        assert_eq!(ctx.event_name_to_function("data_update"), "onDataUpdate");
        assert_eq!(
            ctx.event_name_to_function("status_changed"),
            "onStatusChanged"
        );

        // Kebab-case is normalized to snake_case before conversion
        assert_eq!(
            ctx.event_name_to_function("progress-update"),
            "onProgressUpdate"
        );
        assert_eq!(
            ctx.event_name_to_function("user-notification"),
            "onUserNotification"
        );
    }

    #[test]
    fn test_serde_rename_priority_order() {
        let ctx = MockContext {
            config: mock_config_with_snake_case(),
        };

        // Priority: explicit rename > container rename_all > config default

        // 1. Explicit rename wins
        let result = ctx.compute_field_name(
            "field_name",
            &Some("customName".to_string()),
            &Some(RenameRule::CamelCase),
        );
        assert_eq!(result, "customName");

        // 2. Container rename_all beats config
        let result = ctx.compute_field_name("field_name", &None, &Some(RenameRule::CamelCase));
        assert_eq!(result, "fieldName"); // Not field_name from config

        // 3. Config default is last resort - input should be Rust field name (snake_case)
        let result = ctx.compute_field_name("field_name", &None, &None);
        assert_eq!(result, "field_name"); // Config has snake_case, so it stays as-is
    }

    #[test]
    fn test_command_context_builder_pattern() {
        let config = mock_config();
        let ctx = CommandContext::new(&config);

        assert_eq!(ctx.name, "");
        assert_eq!(ctx.parameters.len(), 0);
        assert_eq!(ctx.config.default_parameter_case, "camelCase");
    }

    #[test]
    fn test_parameter_context_builder_pattern() {
        let config = mock_config();
        let ctx = ParameterContext::new(&config);

        assert_eq!(ctx.name, "");
        assert_eq!(ctx.rust_type, "");
        assert!(matches!(ctx.type_structure, TypeStructure::Primitive(_)));
    }

    #[test]
    fn test_field_context_builder_pattern() {
        let config = mock_config();
        let ctx = FieldContext::new(&config);

        assert_eq!(ctx.name, "");
        assert!(!ctx.is_optional);
        assert!(ctx.validator_attributes.is_none());
    }

    #[test]
    fn test_struct_context_builder_pattern() {
        let config = mock_config();
        let ctx = StructContext::new(&config);

        assert_eq!(ctx.name, "");
        assert_eq!(ctx.fields.len(), 0);
        assert!(!ctx.is_enum);
        assert!(!ctx.is_simple_enum);
        assert_eq!(ctx.discriminator_tag, "type");
        assert_eq!(ctx.enum_variants.len(), 0);
    }

    #[test]
    fn test_enum_variant_context_builder_pattern() {
        let config = mock_config();
        let ctx = EnumVariantContext::new(&config);

        assert_eq!(ctx.name, "");
        assert_eq!(ctx.serialized_name, "");
        assert_eq!(ctx.kind, "");
        assert_eq!(ctx.tuple_types.len(), 0);
        assert_eq!(ctx.tuple_zod_types.len(), 0);
        assert_eq!(ctx.struct_fields.len(), 0);
    }

    #[test]
    fn test_channel_context_builder_pattern() {
        let config = mock_config();
        let ctx = ChannelContext::new(&config);

        assert_eq!(ctx.parameter_name, "");
        assert_eq!(ctx.message_type, "");
        assert_eq!(ctx.line_number, 0);
    }

    #[test]
    fn test_event_context_builder_pattern() {
        let config = mock_config();
        let ctx = EventContext::new(&config);

        assert_eq!(ctx.event_name, "");
        assert_eq!(ctx.payload_type, "");
        assert_eq!(ctx.ts_function_name, "");
    }
}
