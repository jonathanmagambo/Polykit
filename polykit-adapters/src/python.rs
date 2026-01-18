use std::fs;
use std::path::Path;

use polykit_core::adapter::{LangMetadata, LanguageAdapter};
use polykit_core::error::{Error, Result};
use regex::Regex;
use toml::Value;

pub struct PythonAdapter;

impl LanguageAdapter for PythonAdapter {
    fn language(&self) -> &'static str {
        "python"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("pyproject.toml").exists()
    }

    fn read_metadata(&self, path: &Path) -> Result<LangMetadata> {
        let pyproject_path = path.join("pyproject.toml");
        let package_name =
            path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| Error::Adapter {
                    package: path.display().to_string(),
                    message: format!("Invalid package path: {}", path.display()),
                })?;
        let content = fs::read_to_string(&pyproject_path).map_err(|_| Error::Adapter {
            package: package_name.to_string(),
            message: format!("pyproject.toml not found in {}", path.display()),
        })?;

        let toml: Value = content.parse().map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to parse pyproject.toml: {}", e),
        })?;

        let version = toml
            .get("project")
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                toml.get("tool")
                    .and_then(|t| t.get("poetry"))
                    .and_then(|p| p.get("version"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            });

        Ok(LangMetadata { version })
    }

    fn bump_version(&self, path: &Path, new_version: &str) -> Result<()> {
        let pyproject_path = path.join("pyproject.toml");
        let package_name =
            path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| Error::Adapter {
                    package: path.display().to_string(),
                    message: format!("Invalid package path: {}", path.display()),
                })?;
        let content = fs::read_to_string(&pyproject_path).map_err(|_| Error::Adapter {
            package: package_name.to_string(),
            message: format!("pyproject.toml not found in {}", path.display()),
        })?;

        let version_re = Regex::new(r#"version\s*=\s*"[^"]+""#).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to create regex: {}", e),
        })?;

        let updated = version_re.replace(&content, format!(r#"version = "{}""#, new_version));

        fs::write(&pyproject_path, updated.as_ref()).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to write pyproject.toml: {}", e),
        })?;

        Ok(())
    }
}
