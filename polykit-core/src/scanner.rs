//! Repository scanner for discovering packages.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rayon::prelude::*;
use rustc_hash::FxHashMap;
use jwalk::WalkDir as JWalkDir;
use memmap2::Mmap;
use std::fs::File;

use crate::cache::Cache;
use crate::config::{Config, WorkspaceConfig};
use crate::error::Result;
use crate::package::Package;
use crate::simd_utils;

fn get_default_cache_dir() -> std::path::PathBuf {
    dirs::cache_dir()
        .map(|d| d.join("polykit"))
        .unwrap_or_else(|| std::env::temp_dir().join("polykit-cache"))
}

/// Scans a directory for packages.
///
/// Looks for `polykit.toml` files and parses them into `Package` structures.
/// Uses caching for fast incremental scans.
pub struct Scanner {
    packages_dir: PathBuf,
    cache: Option<Cache>,
    workspace_config: Option<WorkspaceConfig>,
}

impl Scanner {
    fn load_workspace_config(packages_dir: &Path) -> Option<WorkspaceConfig> {
        let mut current_dir = packages_dir.parent()?;

        loop {
            let workspace_toml = current_dir.join("polykit.toml");
            if workspace_toml.exists() {
                let content = std::fs::read_to_string(&workspace_toml).ok()?;
                let mut table: toml::Value = toml::from_str(&content).ok()?;
                let workspace_table = table.get_mut("workspace")?.as_table_mut()?;

                let mut config = WorkspaceConfig {
                    cache_dir: workspace_table
                        .get("cache_dir")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    default_parallel: workspace_table
                        .get("default_parallel")
                        .and_then(|v| v.as_integer())
                        .map(|i| i as usize),
                    workspace_config_path: Some(workspace_toml),
                    tasks: FxHashMap::default(),
                    remote_cache: None,
                };

                if let Some(tasks_table) = workspace_table.get("tasks").and_then(|v| v.as_table()) {
                    config.tasks = crate::config::parse_tasks_from_toml_map(tasks_table);
                }

                return Some(config);
            }

            if current_dir.join(".git").exists() {
                break;
            }

            match current_dir.parent() {
                Some(parent) => {
                    if parent == current_dir {
                        break;
                    }
                    current_dir = parent;
                }
                None => break,
            }
        }

        None
    }

    pub fn new(packages_dir: impl AsRef<Path>) -> Self {
        let packages_dir = packages_dir.as_ref().to_path_buf();
        let workspace_config = Self::load_workspace_config(&packages_dir);
        Self {
            packages_dir,
            cache: None,
            workspace_config,
        }
    }

    pub fn with_default_cache(packages_dir: impl AsRef<Path>) -> Self {
        let packages_dir = packages_dir.as_ref().to_path_buf();
        let workspace_config = Self::load_workspace_config(&packages_dir);
        let cache_dir = workspace_config
            .as_ref()
            .and_then(|wc| {
                wc.cache_dir.as_ref().map(|cache_dir_str| {
                    let cache_path = PathBuf::from(cache_dir_str);
                    if cache_path.is_absolute() {
                        cache_path
                    } else {
                        wc.workspace_config_path
                            .as_ref()
                            .and_then(|config_path| config_path.parent())
                            .map(|config_dir| config_dir.join(&cache_path))
                            .unwrap_or_else(|| PathBuf::from(cache_dir_str))
                    }
                })
            })
            .unwrap_or_else(get_default_cache_dir);
        Self {
            packages_dir,
            cache: Some(Cache::new(cache_dir)),
            workspace_config,
        }
    }

    pub fn with_cache(packages_dir: impl AsRef<Path>, cache_dir: impl AsRef<Path>) -> Self {
        let packages_dir = packages_dir.as_ref().to_path_buf();
        let workspace_config = Self::load_workspace_config(&packages_dir);
        Self {
            packages_dir,
            cache: Some(Cache::new(cache_dir)),
            workspace_config,
        }
    }

    pub fn workspace_config(&self) -> Option<&WorkspaceConfig> {
        self.workspace_config.as_ref()
    }

    pub fn cache_stats(&self) -> Option<&crate::cache::CacheStats> {
        self.cache.as_ref().map(|c| c.stats())
    }

    pub fn scan(&mut self) -> Result<Vec<Package>> {
        if let Some(ref mut cache) = self.cache {
            if let Some(cached) = cache.load(&self.packages_dir)? {
                return Ok(cached);
            }
        }

        let packages = self.scan_internal()?;

        if let Some(ref mut cache) = self.cache {
            cache.save(&self.packages_dir, &packages)?;
        }

        Ok(packages)
    }

    #[inline]
    fn scan_internal(&self) -> Result<Vec<Package>> {
        let workspace_config = Arc::new(self.workspace_config.clone());
        let packages_dir = Arc::new(self.packages_dir.clone());

        let config_files: Vec<PathBuf> = JWalkDir::new(&self.packages_dir)
            .max_depth(2)
            .follow_links(false)
            .parallelism(jwalk::Parallelism::RayonNewPool(rayon::current_num_threads()))
            .into_iter()
            .filter_map(|e| {
                let entry = e.ok()?;
                let name_bytes = entry.file_name().as_encoded_bytes();
                if simd_utils::fast_str_eq(
                    std::str::from_utf8(name_bytes).unwrap_or(""),
                    "polykit.toml",
                ) {
                    Some(entry.path().to_path_buf())
                } else {
                    None
                }
            })
            .collect();

        let packages: Result<Vec<Package>> = config_files
            .into_par_iter()
            .map(|config_path| {
                let package_path = config_path
                    .parent()
                    .ok_or_else(|| crate::error::Error::ConfigNotFound(config_path.clone()))?;

                let config = Self::read_config_mmap(&config_path)?;

                crate::command_validator::CommandValidator::validate_identifier(
                    &config.name,
                    "Package name",
                )?;

                for dep_name in &config.deps.internal {
                    crate::command_validator::CommandValidator::validate_identifier(
                        dep_name,
                        "Dependency name",
                    )?;
                }

                let language = config.parse_language()?;
                let relative_path = package_path
                    .strip_prefix(packages_dir.as_ref())
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|_| package_path.to_path_buf());

                let mut package_tasks = config.to_tasks();

                for task in &package_tasks {
                    crate::command_validator::CommandValidator::validate_identifier(
                        &task.name,
                        "Task name",
                    )?;
                }

                if let Some(ref ws_config) = workspace_config.as_ref() {
                    let workspace_tasks = ws_config.to_tasks();
                    for workspace_task in workspace_tasks {
                        crate::command_validator::CommandValidator::validate_identifier(
                            &workspace_task.name,
                            "Workspace task name",
                        )?;
                        if !package_tasks.iter().any(|t| t.name == workspace_task.name) {
                            package_tasks.push(workspace_task);
                        }
                    }
                }

                Ok(Package::new(
                    config.name,
                    language,
                    config.public,
                    relative_path,
                    config.deps.internal,
                    package_tasks,
                ))
            })
            .collect();

        let mut packages = packages?;
        packages.sort_unstable_by(|a, b| a.name.cmp(&b.name));
        Ok(packages)
    }

    pub fn scan_as_map(&mut self) -> Result<FxHashMap<String, Package>> {
        let packages = self.scan()?;
        let mut map = FxHashMap::with_capacity_and_hasher(packages.len(), Default::default());
        for p in packages {
            map.insert(p.name.clone(), p);
        }
        Ok(map)
    }

    fn read_config_mmap(path: &Path) -> Result<Config> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;

        if metadata.len() > 4096 {
            let mmap = unsafe { Mmap::map(&file).map_err(crate::error::Error::Io)? };
            let s = std::str::from_utf8(&mmap)
                .map_err(|e| crate::error::Error::Adapter {
                    package: "scanner".to_string(),
                    message: format!("Invalid UTF-8 in config file: {}", e),
                })?;
            Ok(toml::from_str(s)?)
        } else {
            let config_content = std::fs::read_to_string(path)?;
            Ok(toml::from_str(&config_content)?)
        }
    }

    /// Scans packages and returns both the packages and detected changes.
    ///
    /// Useful for incremental graph updates.
    pub fn scan_with_changes(
        &mut self,
        old_packages: &FxHashMap<String, Package>,
    ) -> Result<(Vec<Package>, crate::graph::GraphChange)> {
        let new_packages = self.scan_as_map()?;
        let change = detect_graph_changes(old_packages, &new_packages);
        Ok((new_packages.values().cloned().collect(), change))
    }
}

/// Detects changes between old and new package sets.
pub fn detect_graph_changes(
    old_packages: &FxHashMap<String, Package>,
    new_packages: &FxHashMap<String, Package>,
) -> crate::graph::GraphChange {
    let mut change = crate::graph::GraphChange {
        added: Vec::new(),
        modified: Vec::new(),
        removed: Vec::new(),
        dependency_changes: Vec::new(),
    };

    for (name, new_pkg) in new_packages {
        match old_packages.get(name) {
            Some(old_pkg) => {
                if old_pkg.deps != new_pkg.deps || old_pkg.tasks != new_pkg.tasks {
                    change.modified.push(new_pkg.clone());
                    if old_pkg.deps != new_pkg.deps {
                        change.dependency_changes.push((
                            name.clone(),
                            new_pkg.deps.iter().cloned().collect(),
                        ));
                    }
                }
            }
            None => change.added.push(new_pkg.clone()),
        }
    }

    for name in old_packages.keys() {
        if !new_packages.contains_key(name) {
            change.removed.push(name.clone());
        }
    }

    change
}
