//! Command validation for security.

use crate::error::{Error, Result};

/// Validates shell commands before execution to prevent injection attacks.
///
/// This validator checks for dangerous patterns that could allow command
/// injection or arbitrary code execution.
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
                message: format!(
                    "Command contains dangerous shell features: {}. Use strict mode to disable shell features.",
                    command
                ),
            });
        }

        if command.trim().is_empty() {
            return Err(Error::TaskExecution {
                package: "unknown".to_string(),
                task: "unknown".to_string(),
                message: "Command cannot be empty".to_string(),
            });
        }

        Ok(())
    }
}
