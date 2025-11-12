use crate::analysis::type_resolver::TypeResolver;
use crate::models::ChannelInfo;
use std::path::Path;
use syn::spanned::Spanned;
use syn::{
    AngleBracketedGenericArguments, FnArg, GenericArgument, ItemFn, PathArguments, PathSegment,
    Type,
};

/// Parser for Tauri Channel parameters in command signatures
#[derive(Debug)]
pub struct ChannelParser;

impl ChannelParser {
    pub fn new() -> Self {
        Self
    }

    /// Extract channel parameters from a command function signature
    /// Looks for parameters with type Channel<T>, tauri::ipc::Channel<T>, etc.
    pub fn extract_channels_from_command(
        &self,
        func: &ItemFn,
        command_name: &str,
        file_path: &Path,
        type_resolver: &mut TypeResolver,
    ) -> Result<Vec<ChannelInfo>, Box<dyn std::error::Error>> {
        let mut channels = Vec::new();

        // Iterate through function parameters
        for input in &func.sig.inputs {
            if let FnArg::Typed(pat_type) = input {
                // Extract parameter name
                let param_name = if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    pat_ident.ident.to_string()
                } else {
                    continue;
                };

                // Check if this parameter is a Channel type
                if let Some(message_type) = self.extract_channel_message_type(&pat_type.ty) {
                    // Map Rust type to TypeScript
                    let typescript_message_type =
                        type_resolver.map_rust_type_to_typescript(&message_type);

                    // Get line number from parameter span
                    let line_number = pat_type.ty.span().start().line;

                    channels.push(ChannelInfo {
                        parameter_name: param_name,
                        message_type: message_type.clone(),
                        typescript_message_type,
                        command_name: command_name.to_string(),
                        file_path: file_path.to_string_lossy().to_string(),
                        line_number,
                    });
                }
            }
        }

        Ok(channels)
    }

    /// Extract the message type T from Channel<T>
    /// Returns Some(T) if the type is a Channel, None otherwise
    fn extract_channel_message_type(&self, ty: &Type) -> Option<String> {
        match ty {
            Type::Path(type_path) => {
                // Get the last segment of the path (e.g., "Channel" from "tauri::ipc::Channel")
                let last_segment = type_path.path.segments.last()?;

                // Check if this is a Channel type
                if self.is_channel_segment(last_segment, &type_path.path.segments) {
                    // Extract the generic argument T from Channel<T>
                    if let PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                        args,
                        ..
                    }) = &last_segment.arguments
                    {
                        if let Some(GenericArgument::Type(inner_type)) = args.first() {
                            return Some(self.type_to_string(inner_type));
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Check if a path segment represents a Channel type
    /// Handles: Channel, tauri::ipc::Channel, tauri::Channel
    fn is_channel_segment(
        &self,
        segment: &PathSegment,
        all_segments: &syn::punctuated::Punctuated<PathSegment, syn::Token![::]>,
    ) -> bool {
        let segment_name = segment.ident.to_string();

        // Must be named "Channel"
        if segment_name != "Channel" {
            return false;
        }

        // Accept bare "Channel" or qualified paths like "tauri::ipc::Channel"
        if all_segments.len() == 1 {
            // Bare "Channel"
            return true;
        }

        // Check for tauri::* namespace
        if all_segments.len() >= 2 {
            let first = all_segments.first().unwrap().ident.to_string();
            if first == "tauri" {
                return true;
            }
        }

        false
    }

    /// Convert a syn::Type to a string representation
    /// Simplified version - handles common cases
    #[allow(clippy::only_used_in_recursion)]
    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::Path(type_path) => {
                let segments: Vec<String> = type_path
                    .path
                    .segments
                    .iter()
                    .map(|seg| {
                        let ident = seg.ident.to_string();
                        // Handle generic arguments if present
                        if let PathArguments::AngleBracketed(args) = &seg.arguments {
                            let generic_args: Vec<String> = args
                                .args
                                .iter()
                                .filter_map(|arg| {
                                    if let GenericArgument::Type(t) = arg {
                                        Some(self.type_to_string(t))
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            if !generic_args.is_empty() {
                                return format!("{}<{}>", ident, generic_args.join(", "));
                            }
                        }
                        ident
                    })
                    .collect();
                segments.join("::")
            }
            Type::Reference(type_ref) => {
                format!("&{}", self.type_to_string(&type_ref.elem))
            }
            Type::Tuple(tuple) => {
                if tuple.elems.is_empty() {
                    "()".to_string()
                } else {
                    let types: Vec<String> =
                        tuple.elems.iter().map(|t| self.type_to_string(t)).collect();
                    format!("({})", types.join(", "))
                }
            }
            Type::Array(arr) => {
                format!("[{}]", self.type_to_string(&arr.elem))
            }
            _ => "unknown".to_string(),
        }
    }
}

impl Default for ChannelParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_detect_simple_channel() {
        let parser = ChannelParser::new();
        let ty: Type = parse_quote!(Channel<ProgressUpdate>);
        let result = parser.extract_channel_message_type(&ty);
        assert_eq!(result, Some("ProgressUpdate".to_string()));
    }

    #[test]
    fn test_detect_qualified_channel() {
        let parser = ChannelParser::new();
        let ty: Type = parse_quote!(tauri::ipc::Channel<DownloadEvent>);
        let result = parser.extract_channel_message_type(&ty);
        assert_eq!(result, Some("DownloadEvent".to_string()));
    }

    #[test]
    fn test_detect_channel_with_primitive() {
        let parser = ChannelParser::new();
        let ty: Type = parse_quote!(Channel<i32>);
        let result = parser.extract_channel_message_type(&ty);
        assert_eq!(result, Some("i32".to_string()));
    }

    #[test]
    fn test_non_channel_type() {
        let parser = ChannelParser::new();
        let ty: Type = parse_quote!(String);
        let result = parser.extract_channel_message_type(&ty);
        assert_eq!(result, None);
    }

    #[test]
    fn test_channel_with_complex_type() {
        let parser = ChannelParser::new();
        let ty: Type = parse_quote!(Channel<Vec<String>>);
        let result = parser.extract_channel_message_type(&ty);
        assert_eq!(result, Some("Vec<String>".to_string()));
    }

    #[test]
    fn test_type_to_string() {
        let parser = ChannelParser::new();

        let ty: Type = parse_quote!(String);
        assert_eq!(parser.type_to_string(&ty), "String");

        let ty: Type = parse_quote!(Vec<i32>);
        assert_eq!(parser.type_to_string(&ty), "Vec<i32>");

        let ty: Type = parse_quote!(Option<User>);
        assert_eq!(parser.type_to_string(&ty), "Option<User>");
    }
}
