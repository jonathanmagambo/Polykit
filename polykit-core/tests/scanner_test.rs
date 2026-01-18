use std::fs;
use tempfile::TempDir;

use polykit_core::scanner::Scanner;

fn create_test_package(dir: &std::path::Path, name: &str, language: &str, deps: &[&str]) {
    let pkg_dir = dir.join(name);
    fs::create_dir_all(&pkg_dir).unwrap();

    let deps_str = deps
        .iter()
        .map(|d| format!("\"{}\"", d))
        .collect::<Vec<_>>()
        .join(", ");

    let config = format!(
        r#"
name = "{}"
language = "{}"
public = true

[deps]
internal = [{}]

[tasks]
build = "echo build"
test = "echo test"
"#,
        name, language, deps_str
    );

    fs::write(pkg_dir.join("polykit.toml"), config).unwrap();
}

#[test]
fn test_scan_packages() {
    let temp_dir = TempDir::new().unwrap();
    let packages_dir = temp_dir.path().join("packages");
    fs::create_dir_all(&packages_dir).unwrap();

    create_test_package(&packages_dir, "pkg-a", "rust", &[]);
    create_test_package(&packages_dir, "pkg-b", "js", &["pkg-a"]);

    let mut scanner = Scanner::new(&packages_dir);
    let packages = scanner.scan().unwrap();

    assert_eq!(packages.len(), 2);
    assert_eq!(packages[0].name, "pkg-a");
    assert_eq!(packages[1].name, "pkg-b");
    assert_eq!(packages[1].deps.len(), 1);
}

#[test]
fn test_scan_as_map() {
    let temp_dir = TempDir::new().unwrap();
    let packages_dir = temp_dir.path().join("packages");
    fs::create_dir_all(&packages_dir).unwrap();

    create_test_package(&packages_dir, "pkg-a", "rust", &[]);

    let mut scanner = Scanner::new(&packages_dir);
    let map = scanner.scan_as_map().unwrap();

    assert_eq!(map.len(), 1);
    assert!(map.contains_key("pkg-a"));
}
