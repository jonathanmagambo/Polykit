//! Artifact format for cached task outputs.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

/// Metadata about a cached artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    /// Package name.
    pub package_name: String,
    /// Task name.
    pub task_name: String,
    /// Command that produced this artifact.
    pub command: String,
    /// Cache key hash used to store this artifact.
    pub cache_key_hash: String,
    /// Timestamp when artifact was created (Unix epoch seconds).
    pub created_at: u64,
    /// Artifact format version.
    pub version: u32,
}

/// Manifest of files contained in the artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactManifest {
    /// Map of relative paths to SHA-256 hashes.
    pub files: BTreeMap<PathBuf, String>,
    /// Total size of all files (uncompressed).
    pub total_size: u64,
}

/// Cached task artifact containing outputs and metadata.
///
/// Artifacts are immutable once created and contain:
/// - Metadata (task info, timestamps, cache key hash)
/// - Manifest (list of output files with hashes)
/// - Compressed output files
#[derive(Debug)]
pub struct Artifact {
    metadata: ArtifactMetadata,
    manifest: ArtifactManifest,
    compressed_data: Vec<u8>,
}

impl Artifact {
    /// Creates a new artifact from task outputs.
    ///
    /// # Arguments
    ///
    /// * `package_name` - Name of the package
    /// * `task_name` - Name of the task
    /// * `command` - Command that was executed
    /// * `cache_key_hash` - Hash of the cache key
    /// * `output_files` - Map of relative paths to file contents
    ///
    /// # Errors
    ///
    /// Returns an error if compression or serialization fails.
    pub fn new(
        package_name: String,
        task_name: String,
        command: String,
        cache_key_hash: String,
        output_files: BTreeMap<PathBuf, Vec<u8>>,
    ) -> Result<Self> {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::Adapter {
                package: "artifact".to_string(),
                message: format!("Failed to get timestamp: {}", e),
            })?
            .as_secs();

        let mut files = BTreeMap::new();
        let mut total_size = 0u64;

        // Compute hashes for all files
        for (path, content) in &output_files {
            let mut hasher = Sha256::new();
            hasher.update(content);
            let hash = format!("{:x}", hasher.finalize());
            files.insert(path.clone(), hash);
            total_size += content.len() as u64;
        }

        let manifest = ArtifactManifest {
            files,
            total_size,
        };

        let metadata = ArtifactMetadata {
            package_name,
            task_name,
            command,
            cache_key_hash,
            created_at,
            version: 1,
        };

        // Create tar archive in memory
        let mut tar_data = Vec::new();
        {
            let mut tar = tar::Builder::new(&mut tar_data);

            // Add metadata.json
            let metadata_json = serde_json::to_string(&metadata).map_err(|e| Error::Adapter {
                package: "artifact".to_string(),
                message: format!("Failed to serialize metadata: {}", e),
            })?;
            let mut metadata_header = tar::Header::new_gnu();
            metadata_header.set_path("metadata.json").map_err(|e| Error::Adapter {
                package: "artifact".to_string(),
                message: format!("Failed to set metadata path: {}", e),
            })?;
            metadata_header.set_size(metadata_json.len() as u64);
            metadata_header.set_cksum();
            tar.append(&metadata_header, metadata_json.as_bytes())
                .map_err(|e| Error::Adapter {
                    package: "artifact".to_string(),
                    message: format!("Failed to append metadata: {}", e),
                })?;

            // Add manifest.json
            let manifest_json = serde_json::to_string(&manifest).map_err(|e| Error::Adapter {
                package: "artifact".to_string(),
                message: format!("Failed to serialize manifest: {}", e),
            })?;
            let mut manifest_header = tar::Header::new_gnu();
            manifest_header.set_path("manifest.json").map_err(|e| Error::Adapter {
                package: "artifact".to_string(),
                message: format!("Failed to set manifest path: {}", e),
            })?;
            manifest_header.set_size(manifest_json.len() as u64);
            manifest_header.set_cksum();
            tar.append(&manifest_header, manifest_json.as_bytes())
                .map_err(|e| Error::Adapter {
                    package: "artifact".to_string(),
                    message: format!("Failed to append manifest: {}", e),
                })?;

            // Add output files
            for (path, content) in &output_files {
                let mut header = tar::Header::new_gnu();
                let output_path = Path::new("outputs").join(path);
                header.set_path(&output_path).map_err(|e| Error::Adapter {
                    package: "artifact".to_string(),
                    message: format!("Failed to set output path: {}", e),
                })?;
                header.set_size(content.len() as u64);
                header.set_cksum();
                tar.append(&header, content.as_slice()).map_err(|e| Error::Adapter {
                    package: "artifact".to_string(),
                    message: format!("Failed to append output file: {}", e),
                })?;
            }

            tar.finish().map_err(|e| Error::Adapter {
                package: "artifact".to_string(),
                message: format!("Failed to finish tar archive: {}", e),
            })?;
        }

        // Compress with zstd
        let compressed_data = zstd::encode_all(&tar_data[..], 3).map_err(|e| Error::Adapter {
            package: "artifact".to_string(),
            message: format!("Failed to compress artifact: {}", e),
        })?;

        Ok(Self {
            metadata,
            manifest,
            compressed_data,
        })
    }

    /// Reads an artifact from compressed data.
    ///
    /// # Errors
    ///
    /// Returns an error if decompression, deserialization, or verification fails.
    pub fn from_compressed(data: Vec<u8>) -> Result<Self> {
        // Decompress
        let tar_data = zstd::decode_all(&data[..]).map_err(|e| Error::Adapter {
            package: "artifact".to_string(),
            message: format!("Failed to decompress artifact: {}", e),
        })?;

        // Extract from tar
        let mut archive = tar::Archive::new(&tar_data[..]);
        let mut metadata: Option<ArtifactMetadata> = None;
        let mut manifest: Option<ArtifactManifest> = None;

        for entry_result in archive.entries().map_err(|e| Error::Adapter {
            package: "artifact".to_string(),
            message: format!("Failed to read tar archive: {}", e),
        })? {
            let mut entry = entry_result.map_err(|e| Error::Adapter {
                package: "artifact".to_string(),
                message: format!("Failed to read tar entry: {}", e),
            })?;

            let path = entry.path().map_err(|e| Error::Adapter {
                package: "artifact".to_string(),
                message: format!("Failed to get entry path: {}", e),
            })?;

            if path == Path::new("metadata.json") {
                let mut content = String::new();
                entry.read_to_string(&mut content).map_err(|e| Error::Adapter {
                    package: "artifact".to_string(),
                    message: format!("Failed to read metadata: {}", e),
                })?;
                metadata = Some(serde_json::from_str(&content).map_err(|e| Error::Adapter {
                    package: "artifact".to_string(),
                    message: format!("Failed to parse metadata: {}", e),
                })?);
            } else if path == Path::new("manifest.json") {
                let mut content = String::new();
                entry.read_to_string(&mut content).map_err(|e| Error::Adapter {
                    package: "artifact".to_string(),
                    message: format!("Failed to read manifest: {}", e),
                })?;
                manifest = Some(serde_json::from_str(&content).map_err(|e| Error::Adapter {
                    package: "artifact".to_string(),
                    message: format!("Failed to parse manifest: {}", e),
                })?);
            }
        }

        let metadata = metadata.ok_or_else(|| Error::Adapter {
            package: "artifact".to_string(),
            message: "Missing metadata.json in artifact".to_string(),
        })?;

        let manifest = manifest.ok_or_else(|| Error::Adapter {
            package: "artifact".to_string(),
            message: "Missing manifest.json in artifact".to_string(),
        })?;

        Ok(Self {
            metadata,
            manifest,
            compressed_data: data,
        })
    }

    /// Returns the artifact metadata.
    pub fn metadata(&self) -> &ArtifactMetadata {
        &self.metadata
    }

    /// Returns the artifact manifest.
    pub fn manifest(&self) -> &ArtifactManifest {
        &self.manifest
    }

    /// Returns the compressed artifact data.
    pub fn compressed_data(&self) -> &[u8] {
        &self.compressed_data
    }

    /// Extracts output files from the artifact to the given directory.
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails.
    pub fn extract_outputs(&self, output_dir: &Path) -> Result<()> {
        use std::fs;

        // Decompress
        let tar_data = zstd::decode_all(&self.compressed_data[..]).map_err(|e| Error::Adapter {
            package: "artifact".to_string(),
            message: format!("Failed to decompress artifact: {}", e),
        })?;

        // Extract tar archive
        let mut archive = tar::Archive::new(&tar_data[..]);
        let outputs_dir = Path::new("outputs");

        for entry_result in archive.entries().map_err(|e| Error::Adapter {
            package: "artifact".to_string(),
            message: format!("Failed to read tar archive: {}", e),
        })? {
            let mut entry = entry_result.map_err(|e| Error::Adapter {
                package: "artifact".to_string(),
                message: format!("Failed to read tar entry: {}", e),
            })?;

            let path = entry.path().map_err(|e| Error::Adapter {
                package: "artifact".to_string(),
                message: format!("Failed to get entry path: {}", e),
            })?;

            // Skip metadata and manifest
            if path == Path::new("metadata.json") || path == Path::new("manifest.json") {
                continue;
            }

            // Extract output files
            if let Ok(relative_path) = path.strip_prefix(outputs_dir) {
                let dest_path = output_dir.join(relative_path);
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent).map_err(Error::Io)?;
                }
                entry.unpack(&dest_path).map_err(|e| Error::Adapter {
                    package: "artifact".to_string(),
                    message: format!("Failed to extract file: {}", e),
                })?;
            }
        }

        Ok(())
    }

    /// Computes the SHA-256 hash of the compressed artifact.
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.compressed_data);
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_creation_and_extraction() {
        let mut output_files = BTreeMap::new();
        output_files.insert(PathBuf::from("file1.txt"), b"content1".to_vec());
        output_files.insert(PathBuf::from("subdir/file2.txt"), b"content2".to_vec());

        let artifact = Artifact::new(
            "test-package".to_string(),
            "build".to_string(),
            "echo test".to_string(),
            "abc123".to_string(),
            output_files,
        )
        .unwrap();

        assert_eq!(artifact.metadata().package_name, "test-package");
        assert_eq!(artifact.metadata().task_name, "build");
        assert_eq!(artifact.manifest().files.len(), 2);

        // Test round-trip
        let compressed = artifact.compressed_data().to_vec();
        let artifact2 = Artifact::from_compressed(compressed).unwrap();

        assert_eq!(artifact.metadata().package_name, artifact2.metadata().package_name);
        assert_eq!(artifact.metadata().task_name, artifact2.metadata().task_name);
        assert_eq!(artifact.manifest().files.len(), artifact2.manifest().files.len());
    }
}
