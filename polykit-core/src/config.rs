//! TOML configuration parsing for package definitions.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::package::{Language, Task};

/// Package configuration as defined in `polykit.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub language: String,
    pub public: bool,
    #[serde(default)]
    pub deps: Deps,
    #[serde(default)]
    pub tasks: HashMap<String, String>,
}

/// Package dependencies configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Deps {
    /// List of internal package dependencies.
    #[serde(default)]
    pub internal: Vec<String>,
}

impl Config {
    pub fn parse_language(&self) -> Result<Language, crate::Error> {
        Language::from_str(&self.language).ok_or_else(|| crate::Error::InvalidLanguage {
            lang: self.language.clone(),
        })
    }

    pub fn to_tasks(&self) -> Vec<Task> {
        self.tasks
            .iter()
            .map(|(name, command)| Task {
                name: name.clone(),
                command: command.clone(),
            })
            .collect()
    }
}
