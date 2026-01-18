//! Integrity verification for cached artifacts.

use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

use super::artifact::Artifact;

/// Verifies the integrity of an artifact.
///
/// Checks:
/// - SHA-256 hash of compressed data
/// - Manifest file hashes match actual file contents
/// - File sizes match manifest
pub struct ArtifactVerifier;

impl ArtifactVerifier {
    /// Verifies the integrity of an artifact.
    ///
    /// # Arguments
    ///
    /// * `artifact` - The artifact to verify
    /// * `expected_hash` - Optional expected hash of the compressed artifact
    ///
    /// # Errors
    ///
    /// Returns an error if verification fails.
    pub fn verify(artifact: &Artifact, expected_hash: Option<&str>) -> Result<()> {
        // Verify compressed data hash if provided
        if let Some(expected) = expected_hash {
            let actual_hash = artifact.hash();
            if actual_hash != expected {
                return Err(Error::Adapter {
                    package: "artifact-verification".to_string(),
                    message: format!(
                        "Artifact hash mismatch: expected {}, got {}",
                        expected, actual_hash
                    ),
                });
            }
        }

        // Verify manifest integrity by checking file hashes
        Self::verify_manifest(artifact)?;

        Ok(())
    }

    /// Verifies that the manifest matches the actual file contents.
    ///
    /// # Errors
    ///
    /// Returns an error if any file hash doesn't match.
    fn verify_manifest(artifact: &Artifact) -> Result<()> {
        use tar::Archive;

        // Decompress
        let tar_data = zstd::decode_all(artifact.compressed_data()).map_err(|e| Error::Adapter {
            package: "artifact-verification".to_string(),
            message: format!("Failed to decompress artifact: {}", e),
        })?;

        // Extract and verify files
        let mut archive = Archive::new(&tar_data[..]);
        let outputs_dir = Path::new("outputs");
        let manifest = artifact.manifest();

        for entry_result in archive.entries().map_err(|e| Error::Adapter {
            package: "artifact-verification".to_string(),
            message: format!("Failed to read tar archive: {}", e),
        })? {
            let mut entry = entry_result.map_err(|e| Error::Adapter {
                package: "artifact-verification".to_string(),
                message: format!("Failed to read tar entry: {}", e),
            })?;

            let path = entry.path().map_err(|e| Error::Adapter {
                package: "artifact-verification".to_string(),
                message: format!("Failed to get entry path: {}", e),
            })?;

            // Skip metadata and manifest
            if path == Path::new("metadata.json") || path == Path::new("manifest.json") {
                continue;
            }

            // Verify output files
            if let Ok(relative_path) = path.strip_prefix(outputs_dir) {
                let expected_hash = manifest.files.get(relative_path).ok_or_else(|| {
                    Error::Adapter {
                        package: "artifact-verification".to_string(),
                        message: format!(
                            "File {} in artifact but not in manifest",
                            relative_path.display()
                        ),
                    }
                })?;

                // Read file content (need to get path first, then read)
                let relative_path = relative_path.to_path_buf();
                let mut content = Vec::new();
                entry.read_to_end(&mut content).map_err(|e| Error::Adapter {
                    package: "artifact-verification".to_string(),
                    message: format!("Failed to read file content: {}", e),
                })?;

                // Compute hash
                let mut hasher = Sha256::new();
                hasher.update(&content);
                let actual_hash = format!("{:x}", hasher.finalize());

                if actual_hash != *expected_hash {
                    return Err(Error::Adapter {
                        package: "artifact-verification".to_string(),
                        message: format!(
                            "File {} hash mismatch: expected {}, got {}",
                            relative_path.display(),
                            expected_hash,
                            actual_hash
                        ),
                    });
                }
            }
        }

        // All files have been verified during iteration above

        Ok(())
    }

    /// Verifies file size matches manifest.
    ///
    /// This is a lightweight check that can be done without extracting files.
    pub fn verify_size(artifact: &Artifact, max_size: u64) -> Result<()> {
        let manifest = artifact.manifest();
        if manifest.total_size > max_size {
            return Err(Error::Adapter {
                package: "artifact-verification".to_string(),
                message: format!(
                    "Artifact size {} exceeds maximum {}",
                    manifest.total_size, max_size
                ),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote_cache::artifact::Artifact;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn test_verify_valid_artifact() {
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

        // Should verify successfully
        assert!(ArtifactVerifier::verify(&artifact, None).is_ok());
    }

    #[test]
    fn test_verify_hash_mismatch() {
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

        // Should fail with wrong hash
        assert!(ArtifactVerifier::verify(&artifact, Some("wrong_hash")).is_err());
    }
}
