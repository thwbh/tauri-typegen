use crate::generators::base::type_visitor::TypeVisitor;
use crate::models::{ChannelInfo, CommandInfo, EventInfo, FieldInfo, ParameterInfo};
use crate::TypeStructure;
use serde::{Deserialize, Serialize};
use serde_rename_rule::RenameRule;

/// Convert an event name to a TypeScript event listener function name
/// Example: "user_login" -> "onUserLogin"
fn event_name_to_function(event_name: &str) -> String {
    format!(
        "on{}",
        apply_naming_convention(event_name, RenameRule::PascalCase)
    )
}

/// Apply serde naming convention transformations
/// Use extension functions provided by `heck`
fn apply_naming_convention(field_name: &str, convention: RenameRule) -> String {
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
/// 3. Otherwise, keep name as-is to match what tauri expects
fn compute_serialized_name(
    field_name: &str,
    field_rename: &Option<String>,
    struct_rename_all: &Option<RenameRule>,
) -> String {
    if let Some(rename) = field_rename {
        // Explicit field-level rename takes precedence
        rename.to_string()
    } else if let Some(convention) = struct_rename_all {
        // Apply struct-level naming convention
        apply_naming_convention(field_name, *convention)
    } else {
        // No serde attributes, keep as-is
        field_name.to_string()
    }
}

/// Compute the TypeScript name for a function based on serde attributes
///
/// Priority
/// 1. Command-level `#[serde(rename_all = "..." )]` applies naming convention
/// 2. Otherwise, use typescript conventions (camelCase for functions)
fn compute_function_name(name: &str, rename_all: &Option<RenameRule>) -> String {
    if let Some(convention) = rename_all {
        apply_naming_convention(name, *convention)
    } else {
        // No serde attributes, use typescript conventions (camelCase for functions)
        apply_naming_convention(name, RenameRule::CamelCase)
    }
}

/// Compute the TypeScript type name (PascalCase) based on serde attributes
///
/// Priority:
/// 1. Struct-level `#[serde(rename_all = "...")]` applies naming convention
/// 2. Otherwise, use typescript conventions (PascalCase for types)
fn compute_type_name(name: &str, rename_all: &Option<RenameRule>) -> String {
    if let Some(convention) = rename_all {
        // Apply naming convention
        apply_naming_convention(name, *convention)
    } else {
        // No serde attributes, use typescript convention (PascalCase for types)
        apply_naming_convention(name, RenameRule::PascalCase)
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
}

impl CommandContext {
    pub fn from_command_info<V: TypeVisitor>(
        cmd: &CommandInfo,
        visitor: &V,
        type_resolver: &dyn Fn(&str) -> TypeStructure,
    ) -> Self {
        // Use pre-parsed type structure from CommandInfo
        let return_type_ts = visitor.visit_type(&cmd.return_type_structure);

        // serde rename_all attribute on command level affects function and type names
        let ts_function_name = compute_function_name(&cmd.name, &cmd.serde_rename_all);
        let ts_type_name = compute_type_name(&cmd.name, &cmd.serde_rename_all);

        Self {
            name: cmd.name.clone(),
            file_path: cmd.file_path.clone(),
            line_number: cmd.line_number,
            parameters: cmd
                .parameters
                .iter()
                .map(|p| ParameterContext::from_parameter_info(p, &cmd.serde_rename_all, visitor))
                .collect(),
            return_type: cmd.return_type.clone(),
            return_type_ts,
            is_async: cmd.is_async,
            channels: cmd
                .channels
                .iter()
                .map(|c| {
                    ChannelContext::from_channel_info(
                        c,
                        &cmd.serde_rename_all,
                        visitor,
                        type_resolver,
                    )
                })
                .collect(),
            ts_function_name,
            ts_type_name,
        }
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
}

impl ParameterContext {
    pub fn from_parameter_info<V: TypeVisitor>(
        param: &ParameterInfo,
        command_rename_all: &Option<RenameRule>,
        visitor: &V,
    ) -> Self {
        // NO prefix - this is used in type definitions (Params interfaces in types.ts)
        // Prefix is added in command templates for function signatures
        let typescript_type = visitor.visit_type(&param.type_structure);

        let serialized_name =
            compute_serialized_name(&param.name, &param.serde_rename, command_rename_all);

        Self {
            name: param.name.clone(),
            rust_type: param.rust_type.clone(),
            typescript_type,
            is_optional: param.is_optional,
            serialized_name,
            type_structure: param.type_structure.clone(),
        }
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
}

impl FieldContext {
    pub fn from_field_info<V: TypeVisitor>(
        field: &FieldInfo,
        struct_rename_all: &Option<RenameRule>,
        visitor: &V,
    ) -> Self {
        let typescript_type = visitor.visit_type(&field.type_structure);

        // Compute serialized name from serde attributes
        let serialized_name =
            compute_serialized_name(&field.name, &field.serde_rename, struct_rename_all);

        Self {
            name: field.name.clone(),
            rust_type: field.rust_type.clone(),
            typescript_type,
            is_optional: field.is_optional,
            serialized_name,
            validator_attributes: field.validator_attributes.clone(),
            type_structure: field.type_structure.clone(),
        }
    }
}

/// Template context wrapper for StructInfo with computed TypeScript-specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructContext {
    pub name: String,
    pub fields: Vec<FieldContext>,
    pub is_enum: bool,
}

impl StructContext {
    pub fn from_struct_info<V: TypeVisitor>(
        name: &str,
        struct_info: &crate::models::StructInfo,
        visitor: &V,
    ) -> Self {
        let field_contexts: Vec<FieldContext> = struct_info
            .fields
            .iter()
            .map(|field| {
                FieldContext::from_field_info(field, &struct_info.serde_rename_all, visitor)
            })
            .collect();

        Self {
            name: name.to_string(),
            fields: field_contexts,
            is_enum: struct_info.is_enum,
        }
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
}

impl ChannelContext {
    pub fn from_channel_info<V: TypeVisitor>(
        channel: &ChannelInfo,
        command_rename_all: &Option<RenameRule>,
        visitor: &V,
        type_resolver: &dyn Fn(&str) -> TypeStructure,
    ) -> Self {
        let message_type_structure = type_resolver(&channel.message_type);
        let typescript_message_type = visitor.visit_type(&message_type_structure);

        let serialized_parameter_name = compute_serialized_name(
            &channel.parameter_name,
            &channel.serde_rename,
            command_rename_all,
        );

        Self {
            parameter_name: channel.parameter_name.clone(),
            message_type: channel.message_type.clone(),
            typescript_message_type,
            command_name: channel.command_name.clone(),
            file_path: channel.file_path.clone(),
            line_number: channel.line_number,
            serialized_parameter_name,
        }
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
}

impl EventContext {
    pub fn from_event_info<V: TypeVisitor>(
        event: &EventInfo,
        visitor: &V,
        type_resolver: &dyn Fn(&str) -> TypeStructure,
    ) -> Self {
        let payload_type_structure = type_resolver(&event.payload_type);
        let typescript_payload_type = visitor.visit_type(&payload_type_structure);
        let ts_function_name = event_name_to_function(&event.event_name);

        Self {
            event_name: event.event_name.clone(),
            payload_type: event.payload_type.clone(),
            typescript_payload_type,
            file_path: event.file_path.clone(),
            line_number: event.line_number,
            ts_function_name,
        }
    }
}
