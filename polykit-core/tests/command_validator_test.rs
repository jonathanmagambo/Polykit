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
