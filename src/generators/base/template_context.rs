use crate::generators::base::type_visitor::TypeVisitor;
use crate::models::{ChannelInfo, CommandInfo, EventInfo, FieldInfo, ParameterInfo};
use serde::{Deserialize, Serialize};

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
        type_resolver: &dyn Fn(&str) -> crate::models::TypeStructure,
    ) -> Self {
        let return_type_structure = type_resolver(&cmd.return_type);
        let return_type_ts = visitor.visit_type(&return_type_structure);

        Self {
            name: cmd.name.clone(),
            file_path: cmd.file_path.clone(),
            line_number: cmd.line_number,
            parameters: cmd
                .parameters
                .iter()
                .map(|p| ParameterContext::from_parameter_info(p, visitor))
                .collect(),
            return_type: cmd.return_type.clone(),
            return_type_ts,
            is_async: cmd.is_async,
            channels: cmd
                .channels
                .iter()
                .map(|c| ChannelContext::from_channel_info(c, visitor, type_resolver))
                .collect(),
            ts_function_name: cmd.ts_function_name(),
            ts_type_name: cmd.ts_type_name(),
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
    pub type_structure: crate::models::TypeStructure,
}

impl ParameterContext {
    pub fn from_parameter_info<V: TypeVisitor>(param: &ParameterInfo, visitor: &V) -> Self {
        let typescript_type = visitor.visit_type(&param.type_structure);

        Self {
            name: param.name.clone(),
            rust_type: param.rust_type.clone(),
            typescript_type,
            is_optional: param.is_optional,
            serialized_name: param.serialized_name(),
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
    pub type_structure: crate::models::TypeStructure,
}

impl FieldContext {
    pub fn from_field_info<V: TypeVisitor>(field: &FieldInfo, visitor: &V) -> Self {
        let typescript_type = visitor.visit_type(&field.type_structure);

        Self {
            name: field.name.clone(),
            rust_type: field.rust_type.clone(),
            typescript_type,
            is_optional: field.is_optional,
            serialized_name: field.serialized_name.clone(),
            validator_attributes: field.validator_attributes.clone(),
            type_structure: field.type_structure.clone(),
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
        visitor: &V,
        type_resolver: &dyn Fn(&str) -> crate::models::TypeStructure,
    ) -> Self {
        let message_type_structure = type_resolver(&channel.message_type);
        let typescript_message_type = visitor.visit_type(&message_type_structure);

        Self {
            parameter_name: channel.parameter_name.clone(),
            message_type: channel.message_type.clone(),
            typescript_message_type,
            command_name: channel.command_name.clone(),
            file_path: channel.file_path.clone(),
            line_number: channel.line_number,
            serialized_parameter_name: channel.serialized_parameter_name(),
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
        type_resolver: &dyn Fn(&str) -> crate::models::TypeStructure,
    ) -> Self {
        let payload_type_structure = type_resolver(&event.payload_type);
        let typescript_payload_type = visitor.visit_type(&payload_type_structure);

        Self {
            event_name: event.event_name.clone(),
            payload_type: event.payload_type.clone(),
            typescript_payload_type,
            file_path: event.file_path.clone(),
            line_number: event.line_number,
            ts_function_name: event.ts_function_name(),
        }
    }
}
