//! Backend trait for remote cache storage.

use async_trait::async_trait;

use crate::error::Result;

use super::artifact::Artifact;
use super::cache_key::CacheKey;

/// Error types for remote cache backend operations.
#[derive(thiserror::Error, Debug)]
pub enum BackendError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Artifact not found")]
    NotFound,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Backend error: {0}")]
    Other(String),
}

/// Trait for remote cache backends.
///
/// Backends are responsible for storing and retrieving artifacts.
/// All operations are async and should support streaming for large artifacts.
#[async_trait]
pub trait RemoteCacheBackend: Send + Sync {
    /// Uploads an artifact to the remote cache.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key for this artifact
    /// * `artifact` - The artifact to upload
    ///
    /// # Errors
    ///
    /// Returns an error if upload fails. Errors should be non-fatal and allow
    /// fallback to local execution.
    async fn upload_artifact(&self, key: &CacheKey, artifact: &Artifact) -> Result<()>;

    /// Fetches an artifact from the remote cache.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to fetch
    ///
    /// # Returns
    ///
    /// Returns `Some(artifact)` if found, `None` if not found.
    ///
    /// # Errors
    ///
    /// Returns an error only for unexpected failures (network errors, etc.).
    /// Cache misses should return `Ok(None)`.
    async fn fetch_artifact(&self, key: &CacheKey) -> Result<Option<Artifact>>;

    /// Checks if an artifact exists in the remote cache.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to check
    ///
    /// # Returns
    ///
    /// Returns `true` if the artifact exists, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error only for unexpected failures. Cache misses should return `Ok(false)`.
    async fn has_artifact(&self, key: &CacheKey) -> Result<bool>;
}
