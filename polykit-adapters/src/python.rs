//! Python language adapter for reading and modifying Python package metadata.
//!
//! This adapter reads version information from pyproject.toml files, supporting
//! both PEP 621 format (project.version) and Poetry format (tool.poetry.version).

use std::fs;
use std::path::Path;

use polykit_core::adapter::{LangMetadata, LanguageAdapter};
use polykit_core::error::{Error, Result};
use semver::Version;
use toml::Value;

/// Adapter for Python packages using pyproject.toml.
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
        let content = fs::read_to_string(&pyproject_path).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!(
                "Failed to read pyproject.toml at {}: {}",
                pyproject_path.display(),
                e
            ),
        })?;

        let toml: Value = content.parse().map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!(
                "Failed to parse pyproject.toml at {}: {}. File may be malformed.",
                pyproject_path.display(),
                e
            ),
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
        Version::parse(new_version).map_err(|e| Error::Adapter {
            package: path.display().to_string(),
            message: format!(
                "Invalid version format '{}': {}. Expected semver format (e.g., 1.2.3)",
                new_version, e
            ),
        })?;

        let pyproject_path = path.join("pyproject.toml");
        let package_name =
            path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| Error::Adapter {
                    package: path.display().to_string(),
                    message: format!("Invalid package path: {}", path.display()),
                })?;
        let content = fs::read_to_string(&pyproject_path).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!(
                "Failed to read pyproject.toml at {}: {}",
                pyproject_path.display(),
                e
            ),
        })?;

        let mut toml: Value = content.parse().map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!(
                "Failed to parse pyproject.toml at {}: {}. File may be malformed.",
                pyproject_path.display(),
                e
            ),
        })?;

        let updated = if let Some(project) = toml.get_mut("project").and_then(|p| p.as_table_mut())
        {
            project.insert(
                "version".to_string(),
                Value::String(new_version.to_string()),
            );
            true
        } else if let Some(tool) = toml.get_mut("tool").and_then(|t| t.as_table_mut()) {
            if let Some(poetry) = tool.get_mut("poetry").and_then(|p| p.as_table_mut()) {
                poetry.insert(
                    "version".to_string(),
                    Value::String(new_version.to_string()),
                );
                true
            } else {
                false
            }
        } else {
            false
        };

        if !updated {
            return Err(Error::Adapter {
                package: package_name.to_string(),
                message: format!(
                    "Could not find 'project.version' or 'tool.poetry.version' in pyproject.toml at {}. \
                    Ensure the file contains one of these sections.",
                    pyproject_path.display()
                ),
            });
        }

        let updated_content = toml::to_string_pretty(&toml).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to serialize pyproject.toml: {}", e),
        })?;

        fs::write(&pyproject_path, updated_content).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to write pyproject.toml: {}", e),
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_python_package() {
        let temp_dir = TempDir::new().unwrap();
        let package_dir = temp_dir.path().join("test-python");
        fs::create_dir_all(&package_dir).unwrap();

        let adapter = PythonAdapter;
        assert!(!adapter.detect(&package_dir));

        fs::write(package_dir.join("pyproject.toml"), "[project]\nname = \"test\"").unwrap();
        assert!(adapter.detect(&package_dir));
    }

    #[test]
    fn test_read_metadata_pep621() {
        let temp_dir = TempDir::new().unwrap();
        let package_dir = temp_dir.path().join("test-python");
        fs::create_dir_all(&package_dir).unwrap();

        let content = r#"
[project]
name = "test-package"
version = "1.2.3"
"#;
        fs::write(package_dir.join("pyproject.toml"), content).unwrap();

        let adapter = PythonAdapter;
        let metadata = adapter.read_metadata(&package_dir).unwrap();
        assert_eq!(metadata.version, Some("1.2.3".to_string()));
    }

    #[test]
    fn test_read_metadata_poetry() {
        let temp_dir = TempDir::new().unwrap();
        let package_dir = temp_dir.path().join("test-python");
        fs::create_dir_all(&package_dir).unwrap();

        let content = r#"
[tool.poetry]
name = "test-package"
version = "2.3.4"
"#;
        fs::write(package_dir.join("pyproject.toml"), content).unwrap();

        let adapter = PythonAdapter;
        let metadata = adapter.read_metadata(&package_dir).unwrap();
        assert_eq!(metadata.version, Some("2.3.4".to_string()));
    }

    #[test]
    fn test_read_metadata_no_version() {
        let temp_dir = TempDir::new().unwrap();
        let package_dir = temp_dir.path().join("test-python");
        fs::create_dir_all(&package_dir).unwrap();

        let content = r#"
[project]
name = "test-package"
"#;
        fs::write(package_dir.join("pyproject.toml"), content).unwrap();

        let adapter = PythonAdapter;
        let metadata = adapter.read_metadata(&package_dir).unwrap();
        assert_eq!(metadata.version, None);
    }

    #[test]
    fn test_bump_version_pep621() {
        let temp_dir = TempDir::new().unwrap();
        let package_dir = temp_dir.path().join("test-python");
        fs::create_dir_all(&package_dir).unwrap();

        let content = r#"
[project]
name = "test-package"
version = "1.0.0"
"#;
        fs::write(package_dir.join("pyproject.toml"), content).unwrap();

        let adapter = PythonAdapter;
        adapter.bump_version(&package_dir, "1.2.3").unwrap();

        let updated = fs::read_to_string(package_dir.join("pyproject.toml")).unwrap();
        assert!(updated.contains("version = \"1.2.3\""));
        assert!(!updated.contains("version = \"1.0.0\""));
    }

    #[test]
    fn test_bump_version_poetry() {
        let temp_dir = TempDir::new().unwrap();
        let package_dir = temp_dir.path().join("test-python");
        fs::create_dir_all(&package_dir).unwrap();

        let content = r#"
[tool.poetry]
name = "test-package"
version = "1.0.0"
"#;
        fs::write(package_dir.join("pyproject.toml"), content).unwrap();

        let adapter = PythonAdapter;
        adapter.bump_version(&package_dir, "2.0.0").unwrap();

        let updated = fs::read_to_string(package_dir.join("pyproject.toml")).unwrap();
        assert!(updated.contains("version = \"2.0.0\""));
        assert!(!updated.contains("version = \"1.0.0\""));
    }

    #[test]
    fn test_bump_version_invalid_format() {
        let temp_dir = TempDir::new().unwrap();
        let package_dir = temp_dir.path().join("test-python");
        fs::create_dir_all(&package_dir).unwrap();

        let content = r#"
[project]
name = "test-package"
version = "1.0.0"
"#;
        fs::write(package_dir.join("pyproject.toml"), content).unwrap();

        let adapter = PythonAdapter;
        let result = adapter.bump_version(&package_dir, "invalid-version");
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = format!("{}", e);
            assert!(err_msg.contains("Invalid version format"));
        }
    }

    #[test]
    fn test_bump_version_no_version_section() {
        let temp_dir = TempDir::new().unwrap();
        let package_dir = temp_dir.path().join("test-python");
        fs::create_dir_all(&package_dir).unwrap();

        let content = r#"
[build-system]
requires = ["setuptools"]
"#;
        fs::write(package_dir.join("pyproject.toml"), content).unwrap();

        let adapter = PythonAdapter;
        let result = adapter.bump_version(&package_dir, "1.0.0");
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = format!("{}", e);
            assert!(err_msg.contains("Could not find 'project.version'"));
        }
    }
}
