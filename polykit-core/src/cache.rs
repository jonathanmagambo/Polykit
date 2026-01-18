//! Caching system for scan results.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bincode;
use memmap2::{Mmap, MmapMut};
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64;

use crate::error::{Error, Result};
use crate::package::Package;

const CACHE_VERSION: u32 = 3;
const MAX_SCAN_DEPTH: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    version: u32,
    packages: Vec<Package>,
    mtimes: FxHashMap<PathBuf, u64>,
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
        let filename = format!("scan_{}.bin", cache_key);
        let cache_path = self.cache_dir.join(&filename);

        if let Ok(canonical_cache_dir) = self.cache_dir.canonicalize() {
            if let Ok(canonical_cache_path) = cache_path.canonicalize() {
                if canonical_cache_path.starts_with(&canonical_cache_dir) {
                    return cache_path;
                }
            } else if let Some(parent) = cache_path.parent() {
                if let Ok(canonical_parent) = parent.canonicalize() {
                    let canonical_cache_path = canonical_parent.join(&filename);
                    if canonical_cache_path.starts_with(&canonical_cache_dir) {
                        return cache_path;
                    }
                }
            }
        }

        cache_path
    }

    pub fn load(&mut self, packages_dir: &Path) -> Result<Option<Vec<Package>>> {
        let cache_path = self.get_cache_path(packages_dir);
        if !cache_path.exists() {
            self.stats.misses += 1;
            return Ok(None);
        }

        let file = File::open(&cache_path).map_err(Error::Io)?;
        let metadata = file.metadata().map_err(Error::Io)?;

        if metadata.len() == 0 {
            self.stats.misses += 1;
            return Ok(None);
        }

        let mmap = unsafe {
            Mmap::map(&file).map_err(|e| Error::Adapter {
                package: "cache".to_string(),
                message: format!("Failed to memory-map cache file: {}", e),
            })?
        };

        let content = zstd::decode_all(&mmap[..]).map_err(|e| Error::Adapter {
            package: "cache".to_string(),
            message: format!("Failed to decompress cache: {}", e),
        })?;

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
        let serialized = bincode::serialize(&entry).map_err(|e| Error::Adapter {
            package: "cache".to_string(),
            message: format!("Failed to serialize cache: {}", e),
        })?;

        let compressed = zstd::encode_all(&serialized[..], 3).map_err(|e| Error::Adapter {
            package: "cache".to_string(),
            message: format!("Failed to compress cache: {}", e),
        })?;

        if compressed.len() < 4096 {
            fs::write(&cache_path, compressed).map_err(Error::Io)?;
        } else {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&cache_path)
                .map_err(Error::Io)?;

            file.set_len(compressed.len() as u64).map_err(Error::Io)?;

            let mut mmap = unsafe {
                MmapMut::map_mut(&file).map_err(|e| Error::Adapter {
                    package: "cache".to_string(),
                    message: format!("Failed to memory-map cache file for writing: {}", e),
                })?
            };

            mmap.copy_from_slice(&compressed);
            mmap.flush().map_err(|e| Error::Adapter {
                package: "cache".to_string(),
                message: format!("Failed to flush memory-mapped cache: {}", e),
            })?;
        }

        Ok(())
    }

    fn compute_cache_key(&self, packages_dir: &Path) -> String {
        let path_bytes = packages_dir.as_os_str().as_encoded_bytes();
        let hash = xxh3_64(path_bytes);
        format!("{:x}", hash)
    }

    fn collect_mtimes(&self, packages_dir: &Path) -> Result<FxHashMap<PathBuf, u64>> {
        let packages_dir = packages_dir.to_path_buf();

        let polykit_files: Vec<PathBuf> = walkdir::WalkDir::new(&packages_dir)
            .max_depth(MAX_SCAN_DEPTH)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() == "polykit.toml")
            .map(|e| e.path().to_path_buf())
            .collect();

        let mtimes_vec: Vec<(PathBuf, u64)> = polykit_files
            .into_par_iter()
            .flat_map(|path| {
                let mut results = Vec::new();

                if let Ok(metadata) = path.metadata() {
                    if let Ok(mtime) = metadata.modified() {
                        if let Ok(duration) = mtime.duration_since(SystemTime::UNIX_EPOCH) {
                            let relative_path = path
                                .strip_prefix(&packages_dir)
                                .unwrap_or(&path)
                                .to_path_buf();
                            results.push((relative_path, duration.as_secs()));
                        }
                    }
                }

                if let Some(package_dir) = path.parent() {
                    if let Ok(metadata) = package_dir.metadata() {
                        if let Ok(mtime) = metadata.modified() {
                            if let Ok(duration) = mtime.duration_since(SystemTime::UNIX_EPOCH) {
                                let relative_dir = package_dir
                                    .strip_prefix(&packages_dir)
                                    .unwrap_or(package_dir)
                                    .to_path_buf();
                                let dir_key = relative_dir.join(".dir");
                                results.push((dir_key, duration.as_secs()));
                            }
                        }
                    }
                }

                results
            })
            .collect();

        let mut mtimes = FxHashMap::with_capacity_and_hasher(mtimes_vec.len(), Default::default());
        let mut package_dirs = rustc_hash::FxHashSet::default();

        for (path, mtime) in mtimes_vec {
            if path.file_name().and_then(|n| n.to_str()) == Some(".dir") {
                if let Some(parent) = path.parent() {
                    package_dirs.insert(parent.to_path_buf());
                }
            }
            mtimes.insert(path, mtime);
        }

        let package_count_key = PathBuf::from(".package_count");
        mtimes.insert(package_count_key, package_dirs.len() as u64);

        Ok(mtimes)
    }

    fn validate_mtimes(
        &self,
        packages_dir: &Path,
        cached_mtimes: &FxHashMap<PathBuf, u64>,
    ) -> Result<bool> {
        if cached_mtimes.is_empty() {
            return Ok(false);
        }

        let package_count_key = PathBuf::from(".package_count");
        if let Some(cached_count) = cached_mtimes.get(&package_count_key) {
            let current_count = self.count_packages_fast(packages_dir)?;
            if current_count != *cached_count {
                return Ok(false);
            }
        }

        let packages_dir = packages_dir.to_path_buf();

        for (path, cached_time) in cached_mtimes {
            if path == &package_count_key {
                continue;
            }

            let path_to_check = if path.file_name().and_then(|n| n.to_str()) == Some(".dir") {
                if let Some(parent) = path.parent() {
                    packages_dir.join(parent)
                } else {
                    packages_dir.join(path)
                }
            } else {
                packages_dir.join(path)
            };

            let is_valid = if let Ok(metadata) = path_to_check.metadata() {
                if let Ok(mtime) = metadata.modified() {
                    if let Ok(duration) = mtime.duration_since(SystemTime::UNIX_EPOCH) {
                        duration.as_secs() == *cached_time
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if !is_valid {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn count_packages_fast(&self, packages_dir: &Path) -> Result<u64> {
        let count = walkdir::WalkDir::new(packages_dir)
            .max_depth(MAX_SCAN_DEPTH)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() == "polykit.toml")
            .count();
        Ok(count as u64)
    }

    pub fn clear(&self, packages_dir: &Path) -> Result<()> {
        let cache_path = self.get_cache_path(packages_dir);
        if cache_path.exists() {
            fs::remove_file(&cache_path).map_err(Error::Io)?;
        }
        Ok(())
    }
}
