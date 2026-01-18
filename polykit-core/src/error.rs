//! Error types and result aliases.

use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error in {context}: {error}")]
    Toml {
        error: toml::de::Error,
        context: String,
    },

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("Package not found: {name}. Available packages: {available}")]
    PackageNotFound { name: String, available: String },

    #[error("Invalid package name: {0}")]
    InvalidPackageName(String),

    #[error("Invalid language: {lang}. Supported languages: js, ts, python, go, rust")]
    InvalidLanguage { lang: String },

    #[error("Circular dependency detected: {0}. Use 'polykit graph' to visualize dependencies.")]
    CircularDependency(String),

    #[error("Config file not found: {0}. Expected 'polykit.toml' in package directory.")]
    ConfigNotFound(PathBuf),

    #[error("Adapter error for {package}: {message}")]
    Adapter { package: String, message: String },

    #[error("Graph error: {0}")]
    Graph(String),

    #[error("Task execution failed for {package}::{task}: {message}")]
    TaskExecution {
        package: String,
        task: String,
        message: String,
    },

    #[error("Release error: {0}")]
    Release(String),

    #[error("Mutex lock error: {0}")]
    MutexLock(String),
}

impl From<toml::de::Error> for Error {
    fn from(error: toml::de::Error) -> Self {
        Error::Toml {
            error,
            context: "polykit.toml".to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
