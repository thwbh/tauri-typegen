use crate::analysis::type_resolver::TypeResolver;
use crate::models::EventInfo;
use std::collections::HashMap;
use std::path::Path;
use syn::{Expr, ExprMethodCall, File as SynFile, FnArg, Lit, Pat, Type};

/// Parser for Tauri event emissions
#[derive(Debug)]
pub struct EventParser;

/// Simple symbol table to track variable names to their types
type SymbolTable = HashMap<String, String>;

impl EventParser {
    pub fn new() -> Self {
        Self
    }

    /// Extract event emissions from a cached AST
    /// Looks for patterns like:
    /// - app.emit("event-name", payload)
    /// - window.emit("event-name", payload)
    /// - app.emit_to("label", "event-name", payload)
    pub fn extract_events_from_ast(
        &self,
        ast: &SynFile,
        file_path: &Path,
        type_resolver: &mut TypeResolver,
    ) -> Result<Vec<EventInfo>, Box<dyn std::error::Error>> {
        let mut events = Vec::new();

        // Visit all items in the AST looking for emit calls
        for item in &ast.items {
            if let syn::Item::Fn(func) = item {
                // Build symbol table from function parameters
                let mut symbols = SymbolTable::new();
                self.extract_param_types(&func.sig.inputs, &mut symbols);

                // Search within function bodies with symbol context
                self.extract_events_from_block(
                    &func.block.stmts,
                    file_path,
                    type_resolver,
                    &mut events,
                    &mut symbols,
                );
            }
        }

        Ok(events)
    }

    /// Extract parameter types from function signature into symbol table
    fn extract_param_types(
        &self,
        inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
        symbols: &mut SymbolTable,
    ) {
        for arg in inputs {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    let param_name = pat_ident.ident.to_string();
                    let param_type = self.extract_type_name(&pat_type.ty);
                    symbols.insert(param_name, param_type);
                }
            }
        }
    }

    /// Extract the type name from a Type, handling references and generic wrappers
    fn extract_type_name(&self, ty: &Type) -> String {
        match ty {
            Type::Reference(type_ref) => {
                // Handle &T and &mut T - extract the inner type
                self.extract_type_name(&type_ref.elem)
            }
            Type::Path(type_path) => {
                // Get the last segment of the path (the actual type name)
                if let Some(segment) = type_path.path.segments.last() {
                    segment.ident.to_string()
                } else {
                    "unknown".to_string()
                }
            }
            _ => "unknown".to_string(),
        }
    }

    /// Recursively search through statements for emit calls
    fn extract_events_from_block(
        &self,
        stmts: &[syn::Stmt],
        file_path: &Path,
        type_resolver: &mut TypeResolver,
        events: &mut Vec<EventInfo>,
        symbols: &mut SymbolTable,
    ) {
        for stmt in stmts {
            self.extract_events_from_stmt(stmt, file_path, type_resolver, events, symbols);
        }
    }

    /// Extract events from a single statement
    fn extract_events_from_stmt(
        &self,
        stmt: &syn::Stmt,
        file_path: &Path,
        type_resolver: &mut TypeResolver,
        events: &mut Vec<EventInfo>,
        symbols: &mut SymbolTable,
    ) {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                self.extract_events_from_expr(expr, file_path, type_resolver, events, symbols);
            }
            syn::Stmt::Local(local) => {
                // Track let bindings with explicit types
                self.extract_local_binding(local, symbols);

                if let Some(init) = &local.init {
                    self.extract_events_from_expr(
                        &init.expr,
                        file_path,
                        type_resolver,
                        events,
                        symbols,
                    );
                }
            }
            _ => {}
        }
    }

    /// Extract variable binding from let statement
    fn extract_local_binding(&self, local: &syn::Local, symbols: &mut SymbolTable) {
        // Handle let var: Type = ...
        if let Pat::Ident(pat_ident) = &local.pat {
            let var_name = pat_ident.ident.to_string();

            // If there's an explicit type annotation, use it
            if let Some(local_init) = &local.init {
                // Try to infer type from the initialization expression
                let inferred_type = self.infer_type_from_init(&local_init.expr, symbols);
                if inferred_type != "unknown" {
                    symbols.insert(var_name, inferred_type);
                }
            }
        }

        // Handle let var: Type (with type annotation via local.ty if it were available)
        // syn's Local doesn't have direct type annotation in newer versions,
        // but we can handle patterns with type annotations
        if let Pat::Type(pat_type) = &local.pat {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                let var_name = pat_ident.ident.to_string();
                let var_type = self.extract_type_name(&pat_type.ty);
                symbols.insert(var_name, var_type);
            }
        }
    }

    /// Try to infer type from initialization expression
    fn infer_type_from_init(&self, expr: &Expr, symbols: &SymbolTable) -> String {
        match expr {
            Expr::Struct(expr_struct) => {
                // Struct construction: Type { ... }
                if let Some(segment) = expr_struct.path.segments.last() {
                    return segment.ident.to_string();
                }
            }
            Expr::Call(call) => {
                // Function call like Type::new() or Type::default()
                if let Expr::Path(path) = &*call.func {
                    // Check for Type::method() pattern
                    if path.path.segments.len() >= 2 {
                        return path.path.segments[0].ident.to_string();
                    }
                }
            }
            Expr::Path(path) => {
                // Variable reference - look up in symbol table
                if let Some(ident) = path.path.get_ident() {
                    let name = ident.to_string();
                    if let Some(typ) = symbols.get(&name) {
                        return typ.clone();
                    }
                }
            }
            Expr::Reference(expr_ref) => {
                // &expr - recurse into inner expression
                return self.infer_type_from_init(&expr_ref.expr, symbols);
            }
            _ => {}
        }
        "unknown".to_string()
    }

    /// Extract events from an expression
    fn extract_events_from_expr(
        &self,
        expr: &Expr,
        file_path: &Path,
        type_resolver: &mut TypeResolver,
        events: &mut Vec<EventInfo>,
        symbols: &mut SymbolTable,
    ) {
        match expr {
            Expr::MethodCall(method_call) => {
                self.handle_method_call(method_call, file_path, type_resolver, events, symbols);
            }
            Expr::Block(block) => {
                self.extract_events_from_block(
                    &block.block.stmts,
                    file_path,
                    type_resolver,
                    events,
                    symbols,
                );
            }
            Expr::If(expr_if) => {
                self.extract_events_from_block(
                    &expr_if.then_branch.stmts,
                    file_path,
                    type_resolver,
                    events,
                    symbols,
                );
                if let Some((_, else_branch)) = &expr_if.else_branch {
                    self.extract_events_from_expr(
                        else_branch,
                        file_path,
                        type_resolver,
                        events,
                        symbols,
                    );
                }
            }
            Expr::Match(expr_match) => {
                for arm in &expr_match.arms {
                    self.extract_events_from_expr(
                        &arm.body,
                        file_path,
                        type_resolver,
                        events,
                        symbols,
                    );
                }
            }
            Expr::Loop(expr_loop) => {
                self.extract_events_from_block(
                    &expr_loop.body.stmts,
                    file_path,
                    type_resolver,
                    events,
                    symbols,
                );
            }
            Expr::While(expr_while) => {
                self.extract_events_from_block(
                    &expr_while.body.stmts,
                    file_path,
                    type_resolver,
                    events,
                    symbols,
                );
            }
            Expr::ForLoop(expr_for) => {
                self.extract_events_from_block(
                    &expr_for.body.stmts,
                    file_path,
                    type_resolver,
                    events,
                    symbols,
                );
            }
            Expr::Await(expr_await) => {
                self.extract_events_from_expr(
                    &expr_await.base,
                    file_path,
                    type_resolver,
                    events,
                    symbols,
                );
            }
            Expr::Try(expr_try) => {
                self.extract_events_from_expr(
                    &expr_try.expr,
                    file_path,
                    type_resolver,
                    events,
                    symbols,
                );
            }
            _ => {}
        }
    }

    /// Handle method call expressions, looking for emit() and emit_to()
    fn handle_method_call(
        &self,
        method_call: &ExprMethodCall,
        file_path: &Path,
        type_resolver: &mut TypeResolver,
        events: &mut Vec<EventInfo>,
        symbols: &mut SymbolTable,
    ) {
        let method_name = method_call.method.to_string();

        if method_name == "emit" || method_name == "emit_to" {
            // Check if the receiver looks like app/window (basic heuristic)
            if self.is_likely_tauri_emitter(&method_call.receiver) {
                self.extract_emit_event(method_call, file_path, type_resolver, events, symbols);
            }
        }

        // Recursively check receiver and arguments for nested emits
        self.extract_events_from_expr(
            &method_call.receiver,
            file_path,
            type_resolver,
            events,
            symbols,
        );
        for arg in &method_call.args {
            self.extract_events_from_expr(arg, file_path, type_resolver, events, symbols);
        }
    }

    /// Check if the receiver expression is likely a Tauri emitter (app/window)
    /// This checks the expression pattern to identify Tauri framework types
    fn is_likely_tauri_emitter(&self, receiver: &Expr) -> bool {
        match receiver {
            Expr::Path(path) => {
                // Check if this is a path that could be a Tauri type
                let segments = &path.path.segments;

                // Check for fully qualified paths: tauri::AppHandle, tauri::WebviewWindow
                if segments.len() >= 2 && segments[0].ident == "tauri" {
                    let second = &segments[1].ident;
                    return second == "AppHandle"
                        || second == "Window"
                        || second == "WebviewWindow";
                }

                // Check for simple identifiers - be conservative
                // Only match very specific common patterns for Tauri types
                if let Some(ident) = path.path.get_ident() {
                    let name = ident.to_string();
                    // Only match exact common parameter names used in Tauri commands
                    // Avoid matching user variables with similar names
                    return name == "app" || name == "window" || name == "webview";
                }

                // For complex paths, check if any segment looks like a Tauri type
                for segment in segments {
                    let seg_name = segment.ident.to_string();
                    if seg_name == "AppHandle" || seg_name == "WebviewWindow" {
                        return true;
                    }
                }

                false
            }
            Expr::Field(field_expr) => {
                // Check field access like self.app
                if let syn::Member::Named(ident) = &field_expr.member {
                    let name = ident.to_string();
                    // Only match exact common field names
                    return name == "app" || name == "window" || name == "webview";
                }
                false
            }
            Expr::MethodCall(_) => {
                // Could be something like get_app().emit()
                // Be permissive here since method calls that return handles are common
                true
            }
            _ => false,
        }
    }

    /// Extract event information from an emit or emit_to call
    fn extract_emit_event(
        &self,
        method_call: &ExprMethodCall,
        file_path: &Path,
        type_resolver: &mut TypeResolver,
        events: &mut Vec<EventInfo>,
        symbols: &SymbolTable,
    ) {
        let method_name = method_call.method.to_string();
        let args = &method_call.args;

        let (event_name, payload_expr) = if method_name == "emit_to" {
            // emit_to(label, event_name, payload)
            if args.len() >= 3 {
                (self.extract_string_literal(&args[1]), Some(&args[2]))
            } else {
                return;
            }
        } else {
            // emit(event_name, payload)
            if args.len() >= 2 {
                (self.extract_string_literal(&args[0]), Some(&args[1]))
            } else {
                return;
            }
        };

        if let Some(event_name) = event_name {
            let payload_type = if let Some(payload_expr) = payload_expr {
                self.infer_payload_type(payload_expr, symbols)
            } else {
                "()".to_string()
            };

            let line_number = method_call.method.span().start().line;
            let payload_type_structure = type_resolver.parse_type_structure(&payload_type);

            events.push(EventInfo {
                event_name,
                payload_type,
                payload_type_structure,
                file_path: file_path.to_string_lossy().to_string(),
                line_number,
            });
        }
    }

    /// Extract a string literal from an expression
    fn extract_string_literal(&self, expr: &Expr) -> Option<String> {
        if let Expr::Lit(expr_lit) = expr {
            if let Lit::Str(lit_str) = &expr_lit.lit {
                return Some(lit_str.value());
            }
        }
        None
    }

    /// Infer the type of the payload expression
    /// Uses symbol table to resolve variable names to their types
    fn infer_payload_type(&self, expr: &Expr, symbols: &SymbolTable) -> String {
        match expr {
            // Reference to a variable: &some_var
            Expr::Reference(expr_ref) => {
                // Try to infer from the inner expression
                self.infer_payload_type(&expr_ref.expr, symbols)
            }
            // Struct construction: User { ... }
            Expr::Struct(expr_struct) => {
                if let Some(segment) = expr_struct.path.segments.last() {
                    return segment.ident.to_string();
                }
                "unknown".to_string()
            }
            // Variable or path: some_var, module::Type
            Expr::Path(path) => {
                if let Some(ident) = path.path.get_ident() {
                    let name = ident.to_string();
                    // Look up variable in symbol table
                    if let Some(typ) = symbols.get(&name) {
                        return typ.clone();
                    }
                    // Fallback: might be a type name used directly (like Status::Active)
                    return name;
                }
                // For qualified paths, return the last segment
                if let Some(segment) = path.path.segments.last() {
                    return segment.ident.to_string();
                }
                "unknown".to_string()
            }
            // Tuple: (a, b, c)
            Expr::Tuple(tuple) => {
                if tuple.elems.is_empty() {
                    return "()".to_string();
                }
                // For now, just mark as tuple
                "tuple".to_string()
            }
            // Literal values
            Expr::Lit(lit) => match &lit.lit {
                Lit::Str(_) => "String".to_string(),
                Lit::Int(_) => "i32".to_string(),
                Lit::Float(_) => "f64".to_string(),
                Lit::Bool(_) => "bool".to_string(),
                _ => "unknown".to_string(),
            },
            // Clone call: var.clone()
            Expr::MethodCall(method_call) => {
                let method_name = method_call.method.to_string();
                if method_name == "clone" {
                    // Try to infer type from the receiver
                    return self.infer_payload_type(&method_call.receiver, symbols);
                }
                // Can't easily infer return type without type checker
                "unknown".to_string()
            }
            // Function calls
            Expr::Call(_) => {
                // Can't easily infer return type without type checker
                "unknown".to_string()
            }
            _ => "unknown".to_string(),
        }
    }
}

impl Default for EventParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    // extract_string_literal tests
    mod extract_string_literal {
        use super::*;

        #[test]
        fn test_extract_from_string_literal() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!("hello");
            assert_eq!(
                parser.extract_string_literal(&expr),
                Some("hello".to_string())
            );
        }

        #[test]
        fn test_extract_from_non_string() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(42);
            assert_eq!(parser.extract_string_literal(&expr), None);
        }

        #[test]
        fn test_extract_from_variable() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(some_var);
            assert_eq!(parser.extract_string_literal(&expr), None);
        }
    }

    // infer_payload_type tests
    mod infer_payload_type {
        use super::*;

        #[test]
        fn test_infer_from_struct_construction() {
            let parser = EventParser::new();
            let symbols = SymbolTable::new();
            let expr: Expr = parse_quote!(User {
                name: "test".to_string()
            });
            assert_eq!(parser.infer_payload_type(&expr, &symbols), "User");
        }

        #[test]
        fn test_infer_from_variable_with_symbol_table() {
            let parser = EventParser::new();
            let mut symbols = SymbolTable::new();
            symbols.insert("update".to_string(), "ProgressUpdate".to_string());

            let expr: Expr = parse_quote!(update);
            assert_eq!(parser.infer_payload_type(&expr, &symbols), "ProgressUpdate");
        }

        #[test]
        fn test_infer_from_reference_with_symbol_table() {
            let parser = EventParser::new();
            let mut symbols = SymbolTable::new();
            symbols.insert("update".to_string(), "ProgressUpdate".to_string());

            let expr: Expr = parse_quote!(&update);
            assert_eq!(parser.infer_payload_type(&expr, &symbols), "ProgressUpdate");
        }

        #[test]
        fn test_infer_from_clone_with_symbol_table() {
            let parser = EventParser::new();
            let mut symbols = SymbolTable::new();
            symbols.insert("update".to_string(), "ProgressUpdate".to_string());

            let expr: Expr = parse_quote!(update.clone());
            assert_eq!(parser.infer_payload_type(&expr, &symbols), "ProgressUpdate");
        }

        #[test]
        fn test_infer_from_variable_without_symbol() {
            let parser = EventParser::new();
            let symbols = SymbolTable::new();
            let expr: Expr = parse_quote!(some_var);
            // Without symbol table entry, returns the variable name
            assert_eq!(parser.infer_payload_type(&expr, &symbols), "some_var");
        }

        #[test]
        fn test_infer_from_string_literal() {
            let parser = EventParser::new();
            let symbols = SymbolTable::new();
            let expr: Expr = parse_quote!("hello");
            assert_eq!(parser.infer_payload_type(&expr, &symbols), "String");
        }

        #[test]
        fn test_infer_from_integer_literal() {
            let parser = EventParser::new();
            let symbols = SymbolTable::new();
            let expr: Expr = parse_quote!(42);
            assert_eq!(parser.infer_payload_type(&expr, &symbols), "i32");
        }

        #[test]
        fn test_infer_from_bool_literal() {
            let parser = EventParser::new();
            let symbols = SymbolTable::new();
            let expr: Expr = parse_quote!(true);
            assert_eq!(parser.infer_payload_type(&expr, &symbols), "bool");
        }

        #[test]
        fn test_infer_from_empty_tuple() {
            let parser = EventParser::new();
            let symbols = SymbolTable::new();
            let expr: Expr = parse_quote!(());
            assert_eq!(parser.infer_payload_type(&expr, &symbols), "()");
        }
    }

    // extract_param_types tests
    mod extract_param_types {
        use super::*;

        #[test]
        fn test_extract_simple_param() {
            let parser = EventParser::new();
            let func: syn::ItemFn = parse_quote! {
                fn test(name: String) {}
            };
            let mut symbols = SymbolTable::new();
            parser.extract_param_types(&func.sig.inputs, &mut symbols);

            assert_eq!(symbols.get("name"), Some(&"String".to_string()));
        }

        #[test]
        fn test_extract_reference_param() {
            let parser = EventParser::new();
            let func: syn::ItemFn = parse_quote! {
                fn test(update: &ProgressUpdate) {}
            };
            let mut symbols = SymbolTable::new();
            parser.extract_param_types(&func.sig.inputs, &mut symbols);

            assert_eq!(symbols.get("update"), Some(&"ProgressUpdate".to_string()));
        }

        #[test]
        fn test_extract_mutable_reference_param() {
            let parser = EventParser::new();
            let func: syn::ItemFn = parse_quote! {
                fn test(update: &mut ProgressUpdate) {}
            };
            let mut symbols = SymbolTable::new();
            parser.extract_param_types(&func.sig.inputs, &mut symbols);

            assert_eq!(symbols.get("update"), Some(&"ProgressUpdate".to_string()));
        }

        #[test]
        fn test_extract_multiple_params() {
            let parser = EventParser::new();
            let func: syn::ItemFn = parse_quote! {
                fn test(app: AppHandle, update: &ProgressUpdate, count: i32) {}
            };
            let mut symbols = SymbolTable::new();
            parser.extract_param_types(&func.sig.inputs, &mut symbols);

            assert_eq!(symbols.get("app"), Some(&"AppHandle".to_string()));
            assert_eq!(symbols.get("update"), Some(&"ProgressUpdate".to_string()));
            assert_eq!(symbols.get("count"), Some(&"i32".to_string()));
        }
    }

    // is_likely_tauri_emitter tests
    mod is_likely_tauri_emitter {
        use super::*;

        #[test]
        fn test_app_identifier() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(app);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_window_identifier() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(window);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_webview_identifier() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(webview);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_qualified_tauri_app_handle() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(tauri::AppHandle);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_qualified_tauri_window() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(tauri::Window);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_self_app_field() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(self.app);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_random_variable_not_emitter() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(some_var);
            assert!(!parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_method_call_is_permissive() {
            let parser = EventParser::new();
            // Method calls like self.get_app() are permissive
            let expr: Expr = parse_quote!(self.get_app());
            assert!(parser.is_likely_tauri_emitter(&expr));
        }
    }

    // Integration tests for event extraction
    mod event_extraction {
        use super::*;
        use crate::analysis::type_resolver::TypeResolver;

        #[test]
        fn test_extract_event_with_struct_payload() {
            let parser = EventParser::new();
            let mut type_resolver = TypeResolver::new();

            let file: SynFile = parse_quote! {
                fn emit_progress(app: AppHandle) {
                    app.emit("progress", ProgressUpdate { value: 50 }).unwrap();
                }
            };

            let events = parser
                .extract_events_from_ast(&file, Path::new("test.rs"), &mut type_resolver)
                .unwrap();

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].event_name, "progress");
            assert_eq!(events[0].payload_type, "ProgressUpdate");
        }

        #[test]
        fn test_extract_event_with_variable_payload_from_param() {
            let parser = EventParser::new();
            let mut type_resolver = TypeResolver::new();

            let file: SynFile = parse_quote! {
                fn emit_externally(app: AppHandle, update: &ProgressUpdate) {
                    app.emit("progress-update", update).unwrap();
                }
            };

            let events = parser
                .extract_events_from_ast(&file, Path::new("test.rs"), &mut type_resolver)
                .unwrap();

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].event_name, "progress-update");
            assert_eq!(events[0].payload_type, "ProgressUpdate");
        }

        #[test]
        fn test_extract_emit_to_with_variable_payload() {
            let parser = EventParser::new();
            let mut type_resolver = TypeResolver::new();

            let file: SynFile = parse_quote! {
                fn emit_to_window(app: AppHandle, update: &ProgressUpdate) {
                    app.emit_to("main", "progress-update", update).unwrap();
                }
            };

            let events = parser
                .extract_events_from_ast(&file, Path::new("test.rs"), &mut type_resolver)
                .unwrap();

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].event_name, "progress-update");
            assert_eq!(events[0].payload_type, "ProgressUpdate");
        }

        #[test]
        fn test_extract_event_with_cloned_variable() {
            let parser = EventParser::new();
            let mut type_resolver = TypeResolver::new();

            let file: SynFile = parse_quote! {
                fn emit_cloned(app: AppHandle, update: ProgressUpdate) {
                    app.emit("progress", update.clone()).unwrap();
                }
            };

            let events = parser
                .extract_events_from_ast(&file, Path::new("test.rs"), &mut type_resolver)
                .unwrap();

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].event_name, "progress");
            assert_eq!(events[0].payload_type, "ProgressUpdate");
        }
    }
}
