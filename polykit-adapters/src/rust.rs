use std::fs;
use std::path::Path;

use polykit_core::adapter::{LangMetadata, LanguageAdapter};
use polykit_core::error::{Error, Result};
use toml::Value;

pub struct RustAdapter;

impl LanguageAdapter for RustAdapter {
    fn language(&self) -> &'static str {
        "rust"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("Cargo.toml").exists()
    }

    fn read_metadata(&self, path: &Path) -> Result<LangMetadata> {
        let cargo_toml_path = path.join("Cargo.toml");
        let package_name =
            path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| Error::Adapter {
                    package: path.display().to_string(),
                    message: format!("Invalid package path: {}", path.display()),
                })?;
        let content = fs::read_to_string(&cargo_toml_path).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!(
                "Failed to read Cargo.toml at {}: {}",
                cargo_toml_path.display(),
                e
            ),
        })?;

        let toml: Value = content.parse().map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!(
                "Failed to parse Cargo.toml at {}: {}. File may be malformed.",
                cargo_toml_path.display(),
                e
            ),
        })?;

        let version = toml
            .get("package")
            .and_then(|p| p.get("version"))
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

        let cargo_toml_path = path.join("Cargo.toml");
        let package_name =
            path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| Error::Adapter {
                    package: path.display().to_string(),
                    message: format!("Invalid package path: {}", path.display()),
                })?;
        let content = fs::read_to_string(&cargo_toml_path).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!(
                "Failed to read Cargo.toml at {}: {}",
                cargo_toml_path.display(),
                e
            ),
        })?;

        let mut toml: Value = content.parse().map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!(
                "Failed to parse Cargo.toml at {}: {}. File may be malformed.",
                cargo_toml_path.display(),
                e
            ),
        })?;

        // Update version in package.version
        if let Some(package) = toml.get_mut("package").and_then(|p| p.as_table_mut()) {
            package.insert(
                "version".to_string(),
                Value::String(new_version.to_string()),
            );
        } else {
            return Err(Error::Adapter {
                package: package_name.to_string(),
                message: format!(
                    "Could not find 'package.version' in Cargo.toml at {}. \
                    Ensure the file contains a [package] section with a version field.",
                    cargo_toml_path.display()
                ),
            });
        }

        let updated_content = toml::to_string_pretty(&toml).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to serialize Cargo.toml: {}", e),
        })?;

        fs::write(&cargo_toml_path, updated_content).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to write Cargo.toml: {}", e),
        })?;

        Ok(())
    }
}
