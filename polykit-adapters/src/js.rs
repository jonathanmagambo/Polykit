use std::fs;
use std::path::Path;

use polykit_core::adapter::{LangMetadata, LanguageAdapter};
use polykit_core::error::{Error, Result};
use regex::Regex;
use serde_json::Value;

pub struct JsAdapter;

impl LanguageAdapter for JsAdapter {
    fn language(&self) -> &'static str {
        "js"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("package.json").exists()
    }

    fn read_metadata(&self, path: &Path) -> Result<LangMetadata> {
        let package_json_path = path.join("package.json");
        let package_name =
            path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| Error::Adapter {
                    package: path.display().to_string(),
                    message: format!("Invalid package path: {}", path.display()),
                })?;
        let content = fs::read_to_string(&package_json_path).map_err(|_| Error::Adapter {
            package: package_name.to_string(),
            message: format!("package.json not found in {}", path.display()),
        })?;

        let json: Value = serde_json::from_str(&content).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to parse package.json: {}", e),
        })?;

        let version = json
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(LangMetadata { version })
    }

    fn bump_version(&self, path: &Path, new_version: &str) -> Result<()> {
        let package_json_path = path.join("package.json");
        let package_name =
            path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| Error::Adapter {
                    package: path.display().to_string(),
                    message: format!("Invalid package path: {}", path.display()),
                })?;
        let content = fs::read_to_string(&package_json_path).map_err(|_| Error::Adapter {
            package: package_name.to_string(),
            message: format!("package.json not found in {}", path.display()),
        })?;

        let version_re = Regex::new(r#""version"\s*:\s*"[^"]+""#).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to create regex: {}", e),
        })?;

        let updated = version_re.replace(&content, format!(r#""version": "{}""#, new_version));

        fs::write(&package_json_path, updated.as_ref()).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to write package.json: {}", e),
        })?;

        Ok(())
    }
}
