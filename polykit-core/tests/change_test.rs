use std::fs;
use tempfile::TempDir;

use polykit_core::change::ChangeDetector;
use polykit_core::graph::DependencyGraph;
use polykit_core::package::{Language, Package};

fn create_test_packages() -> Vec<Package> {
    vec![
        Package::new(
            "pkg-a".to_string(),
            Language::Rust,
            true,
            "pkg-a".into(),
            vec![],
            vec![],
        ),
        Package::new(
            "pkg-b".to_string(),
            Language::Rust,
            true,
            "pkg-b".into(),
            vec!["pkg-a".to_string()],
            vec![],
        ),
    ]
}

#[test]
fn test_detect_affected_packages() {
    let temp_dir = TempDir::new().unwrap();
    let packages_dir = temp_dir.path().join("packages");
    fs::create_dir_all(&packages_dir).unwrap();
    fs::create_dir_all(packages_dir.join("pkg-a")).unwrap();
    fs::create_dir_all(packages_dir.join("pkg-b")).unwrap();

    let packages = create_test_packages();
    let graph = DependencyGraph::new(packages).unwrap();

    let changed_files = vec![packages_dir.join("pkg-a").join("src").join("lib.rs")];
    let affected =
        ChangeDetector::detect_affected_packages(&graph, &changed_files, &packages_dir).unwrap();

    assert_eq!(affected.len(), 2);
    assert!(affected.contains("pkg-a"));
    assert!(affected.contains("pkg-b"));
}

#[test]
fn test_detect_single_package() {
    let temp_dir = TempDir::new().unwrap();
    let packages_dir = temp_dir.path().join("packages");
    fs::create_dir_all(&packages_dir).unwrap();
    fs::create_dir_all(packages_dir.join("pkg-b")).unwrap();

    let packages = create_test_packages();
    let graph = DependencyGraph::new(packages).unwrap();

    let changed_files = vec![packages_dir.join("pkg-b").join("src").join("main.rs")];
    let affected =
        ChangeDetector::detect_affected_packages(&graph, &changed_files, &packages_dir).unwrap();

    assert_eq!(affected.len(), 1);
    assert!(affected.contains("pkg-b"));
}
