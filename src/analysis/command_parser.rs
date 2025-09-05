use crate::analysis::type_resolver::TypeResolver;
use crate::models::{CommandInfo, ParameterInfo};
use std::path::PathBuf;
use syn::{File as SynFile, FnArg, ItemFn, PatType, ReturnType, Type};

/// Parser for Tauri command functions
#[derive(Debug)]
pub struct CommandParser;

impl CommandParser {
    pub fn new() -> Self {
        Self
    }

    /// Extract commands from a cached AST
    pub fn extract_commands_from_ast(
        &self,
        ast: &SynFile,
        file_path: &PathBuf,
        type_resolver: &mut TypeResolver,
    ) -> Result<Vec<CommandInfo>, Box<dyn std::error::Error>> {
        let mut commands = Vec::new();

        for item in &ast.items {
            if let syn::Item::Fn(func) = item {
                if self.is_tauri_command(func) {
                    if let Some(command_info) = self.extract_command_info(func, file_path, type_resolver) {
                        commands.push(command_info);
                    }
                }
            }
        }

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
    fn extract_command_info(&self, func: &ItemFn, file_path: &PathBuf, type_resolver: &mut TypeResolver) -> Option<CommandInfo> {
        let name = func.sig.ident.to_string();
        let parameters = self.extract_parameters(&func.sig.inputs, type_resolver);
        let return_type = self.extract_return_type(&func.sig.output, type_resolver);
        let is_async = func.sig.asyncness.is_some();

        // Get line number from the function's span
        let line_number = func.sig.ident.span().start().line;

        Some(CommandInfo {
            name,
            parameters,
            return_type,
            file_path: file_path.to_string_lossy().to_string(),
            line_number,
            is_async,
        })
    }

    /// Extract parameters from function signature
    fn extract_parameters(
        &self,
        inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
        type_resolver: &mut TypeResolver,
    ) -> Vec<ParameterInfo> {
        let mut parameters = Vec::new();

        for input in inputs {
            if let FnArg::Typed(PatType { pat, ty, .. }) = input {
                if let syn::Pat::Ident(pat_ident) = pat.as_ref() {
                    let name = pat_ident.ident.to_string();
                    let rust_type = self.type_to_string(ty);
                    
                    // Skip Tauri-specific parameters
                    if self.is_tauri_parameter(&name, &rust_type) {
                        continue;
                    }
                    
                    let typescript_type = type_resolver.map_rust_type_to_typescript(&rust_type);
                    let is_optional = self.is_optional_type(ty);

                    parameters.push(ParameterInfo {
                        name,
                        rust_type,
                        typescript_type,
                        is_optional,
                    });
                }
            }
        }

        parameters
    }
    
    /// Check if a parameter is a Tauri-specific parameter that should be skipped
    fn is_tauri_parameter(&self, name: &str, rust_type: &str) -> bool {
        // Common Tauri parameter names
        if matches!(name, "app" | "window" | "state" | "handle") {
            return true;
        }
        
        // Common Tauri parameter types
        if rust_type.contains("AppHandle") 
            || rust_type.contains("Window") 
            || rust_type.contains("State") 
            || rust_type.contains("Manager") {
            return true;
        }
        
        false
    }

    /// Extract return type from function signature
    fn extract_return_type(&self, output: &ReturnType, type_resolver: &mut TypeResolver) -> String {
        match output {
            ReturnType::Default => "void".to_string(),
            ReturnType::Type(_, ty) => {
                let rust_type = self.type_to_string(ty);
                type_resolver.map_rust_type_to_typescript(&rust_type)
            }
        }
    }

    /// Convert a Type to its string representation
    fn type_to_string(&self, ty: &Type) -> String {
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
                                                Some(self.type_to_string(inner_ty))
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
                format!("&{}", self.type_to_string(&type_ref.elem))
            }
            Type::Tuple(type_tuple) => {
                if type_tuple.elems.is_empty() {
                    "()".to_string()
                } else {
                    let types: Vec<String> = type_tuple
                        .elems
                        .iter()
                        .map(|t| self.type_to_string(t))
                        .collect();
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