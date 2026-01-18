use std::fs;
use std::path::Path;

use polykit_core::adapter::{LangMetadata, LanguageAdapter};
use polykit_core::error::{Error, Result};
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
        let content = fs::read_to_string(&package_json_path).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!(
                "Failed to read package.json at {}: {}",
                package_json_path.display(),
                e
            ),
        })?;

        let json: Value = serde_json::from_str(&content).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!(
                "Failed to parse package.json at {}: {}. File may be malformed JSON.",
                package_json_path.display(),
                e
            ),
        })?;

        let version = json
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(LangMetadata { version })
    }

    fn bump_version(&self, path: &Path, new_version: &str) -> Result<()> {
        // Validate version format
        semver::Version::parse(new_version).map_err(|e| Error::Adapter {
            package: path.display().to_string(),
            message: format!(
                "Invalid version format '{}': {}. Expected semver format (e.g., 1.2.3)",
                new_version, e
            ),
        })?;

        let package_json_path = path.join("package.json");
        let package_name =
            path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| Error::Adapter {
                    package: path.display().to_string(),
                    message: format!("Invalid package path: {}", path.display()),
                })?;
        let content = fs::read_to_string(&package_json_path).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!(
                "Failed to read package.json at {}: {}",
                package_json_path.display(),
                e
            ),
        })?;

        let mut json: Value = serde_json::from_str(&content).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!(
                "Failed to parse package.json at {}: {}. File may be malformed JSON.",
                package_json_path.display(),
                e
            ),
        })?;

        // Update version field
        if let Some(obj) = json.as_object_mut() {
            obj.insert(
                "version".to_string(),
                Value::String(new_version.to_string()),
            );
        } else {
            return Err(Error::Adapter {
                package: package_name.to_string(),
                message: format!(
                    "package.json at {} root is not an object. Expected JSON object.",
                    package_json_path.display()
                ),
            });
        }

        let updated_content = serde_json::to_string_pretty(&json).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to serialize package.json: {}", e),
        })?;

        fs::write(&package_json_path, updated_content).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to write package.json: {}", e),
        })?;

        Ok(())
    }
}
