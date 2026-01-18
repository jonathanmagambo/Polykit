//! TOML configuration parsing for package definitions.

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::package::{Language, Task};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskValue {
    Simple(String),
    Complex {
        command: String,
        #[serde(default)]
        depends_on: Vec<String>,
    },
}

/// Package configuration as defined in `polykit.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub language: String,
    pub public: bool,
    #[serde(default)]
    pub deps: Deps,
    #[serde(deserialize_with = "deserialize_tasks")]
    #[serde(default)]
    pub tasks: HashMap<String, TaskValue>,
}

fn deserialize_tasks<'de, D>(deserializer: D) -> Result<HashMap<String, TaskValue>, D::Error>
where
    D: Deserializer<'de>,
{
    let map: HashMap<String, toml::Value> = HashMap::deserialize(deserializer)?;
    let mut result = HashMap::new();

    for (key, value) in map {
        match value {
            toml::Value::String(s) => {
                result.insert(key, TaskValue::Simple(s));
            }
            toml::Value::Table(t) => {
                let command = t
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        serde::de::Error::custom("Task table must have 'command' field")
                    })?
                    .to_string();
                let depends_on = t
                    .get("depends_on")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                result.insert(
                    key,
                    TaskValue::Complex {
                        command,
                        depends_on,
                    },
                );
            }
            _ => {
                return Err(serde::de::Error::custom(
                    "Task value must be a string or a table",
                ));
            }
        }
    }

    Ok(result)
}

/// Package dependencies configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Deps {
    /// List of internal package dependencies.
    #[serde(default)]
    pub internal: Vec<String>,
}

/// Workspace-level configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Cache directory path.
    #[serde(default)]
    pub cache_dir: Option<String>,
    /// Default number of parallel jobs.
    #[serde(default)]
    pub default_parallel: Option<usize>,
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
            .map(|(name, task_value)| match task_value {
                TaskValue::Simple(command) => Task {
                    name: name.clone(),
                    command: command.clone(),
                    depends_on: Vec::new(),
                },
                TaskValue::Complex {
                    command,
                    depends_on,
                } => Task {
                    name: name.clone(),
                    command: command.clone(),
                    depends_on: depends_on.clone(),
                },
            })
            .collect()
    }
}
