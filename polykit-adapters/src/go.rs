//! Go language adapter for reading and modifying Go package metadata.
//!
//! This adapter uses Git tags for version management, as Go modules do not store
//! version information in go.mod files. Versions are read from Git tags matching
//! the semver pattern (v*.*.*) and new versions are created as Git tags.

use std::path::Path;

use git2::Repository;
use polykit_core::adapter::{LangMetadata, LanguageAdapter};
use polykit_core::error::{Error, Result};
use semver::Version;

/// Adapter for Go packages using Git tag-based versioning.
pub struct GoAdapter;

impl LanguageAdapter for GoAdapter {
    fn language(&self) -> &'static str {
        "go"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("go.mod").exists()
    }

    fn read_metadata(&self, path: &Path) -> Result<LangMetadata> {
        let package_name =
            path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| Error::Adapter {
                    package: path.display().to_string(),
                    message: format!("Invalid package path: {}", path.display()),
                })?;

        let repo = find_repo(path).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to find Git repository: {}", e),
        })?;

        let version = find_latest_version_tag(&repo, path).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to read Git tags: {}", e),
        })?;

        Ok(LangMetadata { version })
    }

    fn bump_version(&self, path: &Path, new_version: &str) -> Result<()> {
        let package_name =
            path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| Error::Adapter {
                    package: path.display().to_string(),
                    message: format!("Invalid package path: {}", path.display()),
                })?;

        Version::parse(new_version).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Invalid version format '{}': {}", new_version, e),
        })?;

        let repo = find_repo(path).map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to find Git repository: {}", e),
        })?;

        let head = repo.head().map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to get HEAD: {}", e),
        })?;
        let commit = head.peel_to_commit().map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to get commit: {}", e),
        })?;

        let tag_name = format!("v{}", new_version);
        repo.tag(
            &tag_name,
            commit.as_object(),
            &repo.signature().map_err(|e| Error::Adapter {
                package: package_name.to_string(),
                message: format!("Failed to get signature: {}", e),
            })?,
            &format!("Release version {}", new_version),
            false,
        )
        .map_err(|e| Error::Adapter {
            package: package_name.to_string(),
            message: format!("Failed to create Git tag '{}': {}", tag_name, e),
        })?;

        Ok(())
    }
}

/// Find Git repository by walking up from the given path.
fn find_repo(path: &Path) -> std::result::Result<Repository, git2::Error> {
    let mut current = path;
    loop {
        match Repository::open(current) {
            Ok(repo) => return Ok(repo),
            Err(_) => {
                current = match current.parent() {
                    Some(parent) => parent,
                    None => {
                        return Err(git2::Error::from_str("No Git repository found"));
                    }
                };
            }
        }
    }
}

/// Find the latest version tag matching semver pattern (v*.*.*).
///
/// Optimized to track the maximum version during iteration rather than
/// collecting all tags and sorting.
fn find_latest_version_tag(repo: &Repository, package_path: &Path) -> Result<Option<String>> {
    let mut latest_version: Option<Version> = None;

    repo.tag_foreach(|_oid, name| {
        if let Ok(name_str) = std::str::from_utf8(name) {
            let tag_name = match name_str.strip_prefix("refs/tags/") {
                Some(stripped) => stripped,
                None => name_str,
            };

            if let Some(version_str) = tag_name.strip_prefix('v') {
                if let Ok(version) = Version::parse(version_str) {
                    match latest_version {
                        None => latest_version = Some(version),
                        Some(ref current) if version > *current => {
                            latest_version = Some(version);
                        }
                        _ => {}
                    }
                }
            }
        }
        true
    })
    .map_err(|e| Error::Adapter {
        package: package_path.display().to_string(),
        message: format!("Failed to iterate tags: {}", e),
    })?;

    Ok(latest_version.map(|v| v.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_go_package() {
        let temp_dir = TempDir::new().unwrap();
        let package_dir = temp_dir.path().join("test-go");
        fs::create_dir_all(&package_dir).unwrap();

        let adapter = GoAdapter;
        assert!(!adapter.detect(&package_dir));

        fs::write(package_dir.join("go.mod"), "module test").unwrap();
        assert!(adapter.detect(&package_dir));
    }

    #[test]
    fn test_read_metadata_no_repo() {
        let temp_dir = TempDir::new().unwrap();
        let package_dir = temp_dir.path().join("test-go");
        fs::create_dir_all(&package_dir).unwrap();
        fs::write(package_dir.join("go.mod"), "module test").unwrap();

        let adapter = GoAdapter;
        let result = adapter.read_metadata(&package_dir);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = format!("{}", e);
            assert!(err_msg.contains("Failed to find Git repository"));
        }
    }

    #[test]
    fn test_read_metadata_with_tags() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(&temp_dir).unwrap();
        let package_dir = temp_dir.path();
        fs::write(package_dir.join("go.mod"), "module test").unwrap();

        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let mut index = repo.index().unwrap();
        index.add_all(["*"], git2::IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let commit = repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();
        let head = repo.find_commit(commit).unwrap();

        repo.tag("v1.0.0", head.as_object(), &sig, "v1.0.0", false)
            .unwrap();
        repo.tag("v1.2.3", head.as_object(), &sig, "v1.2.3", false)
            .unwrap();
        repo.tag("v0.9.0", head.as_object(), &sig, "v0.9.0", false)
            .unwrap();

        let adapter = GoAdapter;
        let metadata = adapter.read_metadata(package_dir).unwrap();
        assert_eq!(metadata.version, Some("1.2.3".to_string()));
    }

    #[test]
    fn test_bump_version_creates_tag() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(&temp_dir).unwrap();
        let package_dir = temp_dir.path();
        fs::write(package_dir.join("go.mod"), "module test").unwrap();

        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let mut index = repo.index().unwrap();
        index.add_all(["*"], git2::IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let commit = repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();
        let head = repo.find_commit(commit).unwrap();
        repo.tag("v1.0.0", head.as_object(), &sig, "v1.0.0", false)
            .unwrap();

        let adapter = GoAdapter;
        adapter.bump_version(package_dir, "1.1.0").unwrap();

        let tags: Vec<String> = repo
            .tag_names(None)
            .unwrap()
            .iter()
            .filter_map(|n| n.map(|s| s.to_string()))
            .collect();
        assert!(tags.contains(&"v1.1.0".to_string()));
    }

    #[test]
    fn test_bump_version_invalid_format() {
        let temp_dir = TempDir::new().unwrap();
        let _repo = Repository::init(&temp_dir).unwrap();
        let package_dir = temp_dir.path();
        fs::write(package_dir.join("go.mod"), "module test").unwrap();

        let adapter = GoAdapter;
        let result = adapter.bump_version(package_dir, "invalid");
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = format!("{}", e);
            assert!(err_msg.contains("Invalid version format"));
        }
    }
}
