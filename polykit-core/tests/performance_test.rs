use polykit_core::graph::DependencyGraph;
use polykit_core::package::{Language, Package, Task};
use polykit_core::scanner::Scanner;
use std::path::PathBuf;
use tempfile::TempDir;

fn generate_packages(count: usize, deps_per_package: usize) -> Vec<Package> {
    let mut packages = Vec::with_capacity(count);

    for i in 0..count {
        let deps = if i > 0 && deps_per_package > 0 {
            let dep_count = deps_per_package.min(i);
            (0..dep_count)
                .map(|j| {
                    let dep_idx = i - 1 - j;
                    format!("package-{}", dep_idx)
                })
                .collect()
        } else {
            Vec::new()
        };

        packages.push(Package::new(
            format!("package-{}", i),
            Language::Rust,
            i % 10 == 0,
            PathBuf::from(format!("packages/package-{}", i)),
            deps,
            vec![Task {
                name: "build".to_string(),
                command: "echo build".to_string(),
                depends_on: Vec::new(),
            }],
        ));
    }

    packages
}

#[test]
fn test_graph_construction_performance() {
    let packages = generate_packages(1000, 3);
    let graph = DependencyGraph::new(packages).expect("Graph construction should succeed");
    assert_eq!(graph.all_packages().len(), 1000);
}

#[test]
fn test_package_lookup_performance() {
    let packages = generate_packages(1000, 3);
    let graph = DependencyGraph::new(packages).expect("Graph construction should succeed");

    for i in 0..100 {
        let name = format!("package-{}", i);
        let package = graph.get_package(&name);
        assert!(package.is_some(), "Package {} should be found", name);
        assert_eq!(package.unwrap().name, name);
    }

    assert!(graph.get_package("nonexistent").is_none());
}

#[test]
fn test_package_lookup_o1_complexity() {
    let packages = generate_packages(1000, 3);
    let graph = DependencyGraph::new(packages).expect("Graph construction should succeed");

    let start = std::time::Instant::now();
    for _ in 0..10000 {
        for i in 0..1000 {
            let name = format!("package-{}", i);
            let _ = graph.get_package(&name);
        }
    }
    let duration = start.elapsed();

    // Allow more time in CI environments (Windows, slower CI runners)
    let max_duration_ms = if std::env::var("CI").is_ok() {
        15000 // 15s for CI
    } else {
        5000 // 5s for local development
    };

    assert!(
        duration.as_millis() < max_duration_ms,
        "10M lookups should complete in under {}ms (O(1) lookup), took {}ms",
        max_duration_ms,
        duration.as_millis()
    );
}

#[test]
fn test_topological_order_large_graph() {
    let packages = generate_packages(1000, 3);
    let graph = DependencyGraph::new(packages).expect("Graph construction should succeed");

    let order = graph.topological_order();
    assert_eq!(order.len(), 1000);

    let mut seen = std::collections::HashSet::new();
    for package_name in &order {
        assert!(
            !seen.contains(package_name),
            "No duplicates in topological order"
        );
        seen.insert(package_name.clone());
    }
}

#[test]
fn test_dependency_levels_large_graph() {
    let packages = generate_packages(1000, 3);
    let graph = DependencyGraph::new(packages).expect("Graph construction should succeed");

    let levels = graph.dependency_levels();
    assert!(!levels.is_empty());

    let total_packages: usize = levels.iter().map(|level| level.len()).sum();
    assert_eq!(total_packages, 1000);
}

#[test]
fn test_affected_packages_performance() {
    let packages = generate_packages(1000, 3);
    let graph = DependencyGraph::new(packages).expect("Graph construction should succeed");

    let changed = vec!["package-0".to_string()];
    let affected = graph
        .affected_packages(&changed)
        .expect("Should compute affected packages");

    assert!(affected.contains("package-0"));
    assert!(!affected.is_empty());
}

#[test]
fn test_scanner_with_many_packages() {
    let temp_dir = TempDir::new().expect("Should create temp directory");
    let packages_dir = temp_dir.path().join("packages");
    std::fs::create_dir_all(&packages_dir).expect("Should create packages directory");

    for i in 0..500 {
        let package_dir = packages_dir.join(format!("package-{}", i));
        std::fs::create_dir_all(&package_dir).expect("Should create package directory");

        let toml_content = format!(
            r#"
name = "package-{}"
language = "rust"
public = {}

[deps]
internal = []
"#,
            i,
            i % 10 == 0
        );

        std::fs::write(package_dir.join("polykit.toml"), toml_content)
            .expect("Should write polykit.toml");
    }

    let mut scanner = Scanner::new(&packages_dir);
    let packages = scanner.scan().expect("Should scan packages");

    assert_eq!(packages.len(), 500);
}

#[test]
fn test_graph_construction_with_many_dependencies() {
    let mut packages = Vec::with_capacity(500);

    for i in 0..500 {
        let deps = if i > 0 {
            (0..i.min(10)).map(|j| format!("package-{}", j)).collect()
        } else {
            Vec::new()
        };

        packages.push(Package::new(
            format!("package-{}", i),
            Language::Rust,
            false,
            PathBuf::from(format!("packages/package-{}", i)),
            deps,
            Vec::new(),
        ));
    }

    let graph = DependencyGraph::new(packages).expect("Graph construction should succeed");
    assert_eq!(graph.all_packages().len(), 500);
}

#[test]
fn test_concurrent_package_lookups() {
    let packages = generate_packages(1000, 3);
    let graph = DependencyGraph::new(packages).expect("Graph construction should succeed");

    use std::sync::Arc;
    let graph = Arc::new(graph);

    let handles: Vec<_> = (0..10)
        .map(|thread_id| {
            let graph = Arc::clone(&graph);
            std::thread::spawn(move || {
                for i in 0..100 {
                    let name = format!("package-{}", (thread_id * 100 + i) % 1000);
                    let package = graph.get_package(&name);
                    assert!(package.is_some(), "Package {} should be found", name);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread should complete successfully");
    }
}

#[test]
fn test_dependencies_and_dependents_consistency() {
    let packages = generate_packages(500, 3);
    let graph = DependencyGraph::new(packages).expect("Graph construction should succeed");

    for i in 0..100 {
        let name = format!("package-{}", i);
        let deps = graph.dependencies(&name).expect("Should get dependencies");
        for dep in &deps {
            let dependents = graph.dependents(dep).expect("Should get dependents");
            assert!(
                dependents.contains(&name),
                "If {} depends on {}, then {} should be in {}'s dependents",
                name,
                dep,
                name,
                dep
            );
        }
    }
}

#[test]
fn test_all_dependents_transitive() {
    let packages = vec![
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
    ];

    let graph = DependencyGraph::new(packages).expect("Graph construction should succeed");

    let dependents = graph
        .all_dependents("base")
        .expect("Should get all dependents");

    assert!(dependents.contains("middle"));
    assert!(dependents.contains("top"));
    assert_eq!(dependents.len(), 2);
}

#[test]
fn test_affected_packages_includes_changed() {
    let packages = generate_packages(500, 3);
    let graph = DependencyGraph::new(packages).expect("Graph construction should succeed");

    let changed = vec!["package-100".to_string(), "package-200".to_string()];
    let affected = graph
        .affected_packages(&changed)
        .expect("Should compute affected packages");

    assert!(affected.contains("package-100"));
    assert!(affected.contains("package-200"));
    assert!(affected.len() >= 2);
}
