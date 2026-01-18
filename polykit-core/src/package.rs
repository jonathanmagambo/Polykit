//! Package data models and language definitions.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Supported programming languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Js,
    Ts,
    Python,
    Go,
    Rust,
}

impl Language {
    #[inline]
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Js => "js",
            Language::Ts => "ts",
            Language::Python => "python",
            Language::Go => "go",
            Language::Rust => "rust",
        }
    }

    /// Parses a language string into a `Language` variant.
    ///
    /// Supports aliases (e.g., "javascript" for "js", "typescript" for "ts").
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "js" | "javascript" => Some(Language::Js),
            "ts" | "typescript" => Some(Language::Ts),
            "python" => Some(Language::Python),
            "go" => Some(Language::Go),
            "rust" => Some(Language::Rust),
            _ => None,
        }
    }
}

/// A task that can be executed for a package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

/// Represents a package in the monorepo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub language: Language,
    pub public: bool,
    pub path: PathBuf,
    #[serde(
        deserialize_with = "deserialize_deps",
        serialize_with = "serialize_deps"
    )]
    pub deps: SmallVec<[String; 4]>,
    pub tasks: Vec<Task>,
    pub version: Option<String>,
}

fn deserialize_deps<'de, D>(deserializer: D) -> Result<SmallVec<[String; 4]>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let vec: Vec<String> = Vec::deserialize(deserializer)?;
    Ok(SmallVec::from_vec(vec))
}

fn serialize_deps<S>(deps: &SmallVec<[String; 4]>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    let vec: Vec<&String> = deps.iter().collect();
    vec.serialize(serializer)
}

impl Package {
    pub fn new(
        name: String,
        language: Language,
        public: bool,
        path: PathBuf,
        deps: Vec<String>,
        tasks: Vec<Task>,
    ) -> Self {
        Self {
            name,
            language,
            public,
            path,
            deps: SmallVec::from_vec(deps),
            tasks,
            version: None,
        }
    }

    #[inline]
    pub fn get_task(&self, name: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.name == name)
    }
}
