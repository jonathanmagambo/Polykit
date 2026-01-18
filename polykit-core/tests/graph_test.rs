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
        Package::new(
            "pkg-c".to_string(),
            Language::Rust,
            true,
            "pkg-c".into(),
            vec!["pkg-b".to_string()],
            vec![],
        ),
    ]
}

#[test]
fn test_topological_order() {
    let packages = create_test_packages();
    let graph = DependencyGraph::new(packages).unwrap();
    let order = graph.topological_order();

    assert_eq!(order.len(), 3);
    assert_eq!(order[0], "pkg-a");
    assert_eq!(order[1], "pkg-b");
    assert_eq!(order[2], "pkg-c");
}

#[test]
fn test_dependencies() {
    let packages = create_test_packages();
    let graph = DependencyGraph::new(packages).unwrap();

    let deps = graph.dependencies("pkg-b").unwrap();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0], "pkg-a");

    let deps = graph.dependencies("pkg-a").unwrap();
    assert_eq!(deps.len(), 0);
}

#[test]
fn test_dependents() {
    let packages = create_test_packages();
    let graph = DependencyGraph::new(packages).unwrap();

    let dependents = graph.dependents("pkg-a").unwrap();
    assert_eq!(dependents.len(), 1);
    assert_eq!(dependents[0], "pkg-b");

    let dependents = graph.dependents("pkg-c").unwrap();
    assert_eq!(dependents.len(), 0);
}

#[test]
fn test_all_dependents() {
    let packages = create_test_packages();
    let graph = DependencyGraph::new(packages).unwrap();

    let all_deps = graph.all_dependents("pkg-a").unwrap();
    assert_eq!(all_deps.len(), 2);
    assert!(all_deps.contains("pkg-b"));
    assert!(all_deps.contains("pkg-c"));
}

#[test]
fn test_circular_dependency() {
    let packages = vec![
        Package::new(
            "pkg-a".to_string(),
            Language::Rust,
            true,
            "pkg-a".into(),
            vec!["pkg-b".to_string()],
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
    ];

    let result = DependencyGraph::new(packages);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Circular dependency"));
}

#[test]
fn test_affected_packages() {
    let packages = create_test_packages();
    let graph = DependencyGraph::new(packages).unwrap();

    let affected = graph.affected_packages(&["pkg-a".to_string()]).unwrap();
    assert_eq!(affected.len(), 3);
    assert!(affected.contains("pkg-a"));
    assert!(affected.contains("pkg-b"));
    assert!(affected.contains("pkg-c"));
}
