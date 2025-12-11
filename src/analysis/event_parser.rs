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
        _type_resolver: &mut TypeResolver,
    ) -> Result<Vec<EventInfo>, Box<dyn std::error::Error>> {
        let mut events = Vec::new();

        // Visit all items in the AST looking for emit calls
        for item in &ast.items {
            if let syn::Item::Fn(func) = item {
                // Search within function bodies
                self.extract_events_from_block(
                    &func.block.stmts,
                    file_path,
                    _type_resolver,
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
        _type_resolver: &mut TypeResolver,
        events: &mut Vec<EventInfo>,
    ) {
        for stmt in stmts {
            self.extract_events_from_stmt(stmt, file_path, _type_resolver, events);
        }
    }

    /// Extract events from a single statement
    fn extract_events_from_stmt(
        &self,
        stmt: &syn::Stmt,
        file_path: &Path,
        _type_resolver: &mut TypeResolver,
        events: &mut Vec<EventInfo>,
    ) {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                self.extract_events_from_expr(expr, file_path, _type_resolver, events);
            }
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.extract_events_from_expr(&init.expr, file_path, _type_resolver, events);
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
        _type_resolver: &mut TypeResolver,
        events: &mut Vec<EventInfo>,
    ) {
        match expr {
            Expr::MethodCall(method_call) => {
                self.handle_method_call(method_call, file_path, _type_resolver, events);
            }
            Expr::Block(block) => {
                self.extract_events_from_block(
                    &block.block.stmts,
                    file_path,
                    _type_resolver,
                    events,
                );
            }
            Expr::If(expr_if) => {
                self.extract_events_from_block(
                    &expr_if.then_branch.stmts,
                    file_path,
                    _type_resolver,
                    events,
                );
                if let Some((_, else_branch)) = &expr_if.else_branch {
                    self.extract_events_from_expr(else_branch, file_path, _type_resolver, events);
                }
            }
            Expr::Match(expr_match) => {
                for arm in &expr_match.arms {
                    self.extract_events_from_expr(&arm.body, file_path, _type_resolver, events);
                }
            }
            Expr::Loop(expr_loop) => {
                self.extract_events_from_block(
                    &expr_loop.body.stmts,
                    file_path,
                    _type_resolver,
                    events,
                );
            }
            Expr::While(expr_while) => {
                self.extract_events_from_block(
                    &expr_while.body.stmts,
                    file_path,
                    _type_resolver,
                    events,
                );
            }
            Expr::ForLoop(expr_for) => {
                self.extract_events_from_block(
                    &expr_for.body.stmts,
                    file_path,
                    _type_resolver,
                    events,
                );
            }
            Expr::Await(expr_await) => {
                self.extract_events_from_expr(&expr_await.base, file_path, _type_resolver, events);
            }
            Expr::Try(expr_try) => {
                self.extract_events_from_expr(&expr_try.expr, file_path, _type_resolver, events);
            }
            _ => {}
        }
    }

    /// Handle method call expressions, looking for emit() and emit_to()
    fn handle_method_call(
        &self,
        method_call: &ExprMethodCall,
        file_path: &Path,
        _type_resolver: &mut TypeResolver,
        events: &mut Vec<EventInfo>,
    ) {
        let method_name = method_call.method.to_string();

        if method_name == "emit" || method_name == "emit_to" {
            // Check if the receiver looks like app/window (basic heuristic)
            if self.is_likely_tauri_emitter(&method_call.receiver) {
                self.extract_emit_event(method_call, file_path, _type_resolver, events);
            }
        }

        // Recursively check receiver and arguments for nested emits
        self.extract_events_from_expr(&method_call.receiver, file_path, _type_resolver, events);
        for arg in &method_call.args {
            self.extract_events_from_expr(arg, file_path, _type_resolver, events);
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
        _type_resolver: &mut TypeResolver,
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

            events.push(EventInfo {
                event_name,
                payload_type,
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
