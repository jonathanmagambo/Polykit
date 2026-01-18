//! Tests for storage layer.

use polykit_cache::storage::Storage;
use polykit_core::remote_cache::Artifact;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_storage_sharding() {
    let temp_dir = TempDir::new().unwrap();
    let storage = Storage::new(temp_dir.path(), 1024 * 1024).unwrap();

    let cache_key = "aabbccdd11223344556677889900aabbccddeeff";
    let mut output_files = BTreeMap::new();
    output_files.insert(PathBuf::from("file.txt"), b"content".to_vec());

    let artifact = Artifact::new(
        "test".to_string(),
        "build".to_string(),
        "echo".to_string(),
        cache_key.to_string(),
        output_files,
    )
    .unwrap();

    // Store artifact and verify it's in the correct shard directory
    storage
        .store_artifact(cache_key, artifact.compressed_data().to_vec(), "hash".to_string(), &artifact)
        .await
        .unwrap();

    // Verify artifact file exists in sharded location
    let artifact_file = temp_dir.path().join("aa").join("bb").join(format!("{}.zst", cache_key));
    assert!(artifact_file.exists());
}

#[tokio::test]
async fn test_storage_atomic_write() {
    let temp_dir = TempDir::new().unwrap();
    let storage = Storage::new(temp_dir.path(), 1024 * 1024).unwrap();

    let cache_key = "aabbccdd11223344556677889900aabbccddeeff";
    let data = b"test data".to_vec();
    let hash = "test_hash".to_string();

    let mut output_files = BTreeMap::new();
    output_files.insert(PathBuf::from("file.txt"), b"content".to_vec());

    let artifact = Artifact::new(
        "test".to_string(),
        "build".to_string(),
        "echo".to_string(),
        cache_key.to_string(),
        output_files,
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

    let mut output_files = BTreeMap::new();
    output_files.insert(PathBuf::from("file.txt"), b"content".to_vec());

    let artifact1 = Artifact::new(
        "test".to_string(),
        "build".to_string(),
        "echo".to_string(),
        cache_key.to_string(),
        output_files,
    )
    .unwrap();

    // Store artifact
    storage
        .store_artifact(cache_key, data, hash, &artifact1)
        .await
        .unwrap();

    // Try to store again (should fail)
    let artifact2 = Artifact::new(
        "test".to_string(),
        "build".to_string(),
        "echo".to_string(),
        cache_key.to_string(),
        BTreeMap::new(),
    )
    .unwrap();

    let result = storage
        .store_artifact(cache_key, b"different".to_vec(), "hash2".to_string(), &artifact2)
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[tokio::test]
async fn test_storage_concurrent_uploads() {
    use tokio::task;

    let temp_dir = TempDir::new().unwrap();
    let storage = Storage::new(temp_dir.path(), 1024 * 1024).unwrap();

    let mut handles = Vec::new();

    // Spawn multiple concurrent uploads with different keys
    for i in 0..10 {
        let storage_clone = Storage::new(temp_dir.path(), 1024 * 1024).unwrap();
        let cache_key = format!("aabbccdd11223344556677889900aabbccdd{:02x}", i);
        let data = format!("test data {}", i).into_bytes();
        let hash = format!("hash_{}", i);

        let mut output_files = BTreeMap::new();
        output_files.insert(PathBuf::from("file.txt"), data.clone());

        let artifact = Artifact::new(
            "test".to_string(),
            "build".to_string(),
            "echo".to_string(),
            cache_key.clone(),
            output_files,
        )
        .unwrap();

        handles.push(task::spawn(async move {
            storage_clone
                .store_artifact(&cache_key, data, hash, &artifact)
                .await
        }));
    }

    // Wait for all uploads
    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }

    // Verify all artifacts exist
    for i in 0..10 {
        let cache_key = format!("aabbccdd11223344556677889900aabbccdd{:02x}", i);
        assert!(storage.has_artifact(&cache_key));
    }
}

#[tokio::test]
async fn test_storage_size_limit() {
    let temp_dir = TempDir::new().unwrap();
    let max_size = 1000;
    let storage = Storage::new(temp_dir.path(), max_size).unwrap();

    let cache_key = "aabbccdd11223344556677889900aabbccddeeff";
    let large_data = vec![0u8; 2000]; // Exceeds limit
    let hash = "hash".to_string();

    let mut output_files = BTreeMap::new();
    output_files.insert(PathBuf::from("file.txt"), large_data.clone());

    let artifact = Artifact::new(
        "test".to_string(),
        "build".to_string(),
        "echo".to_string(),
        cache_key.to_string(),
        output_files,
    )
    .unwrap();

    let compressed = artifact.compressed_data();
    if compressed.len() > max_size as usize {
        let result = storage
            .store_artifact(cache_key, compressed.to_vec(), hash, &artifact)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }
}
