use std::fs;
use std::path::Path;

use polykit_core::adapter::{LangMetadata, LanguageAdapter};
use polykit_core::error::{Error, Result};
use regex::Regex;

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
        let package_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let content = fs::read_to_string(&cargo_toml_path).map_err(|_| Error::Adapter {
            package: package_name.clone(),
            message: format!("Cargo.toml not found in {}", path.display()),
        })?;

        let toml: toml::Value = content.parse().map_err(|e| Error::Adapter {
            package: package_name.clone(),
            message: format!("Failed to parse Cargo.toml: {}", e),
        })?;

        let version = toml
            .get("package")
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(LangMetadata { version })
    }

    fn bump_version(&self, path: &Path, new_version: &str) -> Result<()> {
        let cargo_toml_path = path.join("Cargo.toml");
        let package_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let content = fs::read_to_string(&cargo_toml_path).map_err(|_| Error::Adapter {
            package: package_name.clone(),
            message: format!("Cargo.toml not found in {}", path.display()),
        })?;

        let version_re =
            Regex::new(r#"(?m)^version\s*=\s*"[^"]+""#).map_err(|e| Error::Adapter {
                package: package_name.clone(),
                message: format!("Failed to create regex: {}", e),
            })?;

        let updated = version_re.replace(&content, format!(r#"version = "{}""#, new_version));

        fs::write(&cargo_toml_path, updated.as_ref()).map_err(|e| Error::Adapter {
            package: package_name,
            message: format!("Failed to write Cargo.toml: {}", e),
        })?;

        Ok(())
    }
}
