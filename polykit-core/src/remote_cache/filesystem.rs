//! Filesystem backend for remote cache.

use std::fs;
use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::error::{Error, Result};

use super::artifact::Artifact;
use super::backend::RemoteCacheBackend;
use super::cache_key::CacheKey;

/// Filesystem backend for remote cache.
///
/// Stores artifacts in a local directory with git worktree support.
/// Multiple worktrees share the same cache directory based on repository root.
pub struct FilesystemBackend {
    cache_dir: PathBuf,
}

impl FilesystemBackend {
    /// Creates a new filesystem backend.
    ///
    /// # Arguments
    ///
    /// * `cache_dir` - Base directory for cache storage
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created or accessed.
    pub fn new(cache_dir: impl AsRef<Path>) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();

        // Resolve git repository root if possible
        let repo_root = Self::find_repo_root(&cache_dir)?;
        let cache_dir = if let Some(repo_root) = repo_root {
            // Use repository root for stable cache paths across worktrees
            let repo_hash = Self::hash_path(&repo_root)?;
            cache_dir.join("remote").join(repo_hash)
        } else {
            // Fallback to provided cache directory
            cache_dir.join("remote")
        };

        // Create cache directory
        fs::create_dir_all(&cache_dir).map_err(Error::Io)?;

        Ok(Self { cache_dir })
    }

    /// Finds the git repository root starting from the given path.
    fn find_repo_root(start: &Path) -> Result<Option<PathBuf>> {
        use std::process::Command;

        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--git-dir")
            .current_dir(start)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let git_path = PathBuf::from(&git_dir);
                if git_path.is_absolute() {
                    Ok(Some(git_path.parent().unwrap_or(&git_path).to_path_buf()))
                } else {
                    Ok(Some(start.join(&git_dir).parent().unwrap_or(start).to_path_buf()))
                }
            }
            _ => Ok(None),
        }
    }

    /// Computes a stable hash of a path for use in cache directory names.
    fn hash_path(path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};

        let path_str = path.to_string_lossy();
        let mut hasher = Sha256::new();
        hasher.update(path_str.as_bytes());
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash)[..16].to_string()) // Use first 16 chars
    }

    /// Gets the cache path for a given cache key.
    fn cache_path(&self, key: &CacheKey) -> PathBuf {
        let key_str = key.as_string();
        // Use first 2 chars for directory structure to avoid too many files in one dir
        let dir = &key_str[..2];
        let file = &key_str[2..];
        self.cache_dir.join(dir).join(format!("{}.zst", file))
    }

    /// Ensures the parent directory exists for a cache path.
    fn ensure_parent_dir(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(Error::Io)?;
        }
        Ok(())
    }
}

#[async_trait]
impl RemoteCacheBackend for FilesystemBackend {
    async fn upload_artifact(&self, key: &CacheKey, artifact: &Artifact) -> Result<()> {
        let cache_path = self.cache_path(key);
        self.ensure_parent_dir(&cache_path)?;

        // Write atomically using a temp file
        let temp_path = cache_path.with_extension("tmp");
        fs::write(&temp_path, artifact.compressed_data()).map_err(Error::Io)?;

        // Atomic rename
        fs::rename(&temp_path, &cache_path).map_err(|e| {
            // Clean up temp file on error
            let _ = fs::remove_file(&temp_path);
            Error::Io(e)
        })?;

        Ok(())
    }

    async fn fetch_artifact(&self, key: &CacheKey) -> Result<Option<Artifact>> {
        let cache_path = self.cache_path(key);

        if !cache_path.exists() {
            return Ok(None);
        }

        // Read compressed data
        let data = tokio::fs::read(&cache_path)
            .await
            .map_err(Error::Io)?;

        // Parse artifact
        let artifact = Artifact::from_compressed(data)?;

        Ok(Some(artifact))
    }

    async fn has_artifact(&self, key: &CacheKey) -> Result<bool> {
        let cache_path = self.cache_path(key);
        Ok(cache_path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote_cache::artifact::Artifact;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_filesystem_backend() {
        let temp_dir = TempDir::new().unwrap();
        let backend = FilesystemBackend::new(temp_dir.path()).unwrap();

        let mut output_files = BTreeMap::new();
        output_files.insert(PathBuf::from("file.txt"), b"content".to_vec());

        let artifact = Artifact::new(
            "test".to_string(),
            "build".to_string(),
            "echo".to_string(),
            "hash123".to_string(),
            output_files,
        )
        .unwrap();

        let key = CacheKey::builder()
            .package_id("test")
            .task_name("build")
            .command("echo")
            .dependency_graph_hash("abc")
            .toolchain_version("node-v20")
            .build()
            .unwrap();

        // Upload
        backend.upload_artifact(&key, &artifact).await.unwrap();

        // Check exists
        assert!(backend.has_artifact(&key).await.unwrap());

        // Fetch
        let fetched = backend.fetch_artifact(&key).await.unwrap();
        assert!(fetched.is_some());
    }
}
