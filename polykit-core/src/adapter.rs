//! Language adapter trait for reading and modifying language-specific metadata.

use std::path::Path;

use crate::error::Result;

/// Language-specific metadata extracted from package files.
pub struct LangMetadata {
    /// Package version, if available.
    pub version: Option<String>,
}

/// Trait for language-specific package metadata operations.
///
/// Adapters detect packages, read version information, and bump versions.
/// They do not install dependencies—that's delegated to native tools.
pub trait LanguageAdapter: Send + Sync {
    fn language(&self) -> &'static str;
    fn detect(&self, path: &Path) -> bool;
    fn read_metadata(&self, path: &Path) -> Result<LangMetadata>;
    fn bump_version(&self, path: &Path, new_version: &str) -> Result<()>;
}
