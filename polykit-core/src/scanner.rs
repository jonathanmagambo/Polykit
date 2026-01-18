//! Repository scanner for discovering packages.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use walkdir::WalkDir;

use crate::cache::Cache;
use crate::config::Config;
use crate::error::Result;
use crate::package::Package;

/// Scans a directory for packages.
///
/// Looks for `polykit.toml` files and parses them into `Package` structures.
/// Uses caching for fast incremental scans.
pub struct Scanner {
    packages_dir: PathBuf,
    cache: Option<Cache>,
}

impl Scanner {
    pub fn new(packages_dir: impl AsRef<Path>) -> Self {
        Self {
            packages_dir: packages_dir.as_ref().to_path_buf(),
            cache: None,
        }
    }

    pub fn with_cache(packages_dir: impl AsRef<Path>, cache_dir: impl AsRef<Path>) -> Self {
        Self {
            packages_dir: packages_dir.as_ref().to_path_buf(),
            cache: Some(Cache::new(cache_dir)),
        }
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

                Ok(Package::new(
                    config.name.clone(),
                    language,
                    config.public,
                    relative_path,
                    config.deps.internal.clone(),
                    config.to_tasks(),
                ))
            })
            .collect();

        packages
    }

    pub fn scan_as_map(&mut self) -> Result<HashMap<String, Package>> {
        let packages = self.scan()?;
        Ok(packages.into_iter().map(|p| (p.name.clone(), p)).collect())
    }
}
