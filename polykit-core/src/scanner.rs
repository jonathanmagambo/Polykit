//! Repository scanner for discovering packages.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use walkdir::WalkDir;

use crate::cache::Cache;
use crate::config::{Config, WorkspaceConfig};
use crate::error::Result;
use crate::package::Package;

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
                    tasks: HashMap::new(),
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
        let config_files: Vec<PathBuf> = WalkDir::new(&self.packages_dir)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() == "polykit.toml")
            .map(|e| e.path().to_path_buf())
            .collect();

        let packages: Result<Vec<Package>> = config_files
            .into_par_iter()
            .map(|config_path| {
                let package_path = config_path
                    .parent()
                    .ok_or_else(|| crate::error::Error::ConfigNotFound(config_path.clone()))?;

                let config_content = std::fs::read_to_string(&config_path)?;
                let config: Config = toml::from_str(&config_content)?;

                let language = config.parse_language()?;
                let relative_path = package_path
                    .strip_prefix(&self.packages_dir)
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|_| package_path.to_path_buf());

                let mut package_tasks = config.to_tasks();

                if let Some(ref workspace_config) = self.workspace_config {
                    let workspace_tasks = workspace_config.to_tasks();
                    for workspace_task in workspace_tasks {
                        if !package_tasks.iter().any(|t| t.name == workspace_task.name) {
                            package_tasks.push(workspace_task);
                        }
                    }
                }

                Ok(Package::new(
                    config.name.clone(),
                    language,
                    config.public,
                    relative_path,
                    config.deps.internal.clone(),
                    package_tasks,
                ))
            })
            .collect();

        let mut packages = packages?;
        packages.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(packages)
    }

    pub fn scan_as_map(&mut self) -> Result<HashMap<String, Package>> {
        let packages = self.scan()?;
        Ok(packages.into_iter().map(|p| (p.name.clone(), p)).collect())
    }
}
