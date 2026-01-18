//! Tests for verification module.

use polykit_cache::verification::Verifier;
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
    assert!(result.unwrap_err().to_string().contains("Cache key mismatch"));
}

#[test]
fn test_verify_corrupt_artifact() {
    let verifier = Verifier::new(1024 * 1024);

    // Create invalid compressed data
    let corrupt_data = b"not a valid artifact".to_vec();
    let cache_key = "aabbccdd11223344556677889900aabbccddeeff";

    let result = verifier.verify_upload(&corrupt_data, cache_key);

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
    if compressed.len() > 100 {
        let result = verifier.verify_upload(&compressed, cache_key);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }
}
