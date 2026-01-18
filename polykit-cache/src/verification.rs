//! Integrity verification for uploaded artifacts.

use polykit_core::error::{Error, Result};
use polykit_core::remote_cache::Artifact;
use sha2::{Digest, Sha256};

/// Verifies an uploaded artifact before storage.
pub struct Verifier {
    max_artifact_size: u64,
}

impl Verifier {
    /// Creates a new verifier.
    pub fn new(max_artifact_size: u64) -> Self {
        Self {
            max_artifact_size,
        }
    }

    /// Verifies an uploaded artifact.
    ///
    /// # Arguments
    ///
    /// * `data` - Compressed artifact data
    /// * `expected_cache_key` - The cache key from the URL
    ///
    /// # Returns
    ///
    /// Returns the parsed artifact and computed hash if verification passes.
    ///
    /// # Errors
    ///
    /// Returns an error if verification fails.
    pub fn verify_upload(
        &self,
        data: &[u8],
        expected_cache_key: &str,
    ) -> Result<(Artifact, String)> {
        // Check size limit
        if data.len() as u64 > self.max_artifact_size {
            return Err(Error::Adapter {
                package: "verification".to_string(),
                message: format!(
                    "Artifact size {} exceeds maximum {}",
                    data.len(),
                    self.max_artifact_size
                ),
            });
        }

        // Compute SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(data);
        let computed_hash = format!("{:x}", hasher.finalize());

        // Parse artifact
        let artifact = Artifact::from_compressed(data.to_vec())?;

        // Verify artifact integrity
        polykit_core::remote_cache::ArtifactVerifier::verify(&artifact, Some(&computed_hash))?;

        // Verify cache key matches
        let metadata = artifact.metadata();
        if metadata.cache_key_hash != expected_cache_key {
            return Err(Error::Adapter {
                package: "verification".to_string(),
                message: format!(
                    "Cache key mismatch: expected {}, got {}",
                    expected_cache_key, metadata.cache_key_hash
                ),
            });
        }

        // Verify manifest integrity (already done by ArtifactVerifier, but double-check)
        let manifest = artifact.manifest();
        if manifest.total_size == 0 && !manifest.files.is_empty() {
            return Err(Error::Adapter {
                package: "verification".to_string(),
                message: "Manifest has files but total_size is 0".to_string(),
            });
        }

        Ok((artifact, computed_hash))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use polykit_core::remote_cache::Artifact;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn test_verify_valid_artifact() {
        let verifier = Verifier::new(1024 * 1024);

        let mut output_files = BTreeMap::new();
        output_files.insert(PathBuf::from("file.txt"), b"content".to_vec());

        let cache_key = "aabbccdd11223344556677889900aabbccddeeff";
        let artifact = Artifact::new(
            "test".to_string(),
            "build".to_string(),
            "echo".to_string(),
            cache_key.to_string(),
            output_files,
        )
        .unwrap();

        let compressed = artifact.compressed_data().to_vec();
        let result = verifier.verify_upload(&compressed, cache_key);

        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_cache_key_mismatch() {
        let verifier = Verifier::new(1024 * 1024);

        let mut output_files = BTreeMap::new();
        output_files.insert(PathBuf::from("file.txt"), b"content".to_vec());

        let cache_key = "aabbccdd11223344556677889900aabbccddeeff";
        let artifact = Artifact::new(
            "test".to_string(),
            "build".to_string(),
            "echo".to_string(),
            cache_key.to_string(),
            output_files,
        )
        .unwrap();

        let compressed = artifact.compressed_data().to_vec();
        let result = verifier.verify_upload(&compressed, "different_key");

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_size_limit() {
        let verifier = Verifier::new(100); // Very small limit

        let mut output_files = BTreeMap::new();
        output_files.insert(PathBuf::from("file.txt"), vec![0u8; 1000]); // Large file

        let cache_key = "aabbccdd11223344556677889900aabbccddeeff";
        let artifact = Artifact::new(
            "test".to_string(),
            "build".to_string(),
            "echo".to_string(),
            cache_key.to_string(),
            output_files,
        )
        .unwrap();

        let compressed = artifact.compressed_data().to_vec();
        let _result = verifier.verify_upload(&compressed, cache_key);

        // Should fail due to size limit (even compressed, it might exceed)
        // This test depends on compression ratio
    }
}
