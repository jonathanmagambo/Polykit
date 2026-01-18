//! Dependency graph management using petgraph.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::package::Package;

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub package: Package,
    pub index: NodeIndex,
}

/// Serializable graph data for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializableGraph {
    packages: Vec<Package>,
    #[serde(serialize_with = "serialize_arc_str_vec")]
    #[serde(deserialize_with = "deserialize_arc_str_vec")]
    topological_order: Vec<Arc<str>>,
    #[serde(serialize_with = "serialize_arc_str_vec_vec")]
    #[serde(deserialize_with = "deserialize_arc_str_vec_vec")]
    dependency_levels: Vec<Vec<Arc<str>>>,
}

fn serialize_arc_str_vec<S>(vec: &[Arc<str>], serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    let strings: Vec<&str> = vec.iter().map(|s| s.as_ref()).collect();
    strings.serialize(serializer)
}

fn deserialize_arc_str_vec<'de, D>(deserializer: D) -> std::result::Result<Vec<Arc<str>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let strings: Vec<String> = Vec::deserialize(deserializer)?;
    Ok(strings.into_iter().map(Arc::from).collect())
}

fn serialize_arc_str_vec_vec<S>(
    vec: &[Vec<Arc<str>>],
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    let strings: Vec<Vec<&str>> = vec
        .iter()
        .map(|level| level.iter().map(|s| s.as_ref()).collect())
        .collect();
    strings.serialize(serializer)
}

fn deserialize_arc_str_vec_vec<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<Vec<Arc<str>>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let strings: Vec<Vec<String>> = Vec::deserialize(deserializer)?;
    Ok(strings
        .into_iter()
        .map(|level| level.into_iter().map(Arc::from).collect())
        .collect())
}

/// Directed acyclic graph of package dependencies.
#[derive(Debug)]
pub struct DependencyGraph {
    graph: DiGraph<Arc<str>, ()>,
    #[allow(dead_code)]
    node_map: FxHashMap<Arc<str>, NodeIndex>,
    name_to_node: FxHashMap<String, NodeIndex>,
    packages: FxHashMap<NodeIndex, Package>,
    cached_topological_order: Vec<Arc<str>>,
    dependency_levels: Vec<Vec<Arc<str>>>,
}

impl DependencyGraph {
    /// Creates a new dependency graph from a list of packages.
    ///
    /// # Errors
    ///
    /// Returns an error if circular dependencies are detected.
    pub fn new(packages: Vec<Package>) -> Result<Self> {
        let package_count = packages.len();
        let mut graph = DiGraph::with_capacity(package_count, package_count * 2);
        let mut node_map = FxHashMap::with_capacity_and_hasher(package_count, Default::default());
        let mut name_to_node =
            FxHashMap::with_capacity_and_hasher(package_count, Default::default());
        let mut packages_map =
            FxHashMap::with_capacity_and_hasher(package_count, Default::default());

        let mut name_cache: FxHashMap<String, Arc<str>> =
            FxHashMap::with_capacity_and_hasher(package_count, Default::default());
        for package in &packages {
            let name_arc = Arc::from(package.name.as_str());
            name_cache.insert(package.name.clone(), Arc::clone(&name_arc));
        }

        for package in &packages {
            let name_arc = name_cache.get(&package.name).unwrap();
            let node = graph.add_node(Arc::clone(name_arc));
            node_map.insert(Arc::clone(name_arc), node);
            name_to_node.insert(package.name.clone(), node);
            packages_map.insert(node, package.clone());
        }

        for package in &packages {
            let name_arc = name_cache.get(&package.name).unwrap();
            let from_node = node_map
                .get(name_arc)
                .ok_or_else(|| Error::PackageNotFound {
                    name: package.name.clone(),
                    available: format!(
                        "Package '{}' not found during graph construction",
                        package.name
                    ),
                })?;

            for dep_name in &package.deps {
                let dep_arc = name_cache
                    .get(dep_name)
                    .ok_or_else(|| Error::PackageNotFound {
                        name: dep_name.clone(),
                        available: format!("Dependency '{}' not found", dep_name),
                    })?;
                let to_node = node_map.get(dep_arc).ok_or_else(|| {
                    let name = dep_name.clone();
                    Error::PackageNotFound {
                        name: name.clone(),
                        available: format!("Dependency '{}' not found", name),
                    }
                })?;

                graph.add_edge(*from_node, *to_node, ());
            }
        }

        let sorted = toposort(&graph, None).map_err(|cycle| {
            let cycle_node = graph[cycle.node_id()].as_ref();
            Error::CircularDependency(format!("Cycle detected involving: {}", cycle_node))
        })?;

        let topological_order: Vec<Arc<str>> = sorted
            .into_iter()
            .rev()
            .map(|idx| Arc::clone(&graph[idx]))
            .collect();

        let dependency_levels =
            Self::compute_dependency_levels(&graph, &node_map, &topological_order)?;

        Ok(Self {
            graph,
            node_map,
            name_to_node,
            packages: packages_map,
            cached_topological_order: topological_order,
            dependency_levels,
        })
    }

    fn compute_dependency_levels(
        graph: &DiGraph<Arc<str>, ()>,
        node_map: &FxHashMap<Arc<str>, NodeIndex>,
        order: &[Arc<str>],
    ) -> Result<Vec<Vec<Arc<str>>>> {
        let mut levels = Vec::new();
        let mut level_map = FxHashMap::with_capacity_and_hasher(order.len(), Default::default());

        for package_name in order {
            let node = node_map
                .get(package_name)
                .ok_or_else(|| Error::PackageNotFound {
                    name: package_name.to_string(),
                    available: format!("Package '{}' not found in node_map", package_name),
                })?;

            let deps: Vec<Arc<str>> = graph
                .neighbors_directed(*node, Direction::Outgoing)
                .map(|idx| Arc::clone(&graph[idx]))
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

            level_map.insert(Arc::clone(package_name), level);
            while levels.len() <= level {
                levels.push(Vec::new());
            }
            levels[level].push(Arc::clone(package_name));
        }

        Ok(levels)
    }

    /// Retrieves a package by name.
    #[inline]
    pub fn get_package(&self, name: &str) -> Option<&Package> {
        self.name_to_node
            .get(name)
            .and_then(|idx| self.packages.get(idx))
    }

    /// Returns packages in topological order (dependencies before dependents).
    ///
    /// This is cached during graph construction for fast access.
    #[inline]
    pub fn topological_order(&self) -> Vec<String> {
        self.cached_topological_order
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Returns dependency levels for parallel execution.
    ///
    /// Each level contains packages that can be executed in parallel.
    #[inline]
    pub fn dependency_levels(&self) -> Vec<Vec<String>> {
        self.dependency_levels
            .iter()
            .map(|level| level.iter().map(|s| s.to_string()).collect())
            .collect()
    }

    /// Returns direct dependencies of a package.
    ///
    /// # Errors
    ///
    /// Returns an error if the package is not found in the graph.
    pub fn dependencies(&self, package_name: &str) -> Result<Vec<String>> {
        let node = self
            .name_to_node
            .get(package_name)
            .ok_or_else(|| Error::PackageNotFound {
                name: package_name.to_string(),
                available: format!("Package '{}' not found", package_name),
            })?;

        let deps: Vec<String> = self
            .graph
            .neighbors_directed(*node, Direction::Outgoing)
            .map(|idx| self.graph[idx].to_string())
            .collect();

        Ok(deps)
    }

    /// Returns direct dependents of a package (packages that depend on it).
    ///
    /// # Errors
    ///
    /// Returns an error if the package is not found in the graph.
    pub fn dependents(&self, package_name: &str) -> Result<Vec<String>> {
        let node = self
            .name_to_node
            .get(package_name)
            .ok_or_else(|| Error::PackageNotFound {
                name: package_name.to_string(),
                available: format!("Package '{}' not found", package_name),
            })?;

        let dependents: Vec<String> = self
            .graph
            .neighbors_directed(*node, Direction::Incoming)
            .map(|idx| self.graph[idx].to_string())
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

    /// Serializes the graph to a file for fast loading.
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let packages: Vec<Package> = self.packages.values().cloned().collect();

        let serializable = SerializableGraph {
            packages,
            topological_order: self.cached_topological_order.clone(),
            dependency_levels: self.dependency_levels.clone(),
        };

        let serialized = bincode::serialize(&serializable).map_err(|e| Error::Adapter {
            package: "graph".to_string(),
            message: format!("Failed to serialize graph: {}", e),
        })?;

        let compressed = zstd::encode_all(&serialized[..], 3).map_err(|e| Error::Adapter {
            package: "graph".to_string(),
            message: format!("Failed to compress graph: {}", e),
        })?;

        fs::write(path, compressed).map_err(Error::Io)?;
        Ok(())
    }

    /// Loads a graph from a previously saved file.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let compressed = fs::read(path).map_err(Error::Io)?;
        let serialized = zstd::decode_all(&compressed[..]).map_err(|e| Error::Adapter {
            package: "graph".to_string(),
            message: format!("Failed to decompress graph: {}", e),
        })?;

        let serializable: SerializableGraph =
            bincode::deserialize(&serialized).map_err(|e| Error::Adapter {
                package: "graph".to_string(),
                message: format!("Failed to deserialize graph: {}", e),
            })?;

        Self::new(serializable.packages)
    }
}
