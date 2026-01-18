//! Configuration for remote cache.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Configuration for remote cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCacheConfig {
    /// Backend URL (HTTP URL or filesystem path).
    pub url: String,
    /// Authentication token (for HTTP backends).
    pub token: Option<String>,
    /// Environment variables to include in cache keys.
    ///
    /// Only explicitly listed environment variables will be hashed into cache keys.
    pub env_vars: BTreeSet<String>,
    /// Input files to track for cache key generation.
    ///
    /// Patterns are relative to package root. Supports glob patterns.
    pub input_files: Vec<String>,
    /// Maximum artifact size in bytes (default: 1GB).
    pub max_artifact_size: Option<u64>,
    /// Read-only mode (disable uploads).
    pub read_only: bool,
}

impl Default for RemoteCacheConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            token: None,
            env_vars: BTreeSet::new(),
            input_files: Vec::new(),
            max_artifact_size: Some(1024 * 1024 * 1024), // 1GB
            read_only: false,
        }
    }
}

impl RemoteCacheConfig {
    /// Creates a new remote cache configuration.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    /// Sets the authentication token.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Adds an environment variable to track.
    pub fn add_env_var(mut self, var: impl Into<String>) -> Self {
        self.env_vars.insert(var.into());
        self
    }

    /// Adds multiple environment variables to track.
    pub fn add_env_vars<I>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.env_vars.extend(vars);
        self
    }

    /// Adds an input file pattern to track.
    pub fn add_input_file(mut self, pattern: impl Into<String>) -> Self {
        self.input_files.push(pattern.into());
        self
    }

    /// Sets read-only mode.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Sets maximum artifact size.
    pub fn max_artifact_size(mut self, size: u64) -> Self {
        self.max_artifact_size = Some(size);
        self
    }

    /// Checks if this is an HTTP backend.
    pub fn is_http(&self) -> bool {
        self.url.starts_with("http://") || self.url.starts_with("https://")
    }

    /// Checks if this is a filesystem backend.
    pub fn is_filesystem(&self) -> bool {
        !self.is_http()
    }
}
