use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DependencyError {
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
    #[error("Unresolved dependency: {0} required by {1}")]
    UnresolvedDependency(String, String),
    #[error("Invalid dependency specification: {0}")]
    InvalidSpecification(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DependencyNode {
    pub name: String,
    pub path: String,
    pub node_type: DependencyNodeType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DependencyNodeType {
    Command,
    Struct,
    Enum,
    Type,
    Module,
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub from: DependencyNode,
    pub to: DependencyNode,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DependencyType {
    /// Direct usage (parameter, return type)
    Direct,
    /// Field in a struct
    Field,
    /// Variant in an enum
    Variant,
    /// Import/use statement
    Import,
    /// Generic type parameter
    Generic,
}

pub struct DependencyResolver {
    dependencies: Vec<Dependency>,
    nodes: HashSet<DependencyNode>,
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {
            dependencies: Vec::new(),
            nodes: HashSet::new(),
        }
    }

    /// Add a dependency relationship
    pub fn add_dependency(&mut self, dependency: Dependency) {
        self.nodes.insert(dependency.from.clone());
        self.nodes.insert(dependency.to.clone());
        self.dependencies.push(dependency);
    }

    /// Add a node without dependencies
    pub fn add_node(&mut self, node: DependencyNode) {
        self.nodes.insert(node);
    }

    /// Resolve dependencies and return them in topological order
    pub fn resolve_build_order(&self) -> Result<Vec<DependencyNode>, DependencyError> {
        let mut in_degree = HashMap::new();
        let mut adjacency = HashMap::new();

        // Initialize in-degree count and adjacency list
        for node in &self.nodes {
            in_degree.insert(node.clone(), 0);
            adjacency.insert(node.clone(), Vec::new());
        }

        // Build adjacency list and count in-degrees
        // If "from" uses "to", then "to" should be processed before "from"
        // So we create an edge from "to" to "from" for the topological sort
        for dep in &self.dependencies {
            adjacency
                .get_mut(&dep.to)
                .unwrap()
                .push(dep.from.clone());
            
            *in_degree.get_mut(&dep.from).unwrap() += 1;
        }

        // Topological sort using Kahn's algorithm
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Find all nodes with no incoming edges
        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node.clone());
            }
        }

        while let Some(node) = queue.pop_front() {
            result.push(node.clone());

            // Remove this node and update in-degrees of adjacent nodes
            if let Some(adjacent_nodes) = adjacency.get(&node) {
                for adjacent in adjacent_nodes {
                    let degree = in_degree.get_mut(adjacent).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(adjacent.clone());
                    }
                }
            }
        }

        // Check for circular dependencies
        if result.len() != self.nodes.len() {
            let remaining: Vec<String> = self.nodes
                .iter()
                .filter(|n| !result.contains(n))
                .map(|n| n.name.clone())
                .collect();
            return Err(DependencyError::CircularDependency(remaining.join(", ")));
        }

        Ok(result)
    }

    /// Get dependencies for a specific node
    pub fn get_dependencies_for(&self, node: &DependencyNode) -> Vec<&Dependency> {
        self.dependencies
            .iter()
            .filter(|dep| dep.from == *node)
            .collect()
    }

    /// Get reverse dependencies (dependents) for a specific node
    pub fn get_dependents_of(&self, node: &DependencyNode) -> Vec<&Dependency> {
        self.dependencies
            .iter()
            .filter(|dep| dep.to == *node)
            .collect()
    }

    /// Check if there are any unresolved dependencies
    pub fn validate_dependencies(&self) -> Result<(), DependencyError> {
        for dep in &self.dependencies {
            if !self.nodes.contains(&dep.from) {
                return Err(DependencyError::UnresolvedDependency(
                    dep.from.name.clone(),
                    "unknown".to_string(),
                ));
            }
            if !self.nodes.contains(&dep.to) {
                return Err(DependencyError::UnresolvedDependency(
                    dep.to.name.clone(),
                    dep.from.name.clone(),
                ));
            }
        }
        Ok(())
    }

    /// Generate a visual representation of the dependency graph
    pub fn generate_dot_graph(&self) -> String {
        let mut dot = String::from("digraph Dependencies {\n");
        dot.push_str("    rankdir=LR;\n");
        dot.push_str("    node [shape=box];\n\n");

        // Add nodes with different shapes based on type
        for node in &self.nodes {
            let (shape, color) = match node.node_type {
                DependencyNodeType::Command => ("ellipse", "lightblue"),
                DependencyNodeType::Struct => ("box", "lightgreen"),
                DependencyNodeType::Enum => ("diamond", "lightyellow"),
                DependencyNodeType::Type => ("circle", "lightgray"),
                DependencyNodeType::Module => ("folder", "lightcoral"),
            };
            
            dot.push_str(&format!(
                "    \"{}\" [shape={}, fillcolor={}, style=filled];\n",
                node.name, shape, color
            ));
        }

        dot.push('\n');

        // Add edges with different styles based on dependency type
        for dep in &self.dependencies {
            let style = match dep.dependency_type {
                DependencyType::Direct => "solid",
                DependencyType::Field => "dashed",
                DependencyType::Variant => "dotted",
                DependencyType::Import => "bold",
                DependencyType::Generic => "double",
            };
            
            dot.push_str(&format!(
                "    \"{}\" -> \"{}\" [style={}];\n",
                dep.from.name, dep.to.name, style
            ));
        }

        dot.push_str("}\n");
        dot
    }

    /// Generate a text-based visualization of dependencies
    pub fn generate_text_graph(&self) -> String {
        let mut output = String::from("Dependency Graph:\n");
        output.push_str("=================\n\n");

        for node in &self.nodes {
            let deps = self.get_dependencies_for(node);
            let dependents = self.get_dependents_of(node);

            output.push_str(&format!("{} ({:?})\n", node.name, node.node_type));
            
            if !deps.is_empty() {
                output.push_str("  Dependencies:\n");
                for dep in deps {
                    output.push_str(&format!(
                        "    -> {} ({:?})\n",
                        dep.to.name, dep.dependency_type
                    ));
                }
            }
            
            if !dependents.is_empty() {
                output.push_str("  Dependents:\n");
                for dep in dependents {
                    output.push_str(&format!(
                        "    <- {} ({:?})\n",
                        dep.from.name, dep.dependency_type
                    ));
                }
            }
            
            output.push('\n');
        }

        output
    }

    /// Group nodes by their type for organized code generation
    pub fn group_by_type(&self) -> HashMap<DependencyNodeType, Vec<DependencyNode>> {
        let mut groups = HashMap::new();
        
        for node in &self.nodes {
            groups
                .entry(node.node_type.clone())
                .or_insert_with(Vec::new)
                .push(node.clone());
        }
        
        groups
    }

    /// Get the dependency depth for a node (longest path to a leaf)
    pub fn get_dependency_depth(&self, node: &DependencyNode) -> usize {
        let mut visited = HashSet::new();
        self.calculate_depth(node, &mut visited)
    }

    fn calculate_depth(&self, node: &DependencyNode, visited: &mut HashSet<DependencyNode>) -> usize {
        if visited.contains(node) {
            return 0; // Avoid infinite recursion on cycles
        }
        
        visited.insert(node.clone());
        
        let max_child_depth = self.get_dependencies_for(node)
            .iter()
            .map(|dep| self.calculate_depth(&dep.to, visited))
            .max()
            .unwrap_or(0);
        
        visited.remove(node);
        max_child_depth + 1
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_node(name: &str, node_type: DependencyNodeType) -> DependencyNode {
        DependencyNode {
            name: name.to_string(),
            path: format!("/test/{}.rs", name),
            node_type,
        }
    }

    #[test]
    fn test_simple_dependency_resolution() {
        let mut resolver = DependencyResolver::new();
        
        let node_a = create_test_node("A", DependencyNodeType::Struct);
        let node_b = create_test_node("B", DependencyNodeType::Struct);
        
        resolver.add_node(node_a.clone());
        resolver.add_node(node_b.clone());
        
        // B depends on A (B uses A), so A must be processed before B
        // In our system: "from" uses "to", which means "to" should come first
        resolver.add_dependency(Dependency {
            from: node_b.clone(),
            to: node_a.clone(),
            dependency_type: DependencyType::Direct,
        });
        
        let order = resolver.resolve_build_order().unwrap();
        assert_eq!(order.len(), 2);
        
        // A should come before B since B depends on A
        let a_pos = order.iter().position(|n| n.name == "A").unwrap();
        let b_pos = order.iter().position(|n| n.name == "B").unwrap();
        assert!(a_pos < b_pos, "A should come before B, but got order: {:?}", order.iter().map(|n| &n.name).collect::<Vec<_>>());
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut resolver = DependencyResolver::new();
        
        let node_a = create_test_node("A", DependencyNodeType::Struct);
        let node_b = create_test_node("B", DependencyNodeType::Struct);
        
        resolver.add_node(node_a.clone());
        resolver.add_node(node_b.clone());
        resolver.add_dependency(Dependency {
            from: node_a.clone(),
            to: node_b.clone(),
            dependency_type: DependencyType::Direct,
        });
        resolver.add_dependency(Dependency {
            from: node_b.clone(),
            to: node_a.clone(),
            dependency_type: DependencyType::Direct,
        });
        
        let result = resolver.resolve_build_order();
        assert!(result.is_err());
        if let Err(DependencyError::CircularDependency(_)) = result {
            // Expected
        } else {
            panic!("Expected circular dependency error");
        }
    }

    #[test]
    fn test_complex_dependency_chain() {
        let mut resolver = DependencyResolver::new();
        
        let node_a = create_test_node("A", DependencyNodeType::Struct);
        let node_b = create_test_node("B", DependencyNodeType::Struct);
        let node_c = create_test_node("C", DependencyNodeType::Struct);
        let node_d = create_test_node("D", DependencyNodeType::Command);
        
        resolver.add_node(node_a.clone());
        resolver.add_node(node_b.clone());
        resolver.add_node(node_c.clone());
        resolver.add_node(node_d.clone());
        
        // D depends on C, C depends on B, B depends on A
        resolver.add_dependency(Dependency {
            from: node_d.clone(),
            to: node_c.clone(),
            dependency_type: DependencyType::Direct,
        });
        resolver.add_dependency(Dependency {
            from: node_c.clone(),
            to: node_b.clone(),
            dependency_type: DependencyType::Field,
        });
        resolver.add_dependency(Dependency {
            from: node_b.clone(),
            to: node_a.clone(),
            dependency_type: DependencyType::Direct,
        });
        
        let order = resolver.resolve_build_order().unwrap();
        assert_eq!(order.len(), 4);
        
        // Verify ordering: A -> B -> C -> D
        let positions: HashMap<String, usize> = order
            .iter()
            .enumerate()
            .map(|(i, n)| (n.name.clone(), i))
            .collect();
        
        assert!(positions["A"] < positions["B"]);
        assert!(positions["B"] < positions["C"]);
        assert!(positions["C"] < positions["D"]);
    }

    #[test]
    fn test_dependency_depth_calculation() {
        let mut resolver = DependencyResolver::new();
        
        let node_a = create_test_node("A", DependencyNodeType::Struct);
        let node_b = create_test_node("B", DependencyNodeType::Struct);
        let node_c = create_test_node("C", DependencyNodeType::Command);
        
        resolver.add_node(node_a.clone());
        resolver.add_node(node_b.clone());
        resolver.add_node(node_c.clone());
        
        // C -> B -> A
        resolver.add_dependency(Dependency {
            from: node_c.clone(),
            to: node_b.clone(),
            dependency_type: DependencyType::Direct,
        });
        resolver.add_dependency(Dependency {
            from: node_b.clone(),
            to: node_a.clone(),
            dependency_type: DependencyType::Direct,
        });
        
        assert_eq!(resolver.get_dependency_depth(&node_a), 1); // Leaf node
        assert_eq!(resolver.get_dependency_depth(&node_b), 2); // A + itself
        assert_eq!(resolver.get_dependency_depth(&node_c), 3); // A + B + itself
    }

    #[test]
    fn test_group_by_type() {
        let mut resolver = DependencyResolver::new();
        
        let struct_a = create_test_node("StructA", DependencyNodeType::Struct);
        let struct_b = create_test_node("StructB", DependencyNodeType::Struct);
        let cmd_a = create_test_node("CommandA", DependencyNodeType::Command);
        let enum_a = create_test_node("EnumA", DependencyNodeType::Enum);
        
        resolver.add_node(struct_a);
        resolver.add_node(struct_b);
        resolver.add_node(cmd_a);
        resolver.add_node(enum_a);
        
        let groups = resolver.group_by_type();
        
        assert_eq!(groups.get(&DependencyNodeType::Struct).unwrap().len(), 2);
        assert_eq!(groups.get(&DependencyNodeType::Command).unwrap().len(), 1);
        assert_eq!(groups.get(&DependencyNodeType::Enum).unwrap().len(), 1);
    }
}