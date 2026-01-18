use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn create_test_package(dir: &Path, name: &str, language: &str) {
    let pkg_dir = dir.join(name);
    fs::create_dir_all(&pkg_dir).unwrap();

    let config = format!(
        r#"
name = "{}"
language = "{}"
public = true

[deps]
internal = []

[tasks]
build = "echo 'Building {}'"
test = "echo 'Testing {}'"
"#,
        name, language, name, name
    );

    fs::write(pkg_dir.join("polykit.toml"), config).unwrap();

    match language {
        "js" | "ts" => {
            fs::write(
                pkg_dir.join("package.json"),
                r#"{"name": "test", "version": "1.0.0"}"#,
            )
            .unwrap();
        }
        "rust" => {
            fs::write(
                pkg_dir.join("Cargo.toml"),
                r#"[package]
name = "test"
version = "0.1.0"
edition = "2021"
"#,
            )
            .unwrap();
        }
        "python" => {
            fs::write(
                pkg_dir.join("pyproject.toml"),
                r#"[project]
name = "test"
version = "0.1.0"
"#,
            )
            .unwrap();
        }
        "go" => {
            fs::write(
                pkg_dir.join("go.mod"),
                r#"module test
go 1.21
"#,
            )
            .unwrap();
        }
        _ => {}
    }
}

fn get_polykit_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.join("target").join("debug").join("polykit")
}

#[test]
#[ignore]
fn test_scan_command() {
    let temp_dir = TempDir::new().unwrap();
    let packages_dir = temp_dir.path().join("packages");
    fs::create_dir_all(&packages_dir).unwrap();

    create_test_package(&packages_dir, "pkg-a", "rust");
    create_test_package(&packages_dir, "pkg-b", "js");

    let binary = get_polykit_binary();
    let output = Command::new(&binary)
        .arg("scan")
        .arg("--packages-dir")
        .arg(&packages_dir)
        .output()
        .expect("Failed to execute polykit scan");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pkg-a"));
    assert!(stdout.contains("pkg-b"));
}

#[test]
#[ignore]
fn test_graph_command() {
    let temp_dir = TempDir::new().unwrap();
    let packages_dir = temp_dir.path().join("packages");
    fs::create_dir_all(&packages_dir).unwrap();

    create_test_package(&packages_dir, "pkg-a", "rust");
    create_test_package(&packages_dir, "pkg-b", "js");

    let binary = get_polykit_binary();
    let output = Command::new(&binary)
        .arg("graph")
        .arg("--packages-dir")
        .arg(&packages_dir)
        .output()
        .expect("Failed to execute polykit graph");

    assert!(output.status.success());
}
