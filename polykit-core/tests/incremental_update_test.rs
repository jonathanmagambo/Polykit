use polykit_core::graph::{DependencyGraph, GraphChange};
use polykit_core::package::{Language, Package, Task};
use std::path::PathBuf;

fn generate_test_packages() -> Vec<Package> {
    vec![
        Package::new(
            "base".to_string(),
            Language::Rust,
            false,
            PathBuf::from("packages/base"),
            Vec::new(),
            Vec::new(),
        ),
        Package::new(
            "middle".to_string(),
            Language::Rust,
            false,
            PathBuf::from("packages/middle"),
            vec!["base".to_string()],
            Vec::new(),
        ),
        Package::new(
            "top".to_string(),
            Language::Rust,
            false,
            PathBuf::from("packages/top"),
            vec!["middle".to_string()],
            Vec::new(),
        ),
    ]
}

#[test]
fn test_incremental_add_package() {
    let packages = generate_test_packages();
    let mut graph = DependencyGraph::new(packages.clone()).unwrap();

    let new_package = Package::new(
        "new".to_string(),
        Language::Rust,
        false,
        PathBuf::from("packages/new"),
        vec!["base".to_string()],
        Vec::new(),
    );

    let change = GraphChange {
        added: vec![new_package.clone()],
        modified: Vec::new(),
        removed: Vec::new(),
        dependency_changes: Vec::new(),
    };

    graph.update_incremental(change).unwrap();

    assert!(graph.get_package("new").is_some());
    assert_eq!(graph.all_packages().len(), 4);
}

#[test]
fn test_incremental_remove_package() {
    let packages = generate_test_packages();
    let mut graph = DependencyGraph::new(packages).unwrap();

    let change = GraphChange {
        added: Vec::new(),
        modified: Vec::new(),
        removed: vec!["top".to_string()],
        dependency_changes: Vec::new(),
    };

    graph.update_incremental(change).unwrap();

    assert!(graph.get_package("top").is_none());
    assert_eq!(graph.all_packages().len(), 2);
}

#[test]
fn test_incremental_modify_package() {
    let packages = generate_test_packages();
    let mut graph = DependencyGraph::new(packages).unwrap();

    // Modify middle to add a new dependency (but not create a cycle)
    let modified = Package::new(
        "middle".to_string(),
        Language::Rust,
        false,
        PathBuf::from("packages/middle"),
        vec!["base".to_string()],
        vec![Task {
            name: "new-task".to_string(),
            command: "echo test".to_string(),
            depends_on: Vec::new(),
        }],
    );

    let change = GraphChange {
        added: Vec::new(),
        modified: vec![modified],
        removed: Vec::new(),
        dependency_changes: vec![("middle".to_string(), vec!["base".to_string()])],
    };

    graph.update_incremental(change).unwrap();

    let deps = graph.dependencies("middle").unwrap();
    assert_eq!(deps.len(), 1);
    assert!(deps.contains(&"base".to_string()));
    
    // Verify task was updated
    let pkg = graph.get_package("middle").unwrap();
    assert_eq!(pkg.tasks.len(), 1);
    assert_eq!(pkg.tasks[0].name, "new-task");
}

#[test]
fn test_incremental_update_preserves_topological_order() {
    let packages = generate_test_packages();
    let mut graph = DependencyGraph::new(packages).unwrap();

    let original_order = graph.topological_order();

    let new_package = Package::new(
        "new".to_string(),
        Language::Rust,
        false,
        PathBuf::from("packages/new"),
        vec!["base".to_string()],
        Vec::new(),
    );

    let change = GraphChange {
        added: vec![new_package],
        modified: Vec::new(),
        removed: Vec::new(),
        dependency_changes: Vec::new(),
    };

    graph.update_incremental(change).unwrap();

    let new_order = graph.topological_order();
    assert!(new_order.len() > original_order.len());
    assert!(new_order.contains(&"new".to_string()));
}

#[test]
fn test_incremental_update_detects_circular_dependency() {
    let packages = generate_test_packages();
    let mut graph = DependencyGraph::new(packages).unwrap();

    let modified_base = Package::new(
        "base".to_string(),
        Language::Rust,
        false,
        PathBuf::from("packages/base"),
        vec!["top".to_string()],
        Vec::new(),
    );

    let change = GraphChange {
        added: Vec::new(),
        modified: vec![modified_base],
        removed: Vec::new(),
        dependency_changes: vec![("base".to_string(), vec!["top".to_string()])],
    };

    let result = graph.update_incremental(change);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Cycle"));
}
