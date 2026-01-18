//! Task output caching for incremental builds.

use std::fs;
use std::path::{Path, PathBuf};

use bincode;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::runner::TaskResult;

const TASK_CACHE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskCacheEntry {
    version: u32,
    package_name: String,
    task_name: String,
    command: String,
    command_hash: u64,
    success: bool,
    stdout: String,
    stderr: String,
}

/// Caches task execution results for incremental builds.
pub struct TaskCache {
    cache_dir: PathBuf,
}

impl TaskCache {
    /// Creates a new task cache.
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
        }
    }

    /// Gets the cache key for a task.
    fn cache_key(package_name: &str, task_name: &str, command: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        package_name.hash(&mut hasher);
        task_name.hash(&mut hasher);
        command.hash(&mut hasher);

        let safe_package = package_name.replace(['/', '\\', '.', ':'], "_");
        let safe_task = task_name.replace(['/', '\\', '.', ':'], "_");
        format!("task_{}_{}_{:x}", safe_package, safe_task, hasher.finish())
    }

    fn get_safe_cache_path(&self, cache_key: &str) -> Result<PathBuf> {
        let filename = format!("{}.bin", cache_key);
        let cache_path = self.cache_dir.join(&filename);

        if let Ok(canonical_cache_dir) = self.cache_dir.canonicalize() {
            if let Ok(canonical_cache_path) = cache_path
                .canonicalize()
                .or_else(|_| self.cache_dir.canonicalize().map(|dir| dir.join(&filename)))
            {
                if !canonical_cache_path.starts_with(&canonical_cache_dir) {
                    return Err(Error::Adapter {
                        package: "task-cache".to_string(),
                        message: "Invalid cache path detected".to_string(),
                    });
                }
                return Ok(cache_path);
            }
        }

        Ok(cache_path)
    }

    /// Retrieves a cached task result if available.
    pub fn get(
        &self,
        package_name: &str,
        task_name: &str,
        command: &str,
    ) -> Result<Option<TaskResult>> {
        let cache_key = Self::cache_key(package_name, task_name, command);
        let cache_path = self.get_safe_cache_path(&cache_key)?;

        if !cache_path.exists() {
            return Ok(None);
        }

        let content = fs::read(&cache_path).map_err(Error::Io)?;
        let entry: TaskCacheEntry = bincode::deserialize(&content).map_err(|e| Error::Adapter {
            package: "task-cache".to_string(),
            message: format!("Failed to parse task cache: {}", e),
        })?;

        if entry.version != TASK_CACHE_VERSION {
            return Ok(None);
        }

        if entry.package_name != package_name
            || entry.task_name != task_name
            || entry.command != command
        {
            return Ok(None);
        }

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        command.hash(&mut hasher);
        if hasher.finish() != entry.command_hash {
            return Ok(None);
        }

        Ok(Some(TaskResult {
            package_name: entry.package_name,
            task_name: entry.task_name,
            success: entry.success,
            stdout: entry.stdout,
            stderr: entry.stderr,
        }))
    }

    /// Stores a task result in the cache.
    pub fn put(
        &self,
        package_name: &str,
        task_name: &str,
        command: &str,
        result: &TaskResult,
    ) -> Result<()> {
        if !result.success {
            return Ok(());
        }

        fs::create_dir_all(&self.cache_dir).map_err(Error::Io)?;

        let cache_key = Self::cache_key(package_name, task_name, command);
        let cache_path = self.get_safe_cache_path(&cache_key)?;

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        command.hash(&mut hasher);

        let entry = TaskCacheEntry {
            version: TASK_CACHE_VERSION,
            package_name: package_name.to_string(),
            task_name: task_name.to_string(),
            command: command.to_string(),
            command_hash: hasher.finish(),
            success: result.success,
            stdout: result.stdout.clone(),
            stderr: result.stderr.clone(),
        };

        let content = bincode::serialize(&entry).map_err(|e| Error::Adapter {
            package: "task-cache".to_string(),
            message: format!("Failed to serialize task cache: {}", e),
        })?;

        fs::write(&cache_path, content).map_err(Error::Io)?;

        Ok(())
    }

    /// Clears the task cache.
    pub fn clear(&self) -> Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir).map_err(Error::Io)?;
        }
        Ok(())
    }
}
