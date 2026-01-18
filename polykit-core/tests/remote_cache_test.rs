//! Tests for remote cache system.

use polykit_core::remote_cache::{
    Artifact, ArtifactVerifier, CacheKey, RemoteCache,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tempfile::TempDir;

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
    assert_eq!(key1.as_string(), key2.as_string());
}

#[test]
fn test_cache_key_different_inputs() {
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
        .command("echo world")
        .dependency_graph_hash("abc123")
        .toolchain_version("node-v20.0.0")
        .build()
        .unwrap();

    assert_ne!(key1.hash(), key2.hash());
}

#[test]
fn test_cache_key_env_vars() {
    let mut env_vars = BTreeMap::new();
    env_vars.insert("VAR1".to_string(), "value1".to_string());
    env_vars.insert("VAR2".to_string(), "value2".to_string());

    let key1 = CacheKey::builder()
        .package_id("test")
        .task_name("build")
        .command("echo")
        .env_vars(env_vars.clone())
        .dependency_graph_hash("abc")
        .toolchain_version("node-v20")
        .build()
        .unwrap();

    let key2 = CacheKey::builder()
        .package_id("test")
        .task_name("build")
        .command("echo")
        .env_vars(env_vars)
        .dependency_graph_hash("abc")
        .toolchain_version("node-v20")
        .build()
        .unwrap();

    assert_eq!(key1.hash(), key2.hash());
}

#[test]
fn test_artifact_creation_and_verification() {
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

    // Verify integrity
    assert!(ArtifactVerifier::verify(&artifact, None).is_ok());
}

#[test]
fn test_artifact_round_trip() {
    let mut output_files = BTreeMap::new();
    output_files.insert(PathBuf::from("file.txt"), b"content".to_vec());

    let artifact1 = Artifact::new(
        "test".to_string(),
        "build".to_string(),
        "echo".to_string(),
        "hash123".to_string(),
        output_files,
    )
    .unwrap();

    let compressed = artifact1.compressed_data().to_vec();
    let artifact2 = Artifact::from_compressed(compressed).unwrap();

    assert_eq!(
        artifact1.metadata().package_name,
        artifact2.metadata().package_name
    );
    assert_eq!(artifact1.metadata().task_name, artifact2.metadata().task_name);
    assert_eq!(
        artifact1.manifest().files.len(),
        artifact2.manifest().files.len()
    );
}

#[test]
fn test_artifact_extraction() {
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

    let temp_dir = TempDir::new().unwrap();
    artifact.extract_outputs(temp_dir.path()).unwrap();

    let extracted_file = temp_dir.path().join("file.txt");
    assert!(extracted_file.exists());
    let content = std::fs::read_to_string(&extracted_file).unwrap();
    assert_eq!(content, "content");
}

#[tokio::test]
async fn test_filesystem_backend() {
    use polykit_core::remote_cache::{FilesystemBackend, RemoteCacheBackend};

    let temp_dir = TempDir::new().unwrap();
    let backend = FilesystemBackend::new(temp_dir.path()).unwrap();

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

    let key = CacheKey::builder()
        .package_id("test")
        .task_name("build")
        .command("echo")
        .dependency_graph_hash("abc")
        .toolchain_version("node-v20")
        .build()
        .unwrap();

    // Upload
    backend.upload_artifact(&key, &artifact).await.unwrap();

    // Check exists
    assert!(backend.has_artifact(&key).await.unwrap());

    // Fetch
    let fetched = backend.fetch_artifact(&key).await.unwrap();
    assert!(fetched.is_some());

    let fetched_artifact = fetched.unwrap();
    assert_eq!(
        artifact.metadata().package_name,
        fetched_artifact.metadata().package_name
    );
}

#[test]
fn test_remote_cache_disabled() {
    let cache = RemoteCache::disabled();
    assert!(!cache.is_enabled());
}

#[test]
fn test_cache_key_builder_validation() {
    // Missing required fields should fail
    assert!(CacheKey::builder()
        .package_id("test")
        .task_name("build")
        .build()
        .is_err());

    assert!(CacheKey::builder()
        .package_id("test")
        .task_name("build")
        .command("echo")
        .build()
        .is_err());
}

#[test]
fn test_artifact_hash() {
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

    let hash = artifact.hash();
    assert!(!hash.is_empty());
    assert_eq!(hash.len(), 64); // SHA-256 hex string length
}
