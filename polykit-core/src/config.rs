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

pub(crate) fn deserialize_tasks<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, TaskValue>, D::Error>
where
    D: Deserializer<'de>,
{
    let map: HashMap<String, toml::Value> = HashMap::deserialize(deserializer)?;
    let mut result = HashMap::new();
    let mut dotted_deps: HashMap<String, Vec<String>> = HashMap::new();

    // First pass: parse regular tasks and collect dotted-key dependencies
    for (key, value) in map {
        // Check if this is a dotted-key dependency (e.g., "test.depends_on")
        if let Some((task_name, dep_key)) = key.split_once('.') {
            if dep_key == "depends_on" {
                if let toml::Value::Array(arr) = value {
                    let deps: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    dotted_deps.insert(task_name.to_string(), deps);
                } else {
                    return Err(serde::de::Error::custom(format!(
                        "Task dependency '{}' must be an array",
                        key
                    )));
                }
                continue;
            }
        }

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

    // Second pass: merge dotted-key dependencies into existing tasks
    for (task_name, deps) in dotted_deps {
        let task_value = result.remove(&task_name).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "Task '{}' referenced in dotted-key dependency does not exist",
                task_name
            ))
        })?;

        match task_value {
            TaskValue::Simple(command) => {
                result.insert(
                    task_name,
                    TaskValue::Complex {
                        command,
                        depends_on: deps,
                    },
                );
            }
            TaskValue::Complex { command, .. } => {
                // Replace dependencies (dotted-key takes precedence)
                result.insert(
                    task_name,
                    TaskValue::Complex {
                        command,
                        depends_on: deps,
                    },
                );
            }
        }
    }

    Ok(result)
}

pub(crate) fn parse_tasks_from_toml_map(
    map: &toml::map::Map<String, toml::Value>,
) -> HashMap<String, TaskValue> {
    let mut result = HashMap::new();
    let mut dotted_deps: HashMap<String, Vec<String>> = HashMap::new();

    for (key, value) in map {
        if let Some((task_name, dep_key)) = key.split_once('.') {
            if dep_key == "depends_on" {
                if let toml::Value::Array(arr) = value {
                    let deps: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    dotted_deps.insert(task_name.to_string(), deps);
                }
                continue;
            }
        }

        match value {
            toml::Value::String(s) => {
                result.insert(key.clone(), TaskValue::Simple(s.clone()));
            }
            toml::Value::Table(t) => {
                if let Some(command_val) = t.get("command").and_then(|v| v.as_str()) {
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
                        key.clone(),
                        TaskValue::Complex {
                            command: command_val.to_string(),
                            depends_on,
                        },
                    );
                }
            }
            _ => {}
        }
    }

    for (task_name, deps) in dotted_deps {
        if let Some(task_value) = result.remove(&task_name) {
            match task_value {
                TaskValue::Simple(command) => {
                    result.insert(
                        task_name,
                        TaskValue::Complex {
                            command,
                            depends_on: deps,
                        },
                    );
                }
                TaskValue::Complex { command, .. } => {
                    result.insert(
                        task_name,
                        TaskValue::Complex {
                            command,
                            depends_on: deps,
                        },
                    );
                }
            }
        }
    }

    result
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
    pub cache_dir: Option<String>,
    /// Default number of parallel jobs.
    pub default_parallel: Option<usize>,
    /// Path to the workspace config file (for resolving relative paths).
    #[serde(skip)]
    pub workspace_config_path: Option<std::path::PathBuf>,
    /// Workspace-level tasks that apply to all packages.
    #[serde(default, deserialize_with = "deserialize_tasks")]
    pub tasks: HashMap<String, TaskValue>,
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

impl WorkspaceConfig {
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
