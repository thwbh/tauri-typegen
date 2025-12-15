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
                if let FnArg::Typed(PatType { pat, ty, .. }) = input {
                    if let syn::Pat::Ident(pat_ident) = pat.as_ref() {
                        let name = pat_ident.ident.to_string();

                        // Skip Tauri-specific parameters
                        if self.is_tauri_parameter_type(ty) {
                            return None;
                        }

                        let rust_type = Self::type_to_string(ty);
                        let type_structure = type_resolver.parse_type_structure(&rust_type);
                        let is_optional = self.is_optional_type(ty);

                        return Some(ParameterInfo {
                            name,
                            rust_type,
                            is_optional,
                            type_structure,
                            serde_rename: None,
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
