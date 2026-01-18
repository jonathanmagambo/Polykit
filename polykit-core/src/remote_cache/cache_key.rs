//! Deterministic cache key generation for remote caching.

use std::collections::BTreeMap;
use std::path::PathBuf;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};
use crate::package::Language;

/// Deterministic cache key for task execution results.
///
/// The cache key includes all inputs that affect task output:
/// - Package identifier
/// - Task name and command
/// - Environment variables (explicit allowlist)
/// - Input file hashes (tracked files only)
/// - Dependency graph hash
/// - Toolchain version
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheKey {
    /// Package identifier (name + normalized path hash).
    pub package_id: String,
    /// Task name.
    pub task_name: String,
    /// Command string.
    pub command: String,
    /// Environment variables (sorted by key for determinism).
    #[serde(serialize_with = "serialize_env_vars")]
    #[serde(deserialize_with = "deserialize_env_vars")]
    pub env_vars: BTreeMap<String, String>,
    /// Input file hashes (relative path -> SHA-256 hash).
    pub input_file_hashes: FxHashMap<PathBuf, String>,
    /// Dependency graph hash (transitive dependencies).
    pub dependency_graph_hash: String,
    /// Toolchain version (e.g., "node-v20.0.0", "rustc-1.75.0").
    pub toolchain_version: String,
}

fn serialize_env_vars<S>(
    env_vars: &BTreeMap<String, String>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    let vec: Vec<(&String, &String)> = env_vars.iter().collect();
    vec.serialize(serializer)
}

fn deserialize_env_vars<'de, D>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let vec: Vec<(String, String)> = Vec::deserialize(deserializer)?;
    Ok(vec.into_iter().collect())
}

impl CacheKey {
    /// Creates a new cache key builder.
    pub fn builder() -> CacheKeyBuilder {
        CacheKeyBuilder::new()
    }

    /// Computes the deterministic hash of this cache key.
    ///
    /// Returns a hex-encoded SHA-256 hash.
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();

        // Serialize key components deterministically
        let serialized = bincode::serialize(self).unwrap_or_else(|_| {
            // Fallback: manual serialization if bincode fails
            format!(
                "{}\0{}\0{}\0{:?}\0{:?}\0{}\0{}",
                self.package_id,
                self.task_name,
                self.command,
                self.env_vars,
                self.input_file_hashes,
                self.dependency_graph_hash,
                self.toolchain_version
            )
            .into_bytes()
        });

        hasher.update(&serialized);
        format!("{:x}", hasher.finalize())
    }

    /// Returns the cache key as a string identifier.
    ///
    /// This is the hash of the cache key, used for storage and retrieval.
    pub fn as_string(&self) -> String {
        self.hash()
    }
}

/// Builder for constructing cache keys.
pub struct CacheKeyBuilder {
    package_id: Option<String>,
    task_name: Option<String>,
    command: Option<String>,
    env_vars: BTreeMap<String, String>,
    input_file_hashes: FxHashMap<PathBuf, String>,
    dependency_graph_hash: Option<String>,
    toolchain_version: Option<String>,
}

impl CacheKeyBuilder {
    fn new() -> Self {
        Self {
            package_id: None,
            task_name: None,
            command: None,
            env_vars: BTreeMap::new(),
            input_file_hashes: FxHashMap::default(),
            dependency_graph_hash: None,
            toolchain_version: None,
        }
    }

    /// Sets the package identifier.
    pub fn package_id(mut self, package_id: impl Into<String>) -> Self {
        self.package_id = Some(package_id.into());
        self
    }

    /// Sets the task name.
    pub fn task_name(mut self, task_name: impl Into<String>) -> Self {
        self.task_name = Some(task_name.into());
        self
    }

    /// Sets the command string.
    pub fn command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    /// Adds an environment variable to the cache key.
    ///
    /// Only explicitly allowed environment variables should be added.
    pub fn env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Adds multiple environment variables.
    pub fn env_vars(mut self, vars: BTreeMap<String, String>) -> Self {
        self.env_vars.extend(vars);
        self
    }

    /// Adds an input file hash.
    pub fn input_file(mut self, path: PathBuf, hash: impl Into<String>) -> Self {
        self.input_file_hashes.insert(path, hash.into());
        self
    }

    /// Adds multiple input file hashes.
    pub fn input_files(mut self, files: FxHashMap<PathBuf, String>) -> Self {
        self.input_file_hashes.extend(files);
        self
    }

    /// Sets the dependency graph hash.
    pub fn dependency_graph_hash(mut self, hash: impl Into<String>) -> Self {
        self.dependency_graph_hash = Some(hash.into());
        self
    }

    /// Sets the toolchain version.
    pub fn toolchain_version(mut self, version: impl Into<String>) -> Self {
        self.toolchain_version = Some(version.into());
        self
    }

    /// Builds the cache key.
    ///
    /// # Errors
    ///
    /// Returns an error if any required field is missing.
    pub fn build(self) -> Result<CacheKey> {
        Ok(CacheKey {
            package_id: self.package_id.ok_or_else(|| Error::Adapter {
                package: "cache-key".to_string(),
                message: "package_id is required".to_string(),
            })?,
            task_name: self.task_name.ok_or_else(|| Error::Adapter {
                package: "cache-key".to_string(),
                message: "task_name is required".to_string(),
            })?,
            command: self.command.ok_or_else(|| Error::Adapter {
                package: "cache-key".to_string(),
                message: "command is required".to_string(),
            })?,
            env_vars: self.env_vars,
            input_file_hashes: self.input_file_hashes,
            dependency_graph_hash: self.dependency_graph_hash.ok_or_else(|| Error::Adapter {
                package: "cache-key".to_string(),
                message: "dependency_graph_hash is required".to_string(),
            })?,
            toolchain_version: self.toolchain_version.ok_or_else(|| Error::Adapter {
                package: "cache-key".to_string(),
                message: "toolchain_version is required".to_string(),
            })?,
        })
    }
}

/// Detects the toolchain version for a given language.
///
/// Returns a version string like "node-v20.0.0" or "rustc-1.75.0".
pub fn detect_toolchain_version(language: Language) -> Result<String> {
    use std::process::Command;

    let (command, version_flag) = match language {
        Language::Js | Language::Ts => ("node", "--version"),
        Language::Rust => ("rustc", "--version"),
        Language::Go => ("go", "version"),
        Language::Python => ("python3", "--version"),
    };

    let output = Command::new(command)
        .arg(version_flag)
        .output()
        .map_err(|e| Error::Adapter {
            package: "toolchain-detection".to_string(),
            message: format!("Failed to detect {} version: {}", command, e),
        })?;

    if !output.status.success() {
        return Err(Error::Adapter {
            package: "toolchain-detection".to_string(),
            message: format!("Failed to get {} version", command),
        });
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version = version_str
        .lines()
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_string();

    Ok(format!("{}-{}", command, version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_determinism() {
        let key1 = CacheKey::builder()
            .package_id("test-package")
            .task_name("build")
            .command("echo hello")
            .dependency_graph_hash("abc123")
            .toolchain_version("node-v20.0.0")
            .build()
            .unwrap();

        let key2 = CacheKey::builder()
            .package_id("test-package")
            .task_name("build")
            .command("echo hello")
            .dependency_graph_hash("abc123")
            .toolchain_version("node-v20.0.0")
            .build()
            .unwrap();

        assert_eq!(key1.hash(), key2.hash());
    }

    #[test]
    fn test_cache_key_env_vars_order() {
        let mut env1 = BTreeMap::new();
        env1.insert("VAR1".to_string(), "value1".to_string());
        env1.insert("VAR2".to_string(), "value2".to_string());

        let mut env2 = BTreeMap::new();
        env2.insert("VAR2".to_string(), "value2".to_string());
        env2.insert("VAR1".to_string(), "value1".to_string());

        let key1 = CacheKey::builder()
            .package_id("test")
            .task_name("build")
            .command("echo")
            .env_vars(env1)
            .dependency_graph_hash("abc")
            .toolchain_version("node-v20")
            .build()
            .unwrap();

        let key2 = CacheKey::builder()
            .package_id("test")
            .task_name("build")
            .command("echo")
            .env_vars(env2)
            .dependency_graph_hash("abc")
            .toolchain_version("node-v20")
            .build()
            .unwrap();

        // BTreeMap ensures order, so hashes should be equal
        assert_eq!(key1.hash(), key2.hash());
    }

    #[test]
    fn test_cache_key_different_commands() {
        let key1 = CacheKey::builder()
            .package_id("test")
            .task_name("build")
            .command("echo hello")
            .dependency_graph_hash("abc")
            .toolchain_version("node-v20")
            .build()
            .unwrap();

        let key2 = CacheKey::builder()
            .package_id("test")
            .task_name("build")
            .command("echo world")
            .dependency_graph_hash("abc")
            .toolchain_version("node-v20")
            .build()
            .unwrap();

        assert_ne!(key1.hash(), key2.hash());
    }
}
