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
                    // Get line number from parameter span
                    let line_number = pat_type.ty.span().start().line;

                    // Parse message type into TypeStructure
                    let message_type_structure = type_resolver.parse_type_structure(&message_type);

                    channels.push(ChannelInfo {
                        parameter_name: param_name,
                        message_type: message_type.clone(),
                        command_name: command_name.to_string(),
                        file_path: file_path.to_string_lossy().to_string(),
                        line_number,
                        serde_rename: None,
                        message_type_structure,
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
                            return Some(Self::type_to_string(inner_type));
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
    fn type_to_string(ty: &Type) -> String {
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
                                        Some(Self::type_to_string(t))
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
                format!("&{}", Self::type_to_string(&type_ref.elem))
            }
            Type::Tuple(tuple) => {
                if tuple.elems.is_empty() {
                    "()".to_string()
                } else {
                    let types: Vec<String> = tuple.elems.iter().map(Self::type_to_string).collect();
                    format!("({})", types.join(", "))
                }
            }
            Type::Array(arr) => {
                format!("[{}]", Self::type_to_string(&arr.elem))
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
    use std::path::Path;
    use syn::{parse_quote, ItemFn};

    fn parser() -> ChannelParser {
        ChannelParser::new()
    }

    fn type_resolver() -> TypeResolver {
        TypeResolver::new()
    }

    mod initialization {
        use super::*;

        #[test]
        fn test_new_creates_parser() {
            let parser = ChannelParser::new();
            assert!(format!("{:?}", parser).contains("ChannelParser"));
        }

        #[test]
        fn test_default_creates_parser() {
            let parser = ChannelParser::default();
            assert!(format!("{:?}", parser).contains("ChannelParser"));
        }
    }

    mod channel_detection {
        use super::*;

        #[test]
        fn test_detect_simple_channel() {
            let parser = parser();
            let ty: Type = parse_quote!(Channel<ProgressUpdate>);
            let result = parser.extract_channel_message_type(&ty);
            assert_eq!(result, Some("ProgressUpdate".to_string()));
        }

        #[test]
        fn test_detect_qualified_channel() {
            let parser = parser();
            let ty: Type = parse_quote!(tauri::ipc::Channel<DownloadEvent>);
            let result = parser.extract_channel_message_type(&ty);
            assert_eq!(result, Some("DownloadEvent".to_string()));
        }

        #[test]
        fn test_detect_tauri_channel() {
            let parser = parser();
            let ty: Type = parse_quote!(tauri::Channel<Message>);
            let result = parser.extract_channel_message_type(&ty);
            assert_eq!(result, Some("Message".to_string()));
        }

        #[test]
        fn test_detect_channel_with_primitive() {
            let parser = parser();
            let ty: Type = parse_quote!(Channel<i32>);
            let result = parser.extract_channel_message_type(&ty);
            assert_eq!(result, Some("i32".to_string()));
        }

        #[test]
        fn test_non_channel_type() {
            let parser = parser();
            let ty: Type = parse_quote!(String);
            let result = parser.extract_channel_message_type(&ty);
            assert_eq!(result, None);
        }

        #[test]
        fn test_channel_with_complex_type() {
            let parser = parser();
            let ty: Type = parse_quote!(Channel<Vec<String>>);
            let result = parser.extract_channel_message_type(&ty);
            assert_eq!(result, Some("Vec<String>".to_string()));
        }

        #[test]
        fn test_non_tauri_qualified_channel() {
            let parser = parser();
            let ty: Type = parse_quote!(my_lib::Channel<String>);
            let result = parser.extract_channel_message_type(&ty);
            // Should not match - not tauri namespace
            assert_eq!(result, None);
        }

        #[test]
        fn test_channel_without_generic() {
            let parser = parser();
            let ty: Type = parse_quote!(Channel);
            let result = parser.extract_channel_message_type(&ty);
            assert_eq!(result, None);
        }

        #[test]
        fn test_other_generic_type() {
            let parser = parser();
            let ty: Type = parse_quote!(Handler<String>);
            let result = parser.extract_channel_message_type(&ty);
            assert_eq!(result, None);
        }
    }

    mod type_to_string_conversion {
        use super::*;

        #[test]
        fn test_simple_type() {
            let ty: Type = parse_quote!(String);
            assert_eq!(ChannelParser::type_to_string(&ty), "String");
        }

        #[test]
        fn test_generic_type() {
            let ty: Type = parse_quote!(Vec<i32>);
            assert_eq!(ChannelParser::type_to_string(&ty), "Vec<i32>");
        }

        #[test]
        fn test_option_type() {
            let ty: Type = parse_quote!(Option<User>);
            assert_eq!(ChannelParser::type_to_string(&ty), "Option<User>");
        }

        #[test]
        fn test_nested_generic() {
            let ty: Type = parse_quote!(Vec<Option<String>>);
            assert_eq!(ChannelParser::type_to_string(&ty), "Vec<Option<String>>");
        }

        #[test]
        fn test_multiple_generics() {
            let ty: Type = parse_quote!(HashMap<String, i32>);
            assert_eq!(ChannelParser::type_to_string(&ty), "HashMap<String, i32>");
        }

        #[test]
        fn test_reference_type() {
            let ty: Type = parse_quote!(&String);
            assert_eq!(ChannelParser::type_to_string(&ty), "&String");
        }

        #[test]
        fn test_reference_with_generic() {
            let ty: Type = parse_quote!(&Vec<i32>);
            assert_eq!(ChannelParser::type_to_string(&ty), "&Vec<i32>");
        }

        #[test]
        fn test_tuple_type() {
            let ty: Type = parse_quote!((String, i32));
            assert_eq!(ChannelParser::type_to_string(&ty), "(String, i32)");
        }

        #[test]
        fn test_empty_tuple() {
            let ty: Type = parse_quote!(());
            assert_eq!(ChannelParser::type_to_string(&ty), "()");
        }

        #[test]
        fn test_slice_type() {
            let ty: Type = parse_quote!(&[i32]);
            // Slice types are references to arrays
            assert_eq!(ChannelParser::type_to_string(&ty), "&unknown");
        }

        #[test]
        fn test_tuple_with_multiple_elements() {
            let ty: Type = parse_quote!((String, i32, bool));
            assert_eq!(ChannelParser::type_to_string(&ty), "(String, i32, bool)");
        }

        #[test]
        fn test_qualified_path() {
            let ty: Type = parse_quote!(std::string::String);
            assert_eq!(ChannelParser::type_to_string(&ty), "std::string::String");
        }
    }

    mod extract_channels_from_command {
        use super::*;

        #[test]
        fn test_extract_no_channels() {
            let parser = parser();
            let mut resolver = type_resolver();
            let func: ItemFn = parse_quote! {
                #[tauri::command]
                fn greet(name: String) -> String {
                    format!("Hello {}", name)
                }
            };

            let result = parser.extract_channels_from_command(
                &func,
                "greet",
                Path::new("test.rs"),
                &mut resolver,
            );

            assert!(result.is_ok());
            assert_eq!(result.unwrap().len(), 0);
        }

        #[test]
        fn test_extract_single_channel() {
            let parser = parser();
            let mut resolver = type_resolver();
            let func: ItemFn = parse_quote! {
                #[tauri::command]
                fn download(progress: Channel<ProgressUpdate>) {
                    // implementation
                }
            };

            let result = parser.extract_channels_from_command(
                &func,
                "download",
                Path::new("test.rs"),
                &mut resolver,
            );

            assert!(result.is_ok());
            let channels = result.unwrap();
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].parameter_name, "progress");
            assert_eq!(channels[0].message_type, "ProgressUpdate");
            assert_eq!(channels[0].command_name, "download");
        }

        #[test]
        fn test_extract_multiple_channels() {
            let parser = parser();
            let mut resolver = type_resolver();
            let func: ItemFn = parse_quote! {
                #[tauri::command]
                fn process(
                    progress: Channel<Progress>,
                    logs: Channel<LogEntry>,
                ) {
                    // implementation
                }
            };

            let result = parser.extract_channels_from_command(
                &func,
                "process",
                Path::new("test.rs"),
                &mut resolver,
            );

            assert!(result.is_ok());
            let channels = result.unwrap();
            assert_eq!(channels.len(), 2);
            assert_eq!(channels[0].parameter_name, "progress");
            assert_eq!(channels[0].message_type, "Progress");
            assert_eq!(channels[1].parameter_name, "logs");
            assert_eq!(channels[1].message_type, "LogEntry");
        }

        #[test]
        fn test_extract_channel_with_qualified_path() {
            let parser = parser();
            let mut resolver = type_resolver();
            let func: ItemFn = parse_quote! {
                #[tauri::command]
                fn monitor(events: tauri::ipc::Channel<Event>) {
                    // implementation
                }
            };

            let result = parser.extract_channels_from_command(
                &func,
                "monitor",
                Path::new("test.rs"),
                &mut resolver,
            );

            assert!(result.is_ok());
            let channels = result.unwrap();
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].message_type, "Event");
        }

        #[test]
        fn test_extract_mixed_parameters() {
            let parser = parser();
            let mut resolver = type_resolver();
            let func: ItemFn = parse_quote! {
                #[tauri::command]
                fn process(
                    name: String,
                    progress: Channel<Progress>,
                    count: i32,
                ) {
                    // implementation
                }
            };

            let result = parser.extract_channels_from_command(
                &func,
                "process",
                Path::new("test.rs"),
                &mut resolver,
            );

            assert!(result.is_ok());
            let channels = result.unwrap();
            // Should only extract the channel, not other parameters
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].parameter_name, "progress");
        }

        #[test]
        fn test_channel_with_complex_message_type() {
            let parser = parser();
            let mut resolver = type_resolver();
            let func: ItemFn = parse_quote! {
                #[tauri::command]
                fn stream(data: Channel<Vec<Option<String>>>) {
                    // implementation
                }
            };

            let result = parser.extract_channels_from_command(
                &func,
                "stream",
                Path::new("test.rs"),
                &mut resolver,
            );

            assert!(result.is_ok());
            let channels = result.unwrap();
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].message_type, "Vec<Option<String>>");
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn test_function_with_no_parameters() {
            let parser = parser();
            let mut resolver = type_resolver();
            let func: ItemFn = parse_quote! {
                #[tauri::command]
                fn simple() -> String {
                    "test".to_string()
                }
            };

            let result = parser.extract_channels_from_command(
                &func,
                "simple",
                Path::new("test.rs"),
                &mut resolver,
            );

            assert!(result.is_ok());
            assert_eq!(result.unwrap().len(), 0);
        }

        #[test]
        fn test_self_parameter_ignored() {
            let parser = parser();
            let mut resolver = type_resolver();
            let func: ItemFn = parse_quote! {
                #[tauri::command]
                fn method(&self, ch: Channel<Event>) {
                    // implementation
                }
            };

            let result = parser.extract_channels_from_command(
                &func,
                "method",
                Path::new("test.rs"),
                &mut resolver,
            );

            assert!(result.is_ok());
            let channels = result.unwrap();
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].parameter_name, "ch");
        }
    }
}
