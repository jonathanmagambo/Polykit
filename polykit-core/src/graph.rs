//! Dependency graph management using petgraph.

use std::collections::{HashMap, HashSet};

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;

use crate::error::{Error, Result};
use crate::package::Package;

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub package: Package,
    pub index: NodeIndex,
}

/// Directed acyclic graph of package dependencies.
#[derive(Debug)]
pub struct DependencyGraph {
    graph: DiGraph<String, ()>,
    node_map: HashMap<String, NodeIndex>,
    packages: HashMap<NodeIndex, Package>,
    cached_topological_order: Vec<String>,
    dependency_levels: Vec<Vec<String>>,
}

impl DependencyGraph {
    /// Creates a new dependency graph from a list of packages.
    ///
    /// # Errors
    ///
    /// Returns an error if circular dependencies are detected.
    pub fn new(packages: Vec<Package>) -> Result<Self> {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();
        let mut packages_map = HashMap::new();

        for package in &packages {
            let node = graph.add_node(package.name.clone());
            node_map.insert(package.name.clone(), node);
            packages_map.insert(node, package.clone());
        }

        let all_names: Vec<String> = packages.iter().map(|p| p.name.clone()).collect();
        let available = all_names.join(", ");

        for package in &packages {
            let from_node = node_map
                .get(&package.name)
                .ok_or_else(|| Error::PackageNotFound {
                    name: package.name.clone(),
                    available: available.clone(),
                })?;

            for dep_name in &package.deps {
                let to_node = node_map
                    .get(dep_name)
                    .ok_or_else(|| Error::PackageNotFound {
                        name: dep_name.clone(),
                        available: available.clone(),
                    })?;

                graph.add_edge(*from_node, *to_node, ());
            }
        }

        let sorted = toposort(&graph, None).map_err(|cycle| {
            let cycle_node = graph[cycle.node_id()].clone();
            Error::CircularDependency(format!("Cycle detected involving: {}", cycle_node))
        })?;

        let topological_order: Vec<String> = sorted
            .into_iter()
            .rev()
            .map(|idx| graph[idx].clone())
            .collect();

        let dependency_levels =
            Self::compute_dependency_levels(&graph, &node_map, &topological_order)?;

        Ok(Self {
            graph,
            node_map,
            packages: packages_map,
            cached_topological_order: topological_order,
            dependency_levels,
        })
    }

    fn compute_dependency_levels(
        graph: &DiGraph<String, ()>,
        node_map: &HashMap<String, NodeIndex>,
        order: &[String],
    ) -> Result<Vec<Vec<String>>> {
        let mut levels = Vec::new();
        let mut level_map = HashMap::new();

        for package_name in order {
            let available: Vec<String> = node_map.keys().cloned().collect();
            let available_str = available.join(", ");
            let node = node_map
                .get(package_name)
                .ok_or_else(|| Error::PackageNotFound {
                    name: package_name.clone(),
                    available: available_str,
                })?;

            let deps: Vec<String> = graph
                .neighbors_directed(*node, Direction::Outgoing)
                .map(|idx| graph[idx].clone())
                .collect();

            let level = if deps.is_empty() {
                0
            } else {
                deps.iter()
                    .filter_map(|dep| level_map.get(dep))
                    .max()
                    .map(|l| l + 1)
                    .unwrap_or(0)
            };

            level_map.insert(package_name.clone(), level);
            while levels.len() <= level {
                levels.push(Vec::new());
            }
            levels[level].push(package_name.clone());
        }

        Ok(levels)
    }

    /// Retrieves a package by name.
    #[inline]
    pub fn get_package(&self, name: &str) -> Option<&Package> {
        self.node_map
            .get(name)
            .and_then(|idx| self.packages.get(idx))
    }

    /// Returns packages in topological order (dependencies before dependents).
    ///
    /// This is cached during graph construction for fast access.
    #[inline]
    pub fn topological_order(&self) -> &[String] {
        &self.cached_topological_order
    }

    /// Returns dependency levels for parallel execution.
    ///
    /// Each level contains packages that can be executed in parallel.
    #[inline]
    pub fn dependency_levels(&self) -> &[Vec<String>] {
        &self.dependency_levels
    }

    /// Returns direct dependencies of a package.
    ///
    /// # Errors
    ///
    /// Returns an error if the package is not found in the graph.
    pub fn dependencies(&self, package_name: &str) -> Result<Vec<String>> {
        let available: Vec<String> = self.node_map.keys().cloned().collect();
        let available_str = available.join(", ");
        let node = self
            .node_map
            .get(package_name)
            .ok_or_else(|| Error::PackageNotFound {
                name: package_name.to_string(),
                available: available_str,
            })?;

        let deps: Vec<String> = self
            .graph
            .neighbors_directed(*node, Direction::Outgoing)
            .map(|idx| self.graph[idx].clone())
            .collect();

        Ok(deps)
    }

    /// Returns direct dependents of a package (packages that depend on it).
    ///
    /// # Errors
    ///
    /// Returns an error if the package is not found in the graph.
    pub fn dependents(&self, package_name: &str) -> Result<Vec<String>> {
        let available: Vec<String> = self.node_map.keys().cloned().collect();
        let available_str = available.join(", ");
        let node = self
            .node_map
            .get(package_name)
            .ok_or_else(|| Error::PackageNotFound {
                name: package_name.to_string(),
                available: available_str,
            })?;

        let dependents: Vec<String> = self
            .graph
            .neighbors_directed(*node, Direction::Incoming)
            .map(|idx| self.graph[idx].clone())
            .collect();

        Ok(dependents)
    }

    /// Returns all transitive dependents of a package.
    ///
    /// This includes both direct and indirect dependents (packages that depend
    /// on packages that depend on this package, etc.).
    ///
    /// # Errors
    ///
    /// Returns an error if the package is not found in the graph.
    pub fn all_dependents(&self, package_name: &str) -> Result<HashSet<String>> {
        let mut result = HashSet::new();
        let mut stack = vec![package_name.to_string()];

        while let Some(current) = stack.pop() {
            if result.contains(&current) {
                continue;
            }
            result.insert(current.clone());

            let direct_dependents = self.dependents(&current)?;
            for dep in direct_dependents {
                if !result.contains(&dep) {
                    stack.push(dep);
                }
            }
        }

        result.remove(package_name);
        Ok(result)
    }

    /// Returns all packages affected by changes to the given packages.
    ///
    /// This includes the changed packages themselves and all their transitive
    /// dependents.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the changed packages are not found in the graph.
    pub fn affected_packages(&self, changed_packages: &[String]) -> Result<HashSet<String>> {
        let mut affected = HashSet::new();

        for package_name in changed_packages {
            affected.insert(package_name.clone());
            let dependents = self.all_dependents(package_name)?;
            affected.extend(dependents);
        }

        Ok(affected)
    }

    /// Returns all packages in the graph.
    pub fn all_packages(&self) -> Vec<&Package> {
        self.packages.values().collect()
    }
}
