use polykit_core::command_validator::CommandValidator;

#[test]
fn test_command_validator_allows_valid_commands() {
    let validator = CommandValidator::new();
    assert!(validator.validate("cargo build").is_ok());
    assert!(validator.validate("npm test").is_ok());
}

#[test]
fn test_command_validator_rejects_empty() {
    let validator = CommandValidator::new();
    assert!(validator.validate("").is_err());
    assert!(validator.validate("   ").is_err());
}

#[test]
fn test_command_validator_strict_mode() {
    let validator = CommandValidator::strict();
    assert!(validator.validate("cargo build").is_ok());
    assert!(validator.validate("echo test; rm -rf /").is_err());
    assert!(validator.validate("test && other").is_err());
    assert!(validator.validate("test || other").is_err());
}

#[test]
fn test_command_validator_rejects_null_bytes() {
    let validator = CommandValidator::new();
    assert!(validator.validate("cargo build\0").is_err());
    assert!(validator.validate("npm\0test").is_err());
}

#[test]
fn test_command_validator_rejects_embedded_newlines() {
    let validator = CommandValidator::new();
    assert!(validator.validate("cargo build\nrm -rf /").is_err());
    assert!(validator.validate("npm test\r\nmalicious").is_err());
}

#[test]
fn test_command_validator_rejects_excessive_length() {
    let validator = CommandValidator::new();
    let long_command = "a".repeat(10_001);
    assert!(validator.validate(&long_command).is_err());
}

#[test]
fn test_validate_identifier_valid() {
    assert!(CommandValidator::validate_identifier("my-package", "Package").is_ok());
    assert!(CommandValidator::validate_identifier("my_package", "Package").is_ok());
    assert!(CommandValidator::validate_identifier("my.package", "Package").is_ok());
    assert!(CommandValidator::validate_identifier("my-package-123", "Package").is_ok());
    assert!(CommandValidator::validate_identifier("package@1.0.0", "Package").is_ok());
}

#[test]
fn test_validate_identifier_rejects_empty() {
    assert!(CommandValidator::validate_identifier("", "Package").is_err());
}

#[test]
fn test_validate_identifier_rejects_path_traversal() {
    assert!(CommandValidator::validate_identifier("../etc/passwd", "Package").is_err());
    assert!(CommandValidator::validate_identifier("package/../other", "Package").is_err());
    assert!(CommandValidator::validate_identifier("..", "Package").is_err());
}

#[test]
fn test_validate_identifier_rejects_path_separators() {
    assert!(CommandValidator::validate_identifier("package/name", "Package").is_err());
    assert!(CommandValidator::validate_identifier("package\\name", "Package").is_err());
}

#[test]
fn test_validate_identifier_rejects_leading_dot_or_dash() {
    assert!(CommandValidator::validate_identifier(".hidden", "Package").is_err());
    assert!(CommandValidator::validate_identifier("-package", "Package").is_err());
}

#[test]
fn test_validate_identifier_rejects_excessive_length() {
    let long_name = "a".repeat(256);
    assert!(CommandValidator::validate_identifier(&long_name, "Package").is_err());
}

#[test]
fn test_validate_identifier_rejects_invalid_chars() {
    assert!(CommandValidator::validate_identifier("package$name", "Package").is_err());
    assert!(CommandValidator::validate_identifier("package;name", "Package").is_err());
    assert!(CommandValidator::validate_identifier("package\0name", "Package").is_err());
}
