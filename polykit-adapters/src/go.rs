use std::fs;
use std::path::Path;

use polykit_core::adapter::{LangMetadata, LanguageAdapter};
use polykit_core::error::{Error, Result};
use regex::Regex;

pub struct GoAdapter;

impl LanguageAdapter for GoAdapter {
    fn language(&self) -> &'static str {
        "go"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("go.mod").exists()
    }

    fn read_metadata(&self, path: &Path) -> Result<LangMetadata> {
        let go_mod_path = path.join("go.mod");
        let package_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let _content = fs::read_to_string(&go_mod_path).map_err(|_| Error::Adapter {
            package: package_name.clone(),
            message: format!("go.mod not found in {}", path.display()),
        })?;

        let _version_re =
            Regex::new(r#"(?m)^module\s+[^\s]+(?:\s+//\s+indirect)?$"#).map_err(|e| {
                Error::Adapter {
                    package: package_name,
                    message: format!("Failed to create regex: {}", e),
                }
            })?;

        Ok(LangMetadata { version: None })
    }

    fn bump_version(&self, _path: &Path, _new_version: &str) -> Result<()> {
        Ok(())
    }
}
