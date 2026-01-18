//! Caching system for scan results.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bincode;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::package::Package;

const CACHE_VERSION: u32 = 3;
const MAX_SCAN_DEPTH: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    version: u32,
    packages: Vec<Package>,
    mtimes: HashMap<PathBuf, u64>,
}

pub struct Cache {
    cache_dir: PathBuf,
    stats: CacheStats,
}

#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

impl Cache {
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
            stats: CacheStats::default(),
        }
    }

    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    pub fn get_cache_path(&self, packages_dir: &Path) -> PathBuf {
        let cache_key = self.compute_cache_key(packages_dir);
        self.cache_dir.join(format!("scan_{}.bin", cache_key))
    }

    pub fn load(&mut self, packages_dir: &Path) -> Result<Option<Vec<Package>>> {
        let cache_path = self.get_cache_path(packages_dir);
        if !cache_path.exists() {
            self.stats.misses += 1;
            return Ok(None);
        }

        let content = fs::read(&cache_path).map_err(Error::Io)?;
        let entry: CacheEntry = bincode::deserialize(&content).map_err(|e| Error::Adapter {
            package: "cache".to_string(),
            message: format!("Failed to parse cache: {}", e),
        })?;

        if entry.version != CACHE_VERSION {
            self.stats.misses += 1;
            return Ok(None);
        }

        if !self.validate_mtimes(packages_dir, &entry.mtimes)? {
            self.stats.misses += 1;
            return Ok(None);
        }

        self.stats.hits += 1;
        Ok(Some(entry.packages))
    }

    pub fn save(&self, packages_dir: &Path, packages: &[Package]) -> Result<()> {
        fs::create_dir_all(&self.cache_dir).map_err(Error::Io)?;

        let mtimes = self.collect_mtimes(packages_dir)?;
        let entry = CacheEntry {
            version: CACHE_VERSION,
            packages: packages.to_vec(),
            mtimes,
        };

        let cache_path = self.get_cache_path(packages_dir);
        let content = bincode::serialize(&entry).map_err(|e| Error::Adapter {
            package: "cache".to_string(),
            message: format!("Failed to serialize cache: {}", e),
        })?;

        fs::write(&cache_path, content).map_err(Error::Io)?;

        Ok(())
    }

    fn compute_cache_key(&self, packages_dir: &Path) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        packages_dir.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn collect_mtimes(&self, packages_dir: &Path) -> Result<HashMap<PathBuf, u64>> {
        let mut mtimes = HashMap::new();
        let mut package_dirs = std::collections::HashSet::new();

        for entry in walkdir::WalkDir::new(packages_dir)
            .max_depth(MAX_SCAN_DEPTH)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if entry.file_name() == "polykit.toml" {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(mtime) = metadata.modified() {
                        if let Ok(duration) = mtime.duration_since(SystemTime::UNIX_EPOCH) {
                            let relative_path = path
                                .strip_prefix(packages_dir)
                                .unwrap_or(path)
                                .to_path_buf();
                            mtimes.insert(relative_path, duration.as_secs());
                        }
                    }
                }
                if let Some(package_dir) = path.parent() {
                    if let Ok(metadata) = package_dir.metadata() {
                        if let Ok(mtime) = metadata.modified() {
                            if let Ok(duration) = mtime.duration_since(SystemTime::UNIX_EPOCH) {
                                let relative_dir = package_dir
                                    .strip_prefix(packages_dir)
                                    .unwrap_or(package_dir)
                                    .to_path_buf();
                                package_dirs.insert(relative_dir.clone());
                                // Use a special key format to distinguish directories
                                let dir_key = relative_dir.join(".dir");
                                mtimes.insert(dir_key, duration.as_secs());
                            }
                        }
                    }
                }
            }
        }

        let package_count_key = PathBuf::from(".package_count");
        mtimes.insert(package_count_key, package_dirs.len() as u64);

        Ok(mtimes)
    }

    fn validate_mtimes(
        &self,
        packages_dir: &Path,
        cached_mtimes: &HashMap<PathBuf, u64>,
    ) -> Result<bool> {
        let current_mtimes = self.collect_mtimes(packages_dir)?;

        if current_mtimes.len() != cached_mtimes.len() {
            return Ok(false);
        }

        for (path, cached_time) in cached_mtimes {
            match current_mtimes.get(path) {
                Some(current_time) => {
                    if current_time != cached_time {
                        return Ok(false);
                    }
                }
                None => return Ok(false),
            }
        }

        Ok(true)
    }

    pub fn clear(&self, packages_dir: &Path) -> Result<()> {
        let cache_path = self.get_cache_path(packages_dir);
        if cache_path.exists() {
            fs::remove_file(&cache_path).map_err(Error::Io)?;
        }
        Ok(())
    }
}
