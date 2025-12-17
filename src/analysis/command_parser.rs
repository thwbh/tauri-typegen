use crate::analysis::serde_parser::SerdeParser;
use crate::analysis::type_resolver::TypeResolver;
use crate::models::{CommandInfo, ParameterInfo};
use std::path::Path;
use syn::{File as SynFile, FnArg, ItemFn, PatType, ReturnType, Type};

/// Parser for Tauri command functions
#[derive(Debug)]
pub struct CommandParser {
    serde_parser: SerdeParser,
}

impl CommandParser {
    pub fn new() -> Self {
        Self {
            serde_parser: SerdeParser::new(),
        }
    }

    /// Extract commands from a cached AST
    pub fn extract_commands_from_ast(
        &self,
        ast: &SynFile,
        file_path: &Path,
        type_resolver: &mut TypeResolver,
    ) -> Result<Vec<CommandInfo>, Box<dyn std::error::Error>> {
        let commands = ast
            .items
            .iter()
            .filter_map(|item| {
                if let syn::Item::Fn(func) = item {
                    if self.is_tauri_command(func) {
                        return self.extract_command_info(func, file_path, type_resolver);
                    }
                }
                None
            })
            .collect();

        Ok(commands)
    }

    /// Check if a function is a Tauri command
    fn is_tauri_command(&self, func: &ItemFn) -> bool {
        func.attrs.iter().any(|attr| {
            attr.path().segments.len() == 2
                && attr.path().segments[0].ident == "tauri"
                && attr.path().segments[1].ident == "command"
                || attr.path().is_ident("command")
        })
    }

    /// Extract command information from a function
    fn extract_command_info(
        &self,
        func: &ItemFn,
        file_path: &Path,
        type_resolver: &mut TypeResolver,
    ) -> Option<CommandInfo> {
        let name = func.sig.ident.to_string();

        let parameters = self.extract_parameters(&func.sig.inputs, type_resolver);
        let return_type = self.extract_return_type(&func.sig.output);
        let return_type_structure = type_resolver.parse_type_structure(&return_type);
        let is_async = func.sig.asyncness.is_some();

        // Get line number from the function's span
        let line_number = func.sig.ident.span().start().line;

        // Parse serde rename_all attribute from function attributes
        let serde_rename_all = self
            .serde_parser
            .parse_struct_serde_attrs(&func.attrs)
            .rename_all;

        Some(CommandInfo {
            name,
            parameters,
            return_type,
            return_type_structure,
            file_path: file_path.to_string_lossy().to_string(),
            line_number,
            is_async,
            channels: Vec::new(), // Will be populated by channel_parser
            serde_rename_all,
        })
    }

    /// Extract parameters from function signature
    fn extract_parameters(
        &self,
        inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
        type_resolver: &mut TypeResolver,
    ) -> Vec<ParameterInfo> {
        inputs
            .iter()
            .filter_map(|input| {
                if let FnArg::Typed(PatType { pat, ty, attrs, .. }) = input {
                    if let syn::Pat::Ident(pat_ident) = pat.as_ref() {
                        let name = pat_ident.ident.to_string();

                        // Skip Tauri-specific parameters
                        if self.is_tauri_parameter_type(ty) {
                            return None;
                        }

                        let rust_type = Self::type_to_string(ty);
                        let type_structure = type_resolver.parse_type_structure(&rust_type);
                        let is_optional = self.is_optional_type(ty);

                        // Parse serde rename attribute from parameter attributes
                        let serde_rename = self.serde_parser.parse_field_serde_attrs(attrs).rename;

                        return Some(ParameterInfo {
                            name,
                            rust_type,
                            is_optional,
                            type_structure,
                            serde_rename,
                        });
                    }
                }
                None
            })
            .collect()
    }

    /// Check if a parameter type is a Tauri-specific type that should be skipped
    /// This checks the actual syn::Type to properly handle both imported and fully-qualified types
    fn is_tauri_parameter_type(&self, ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            let segments = &type_path.path.segments;

            // Check various patterns:
            // 1. Fully qualified: tauri::AppHandle, tauri::State<T>, tauri::ipc::Request
            // 2. Imported: AppHandle, State<T>, Window<T>
            if segments.len() >= 2 {
                // Check for tauri::* or tauri::ipc::*
                if segments[0].ident == "tauri" {
                    if segments.len() == 2 {
                        // tauri::AppHandle, tauri::Window, etc.
                        let second = &segments[1].ident;
                        return second == "AppHandle"
                            || second == "Window"
                            || second == "WebviewWindow"
                            || second == "State"
                            || second == "Manager";
                    } else if segments.len() == 3 && segments[1].ident == "ipc" {
                        // tauri::ipc::Request, tauri::ipc::Channel
                        let third = &segments[2].ident;
                        return third == "Request" || third == "Channel";
                    }
                }
            }

            // Check for imported types (single segment)
            if let Some(last_segment) = segments.last() {
                let type_ident = &last_segment.ident;

                // Only match specific Tauri types that are commonly imported
                // Be careful not to match user types with similar names
                if type_ident == "AppHandle" || type_ident == "WebviewWindow" {
                    return true;
                }

                // Channel should be filtered if it has generic parameters (indicating it's the Tauri IPC channel)
                if type_ident == "Channel"
                    && matches!(
                        last_segment.arguments,
                        syn::PathArguments::AngleBracketed(_)
                    )
                {
                    return true;
                }

                // State and Window are common names, only match if they have generic params
                // (Tauri's State and Window types always have generics like State<T>, Window<R>)
                if (type_ident == "State" || type_ident == "Window")
                    && !last_segment.arguments.is_empty()
                {
                    return true;
                }
            }
        }

        false
    }

    /// Extract return type from function signature - returns rust_type only
    fn extract_return_type(&self, output: &ReturnType) -> String {
        match output {
            ReturnType::Default => "()".to_string(),
            ReturnType::Type(_, ty) => Self::type_to_string(ty),
        }
    }

    /// Convert a Type to its string representation
    fn type_to_string(ty: &Type) -> String {
        match ty {
            Type::Path(type_path) => {
                let segments: Vec<String> = type_path
                    .path
                    .segments
                    .iter()
                    .map(|segment| {
                        if segment.arguments.is_empty() {
                            segment.ident.to_string()
                        } else {
                            match &segment.arguments {
                                syn::PathArguments::AngleBracketed(args) => {
                                    let inner_types: Vec<String> = args
                                        .args
                                        .iter()
                                        .filter_map(|arg| {
                                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                                Some(Self::type_to_string(inner_ty))
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();
                                    format!("{}<{}>", segment.ident, inner_types.join(", "))
                                }
                                _ => segment.ident.to_string(),
                            }
                        }
                    })
                    .collect();
                segments.join("::")
            }
            Type::Reference(type_ref) => {
                format!("&{}", Self::type_to_string(&type_ref.elem))
            }
            Type::Tuple(type_tuple) => {
                if type_tuple.elems.is_empty() {
                    "()".to_string()
                } else {
                    let types: Vec<String> =
                        type_tuple.elems.iter().map(Self::type_to_string).collect();
                    format!("({})", types.join(", "))
                }
            }
            _ => "unknown".to_string(),
        }
    }

    /// Check if a type is Option<T>
    fn is_optional_type(&self, ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "Option";
            }
        }
        false
    }
}

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_new_command_parser() {
        let parser = CommandParser::new();
        // Just verify it constructs without panicking
        let _ = parser;
    }

    #[test]
    fn test_default_impl() {
        let parser = CommandParser::default();
        // Just verify default works
        let _ = parser;
    }

    // is_tauri_command tests
    mod is_tauri_command {
        use super::*;

        #[test]
        fn test_recognizes_tauri_command_attribute() {
            let parser = CommandParser::new();
            let func: ItemFn = parse_quote! {
                #[tauri::command]
                fn greet(name: String) -> String {
                    format!("Hello, {}!", name)
                }
            };

            assert!(parser.is_tauri_command(&func));
        }

        #[test]
        fn test_recognizes_command_attribute() {
            let parser = CommandParser::new();
            let func: ItemFn = parse_quote! {
                #[command]
                fn greet(name: String) -> String {
                    format!("Hello, {}!", name)
                }
            };

            assert!(parser.is_tauri_command(&func));
        }

        #[test]
        fn test_rejects_non_command_function() {
            let parser = CommandParser::new();
            let func: ItemFn = parse_quote! {
                fn greet(name: String) -> String {
                    format!("Hello, {}!", name)
                }
            };

            assert!(!parser.is_tauri_command(&func));
        }

        #[test]
        fn test_rejects_other_attributes() {
            let parser = CommandParser::new();
            let func: ItemFn = parse_quote! {
                #[derive(Debug)]
                fn greet(name: String) -> String {
                    format!("Hello, {}!", name)
                }
            };

            assert!(!parser.is_tauri_command(&func));
        }
    }

    // type_to_string tests
    mod type_to_string {
        use super::*;

        #[test]
        fn test_simple_type() {
            let ty: Type = parse_quote!(String);
            assert_eq!(CommandParser::type_to_string(&ty), "String");
        }

        #[test]
        fn test_generic_type() {
            let ty: Type = parse_quote!(Vec<String>);
            assert_eq!(CommandParser::type_to_string(&ty), "Vec<String>");
        }

        #[test]
        fn test_nested_generic() {
            let ty: Type = parse_quote!(Vec<Option<String>>);
            assert_eq!(CommandParser::type_to_string(&ty), "Vec<Option<String>>");
        }

        #[test]
        fn test_multiple_generics() {
            let ty: Type = parse_quote!(HashMap<String, i32>);
            assert_eq!(CommandParser::type_to_string(&ty), "HashMap<String, i32>");
        }

        #[test]
        fn test_reference_type() {
            let ty: Type = parse_quote!(&str);
            assert_eq!(CommandParser::type_to_string(&ty), "&str");
        }

        #[test]
        fn test_empty_tuple() {
            let ty: Type = parse_quote!(());
            assert_eq!(CommandParser::type_to_string(&ty), "()");
        }

        #[test]
        fn test_tuple_with_elements() {
            let ty: Type = parse_quote!((String, i32));
            assert_eq!(CommandParser::type_to_string(&ty), "(String, i32)");
        }

        #[test]
        fn test_qualified_path() {
            let ty: Type = parse_quote!(std::collections::HashMap<String, i32>);
            assert_eq!(
                CommandParser::type_to_string(&ty),
                "std::collections::HashMap<String, i32>"
            );
        }
    }

    // is_optional_type tests
    mod is_optional_type {
        use super::*;

        #[test]
        fn test_recognizes_option() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(Option<String>);
            assert!(parser.is_optional_type(&ty));
        }

        #[test]
        fn test_recognizes_nested_option() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(Option<Vec<String>>);
            assert!(parser.is_optional_type(&ty));
        }

        #[test]
        fn test_rejects_non_option() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(String);
            assert!(!parser.is_optional_type(&ty));
        }

        #[test]
        fn test_rejects_vec() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(Vec<String>);
            assert!(!parser.is_optional_type(&ty));
        }
    }

    // is_tauri_parameter_type tests
    mod is_tauri_parameter_type {
        use super::*;

        #[test]
        fn test_recognizes_app_handle() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(tauri::AppHandle);
            assert!(parser.is_tauri_parameter_type(&ty));
        }

        #[test]
        fn test_recognizes_imported_app_handle() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(AppHandle);
            assert!(parser.is_tauri_parameter_type(&ty));
        }

        #[test]
        fn test_recognizes_window_with_generics() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(Window<R>);
            assert!(parser.is_tauri_parameter_type(&ty));
        }

        #[test]
        fn test_recognizes_state_with_generics() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(State<AppState>);
            assert!(parser.is_tauri_parameter_type(&ty));
        }

        #[test]
        fn test_recognizes_webview_window() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(tauri::WebviewWindow);
            assert!(parser.is_tauri_parameter_type(&ty));
        }

        #[test]
        fn test_recognizes_imported_webview_window() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(WebviewWindow);
            assert!(parser.is_tauri_parameter_type(&ty));
        }

        #[test]
        fn test_recognizes_ipc_request() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(tauri::ipc::Request);
            assert!(parser.is_tauri_parameter_type(&ty));
        }

        #[test]
        fn test_recognizes_ipc_channel() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(tauri::ipc::Channel<String>);
            assert!(parser.is_tauri_parameter_type(&ty));
        }

        #[test]
        fn test_recognizes_channel_with_generics() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(Channel<ProgressUpdate>);
            assert!(parser.is_tauri_parameter_type(&ty));
        }

        #[test]
        fn test_rejects_user_string_type() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(String);
            assert!(!parser.is_tauri_parameter_type(&ty));
        }

        #[test]
        fn test_rejects_user_custom_type() {
            let parser = CommandParser::new();
            let ty: Type = parse_quote!(User);
            assert!(!parser.is_tauri_parameter_type(&ty));
        }

        #[test]
        fn test_rejects_state_without_generics() {
            let parser = CommandParser::new();
            // User might have their own State type without generics
            let ty: Type = parse_quote!(State);
            assert!(!parser.is_tauri_parameter_type(&ty));
        }

        #[test]
        fn test_rejects_window_without_generics() {
            let parser = CommandParser::new();
            // User might have their own Window type without generics
            let ty: Type = parse_quote!(Window);
            assert!(!parser.is_tauri_parameter_type(&ty));
        }
    }

    // extract_return_type tests
    mod extract_return_type {
        use super::*;

        #[test]
        fn test_extract_simple_return() {
            let parser = CommandParser::new();
            let output: ReturnType = parse_quote!(-> String);
            assert_eq!(parser.extract_return_type(&output), "String");
        }

        #[test]
        fn test_extract_generic_return() {
            let parser = CommandParser::new();
            let output: ReturnType = parse_quote!(-> Vec<String>);
            assert_eq!(parser.extract_return_type(&output), "Vec<String>");
        }

        #[test]
        fn test_extract_result_return() {
            let parser = CommandParser::new();
            let output: ReturnType = parse_quote!(-> Result<String, Error>);
            assert_eq!(parser.extract_return_type(&output), "Result<String, Error>");
        }

        #[test]
        fn test_extract_default_return() {
            let parser = CommandParser::new();
            let output: ReturnType = parse_quote!();
            assert_eq!(parser.extract_return_type(&output), "()");
        }
    }

    // extract_parameters tests
    mod extract_parameters {
        use super::*;

        #[test]
        fn test_extract_simple_parameter() {
            let parser = CommandParser::new();
            let mut type_resolver = TypeResolver::new();
            let inputs = parse_quote!(name: String);

            let params = parser.extract_parameters(&inputs, &mut type_resolver);

            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "name");
            assert_eq!(params[0].rust_type, "String");
            assert!(!params[0].is_optional);
        }

        #[test]
        fn test_extract_optional_parameter() {
            let parser = CommandParser::new();
            let mut type_resolver = TypeResolver::new();
            let inputs = parse_quote!(email: Option<String>);

            let params = parser.extract_parameters(&inputs, &mut type_resolver);

            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "email");
            assert!(params[0].is_optional);
        }

        #[test]
        fn test_extract_multiple_parameters() {
            let parser = CommandParser::new();
            let mut type_resolver = TypeResolver::new();
            let inputs = parse_quote!(name: String, age: i32);

            let params = parser.extract_parameters(&inputs, &mut type_resolver);

            assert_eq!(params.len(), 2);
            assert_eq!(params[0].name, "name");
            assert_eq!(params[1].name, "age");
        }

        #[test]
        fn test_filters_app_handle() {
            let parser = CommandParser::new();
            let mut type_resolver = TypeResolver::new();
            let inputs = parse_quote!(app: AppHandle, name: String);

            let params = parser.extract_parameters(&inputs, &mut type_resolver);

            // AppHandle should be filtered out
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "name");
        }

        #[test]
        fn test_filters_state() {
            let parser = CommandParser::new();
            let mut type_resolver = TypeResolver::new();
            let inputs = parse_quote!(state: State<AppState>, name: String);

            let params = parser.extract_parameters(&inputs, &mut type_resolver);

            // State should be filtered out
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "name");
        }

        #[test]
        fn test_filters_channel() {
            let parser = CommandParser::new();
            let mut type_resolver = TypeResolver::new();
            let inputs = parse_quote!(progress: Channel<u32>, name: String);

            let params = parser.extract_parameters(&inputs, &mut type_resolver);

            // Channel should be filtered out
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "name");
        }

        #[test]
        fn test_empty_parameters() {
            let parser = CommandParser::new();
            let mut type_resolver = TypeResolver::new();
            let inputs = parse_quote!();

            let params = parser.extract_parameters(&inputs, &mut type_resolver);

            assert_eq!(params.len(), 0);
        }
    }

    // extract_command_info tests
    mod extract_command_info {
        use super::*;
        use std::path::PathBuf;

        #[test]
        fn test_extract_simple_command() {
            let parser = CommandParser::new();
            let mut type_resolver = TypeResolver::new();
            let func: ItemFn = parse_quote! {
                #[tauri::command]
                fn greet(name: String) -> String {
                    format!("Hello, {}!", name)
                }
            };
            let path = PathBuf::from("test.rs");

            let info = parser.extract_command_info(&func, &path, &mut type_resolver);

            assert!(info.is_some());
            let info = info.unwrap();
            assert_eq!(info.name, "greet");
            assert_eq!(info.parameters.len(), 1);
            assert_eq!(info.return_type, "String");
            assert!(!info.is_async);
        }

        #[test]
        fn test_extract_async_command() {
            let parser = CommandParser::new();
            let mut type_resolver = TypeResolver::new();
            let func: ItemFn = parse_quote! {
                #[tauri::command]
                async fn fetch_data() -> Result<String, Error> {
                    Ok("data".to_string())
                }
            };
            let path = PathBuf::from("test.rs");

            let info = parser.extract_command_info(&func, &path, &mut type_resolver);

            assert!(info.is_some());
            let info = info.unwrap();
            assert!(info.is_async);
            assert_eq!(info.return_type, "Result<String, Error>");
        }

        #[test]
        fn test_extract_command_with_no_return() {
            let parser = CommandParser::new();
            let mut type_resolver = TypeResolver::new();
            let func: ItemFn = parse_quote! {
                #[tauri::command]
                fn log_message(msg: String) {
                    println!("{}", msg);
                }
            };
            let path = PathBuf::from("test.rs");

            let info = parser.extract_command_info(&func, &path, &mut type_resolver);

            assert!(info.is_some());
            let info = info.unwrap();
            assert_eq!(info.return_type, "()");
        }
    }
}
