//! Command validation for security.

use crate::error::{Error, Result};

/// Validates shell commands before execution to prevent injection attacks.
///
/// This validator checks for dangerous patterns that could allow command
/// injection or arbitrary code execution.
#[derive(Clone)]
pub struct CommandValidator {
    allow_shell: bool,
}

impl Default for CommandValidator {
    fn default() -> Self {
        Self { allow_shell: true }
    }
}

impl CommandValidator {
    /// Creates a new command validator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a validator that disallows shell features.
    pub fn strict() -> Self {
        Self { allow_shell: false }
    }

    /// Validates a command string before execution.
    ///
    /// # Errors
    ///
    /// Returns an error if the command contains dangerous patterns.
    pub fn validate(&self, command: &str) -> Result<()> {
        if command.trim().is_empty() {
            return Err(Error::TaskExecution {
                package: "unknown".to_string(),
                task: "unknown".to_string(),
                message: "Command cannot be empty".to_string(),
            });
        }

        if command.len() > 10_000 {
            return Err(Error::TaskExecution {
                package: "unknown".to_string(),
                task: "unknown".to_string(),
                message: "Command exceeds maximum length of 10,000 characters".to_string(),
            });
        }

        if command.contains('\0') {
            return Err(Error::TaskExecution {
                package: "unknown".to_string(),
                task: "unknown".to_string(),
                message: "Command contains null bytes".to_string(),
            });
        }

        if command.contains("\r\n") || command.contains('\n') {
            let trimmed = command.trim();
            if trimmed.contains('\n') {
                return Err(Error::TaskExecution {
                    package: "unknown".to_string(),
                    task: "unknown".to_string(),
                    message: "Command contains embedded newlines".to_string(),
                });
            }
        }

        if !self.allow_shell
            && (command.contains(';')
                || command.contains("&&")
                || command.contains("||")
                || command.contains('|')
                || command.contains('`')
                || command.contains('$'))
        {
            return Err(Error::TaskExecution {
                package: "unknown".to_string(),
                task: "unknown".to_string(),
                message: "Command contains shell features that are not allowed in strict mode"
                    .to_string(),
            });
        }

        Ok(())
    }

    /// Validates an identifier such as a package name or task name.
    ///
    /// Identifiers must be alphanumeric with hyphens, underscores, or dots only.
    pub fn validate_identifier(identifier: &str, context: &str) -> Result<()> {
        if identifier.is_empty() {
            return Err(Error::InvalidPackageName(format!(
                "{} cannot be empty",
                context
            )));
        }

        if identifier.len() > 255 {
            return Err(Error::InvalidPackageName(format!(
                "{} exceeds maximum length of 255 characters",
                context
            )));
        }

        if identifier.starts_with('.') || identifier.starts_with('-') {
            return Err(Error::InvalidPackageName(format!(
                "{} cannot start with '.' or '-'",
                context
            )));
        }

        if identifier.contains("..") {
            return Err(Error::InvalidPackageName(format!(
                "{} cannot contain '..'",
                context
            )));
        }

        if identifier.contains('/') || identifier.contains('\\') {
            return Err(Error::InvalidPackageName(format!(
                "{} cannot contain path separators",
                context
            )));
        }

        for ch in identifier.chars() {
            if !ch.is_alphanumeric() && ch != '-' && ch != '_' && ch != '.' && ch != '@' {
                return Err(Error::InvalidPackageName(format!(
                    "{} contains invalid character: '{}'",
                    context, ch
                )));
            }
        }

        Ok(())
    }
}
