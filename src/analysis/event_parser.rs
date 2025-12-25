use crate::analysis::type_resolver::TypeResolver;
use crate::models::EventInfo;
use std::path::Path;
use syn::{Expr, ExprMethodCall, File as SynFile, Lit};

/// Parser for Tauri event emissions
#[derive(Debug)]
pub struct EventParser;

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
                // Search within function bodies
                self.extract_events_from_block(
                    &func.block.stmts,
                    file_path,
                    type_resolver,
                    &mut events,
                );
            }
        }

        Ok(events)
    }

    /// Recursively search through statements for emit calls
    fn extract_events_from_block(
        &self,
        stmts: &[syn::Stmt],
        file_path: &Path,
        type_resolver: &mut TypeResolver,
        events: &mut Vec<EventInfo>,
    ) {
        for stmt in stmts {
            self.extract_events_from_stmt(stmt, file_path, type_resolver, events);
        }
    }

    /// Extract events from a single statement
    fn extract_events_from_stmt(
        &self,
        stmt: &syn::Stmt,
        file_path: &Path,
        type_resolver: &mut TypeResolver,
        events: &mut Vec<EventInfo>,
    ) {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                self.extract_events_from_expr(expr, file_path, type_resolver, events);
            }
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.extract_events_from_expr(&init.expr, file_path, type_resolver, events);
                }
            }
            _ => {}
        }
    }

    /// Extract events from an expression
    fn extract_events_from_expr(
        &self,
        expr: &Expr,
        file_path: &Path,
        type_resolver: &mut TypeResolver,
        events: &mut Vec<EventInfo>,
    ) {
        match expr {
            Expr::MethodCall(method_call) => {
                self.handle_method_call(method_call, file_path, type_resolver, events);
            }
            Expr::Block(block) => {
                self.extract_events_from_block(
                    &block.block.stmts,
                    file_path,
                    type_resolver,
                    events,
                );
            }
            Expr::If(expr_if) => {
                self.extract_events_from_block(
                    &expr_if.then_branch.stmts,
                    file_path,
                    type_resolver,
                    events,
                );
                if let Some((_, else_branch)) = &expr_if.else_branch {
                    self.extract_events_from_expr(else_branch, file_path, type_resolver, events);
                }
            }
            Expr::Match(expr_match) => {
                for arm in &expr_match.arms {
                    self.extract_events_from_expr(&arm.body, file_path, type_resolver, events);
                }
            }
            Expr::Loop(expr_loop) => {
                self.extract_events_from_block(
                    &expr_loop.body.stmts,
                    file_path,
                    type_resolver,
                    events,
                );
            }
            Expr::While(expr_while) => {
                self.extract_events_from_block(
                    &expr_while.body.stmts,
                    file_path,
                    type_resolver,
                    events,
                );
            }
            Expr::ForLoop(expr_for) => {
                self.extract_events_from_block(
                    &expr_for.body.stmts,
                    file_path,
                    type_resolver,
                    events,
                );
            }
            Expr::Await(expr_await) => {
                self.extract_events_from_expr(&expr_await.base, file_path, type_resolver, events);
            }
            Expr::Try(expr_try) => {
                self.extract_events_from_expr(&expr_try.expr, file_path, type_resolver, events);
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
    ) {
        let method_name = method_call.method.to_string();

        if method_name == "emit" || method_name == "emit_to" {
            // Check if the receiver looks like app/window (basic heuristic)
            if self.is_likely_tauri_emitter(&method_call.receiver) {
                self.extract_emit_event(method_call, file_path, type_resolver, events);
            }
        }

        // Recursively check receiver and arguments for nested emits
        self.extract_events_from_expr(&method_call.receiver, file_path, type_resolver, events);
        for arg in &method_call.args {
            self.extract_events_from_expr(arg, file_path, type_resolver, events);
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
                Self::infer_payload_type(payload_expr)
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
    /// This is a best-effort heuristic based on the expression structure
    fn infer_payload_type(expr: &Expr) -> String {
        match expr {
            // Reference to a variable: &some_var
            Expr::Reference(expr_ref) => {
                // Try to infer from the inner expression
                Self::infer_payload_type(&expr_ref.expr)
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
            // Method or function calls
            Expr::Call(_) | Expr::MethodCall(_) => {
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
            let expr: Expr = parse_quote!("user-login");

            let result = parser.extract_string_literal(&expr);
            assert_eq!(result, Some("user-login".to_string()));
        }

        #[test]
        fn test_extract_from_empty_string() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!("");

            let result = parser.extract_string_literal(&expr);
            assert_eq!(result, Some("".to_string()));
        }

        #[test]
        fn test_extract_from_non_string() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(42);

            let result = parser.extract_string_literal(&expr);
            assert!(result.is_none());
        }

        #[test]
        fn test_extract_from_variable() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(event_name);

            let result = parser.extract_string_literal(&expr);
            assert!(result.is_none());
        }
    }

    // infer_payload_type tests
    mod infer_payload_type {
        use super::*;

        #[test]
        fn test_infer_string_literal() {
            let expr: Expr = parse_quote!("hello");
            assert_eq!(EventParser::infer_payload_type(&expr), "String");
        }

        #[test]
        fn test_infer_int_literal() {
            let expr: Expr = parse_quote!(42);
            assert_eq!(EventParser::infer_payload_type(&expr), "i32");
        }

        #[test]
        fn test_infer_float_literal() {
            let expr: Expr = parse_quote!(3.14);
            assert_eq!(EventParser::infer_payload_type(&expr), "f64");
        }

        #[test]
        fn test_infer_bool_literal() {
            let expr: Expr = parse_quote!(true);
            assert_eq!(EventParser::infer_payload_type(&expr), "bool");
        }

        #[test]
        fn test_infer_struct_construction() {
            let expr: Expr = parse_quote!(User {
                id: 1,
                name: "Alice"
            });
            assert_eq!(EventParser::infer_payload_type(&expr), "User");
        }

        #[test]
        fn test_infer_qualified_struct() {
            let expr: Expr = parse_quote!(models::User { id: 1 });
            assert_eq!(EventParser::infer_payload_type(&expr), "User");
        }

        #[test]
        fn test_infer_variable_path() {
            let expr: Expr = parse_quote!(user_data);
            assert_eq!(EventParser::infer_payload_type(&expr), "user_data");
        }

        #[test]
        fn test_infer_reference() {
            let expr: Expr = parse_quote!(&data);
            assert_eq!(EventParser::infer_payload_type(&expr), "data");
        }

        #[test]
        fn test_infer_empty_tuple() {
            let expr: Expr = parse_quote!(());
            assert_eq!(EventParser::infer_payload_type(&expr), "()");
        }

        #[test]
        fn test_infer_non_empty_tuple() {
            let expr: Expr = parse_quote!((1, 2, 3));
            assert_eq!(EventParser::infer_payload_type(&expr), "tuple");
        }

        #[test]
        fn test_infer_method_call() {
            let expr: Expr = parse_quote!(get_user());
            assert_eq!(EventParser::infer_payload_type(&expr), "unknown");
        }

        #[test]
        fn test_infer_function_call() {
            let expr: Expr = parse_quote!(calculate(x, y));
            assert_eq!(EventParser::infer_payload_type(&expr), "unknown");
        }
    }

    // is_likely_tauri_emitter tests
    mod is_likely_tauri_emitter {
        use super::*;

        #[test]
        fn test_recognizes_app_identifier() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(app);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_recognizes_window_identifier() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(window);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_recognizes_webview_identifier() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(webview);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_recognizes_qualified_app_handle() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(tauri::AppHandle);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_recognizes_qualified_window() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(tauri::Window);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_recognizes_qualified_webview_window() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(tauri::WebviewWindow);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_recognizes_field_access() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(self.app);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_recognizes_method_call_receiver() {
            let parser = EventParser::new();
            // Method calls are considered permissive emitters (could return AppHandle)
            let expr: Expr = parse_quote!(obj.method());
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_rejects_user_variable() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(my_data);
            assert!(!parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_rejects_user_type() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(User);
            assert!(!parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_recognizes_app_handle_in_path() {
            let parser = EventParser::new();
            // Complex path with AppHandle segment
            let expr: Expr = parse_quote!(tauri::AppHandle);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_recognizes_qualified_tauri_window() {
            let parser = EventParser::new();
            let expr: Expr = parse_quote!(tauri::Window);
            assert!(parser.is_likely_tauri_emitter(&expr));
        }

        #[test]
        fn test_rejects_function_call() {
            let parser = EventParser::new();
            // Function calls (not method calls) are not automatically emitters
            let expr: Expr = parse_quote!(get_app());
            assert!(!parser.is_likely_tauri_emitter(&expr));
        }
    }

    // Integration tests with AST parsing
    mod ast_parsing {
        use super::*;
        use std::path::PathBuf;

        #[test]
        fn test_extract_simple_emit() {
            let parser = EventParser::new();
            let mut type_resolver = TypeResolver::new();
            let ast: SynFile = parse_quote! {
                fn notify_user() {
                    app.emit("user-login", "Alice");
                }
            };
            let path = PathBuf::from("test.rs");

            let events = parser
                .extract_events_from_ast(&ast, &path, &mut type_resolver)
                .unwrap();

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].event_name, "user-login");
            assert_eq!(events[0].payload_type, "String");
        }

        #[test]
        fn test_extract_emit_with_struct() {
            let parser = EventParser::new();
            let mut type_resolver = TypeResolver::new();
            let ast: SynFile = parse_quote! {
                fn notify_user() {
                    app.emit("user-updated", User { id: 1, name: "Alice" });
                }
            };
            let path = PathBuf::from("test.rs");

            let events = parser
                .extract_events_from_ast(&ast, &path, &mut type_resolver)
                .unwrap();

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].event_name, "user-updated");
            assert_eq!(events[0].payload_type, "User");
        }

        #[test]
        fn test_extract_emit_to() {
            let parser = EventParser::new();
            let mut type_resolver = TypeResolver::new();
            let ast: SynFile = parse_quote! {
                fn notify_window() {
                    app.emit_to("main", "progress", 50);
                }
            };
            let path = PathBuf::from("test.rs");

            let events = parser
                .extract_events_from_ast(&ast, &path, &mut type_resolver)
                .unwrap();

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].event_name, "progress");
            assert_eq!(events[0].payload_type, "i32");
        }

        #[test]
        fn test_extract_multiple_emits() {
            let parser = EventParser::new();
            let mut type_resolver = TypeResolver::new();
            let ast: SynFile = parse_quote! {
                fn notify_all() {
                    app.emit("event1", "data1");
                    window.emit("event2", 42);
                }
            };
            let path = PathBuf::from("test.rs");

            let events = parser
                .extract_events_from_ast(&ast, &path, &mut type_resolver)
                .unwrap();

            assert_eq!(events.len(), 2);
            assert_eq!(events[0].event_name, "event1");
            assert_eq!(events[1].event_name, "event2");
        }

        #[test]
        fn test_extract_emit_in_if_block() {
            let parser = EventParser::new();
            let mut type_resolver = TypeResolver::new();
            let ast: SynFile = parse_quote! {
                fn conditional_notify() {
                    if condition {
                        app.emit("success", true);
                    } else {
                        app.emit("failure", false);
                    }
                }
            };
            let path = PathBuf::from("test.rs");

            let events = parser
                .extract_events_from_ast(&ast, &path, &mut type_resolver)
                .unwrap();

            assert_eq!(events.len(), 2);
            assert_eq!(events[0].event_name, "success");
            assert_eq!(events[1].event_name, "failure");
        }

        #[test]
        fn test_extract_emit_in_loop() {
            let parser = EventParser::new();
            let mut type_resolver = TypeResolver::new();
            let ast: SynFile = parse_quote! {
                fn loop_notify() {
                    for i in 0..10 {
                        app.emit("iteration", i);
                    }
                }
            };
            let path = PathBuf::from("test.rs");

            let events = parser
                .extract_events_from_ast(&ast, &path, &mut type_resolver)
                .unwrap();

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].event_name, "iteration");
        }

        #[test]
        fn test_extract_emit_in_match() {
            let parser = EventParser::new();
            let mut type_resolver = TypeResolver::new();
            let ast: SynFile = parse_quote! {
                fn match_notify() {
                    match result {
                        Ok(val) => app.emit("success", val),
                        Err(e) => app.emit("error", e),
                    }
                }
            };
            let path = PathBuf::from("test.rs");

            let events = parser
                .extract_events_from_ast(&ast, &path, &mut type_resolver)
                .unwrap();

            assert_eq!(events.len(), 2);
            assert_eq!(events[0].event_name, "success");
            assert_eq!(events[1].event_name, "error");
        }

        #[test]
        fn test_no_events_in_non_emit_function() {
            let parser = EventParser::new();
            let mut type_resolver = TypeResolver::new();
            let ast: SynFile = parse_quote! {
                fn regular_function() {
                    let x = 42;
                    println!("Hello");
                }
            };
            let path = PathBuf::from("test.rs");

            let events = parser
                .extract_events_from_ast(&ast, &path, &mut type_resolver)
                .unwrap();

            assert_eq!(events.len(), 0);
        }

        #[test]
        fn test_ignores_user_emit_method() {
            let parser = EventParser::new();
            let mut type_resolver = TypeResolver::new();
            let ast: SynFile = parse_quote! {
                fn user_emit() {
                    my_object.emit("not-a-tauri-event", data);
                }
            };
            let path = PathBuf::from("test.rs");

            let events = parser
                .extract_events_from_ast(&ast, &path, &mut type_resolver)
                .unwrap();

            // Should not detect this as a Tauri event since my_object is not a Tauri emitter
            assert_eq!(events.len(), 0);
        }
    }
}
