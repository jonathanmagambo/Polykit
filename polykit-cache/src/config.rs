//! Server configuration.

use std::path::PathBuf;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Storage directory for artifacts.
    pub storage_dir: PathBuf,
    /// Maximum artifact size in bytes.
    pub max_artifact_size: u64,
    /// Bind address.
    pub bind_address: String,
    /// Port number.
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            storage_dir: PathBuf::from("./cache"),
            max_artifact_size: 1024 * 1024 * 1024, // 1GB
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

impl ServerConfig {
    /// Creates a new server configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the storage directory.
    pub fn with_storage_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.storage_dir = dir.into();
        self
    }

    /// Sets the maximum artifact size.
    pub fn with_max_artifact_size(mut self, size: u64) -> Self {
        self.max_artifact_size = size;
        self
    }

    /// Sets the bind address.
    pub fn with_bind_address(mut self, address: impl Into<String>) -> Self {
        self.bind_address = address.into();
        self
    }

    /// Sets the port.
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Returns the bind address as a string.
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }
}
