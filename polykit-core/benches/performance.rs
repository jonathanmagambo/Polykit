use criterion::{black_box, criterion_group, criterion_main, Criterion};
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

fn benchmark_graph_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_construction");

    for count in [100, 500, 1000, 2000, 5000] {
        group.bench_function(format!("{}_packages", count), |b| {
            let packages = generate_packages(count, 3);
            b.iter(|| black_box(DependencyGraph::new(packages.clone()).unwrap()));
        });
    }

    group.finish();
}

fn benchmark_package_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_lookup");

    for count in [100, 500, 1000, 2000, 5000] {
        let packages = generate_packages(count, 3);
        let graph = DependencyGraph::new(packages).unwrap();

        group.bench_function(format!("{}_packages", count), |b| {
            b.iter(|| {
                for i in 0..100 {
                    let name = format!("package-{}", i % count);
                    black_box(graph.get_package(&name));
                }
            });
        });
    }

    group.finish();
}

fn benchmark_topological_order(c: &mut Criterion) {
    let mut group = c.benchmark_group("topological_order");

    for count in [100, 500, 1000, 2000, 5000] {
        let packages = generate_packages(count, 3);
        let graph = DependencyGraph::new(packages).unwrap();

        group.bench_function(format!("{}_packages", count), |b| {
            b.iter(|| {
                black_box(graph.topological_order());
            });
        });
    }

    group.finish();
}

fn benchmark_dependency_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("dependency_levels");

    for count in [100, 500, 1000, 2000, 5000] {
        let packages = generate_packages(count, 3);
        let graph = DependencyGraph::new(packages).unwrap();

        group.bench_function(format!("{}_packages", count), |b| {
            b.iter(|| {
                black_box(graph.dependency_levels());
            });
        });
    }

    group.finish();
}

fn benchmark_affected_packages(c: &mut Criterion) {
    let mut group = c.benchmark_group("affected_packages");

    for count in [100, 500, 1000, 2000, 5000] {
        let packages = generate_packages(count, 3);
        let graph = DependencyGraph::new(packages).unwrap();
        let changed = vec!["package-0".to_string(), format!("package-{}", count / 10)];

        group.bench_function(format!("{}_packages", count), |b| {
            b.iter(|| {
                black_box(graph.affected_packages(&changed).unwrap());
            });
        });
    }

    group.finish();
}

fn benchmark_scanner_with_many_packages(c: &mut Criterion) {
    let mut group = c.benchmark_group("scanner");

    for count in [50, 200, 1000, 2000, 5000] {
        let temp_dir = TempDir::new().unwrap();
        let packages_dir = temp_dir.path().join("packages");
        std::fs::create_dir_all(&packages_dir).unwrap();

        for i in 0..count {
            let package_dir = packages_dir.join(format!("package-{}", i));
            std::fs::create_dir_all(&package_dir).unwrap();

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

            std::fs::write(package_dir.join("polykit.toml"), toml_content).unwrap();
        }

        group.bench_function(format!("scan_{}_packages", count), |b| {
            b.iter(|| {
                let mut scanner = Scanner::new(&packages_dir);
                black_box(scanner.scan().unwrap());
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_graph_construction,
    benchmark_package_lookup,
    benchmark_topological_order,
    benchmark_dependency_levels,
    benchmark_affected_packages,
    benchmark_scanner_with_many_packages
);
criterion_main!(benches);
