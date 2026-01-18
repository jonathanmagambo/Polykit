use polykit_core::package::{Language, Package, Task};
use polykit_core::streaming::StreamingTask;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_streaming_task_spawn() {
    let package = Package::new(
        "test-pkg".to_string(),
        Language::Rust,
        true,
        PathBuf::from("test"),
        vec![],
        vec![Task {
            name: "echo".to_string(),
            command: "echo hello".to_string(),
            depends_on: vec![],
        }],
    );

    let temp_dir = TempDir::new().unwrap();
    let task = StreamingTask::spawn(&package, "echo", temp_dir.path()).await;

    assert!(task.is_ok());
}

#[tokio::test]
async fn test_streaming_task_output() {
    let package = Package::new(
        "test-pkg".to_string(),
        Language::Rust,
        true,
        PathBuf::from("test"),
        vec![],
        vec![Task {
            name: "echo".to_string(),
            command: "echo test output".to_string(),
            depends_on: vec![],
        }],
    );

    let temp_dir = TempDir::new().unwrap();
    let streaming_task = StreamingTask::spawn(&package, "echo", temp_dir.path())
        .await
        .unwrap();

    let mut output_lines = Vec::new();
    let success = streaming_task
        .stream_output(|line, is_stderr| {
            output_lines.push((line.to_string(), is_stderr));
        })
        .await
        .unwrap();

    assert!(success);
    assert!(!output_lines.is_empty());
}
