//! End-to-end integration tests for the cache server.

use polykit_cache::server::{create_router, AppState};
use polykit_cache::storage::Storage;
use polykit_cache::verification::Verifier;
use polykit_core::remote_cache::{Artifact, CacheKey, HttpBackend, RemoteCacheBackend, RemoteCacheConfig};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::task;
use tokio::time::Duration;

async fn start_test_server(temp_dir: &TempDir) -> String {
    let storage = Storage::new(temp_dir.path(), 1024 * 1024 * 1024).unwrap();
    let verifier = Verifier::new(1024 * 1024 * 1024);
    let state = AppState::new(storage, verifier);
    let app = create_router(state);

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{}", addr);

    // Spawn server task
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    url
}

#[tokio::test]
async fn test_e2e_upload_and_download() {
    let temp_dir = TempDir::new().unwrap();
    let server_url = start_test_server(&temp_dir).await;

    // Create cache key first
    let cache_key = CacheKey::builder()
        .package_id("test-package")
        .task_name("build")
        .command("echo test")
        .dependency_graph_hash("abc")
        .toolchain_version("node-v20")
        .build()
        .unwrap();

    // Create test artifact with matching cache key hash
    let mut output_files = BTreeMap::new();
    output_files.insert(PathBuf::from("file.txt"), b"test content".to_vec());

    let cache_key_hash = cache_key.as_string();
    let artifact = Artifact::new(
        "test-package".to_string(),
        "build".to_string(),
        "echo test".to_string(),
        cache_key_hash,
        output_files,
    )
    .unwrap();

    // Create HTTP backend client
    let config = RemoteCacheConfig::new(&server_url);
    let backend = HttpBackend::new(&config).unwrap();

    // Upload artifact
    backend
        .upload_artifact(&cache_key, &artifact)
        .await
        .unwrap();

    // Check if artifact exists
    let exists = backend.has_artifact(&cache_key).await.unwrap();
    assert!(exists);

    // Download artifact
    let downloaded = backend.fetch_artifact(&cache_key).await.unwrap();
    assert!(downloaded.is_some());

    let downloaded_artifact = downloaded.unwrap();
    assert_eq!(
        downloaded_artifact.metadata().package_name,
        artifact.metadata().package_name
    );
    assert_eq!(
        downloaded_artifact.metadata().task_name,
        artifact.metadata().task_name
    );
}

#[tokio::test]
async fn test_e2e_concurrent_uploads() {
    let temp_dir = TempDir::new().unwrap();
    let server_url = start_test_server(&temp_dir).await;

    let config = RemoteCacheConfig::new(&server_url);
    let mut handles = Vec::new();

    // Spawn multiple concurrent uploads
    for i in 0..10 {
        let config_clone = config.clone();
        let mut output_files = BTreeMap::new();
        output_files.insert(PathBuf::from("file.txt"), format!("content {}", i).into_bytes());

        let cache_key = CacheKey::builder()
            .package_id(format!("package-{}", i))
            .task_name("build")
            .command("echo")
            .dependency_graph_hash("abc")
            .toolchain_version("node-v20")
            .build()
            .unwrap();

        let cache_key_hash = cache_key.as_string();
        let artifact = Artifact::new(
            format!("package-{}", i),
            "build".to_string(),
            "echo".to_string(),
            cache_key_hash,
            output_files,
        )
        .unwrap();

        handles.push(task::spawn(async move {
            let backend = HttpBackend::new(&config_clone).unwrap();
            backend.upload_artifact(&cache_key, &artifact).await
        }));
    }

    // Wait for all uploads
    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }

    // Verify all artifacts exist
    let backend = HttpBackend::new(&config).unwrap();
    for i in 0..10 {
        let cache_key = CacheKey::builder()
            .package_id(format!("package-{}", i))
            .task_name("build")
            .command("echo")
            .dependency_graph_hash("abc")
            .toolchain_version("node-v20")
            .build()
            .unwrap();

        assert!(backend.has_artifact(&cache_key).await.unwrap());
    }
}

#[tokio::test]
async fn test_e2e_corrupt_artifact_rejection() {
    let temp_dir = TempDir::new().unwrap();
    let server_url = start_test_server(&temp_dir).await;

    let client = reqwest::Client::new();
    let cache_key = "aabbccdd11223344556677889900aabbccddeeff";
    let url = format!("{}/v1/artifacts/{}", server_url, cache_key);

    // Try to upload corrupt data
    let corrupt_data = b"not a valid artifact".to_vec();
    let response = client.put(&url).body(corrupt_data).send().await.unwrap();

    // Should reject with 422 Unprocessable Entity
    assert_eq!(response.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_e2e_cache_key_mismatch() {
    let temp_dir = TempDir::new().unwrap();
    let server_url = start_test_server(&temp_dir).await;

    // Create artifact with one cache key
    let mut output_files = BTreeMap::new();
    output_files.insert(PathBuf::from("file.txt"), b"content".to_vec());

    let correct_cache_key = "aabbccdd11223344556677889900aabbccddeeff";
    let artifact = Artifact::new(
        "test".to_string(),
        "build".to_string(),
        "echo".to_string(),
        correct_cache_key.to_string(),
        output_files,
    )
    .unwrap();

    // Try to upload with different cache key in URL
    let client = reqwest::Client::new();
    let wrong_cache_key = "ffffffffffffffffffffffffffffffffffffffff";
    let url = format!("{}/v1/artifacts/{}", server_url, wrong_cache_key);

    let compressed = artifact.compressed_data().to_vec();
    let response = client.put(&url).body(compressed).send().await.unwrap();

    // Should reject with 422 Unprocessable Entity
    assert_eq!(response.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_e2e_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let server_url = start_test_server(&temp_dir).await;

    let config = RemoteCacheConfig::new(&server_url);
    let backend = HttpBackend::new(&config).unwrap();

    let cache_key = CacheKey::builder()
        .package_id("nonexistent")
        .task_name("build")
        .command("echo")
        .dependency_graph_hash("abc")
        .toolchain_version("node-v20")
        .build()
        .unwrap();

    // Should return None (not an error)
    let result = backend.fetch_artifact(&cache_key).await.unwrap();
    assert!(result.is_none());

    // HEAD should return false
    assert!(!backend.has_artifact(&cache_key).await.unwrap());
}
