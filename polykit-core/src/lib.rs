//! Core library for monorepo orchestration.

pub mod adapter;
pub mod cache;
pub mod change;
pub mod config;
pub mod error;
pub mod graph;
pub mod package;
pub mod release;
pub mod runner;
pub mod scanner;
pub mod streaming;
pub mod watcher;

pub use adapter::{LangMetadata, LanguageAdapter};
pub use cache::{Cache, CacheStats};
pub use change::ChangeDetector;
pub use config::{Config, WorkspaceConfig};
pub use error::{Error, Result};
pub use graph::{DependencyGraph, GraphNode};
pub use package::{Language, Package, Task};
pub use release::{BumpType, ReleaseEngine, ReleasePackage, ReleasePlan};
pub use runner::{TaskResult, TaskRunner};
pub use scanner::Scanner;
pub use streaming::StreamingTask;
pub use watcher::{FileWatcher, WatcherConfig};
