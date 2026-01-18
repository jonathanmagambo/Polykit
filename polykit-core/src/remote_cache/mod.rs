//! Remote caching system for shared cache across CI/CD and team members.

mod artifact;
mod backend;
mod cache_key;
mod config;
mod filesystem;
mod http;
mod integrity;

pub use artifact::Artifact;
pub use backend::{BackendError, RemoteCacheBackend};
pub use cache_key::{detect_toolchain_version, CacheKey, CacheKeyBuilder};
pub use config::RemoteCacheConfig;
pub use filesystem::FilesystemBackend;
pub use http::HttpBackend;
pub use integrity::ArtifactVerifier;

use crate::error::Result;
use crate::graph::DependencyGraph;
use crate::package::Package;

/// Remote cache orchestrator.
///
/// Handles cache operations and integrates with task execution.
pub struct RemoteCache {
    backend: Box<dyn RemoteCacheBackend>,
    config: RemoteCacheConfig,
}

impl RemoteCache {
    /// Creates a new remote cache with the given backend and configuration.
    pub fn new(backend: Box<dyn RemoteCacheBackend>, config: RemoteCacheConfig) -> Self {
        Self { backend, config }
    }

    /// Creates a remote cache from configuration.
    ///
    /// Automatically selects the appropriate backend based on the URL.
    ///
    /// # Errors
    ///
    /// Returns an error if backend creation fails.
    pub fn from_config(config: RemoteCacheConfig) -> Result<Self> {
        let backend: Box<dyn RemoteCacheBackend> = if config.is_http() {
            Box::new(HttpBackend::new(&config)?)
        } else {
            Box::new(FilesystemBackend::new(&config.url)?)
        };

        Ok(Self::new(backend, config))
    }

    /// Creates a disabled remote cache (no-op).
    pub fn disabled() -> Self {
        Self {
            backend: Box::new(DisabledBackend),
            config: RemoteCacheConfig::default(),
        }
    }

    /// Checks if remote cache is enabled.
    pub fn is_enabled(&self) -> bool {
        !self.config.url.is_empty()
    }

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
    /// Returns an error only for unexpected failures. Cache misses return `Ok(None)`.
    pub async fn fetch_artifact(&self, key: &CacheKey) -> Result<Option<Artifact>> {
        if !self.is_enabled() {
            return Ok(None);
        }

        self.backend.fetch_artifact(key).await
    }

    /// Uploads an artifact to the remote cache.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key for this artifact
    /// * `artifact` - The artifact to upload
    ///
    /// # Errors
    ///
    /// Returns an error if upload fails. Errors are non-fatal.
    pub async fn upload_artifact(&self, key: &CacheKey, artifact: &Artifact) -> Result<()> {
        if !self.is_enabled() || self.config.read_only {
            return Ok(());
        }

        self.backend.upload_artifact(key, artifact).await
    }

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
    /// Returns an error only for unexpected failures.
    pub async fn has_artifact(&self, key: &CacheKey) -> Result<bool> {
        if !self.is_enabled() {
            return Ok(false);
        }

        self.backend.has_artifact(key).await
    }

    /// Builds a cache key for a task execution.
    ///
    /// # Arguments
    ///
    /// * `package` - The package being executed
    /// * `task_name` - The task name
    /// * `command` - The command string
    /// * `graph` - The dependency graph
    /// * `package_path` - Path to the package directory
    ///
    /// # Errors
    ///
    /// Returns an error if cache key construction fails.
    pub async fn build_cache_key(
        &self,
        package: &Package,
        task_name: &str,
        command: &str,
        graph: &DependencyGraph,
        package_path: &std::path::Path,
    ) -> Result<CacheKey> {
        use std::collections::BTreeMap;
        use std::env;
        use std::fs;
        use sha2::{Digest, Sha256};
        use walkdir::WalkDir;

        // Build dependency graph hash
        let deps = graph.dependencies(&package.name).unwrap_or_default();
        let mut dep_hash_input = format!("{}:{}", package.name, task_name);
        for dep in &deps {
            dep_hash_input.push_str(&format!(":{}", dep));
        }
        let mut dep_hasher = Sha256::new();
        dep_hasher.update(dep_hash_input.as_bytes());
        let dependency_graph_hash = format!("{:x}", dep_hasher.finalize());

        // Collect environment variables (only from allowlist)
        let mut env_vars = BTreeMap::new();
        for var_name in &self.config.env_vars {
            if let Ok(value) = env::var(var_name) {
                env_vars.insert(var_name.clone(), value);
            }
        }

        // Collect input file hashes
        let mut input_file_hashes = rustc_hash::FxHashMap::default();
        if !self.config.input_files.is_empty() {
            for pattern in &self.config.input_files {
                // Simple glob matching (can be enhanced)
                let pattern_path = package_path.join(pattern);
                if pattern_path.exists() {
                    if pattern_path.is_file() {
                        if let Ok(content) = fs::read(&pattern_path) {
                            let mut hasher = Sha256::new();
                            hasher.update(&content);
                            let hash = format!("{:x}", hasher.finalize());
                            input_file_hashes.insert(
                                pattern_path
                                    .strip_prefix(package_path)
                                    .unwrap_or(&pattern_path)
                                    .to_path_buf(),
                                hash,
                            );
                        }
                    } else if pattern_path.is_dir() {
                        // Walk directory
                        for entry in WalkDir::new(&pattern_path).into_iter().flatten() {
                            if entry.file_type().is_file() {
                                if let Ok(content) = fs::read(entry.path()) {
                                    let mut hasher = Sha256::new();
                                    hasher.update(&content);
                                    let hash = format!("{:x}", hasher.finalize());
                                    if let Ok(relative) = entry.path().strip_prefix(package_path) {
                                        input_file_hashes.insert(relative.to_path_buf(), hash);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Detect toolchain version
        let toolchain_version = detect_toolchain_version(package.language)?;

        // Build package ID (name + path hash)
        let package_path_str = package_path.to_string_lossy();
        let mut package_hasher = Sha256::new();
        package_hasher.update(package_path_str.as_bytes());
        let package_path_hash = format!("{:x}", package_hasher.finalize())[..8].to_string();
        let package_id = format!("{}-{}", package.name, package_path_hash);

        CacheKey::builder()
            .package_id(package_id)
            .task_name(task_name.to_string())
            .command(command.to_string())
            .env_vars(env_vars)
            .input_files(input_file_hashes)
            .dependency_graph_hash(dependency_graph_hash)
            .toolchain_version(toolchain_version)
            .build()
    }

    /// Returns the configuration.
    pub fn config(&self) -> &RemoteCacheConfig {
        &self.config
    }
}

/// Disabled backend that does nothing.
struct DisabledBackend;

#[async_trait::async_trait]
impl RemoteCacheBackend for DisabledBackend {
    async fn upload_artifact(&self, _key: &CacheKey, _artifact: &Artifact) -> Result<()> {
        Ok(())
    }

    async fn fetch_artifact(&self, _key: &CacheKey) -> Result<Option<Artifact>> {
        Ok(None)
    }

    async fn has_artifact(&self, _key: &CacheKey) -> Result<bool> {
        Ok(false)
    }
}
