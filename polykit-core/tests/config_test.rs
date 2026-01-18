use polykit_core::config::Config;

#[test]
fn test_parse_config() {
    let toml = r#"
name = "test-package"
language = "rust"
public = true

[deps]
internal = ["dep1", "dep2"]

[tasks]
build = "cargo build"
test = "cargo test"
"#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.name, "test-package");
    assert_eq!(config.language, "rust");
    assert!(config.public);
    assert_eq!(config.deps.internal.len(), 2);
    assert_eq!(config.tasks.len(), 2);
}

#[test]
fn test_parse_config_defaults() {
    let toml = r#"
name = "test-package"
language = "js"
public = false
"#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.deps.internal.len(), 0);
    assert_eq!(config.tasks.len(), 0);
}

#[test]
fn test_parse_language() {
    let config = Config {
        name: "test".to_string(),
        language: "rust".to_string(),
        public: true,
        deps: Default::default(),
        tasks: Default::default(),
    };

    let language = config.parse_language().unwrap();
    assert!(matches!(language, polykit_core::package::Language::Rust));
}

#[test]
fn test_invalid_language() {
    let config = Config {
        name: "test".to_string(),
        language: "invalid".to_string(),
        public: true,
        deps: Default::default(),
        tasks: Default::default(),
    };

    assert!(config.parse_language().is_err());
}
