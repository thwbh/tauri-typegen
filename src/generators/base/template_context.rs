use crate::generators::base::type_visitor::TypeVisitor;
use crate::models::{ChannelInfo, CommandInfo, EventInfo, FieldInfo, ParameterInfo};
use serde::{Deserialize, Serialize};

/// Add "types." prefix to custom types for use in function signatures (return types, parameters)
/// This is only needed for references in commands.ts, not in type definitions themselves
fn prefix_custom_types(ts_type: &str) -> String {
    // Handle primitives - no prefix needed
    if matches!(
        ts_type,
        "void" | "string" | "number" | "boolean" | "any" | "unknown" | "null" | "undefined"
    ) {
        return ts_type.to_string();
    }

    // Handle arrays: CustomType[] -> types.CustomType[]
    if let Some(base_type) = ts_type.strip_suffix("[]") {
        if matches!(base_type, "string" | "number" | "boolean" | "void") {
            return ts_type.to_string();
        }
        return format!("types.{}[]", base_type);
    }

    // Handle Record/Map - they contain types but the structure itself doesn't need prefix
    if ts_type.starts_with("Record<") || ts_type.starts_with("Map<") {
        return ts_type.to_string();
    }

    // Handle union with null: CustomType | null -> types.CustomType | null
    if ts_type.ends_with(" | null") {
        let base = ts_type.strip_suffix(" | null").unwrap();
        return format!("{} | null", prefix_custom_types(base));
    }

    // Handle union with undefined: CustomType | undefined -> types.CustomType | undefined
    if ts_type.ends_with(" | undefined") {
        let base = ts_type.strip_suffix(" | undefined").unwrap();
        return format!("{} | undefined", prefix_custom_types(base));
    }

    // Handle tuples [T, U, ...] - keep as is since they're inline
    if ts_type.starts_with('[') && ts_type.ends_with(']') {
        return ts_type.to_string();
    }

    // Custom type - add prefix if not already present
    if ts_type.starts_with("types.") {
        ts_type.to_string()
    } else {
        format!("types.{}", ts_type)
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
        type_resolver: &dyn Fn(&str) -> crate::models::TypeStructure,
    ) -> Self {
        // Use pre-parsed type structure from CommandInfo
        let return_type_ts = visitor.visit_type(&cmd.return_type_structure);
        // Add types. prefix for function signatures
        let return_type_ts = prefix_custom_types(&return_type_ts);

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
        // NO prefix - this is used in type definitions (Params interfaces in types.ts)
        // Prefix is added in command templates for function signatures
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
            .map(|field| FieldContext::from_field_info(field, visitor))
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
        visitor: &V,
        type_resolver: &dyn Fn(&str) -> crate::models::TypeStructure,
    ) -> Self {
        let message_type_structure = type_resolver(&channel.message_type);
        // NO prefix - used in type definitions (Params interfaces)
        // Prefix is added in command templates for function signatures
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
        // Add types. prefix for function signatures (same as commands.ts)
        let typescript_payload_type = prefix_custom_types(&typescript_payload_type);

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
