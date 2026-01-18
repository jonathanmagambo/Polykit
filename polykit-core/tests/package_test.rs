use polykit_core::package::{Language, Package, Task};

#[test]
fn test_package_creation() {
    let package = Package::new(
        "test-pkg".to_string(),
        Language::Rust,
        true,
        "test-pkg".into(),
        vec!["dep1".to_string()],
        vec![Task {
            name: "build".to_string(),
            command: "cargo build".to_string(),
            depends_on: Vec::new(),
        }],
    );

    assert_eq!(package.name, "test-pkg");
    assert!(matches!(package.language, Language::Rust));
    assert!(package.public);
    assert_eq!(package.deps.len(), 1);
    assert_eq!(package.tasks.len(), 1);
}

#[test]
fn test_get_task() {
    let package = Package::new(
        "test-pkg".to_string(),
        Language::Rust,
        true,
        "test-pkg".into(),
        vec![],
        vec![
            Task {
                name: "build".to_string(),
                command: "cargo build".to_string(),
                depends_on: Vec::new(),
            },
            Task {
                name: "test".to_string(),
                command: "cargo test".to_string(),
                depends_on: Vec::new(),
            },
        ],
    );

    let build_task = package.get_task("build");
    assert!(build_task.is_some());
    assert_eq!(build_task.unwrap().command, "cargo build");

    let test_task = package.get_task("test");
    assert!(test_task.is_some());
    assert_eq!(test_task.unwrap().command, "cargo test");

    let missing_task = package.get_task("missing");
    assert!(missing_task.is_none());
}

#[test]
fn test_language_from_str() {
    assert_eq!(Language::from_str("js"), Some(Language::Js));
    assert_eq!(Language::from_str("ts"), Some(Language::Ts));
    assert_eq!(Language::from_str("python"), Some(Language::Python));
    assert_eq!(Language::from_str("go"), Some(Language::Go));
    assert_eq!(Language::from_str("rust"), Some(Language::Rust));
    assert_eq!(Language::from_str("invalid"), None);
}

#[test]
fn test_language_as_str() {
    assert_eq!(Language::Js.as_str(), "js");
    assert_eq!(Language::Ts.as_str(), "ts");
    assert_eq!(Language::Python.as_str(), "python");
    assert_eq!(Language::Go.as_str(), "go");
    assert_eq!(Language::Rust.as_str(), "rust");
}
