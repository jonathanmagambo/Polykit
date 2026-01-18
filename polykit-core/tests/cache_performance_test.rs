use polykit_core::cache::Cache;
use polykit_core::package::{Language, Package, Task};
use std::path::PathBuf;
use tempfile::TempDir;

fn generate_packages(count: usize) -> Vec<Package> {
    let mut packages = Vec::with_capacity(count);

    for i in 0..count {
        packages.push(Package::new(
            format!("package-{}", i),
            Language::Rust,
            i % 10 == 0,
            PathBuf::from(format!("packages/package-{}", i)),
            Vec::new(),
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
fn test_cache_validation_performance() {
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

    let cache = Cache::new(temp_dir.path().join("cache"));
    let packages = generate_packages(500);

    cache
        .save(&packages_dir, &packages)
        .expect("Should save cache");

    let start = std::time::Instant::now();
    let mut cache = Cache::new(temp_dir.path().join("cache"));
    let cached = cache.load(&packages_dir).expect("Should load cache");
    let duration = start.elapsed();

    assert!(cached.is_some(), "Cache should be valid");
    assert_eq!(cached.unwrap().len(), 500);
    assert!(
        duration.as_millis() < 1000,
        "Cache validation should be fast (< 1s for 500 packages)"
    );
}

#[test]
fn test_cache_invalidation_on_change() {
    let temp_dir = TempDir::new().expect("Should create temp directory");
    let packages_dir = temp_dir.path().join("packages");
    std::fs::create_dir_all(&packages_dir).expect("Should create packages directory");

    for i in 0..100 {
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

    let cache = Cache::new(temp_dir.path().join("cache"));
    let packages = generate_packages(100);

    cache
        .save(&packages_dir, &packages)
        .expect("Should save cache");

    let mut cache = Cache::new(temp_dir.path().join("cache"));
    let cached = cache.load(&packages_dir).expect("Should load cache");
    assert!(cached.is_some(), "Cache should be valid initially");

    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(
        packages_dir.join("package-0").join("polykit.toml"),
        r#"
name = "package-0"
language = "rust"
public = true

[deps]
internal = ["package-1"]
"#,
    )
    .expect("Should modify file");

    let mut cache = Cache::new(temp_dir.path().join("cache"));
    let cached = cache.load(&packages_dir).expect("Should load cache");
    assert!(cached.is_none(), "Cache should be invalidated after change");
}
