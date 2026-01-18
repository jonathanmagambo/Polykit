//! Artifact storage with directory sharding and atomic operations.

use std::fs;
use std::path::{Path, PathBuf};

use polykit_core::error::{Error, Result};
use polykit_core::remote_cache::Artifact;

/// Storage metadata for an artifact.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageMetadata {
    /// SHA-256 hash of the compressed artifact.
    pub hash: String,
    /// Size of the compressed artifact in bytes.
    pub size: u64,
    /// Timestamp when artifact was created (Unix epoch seconds).
    pub created_at: u64,
    /// Cache key hash.
    pub cache_key_hash: String,
}

/// Manages artifact storage with directory sharding.
pub struct Storage {
    storage_root: PathBuf,
    max_artifact_size: u64,
}

impl Storage {
    /// Creates a new storage instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage directory cannot be created.
    pub fn new(storage_root: impl AsRef<Path>, max_artifact_size: u64) -> Result<Self> {
        let storage_root = storage_root.as_ref().to_path_buf();
        fs::create_dir_all(&storage_root).map_err(Error::Io)?;

        // Create tmp directory for temporary uploads
        let tmp_dir = storage_root.join("tmp");
        fs::create_dir_all(&tmp_dir).map_err(Error::Io)?;

        Ok(Self {
            storage_root,
            max_artifact_size,
        })
    }

    /// Gets the shard directory path for a cache key.
    ///
    /// Uses first 4 characters of the cache key hash for sharding:
    /// `aa/bb/` from hash `aabb...`
    fn shard_path(&self, cache_key: &str) -> PathBuf {
        if cache_key.len() < 4 {
            // Fallback for very short keys
            return self.storage_root.join("00").join("00");
        }

        let prefix = &cache_key[..4];
        let dir1 = &prefix[..2];
        let dir2 = &prefix[2..4];

        self.storage_root.join(dir1).join(dir2)
    }

    /// Gets the artifact file path for a cache key.
    fn artifact_path(&self, cache_key: &str) -> PathBuf {
        self.shard_path(cache_key).join(format!("{}.zst", cache_key))
    }

    /// Gets the metadata file path for a cache key.
    fn metadata_path(&self, cache_key: &str) -> PathBuf {
        self.shard_path(cache_key).join(format!("{}.json", cache_key))
    }

    /// Checks if an artifact exists.
    pub fn has_artifact(&self, cache_key: &str) -> bool {
        self.artifact_path(cache_key).exists()
    }

    /// Gets the temporary file path for an upload.
    fn temp_path(&self) -> PathBuf {
        let uuid = uuid::Uuid::new_v4();
        self.storage_root.join("tmp").join(format!("{}.tmp", uuid))
    }

    /// Stores an artifact atomically.
    ///
    /// # Arguments
    ///
    /// * `cache_key` - The cache key hash
    /// * `data` - The compressed artifact data
    /// * `hash` - SHA-256 hash of the data
    /// * `artifact` - The artifact (for metadata access)
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails or artifact already exists.
    pub async fn store_artifact(
        &self,
        cache_key: &str,
        data: Vec<u8>,
        hash: String,
        artifact: &Artifact,
    ) -> Result<()> {
        // Validate cache key format (should be hex string)
        if !cache_key.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::Adapter {
                package: "storage".to_string(),
                message: format!("Invalid cache key format: {}", cache_key),
            });
        }

        // Check size limit
        if data.len() as u64 > self.max_artifact_size {
            return Err(Error::Adapter {
                package: "storage".to_string(),
                message: format!(
                    "Artifact size {} exceeds maximum {}",
                    data.len(),
                    self.max_artifact_size
                ),
            });
        }

        // Check if artifact already exists (immutable)
        if self.has_artifact(cache_key) {
            return Err(Error::Adapter {
                package: "storage".to_string(),
                message: format!("Artifact {} already exists", cache_key),
            });
        }

        // Write to temporary file
        let temp_path = self.temp_path();
        tokio::fs::write(&temp_path, &data).await.map_err(Error::Io)?;

        // Determine shard directory
        let shard_dir = self.shard_path(cache_key);
        fs::create_dir_all(&shard_dir).map_err(Error::Io)?;

        // Atomic rename
        let artifact_path = self.artifact_path(cache_key);
        fs::rename(&temp_path, &artifact_path).map_err(|e| {
            // Clean up temp file on error
            let _ = fs::remove_file(&temp_path);
            Error::Io(e)
        })?;

        // Write metadata
        let artifact_metadata = artifact.metadata();
        let storage_metadata = StorageMetadata {
            hash,
            size: data.len() as u64,
            created_at: artifact_metadata.created_at,
            cache_key_hash: artifact_metadata.cache_key_hash.clone(),
        };

        let metadata_json = serde_json::to_string(&storage_metadata).map_err(|e| Error::Adapter {
            package: "storage".to_string(),
            message: format!("Failed to serialize metadata: {}", e),
        })?;

        let metadata_path = self.metadata_path(cache_key);
        fs::write(&metadata_path, metadata_json).map_err(Error::Io)?;

        Ok(())
    }

    /// Reads an artifact.
    ///
    /// # Errors
    ///
    /// Returns an error if the artifact doesn't exist or cannot be read.
    pub async fn read_artifact(&self, cache_key: &str) -> Result<Vec<u8>> {
        let artifact_path = self.artifact_path(cache_key);

        if !artifact_path.exists() {
            return Err(Error::Adapter {
                package: "storage".to_string(),
                message: format!("Artifact {} not found", cache_key),
            });
        }

        tokio::fs::read(&artifact_path).await.map_err(Error::Io)
    }

    /// Reads artifact metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata doesn't exist or cannot be read.
    pub async fn read_metadata(&self, cache_key: &str) -> Result<StorageMetadata> {
        let metadata_path = self.metadata_path(cache_key);

        if !metadata_path.exists() {
            return Err(Error::Adapter {
                package: "storage".to_string(),
                message: format!("Metadata for {} not found", cache_key),
            });
        }

        let content = tokio::fs::read_to_string(&metadata_path)
            .await
            .map_err(Error::Io)?;

        serde_json::from_str(&content).map_err(|e| Error::Adapter {
            package: "storage".to_string(),
            message: format!("Failed to parse metadata: {}", e),
        })
    }

    /// Returns the maximum artifact size.
    pub fn max_artifact_size(&self) -> u64 {
        self.max_artifact_size
    }

    /// Cleans up temporary files older than the specified duration.
    ///
    /// This should be called periodically to clean up failed uploads.
    pub fn cleanup_temp_files(&self) -> Result<()> {
        let tmp_dir = self.storage_root.join("tmp");

        if !tmp_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&tmp_dir).map_err(Error::Io)? {
            let entry = entry.map_err(Error::Io)?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("tmp") {
                // Try to remove, ignore errors for concurrent access
                let _ = fs::remove_file(&path);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_storage_sharding() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::new(temp_dir.path(), 1024 * 1024).unwrap();

        let cache_key = "aabbccdd11223344556677889900aabbccddeeff";
        let shard_path = storage.shard_path(cache_key);

        assert!(shard_path.ends_with("aa/bb"));
    }

    #[tokio::test]
    async fn test_storage_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::new(temp_dir.path(), 1024 * 1024).unwrap();

        let cache_key = "aabbccdd11223344556677889900aabbccddeeff";
        let data = b"test data".to_vec();
        let hash = "test_hash".to_string();

        let artifact = polykit_core::remote_cache::Artifact::new(
            "test".to_string(),
            "build".to_string(),
            "echo".to_string(),
            cache_key.to_string(),
            std::collections::BTreeMap::new(),
        )
        .unwrap();

        // Store artifact
        storage
            .store_artifact(cache_key, data.clone(), hash, &artifact)
            .await
            .unwrap();

        // Verify artifact exists
        assert!(storage.has_artifact(cache_key));

        // Verify we can read it back
        let read_data = storage.read_artifact(cache_key).await.unwrap();
        assert_eq!(read_data, data);

        // Verify metadata
        let read_metadata = storage.read_metadata(cache_key).await.unwrap();
        assert_eq!(read_metadata.cache_key_hash, cache_key);
    }

    #[tokio::test]
    async fn test_storage_immutable() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::new(temp_dir.path(), 1024 * 1024).unwrap();

        let cache_key = "aabbccdd11223344556677889900aabbccddeeff";
        let data = b"test data".to_vec();
        let hash = "test_hash".to_string();

        let artifact1 = polykit_core::remote_cache::Artifact::new(
            "test".to_string(),
            "build".to_string(),
            "echo".to_string(),
            cache_key.to_string(),
            std::collections::BTreeMap::new(),
        )
        .unwrap();

        // Store artifact
        storage
            .store_artifact(cache_key, data, hash, &artifact1)
            .await
            .unwrap();

        // Try to store again (should fail)
        let artifact2 = polykit_core::remote_cache::Artifact::new(
            "test".to_string(),
            "build".to_string(),
            "echo".to_string(),
            cache_key.to_string(),
            std::collections::BTreeMap::new(),
        )
        .unwrap();

        let result = storage
            .store_artifact(cache_key, b"different".to_vec(), "hash2".to_string(), &artifact2)
            .await;

        assert!(result.is_err());
    }
}
