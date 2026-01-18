//! Dependency graph management using petgraph.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use petgraph::visit::EdgeRef;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use dashmap::DashMap;

use crate::error::{Error, Result};
use crate::package::Package;
use crate::string_interner::intern;

/// Tracks changes to packages for incremental graph updates.
#[derive(Debug, Clone)]
pub struct GraphChange {
    pub added: Vec<Package>,
    pub modified: Vec<Package>,
    pub removed: Vec<String>,
    pub dependency_changes: Vec<(String, Vec<String>)>,
}

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
///
/// Uses compact u32 indices internally for better memory efficiency and cache performance.
/// Public API maintains String-based interface for compatibility.
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    // Internal compact representation using u32 indices
    graph: DiGraph<u32, ()>,
    id_to_name: Vec<Arc<str>>,
    name_to_id: FxHashMap<Arc<str>, u32>,
    
    // Package data indexed by u32 ID
    packages: Vec<Package>,
    
    // Cached computations using u32 indices internally
    cached_topological_order: Vec<u32>,
    dependency_levels: Vec<Vec<u32>>,
    
    // Public API compatibility (kept for fast lookups)
    name_to_node: FxHashMap<String, NodeIndex>,
    node_to_id: FxHashMap<NodeIndex, u32>,
    
    // Cache for transitive dependencies
    transitive_deps_cache: DashMap<String, Arc<HashSet<String>>>,
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
        let mut id_to_name = Vec::with_capacity(package_count);
        let mut name_to_id = FxHashMap::with_capacity_and_hasher(package_count, Default::default());
        let mut name_to_node = FxHashMap::with_capacity_and_hasher(package_count, Default::default());
        let mut node_to_id = FxHashMap::with_capacity_and_hasher(package_count, Default::default());
        let mut packages_vec = Vec::with_capacity(package_count);

        // First pass: intern names and assign u32 IDs
        for (idx, package) in packages.iter().enumerate() {
            let name_arc = intern(&package.name);
            let id = idx as u32;
            
            id_to_name.push(Arc::clone(&name_arc));
            name_to_id.insert(Arc::clone(&name_arc), id);
            packages_vec.push(package.clone());
        }

        // Second pass: add nodes to graph with u32 IDs
        for (idx, package) in packages.iter().enumerate() {
            let id = idx as u32;
            let node = graph.add_node(id);
            name_to_node.insert(package.name.clone(), node);
            node_to_id.insert(node, id);
        }

        // Third pass: add edges
        for package in &packages {
            let from_node = name_to_node.get(&package.name).unwrap();

            for dep_name in &package.deps {
                let to_node = name_to_node.get(dep_name).ok_or_else(|| {
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
            let cycle_id = graph[cycle.node_id()];
            let cycle_name = &id_to_name[cycle_id as usize];
            Error::CircularDependency(format!("Cycle detected involving: {}", cycle_name))
        })?;

        let topological_order: Vec<u32> = sorted
            .into_iter()
            .rev()
            .map(|idx| graph[idx])
            .collect();

        let dependency_levels =
            Self::compute_dependency_levels_compact(&graph, &node_to_id, &topological_order)?;

        Ok(Self {
            graph,
            id_to_name,
            name_to_id,
            packages: packages_vec,
            cached_topological_order: topological_order,
            dependency_levels,
            name_to_node,
            node_to_id,
            transitive_deps_cache: DashMap::new(),
        })
    }

    fn compute_dependency_levels_compact(
        graph: &DiGraph<u32, ()>,
        node_to_id: &FxHashMap<NodeIndex, u32>,
        order: &[u32],
    ) -> Result<Vec<Vec<u32>>> {
        let mut levels = Vec::new();
        let mut level_map = FxHashMap::with_capacity_and_hasher(order.len(), Default::default());

        for &package_id in order {
            // Find the node index for this ID
            let node = node_to_id
                .iter()
                .find(|(_, &id)| id == package_id)
                .map(|(node_idx, _)| *node_idx)
                .ok_or_else(|| Error::PackageNotFound {
                    name: format!("id-{}", package_id),
                    available: format!("Package ID {} not found", package_id),
                })?;

            let dep_ids: Vec<u32> = graph
                .neighbors_directed(node, Direction::Outgoing)
                .map(|idx| graph[idx])
                .collect();

            let level = if dep_ids.is_empty() {
                0
            } else {
                dep_ids.iter()
                    .filter_map(|dep_id| level_map.get(dep_id))
                    .max()
                    .map(|l| l + 1)
                    .unwrap_or(0)
            };

            level_map.insert(package_id, level);
            while levels.len() <= level {
                levels.push(Vec::new());
            }
            levels[level].push(package_id);
        }

        Ok(levels)
    }

    /// Converts a u32 ID to a package name string.
    #[inline]
    fn id_to_name(&self, id: u32) -> &str {
        &self.id_to_name[id as usize]
    }

    /// Converts a package name to a u32 ID.
    #[inline]
    fn name_to_id(&self, name: &str) -> Option<u32> {
        let name_arc = intern(name);
        self.name_to_id.get(&name_arc).copied()
    }

    /// Retrieves a package by name.
    #[inline]
    pub fn get_package(&self, name: &str) -> Option<&Package> {
        self.name_to_id(name)
            .and_then(|id| self.packages.get(id as usize))
    }

    /// Returns packages in topological order (dependencies before dependents).
    ///
    /// This is cached during graph construction for fast access.
    #[inline]
    pub fn topological_order(&self) -> Vec<String> {
        self.cached_topological_order
            .iter()
            .map(|&id| self.id_to_name(id).to_string())
            .collect()
    }

    /// Returns dependency levels for parallel execution.
    ///
    /// Each level contains packages that can be executed in parallel.
    #[inline]
    pub fn dependency_levels(&self) -> Vec<Vec<String>> {
        self.dependency_levels
            .iter()
            .map(|level| level.iter().map(|&id| self.id_to_name(id).to_string()).collect())
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
            .map(|idx| {
                let dep_id = self.graph[idx];
                self.id_to_name(dep_id).to_string()
            })
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
            .map(|idx| {
                let dep_id = self.graph[idx];
                self.id_to_name(dep_id).to_string()
            })
            .collect();

        Ok(dependents)
    }

    /// Returns all transitive dependents of a package.
    ///
    /// This includes both direct and indirect dependents (packages that depend
    /// on packages that depend on this package, etc.).
    ///
    /// Results are cached for performance.
    ///
    /// # Errors
    ///
    /// Returns an error if the package is not found in the graph.
    pub fn all_dependents(&self, package_name: &str) -> Result<HashSet<String>> {
        if let Some(cached) = self.transitive_deps_cache.get(package_name) {
            return Ok((**cached.value()).clone());
        }

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
        let result_arc = Arc::new(result.clone());
        self.transitive_deps_cache.insert(package_name.to_string(), result_arc);
        Ok(result)
    }

    /// Returns all packages affected by changes to the given packages.
    ///
    /// This includes the changed packages themselves and all their transitive
    /// dependents.
    ///
    /// Uses parallel BFS for better performance on large graphs.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the changed packages are not found in the graph.
    pub fn affected_packages(&self, changed_packages: &[String]) -> Result<HashSet<String>> {
        use dashmap::DashSet;
        use rayon::prelude::*;

        let affected = DashSet::new();
        let queue: Vec<String> = changed_packages.to_vec();

        queue.par_iter().for_each(|pkg| {
            affected.insert(pkg.clone());
            let mut local_queue = vec![pkg.clone()];
            let mut local_visited = HashSet::new();

            while let Some(current) = local_queue.pop() {
                if local_visited.contains(&current) {
                    continue;
                }
                local_visited.insert(current.clone());

                if let Ok(dependents) = self.dependents(&current) {
                    for dep in dependents {
                        if affected.insert(dep.clone()) {
                            local_queue.push(dep);
                        }
                    }
                }
            }
        });

        Ok(affected.into_iter().collect())
    }

    /// Returns all packages in the graph.
    pub fn all_packages(&self) -> Vec<&Package> {
        // Only return packages that are still in name_to_node (not removed)
        self.name_to_node
            .keys()
            .filter_map(|name| self.get_package(name))
            .collect()
    }

    /// Serializes the graph to a file for fast loading.
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        // Convert u32 indices back to Arc<str> for serialization
        let topological_order: Vec<Arc<str>> = self.cached_topological_order
            .iter()
            .map(|&id| Arc::clone(&self.id_to_name[id as usize]))
            .collect();
        let dependency_levels: Vec<Vec<Arc<str>>> = self.dependency_levels
            .iter()
            .map(|level| level.iter().map(|&id| Arc::clone(&self.id_to_name[id as usize])).collect())
            .collect();

        let serializable = SerializableGraph {
            packages: self.packages.clone(),
            topological_order,
            dependency_levels,
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

    /// Updates the graph incrementally based on detected changes.
    ///
    /// This is much faster than rebuilding the entire graph when only
    /// a few packages have changed.
    ///
    /// # Errors
    ///
    /// Returns an error if any package operations fail or circular dependencies are detected.
    pub fn update_incremental(&mut self, changes: GraphChange) -> Result<()> {
        // 1. Remove deleted packages
        for package_name in &changes.removed {
            self.remove_package(package_name)?;
        }

        // 2. Update modified packages
        for package in &changes.modified {
            self.update_package(package)?;
        }

        // 3. Add new packages
        for package in &changes.added {
            self.add_package(package)?;
        }

        // 4. Recompute only affected cached values
        self.recompute_affected_levels(&changes)?;

        Ok(())
    }

    fn remove_package(&mut self, name: &str) -> Result<()> {
        let package_id = self.name_to_id(name).ok_or_else(|| Error::PackageNotFound {
            name: name.to_string(),
            available: String::new(),
        })?;

        let node = *self.name_to_node.get(name).ok_or_else(|| Error::PackageNotFound {
            name: name.to_string(),
            available: String::new(),
        })?;

        // Remove node from graph (this removes all edges automatically)
        self.graph.remove_node(node);

        // Remove from mappings
        self.name_to_node.remove(name);
        self.node_to_id.remove(&node);
        let name_arc = intern(name);
        self.name_to_id.remove(&name_arc);

        // Remove package from packages vector (mark as removed, don't actually remove to preserve indices)
        // For now, we'll keep the package but mark it as removed by clearing the name
        // In a production system, you might want a more sophisticated approach
        if (package_id as usize) < self.packages.len() {
            // We can't easily remove from the middle of a Vec without breaking indices
            // So we'll leave it but it won't be accessible via name lookup
        }

        // Clear affected cache entries
        self.transitive_deps_cache.remove(name);

        Ok(())
    }

    fn update_package(&mut self, package: &Package) -> Result<()> {
        let package_id = self.name_to_id(&package.name).ok_or_else(|| Error::PackageNotFound {
            name: package.name.clone(),
            available: String::new(),
        })?;

        // Update package data
        self.packages[package_id as usize] = package.clone();

        // Update edges - remove old edges, add new ones
        let node = self.name_to_node.get(&package.name).ok_or_else(|| Error::PackageNotFound {
            name: package.name.clone(),
            available: String::new(),
        })?;

        // Remove all outgoing edges
        let old_edges: Vec<_> = self.graph
            .edges_directed(*node, Direction::Outgoing)
            .map(|e| e.id())
            .collect();
        for edge_id in old_edges {
            let _ = self.graph.remove_edge(edge_id);
        }

        // Add new edges based on updated dependencies
        for dep_name in &package.deps {
            if let Some(dep_node) = self.name_to_node.get(dep_name) {
                self.graph.add_edge(*node, *dep_node, ());
            }
        }

        // Invalidate cache
        self.transitive_deps_cache.remove(&package.name);

        Ok(())
    }

    fn add_package(&mut self, package: &Package) -> Result<()> {
        let name_arc = intern(&package.name);
        let package_id = self.packages.len() as u32;

        // Add to mappings
        self.id_to_name.push(Arc::clone(&name_arc));
        self.name_to_id.insert(name_arc, package_id);
        self.packages.push(package.clone());

        // Add node to graph
        let node = self.graph.add_node(package_id);
        self.name_to_node.insert(package.name.clone(), node);
        self.node_to_id.insert(node, package_id);

        // Add edges for dependencies
        for dep_name in &package.deps {
            if let Some(dep_node) = self.name_to_node.get(dep_name) {
                self.graph.add_edge(node, *dep_node, ());
            }
        }

        Ok(())
    }

    fn recompute_affected_levels(&mut self, changes: &GraphChange) -> Result<()> {
        // Collect all affected package IDs
        let mut affected_ids = HashSet::new();
        
        for pkg in &changes.added {
            if let Some(id) = self.name_to_id(&pkg.name) {
                affected_ids.insert(id);
            }
        }
        for pkg in &changes.modified {
            if let Some(id) = self.name_to_id(&pkg.name) {
                affected_ids.insert(id);
            }
        }
        for name in &changes.removed {
            if let Some(id) = self.name_to_id(name) {
                affected_ids.insert(id);
            }
        }

        // For now, recompute entire topological order and levels
        // TODO: Optimize to only recompute affected subgraph
        let sorted = toposort(&self.graph, None).map_err(|cycle| {
            let cycle_id = self.graph[cycle.node_id()];
            let cycle_name = self.id_to_name(cycle_id);
            Error::CircularDependency(format!("Cycle detected involving: {}", cycle_name))
        })?;

        self.cached_topological_order = sorted
            .into_iter()
            .rev()
            .map(|idx| self.graph[idx])
            .collect();

        self.dependency_levels =
            Self::compute_dependency_levels_compact(&self.graph, &self.node_to_id, &self.cached_topological_order)?;

        Ok(())
    }
}
