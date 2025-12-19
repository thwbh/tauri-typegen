use crate::models::{CommandInfo, StructInfo};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Dependency graph for lazy type resolution
#[derive(Debug, Default)]
pub struct TypeDependencyGraph {
    /// Maps type name to the files where it's defined
    pub type_definitions: HashMap<String, PathBuf>,
    /// Maps type name to types it depends on
    pub dependencies: HashMap<String, HashSet<String>>,
    /// Maps type name to its resolved StructInfo
    pub resolved_types: HashMap<String, StructInfo>,
}

impl TypeDependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a type definition to the graph
    pub fn add_type_definition(&mut self, type_name: String, file_path: PathBuf) {
        self.type_definitions.insert(type_name, file_path);
    }

    /// Add a dependency relationship between types
    pub fn add_dependency(&mut self, dependent: String, dependency: String) {
        self.dependencies
            .entry(dependent)
            .or_default()
            .insert(dependency);
    }

    /// Add multiple dependencies for a type
    pub fn add_dependencies(&mut self, dependent: String, dependencies: HashSet<String>) {
        self.dependencies.insert(dependent, dependencies);
    }

    /// Add a resolved type to the graph
    pub fn add_resolved_type(&mut self, type_name: String, struct_info: StructInfo) {
        self.resolved_types.insert(type_name, struct_info);
    }

    /// Get all resolved types
    pub fn get_resolved_types(&self) -> &HashMap<String, StructInfo> {
        &self.resolved_types
    }

    /// Get dependencies for a type
    pub fn get_dependencies(&self, type_name: &str) -> Option<&HashSet<String>> {
        self.dependencies.get(type_name)
    }

    /// Check if a type is defined in the graph
    pub fn has_type_definition(&self, type_name: &str) -> bool {
        self.type_definitions.contains_key(type_name)
    }

    /// Get the file path where a type is defined
    pub fn get_type_definition_path(&self, type_name: &str) -> Option<&PathBuf> {
        self.type_definitions.get(type_name)
    }

    /// Perform topological sort on the given types using the dependency graph
    pub fn topological_sort_types(&self, types: &HashSet<String>) -> Vec<String> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        for type_name in types {
            if !visited.contains(type_name) {
                self.topological_visit(type_name, &mut sorted, &mut visited, &mut visiting);
            }
        }

        sorted
    }

    /// Recursive helper for topological sorting with cycle detection
    fn topological_visit(
        &self,
        type_name: &str,
        sorted: &mut Vec<String>,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
    ) {
        // Check for cycles
        if visiting.contains(type_name) {
            eprintln!(
                "Warning: Circular dependency detected involving type: {}",
                type_name
            );
            return;
        }

        if visited.contains(type_name) {
            return;
        }

        visiting.insert(type_name.to_string());

        // Visit dependencies first
        if let Some(deps) = self.dependencies.get(type_name) {
            for dep in deps {
                self.topological_visit(dep, sorted, visited, visiting);
            }
        }

        visiting.remove(type_name);
        visited.insert(type_name.to_string());
        sorted.push(type_name.to_string());
    }

    /// Build visualization of the dependency graph
    pub fn visualize_dependencies(&self, entry_commands: &[crate::models::CommandInfo]) -> String {
        let mut output = String::new();
        output.push_str("🌐 Type Dependency Graph\n");
        output.push_str("======================\n\n");

        // Show command entry points
        output.push_str("📋 Command Entry Points:\n");
        for cmd in entry_commands {
            output.push_str(&format!(
                "• {} ({}:{})\n",
                cmd.name, cmd.file_path, cmd.line_number
            ));

            // Show parameters
            for param in &cmd.parameters {
                output.push_str(&format!("  ├─ {}: {}\n", param.name, param.rust_type));
            }

            // Show return type
            output.push_str(&format!("  └─ returns: {}\n", cmd.return_type));
        }

        output.push_str("\n🏗️  Discovered Types:\n");
        for (type_name, struct_info) in &self.resolved_types {
            let type_kind = if struct_info.is_enum {
                "enum"
            } else {
                "struct"
            };
            output.push_str(&format!(
                "• {} ({}) - {} fields - defined in {}\n",
                type_name,
                type_kind,
                struct_info.fields.len(),
                struct_info.file_path
            ));

            // Show dependencies
            if let Some(deps) = self.dependencies.get(type_name) {
                if !deps.is_empty() {
                    let deps_list: Vec<String> = deps.iter().cloned().collect();
                    output.push_str(&format!("  └─ depends on: {}\n", deps_list.join(", ")));
                }
            }
        }

        // Show dependency chains
        output.push_str("\n🔗 Dependency Chains:\n");
        for type_name in self.resolved_types.keys() {
            self.show_dependency_chain(type_name, &mut output, 0);
        }

        output.push_str(&format!(
            "\n📊 Summary:\n• {} commands analyzed\n• {} types discovered\n• {} files with type definitions\n",
            entry_commands.len(),
            self.resolved_types.len(),
            self.type_definitions.len()
        ));

        output
    }

    /// Recursively show dependency chain for a type
    fn show_dependency_chain(&self, type_name: &str, output: &mut String, indent: usize) {
        let indent_str = "  ".repeat(indent);
        output.push_str(&format!("{}├─ {}\n", indent_str, type_name));

        if let Some(deps) = self.dependencies.get(type_name) {
            for dep in deps {
                if indent < 3 {
                    // Prevent too deep recursion in visualization
                    self.show_dependency_chain(dep, output, indent + 1);
                }
            }
        }
    }

    /// Generate a DOT graph representation of the dependency graph
    pub fn generate_dot_graph(&self, commands: &[CommandInfo]) -> String {
        let mut output = String::new();
        output.push_str("digraph Dependencies {\n");
        output.push_str("  rankdir=LR;\n");
        output.push_str("  node [shape=box];\n");
        output.push('\n');

        // Add command nodes
        for command in commands {
            output.push_str(&format!(
                "  \"{}\" [color=blue, style=filled, fillcolor=lightblue];\n",
                command.name
            ));
        }

        // Add type nodes
        for type_name in self.resolved_types.keys() {
            output.push_str(&format!("  \"{}\" [color=green];\n", type_name));
        }

        // Add edges from commands to their parameter/return types
        for command in commands {
            for param in &command.parameters {
                if self.resolved_types.contains_key(&param.rust_type) {
                    output.push_str(&format!(
                        "  \"{}\" -> \"{}\" [label=\"param\"];\n",
                        command.name, param.rust_type
                    ));
                }
            }
            if self.resolved_types.contains_key(&command.return_type) {
                output.push_str(&format!(
                    "  \"{}\" -> \"{}\" [label=\"return\"];\n",
                    command.name, command.return_type
                ));
            }
        }

        // Add type dependency edges
        for (type_name, deps) in &self.dependencies {
            for dep in deps {
                output.push_str(&format!("  \"{}\" -> \"{}\";\n", type_name, dep));
            }
        }

        output.push_str("}\n");
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_struct(name: &str, file: &str) -> StructInfo {
        StructInfo {
            name: name.to_string(),
            fields: vec![],
            file_path: file.to_string(),
            is_enum: false,
            serde_rename_all: None,
        }
    }

    #[test]
    fn test_new_graph() {
        let graph = TypeDependencyGraph::new();
        assert!(graph.type_definitions.is_empty());
        assert!(graph.dependencies.is_empty());
        assert!(graph.resolved_types.is_empty());
    }

    #[test]
    fn test_default_impl() {
        let graph = TypeDependencyGraph::default();
        assert!(graph.type_definitions.is_empty());
    }

    #[test]
    fn test_add_type_definition() {
        let mut graph = TypeDependencyGraph::new();
        graph.add_type_definition("User".to_string(), PathBuf::from("user.rs"));

        assert!(graph.has_type_definition("User"));
        assert_eq!(
            graph.get_type_definition_path("User"),
            Some(&PathBuf::from("user.rs"))
        );
    }

    #[test]
    fn test_add_multiple_type_definitions() {
        let mut graph = TypeDependencyGraph::new();
        graph.add_type_definition("User".to_string(), PathBuf::from("user.rs"));
        graph.add_type_definition("Post".to_string(), PathBuf::from("post.rs"));

        assert!(graph.has_type_definition("User"));
        assert!(graph.has_type_definition("Post"));
        assert_eq!(graph.type_definitions.len(), 2);
    }

    #[test]
    fn test_has_type_definition() {
        let mut graph = TypeDependencyGraph::new();
        graph.add_type_definition("User".to_string(), PathBuf::from("user.rs"));

        assert!(graph.has_type_definition("User"));
        assert!(!graph.has_type_definition("Post"));
    }

    #[test]
    fn test_add_dependency() {
        let mut graph = TypeDependencyGraph::new();
        graph.add_dependency("Post".to_string(), "User".to_string());

        let deps = graph.get_dependencies("Post");
        assert!(deps.is_some());
        assert!(deps.unwrap().contains("User"));
    }

    #[test]
    fn test_add_multiple_dependencies_to_same_type() {
        let mut graph = TypeDependencyGraph::new();
        graph.add_dependency("Post".to_string(), "User".to_string());
        graph.add_dependency("Post".to_string(), "Category".to_string());

        let deps = graph.get_dependencies("Post").unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains("User"));
        assert!(deps.contains("Category"));
    }

    #[test]
    fn test_add_dependencies_set() {
        let mut graph = TypeDependencyGraph::new();
        let mut deps = HashSet::new();
        deps.insert("User".to_string());
        deps.insert("Category".to_string());

        graph.add_dependencies("Post".to_string(), deps);

        let result_deps = graph.get_dependencies("Post").unwrap();
        assert_eq!(result_deps.len(), 2);
        assert!(result_deps.contains("User"));
        assert!(result_deps.contains("Category"));
    }

    #[test]
    fn test_get_dependencies_none() {
        let graph = TypeDependencyGraph::new();
        assert!(graph.get_dependencies("NonExistent").is_none());
    }

    #[test]
    fn test_add_resolved_type() {
        let mut graph = TypeDependencyGraph::new();
        let struct_info = create_test_struct("User", "user.rs");

        graph.add_resolved_type("User".to_string(), struct_info);

        assert!(graph.resolved_types.contains_key("User"));
        assert_eq!(graph.resolved_types.len(), 1);
    }

    #[test]
    fn test_get_resolved_types() {
        let mut graph = TypeDependencyGraph::new();
        let struct_info = create_test_struct("User", "user.rs");

        graph.add_resolved_type("User".to_string(), struct_info);

        let resolved = graph.get_resolved_types();
        assert_eq!(resolved.len(), 1);
        assert!(resolved.contains_key("User"));
    }

    // Topological sort tests
    mod topological_sort {
        use super::*;

        #[test]
        fn test_sort_single_type() {
            let graph = TypeDependencyGraph::new();
            let mut types = HashSet::new();
            types.insert("User".to_string());

            let sorted = graph.topological_sort_types(&types);
            assert_eq!(sorted, vec!["User"]);
        }

        #[test]
        fn test_sort_independent_types() {
            let graph = TypeDependencyGraph::new();
            let mut types = HashSet::new();
            types.insert("User".to_string());
            types.insert("Post".to_string());

            let sorted = graph.topological_sort_types(&types);
            assert_eq!(sorted.len(), 2);
            assert!(sorted.contains(&"User".to_string()));
            assert!(sorted.contains(&"Post".to_string()));
        }

        #[test]
        fn test_sort_linear_dependency() {
            let mut graph = TypeDependencyGraph::new();
            // Post depends on User
            graph.add_dependency("Post".to_string(), "User".to_string());

            let mut types = HashSet::new();
            types.insert("User".to_string());
            types.insert("Post".to_string());

            let sorted = graph.topological_sort_types(&types);
            // User should come before Post
            assert_eq!(sorted, vec!["User", "Post"]);
        }

        #[test]
        fn test_sort_diamond_dependency() {
            let mut graph = TypeDependencyGraph::new();
            // D depends on B and C
            // B depends on A
            // C depends on A
            graph.add_dependency("D".to_string(), "B".to_string());
            graph.add_dependency("D".to_string(), "C".to_string());
            graph.add_dependency("B".to_string(), "A".to_string());
            graph.add_dependency("C".to_string(), "A".to_string());

            let mut types = HashSet::new();
            types.insert("A".to_string());
            types.insert("B".to_string());
            types.insert("C".to_string());
            types.insert("D".to_string());

            let sorted = graph.topological_sort_types(&types);

            // A must come first (no dependencies)
            assert_eq!(sorted[0], "A");
            // D must come last (depends on everything)
            assert_eq!(sorted[3], "D");
            // B and C can be in either order but after A and before D
            let b_pos = sorted.iter().position(|x| x == "B").unwrap();
            let c_pos = sorted.iter().position(|x| x == "C").unwrap();
            assert!(b_pos > 0 && b_pos < 3);
            assert!(c_pos > 0 && c_pos < 3);
        }

        #[test]
        fn test_sort_chain_dependency() {
            let mut graph = TypeDependencyGraph::new();
            // A -> B -> C -> D (each depends on previous)
            graph.add_dependency("D".to_string(), "C".to_string());
            graph.add_dependency("C".to_string(), "B".to_string());
            graph.add_dependency("B".to_string(), "A".to_string());

            let mut types = HashSet::new();
            types.insert("A".to_string());
            types.insert("B".to_string());
            types.insert("C".to_string());
            types.insert("D".to_string());

            let sorted = graph.topological_sort_types(&types);
            assert_eq!(sorted, vec!["A", "B", "C", "D"]);
        }

        #[test]
        fn test_sort_circular_dependency() {
            let mut graph = TypeDependencyGraph::new();
            // A -> B -> C -> A (circular)
            graph.add_dependency("A".to_string(), "B".to_string());
            graph.add_dependency("B".to_string(), "C".to_string());
            graph.add_dependency("C".to_string(), "A".to_string());

            let mut types = HashSet::new();
            types.insert("A".to_string());
            types.insert("B".to_string());
            types.insert("C".to_string());

            // Should handle circular dependency without crashing
            let sorted = graph.topological_sort_types(&types);
            // All types should still be in the result
            assert_eq!(sorted.len(), 3);
        }

        #[test]
        fn test_sort_self_dependency() {
            let mut graph = TypeDependencyGraph::new();
            // A depends on itself
            graph.add_dependency("A".to_string(), "A".to_string());

            let mut types = HashSet::new();
            types.insert("A".to_string());

            // Should handle self-dependency without crashing
            let sorted = graph.topological_sort_types(&types);
            assert!(!sorted.is_empty());
        }

        #[test]
        fn test_sort_empty_set() {
            let graph = TypeDependencyGraph::new();
            let types = HashSet::new();

            let sorted = graph.topological_sort_types(&types);
            assert!(sorted.is_empty());
        }

        #[test]
        fn test_sort_type_with_missing_dependency() {
            let mut graph = TypeDependencyGraph::new();
            // Post depends on User, but User is not in the types set
            graph.add_dependency("Post".to_string(), "User".to_string());

            let mut types = HashSet::new();
            types.insert("Post".to_string());

            let sorted = graph.topological_sort_types(&types);
            // The topological sort follows dependencies, so User is included even though
            // it wasn't in the input set
            assert_eq!(sorted.len(), 2);
            assert_eq!(sorted, vec!["User", "Post"]);
        }
    }

    // Integration tests
    #[test]
    fn test_full_graph_workflow() {
        let mut graph = TypeDependencyGraph::new();

        // Add type definitions
        graph.add_type_definition("User".to_string(), PathBuf::from("user.rs"));
        graph.add_type_definition("Post".to_string(), PathBuf::from("post.rs"));

        // Add dependencies
        graph.add_dependency("Post".to_string(), "User".to_string());

        // Add resolved types
        graph.add_resolved_type("User".to_string(), create_test_struct("User", "user.rs"));
        graph.add_resolved_type("Post".to_string(), create_test_struct("Post", "post.rs"));

        // Verify everything
        assert!(graph.has_type_definition("User"));
        assert!(graph.has_type_definition("Post"));
        assert_eq!(graph.get_resolved_types().len(), 2);

        let deps = graph.get_dependencies("Post");
        assert!(deps.is_some());
        assert!(deps.unwrap().contains("User"));

        // Test topological sort
        let mut types = HashSet::new();
        types.insert("User".to_string());
        types.insert("Post".to_string());
        let sorted = graph.topological_sort_types(&types);
        assert_eq!(sorted, vec!["User", "Post"]);
    }
}
