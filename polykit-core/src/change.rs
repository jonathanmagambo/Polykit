//! Change detection for determining affected packages.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use git2::{DiffOptions, Repository};

use crate::error::{Error, Result};
use crate::graph::DependencyGraph;
use crate::path_utils;

/// Detects packages affected by file changes.
pub struct ChangeDetector;

impl ChangeDetector {
    /// Determines which packages are affected by the given changed files.
    pub fn detect_affected_packages(
        graph: &DependencyGraph,
        changed_files: &[impl AsRef<Path>],
        packages_dir: impl AsRef<Path>,
    ) -> Result<HashSet<String>> {
        let packages_dir = packages_dir.as_ref();
        let mut changed_packages = HashSet::new();

        for file_path in changed_files {
            let path = file_path.as_ref();
            if let Some(package_name) = Self::file_to_package(path, packages_dir) {
                changed_packages.insert(package_name);
            }
        }

        graph.affected_packages(&changed_packages.into_iter().collect::<Vec<_>>())
    }

    pub fn detect_from_git(
        graph: &DependencyGraph,
        packages_dir: impl AsRef<Path>,
        base: Option<&str>,
    ) -> Result<HashSet<String>> {
        let base = base.unwrap_or("HEAD");
        Self::validate_git_ref(base)?;
        let changed_files = Self::git_diff_with_libgit2(base)?;
        Self::detect_affected_packages(graph, &changed_files, packages_dir)
    }

    /// Reads changed files from stdin (one path per line).
    pub fn detect_from_stdin(
        graph: &DependencyGraph,
        packages_dir: impl AsRef<Path>,
    ) -> Result<HashSet<String>> {
        use std::io::{self, BufRead};

        let stdin = io::stdin();
        let mut changed_files = Vec::new();

        for line in stdin.lock().lines() {
            let line = line.map_err(Error::Io)?;
            let path = PathBuf::from(line.trim());
            if !path.as_os_str().is_empty() {
                changed_files.push(path);
            }
        }

        Self::detect_affected_packages(graph, &changed_files, packages_dir)
    }

    fn file_to_package(file_path: &Path, packages_dir: &Path) -> Option<String> {
        path_utils::file_to_package(file_path, packages_dir)
    }

    fn validate_git_ref(git_ref: &str) -> Result<()> {
        if git_ref.is_empty() {
            return Err(Error::Adapter {
                package: "change-detection".to_string(),
                message: "Git reference cannot be empty".to_string(),
            });
        }

        if git_ref.len() > 256 {
            return Err(Error::Adapter {
                package: "change-detection".to_string(),
                message: "Git reference exceeds maximum length".to_string(),
            });
        }

        if git_ref.contains('\0') || git_ref.contains('\n') || git_ref.contains('\r') {
            return Err(Error::Adapter {
                package: "change-detection".to_string(),
                message: "Git reference contains invalid characters".to_string(),
            });
        }

        if git_ref.starts_with('-') {
            return Err(Error::Adapter {
                package: "change-detection".to_string(),
                message: "Git reference cannot start with '-'".to_string(),
            });
        }

        Ok(())
    }

    fn git_diff_with_libgit2(base: &str) -> Result<Vec<PathBuf>> {
        let repo = Repository::open_from_env().map_err(|e| Error::Adapter {
            package: "change-detection".to_string(),
            message: format!("Failed to open git repository: {}", e),
        })?;

        let base_obj = repo.revparse_single(base).map_err(|e| Error::Adapter {
            package: "change-detection".to_string(),
            message: format!("Failed to parse git reference '{}': {}", base, e),
        })?;

        let base_tree = base_obj.peel_to_tree().map_err(|e| Error::Adapter {
            package: "change-detection".to_string(),
            message: format!("Failed to get tree from git reference: {}", e),
        })?;

        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(false);
        diff_opts.recurse_untracked_dirs(false);

        let head = repo.head().map_err(|e| Error::Adapter {
            package: "change-detection".to_string(),
            message: format!("Failed to get HEAD: {}", e),
        })?;

        let head_tree = head.peel_to_tree().map_err(|e| Error::Adapter {
            package: "change-detection".to_string(),
            message: format!("Failed to get tree from HEAD: {}", e),
        })?;

        let diff = repo
            .diff_tree_to_tree(Some(&base_tree), Some(&head_tree), Some(&mut diff_opts))
            .map_err(|e| Error::Adapter {
                package: "change-detection".to_string(),
                message: format!("Failed to compute git diff: {}", e),
            })?;

        let mut changed_files = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    changed_files.push(path.to_path_buf());
                }
                true
            },
            None,
            None,
            None,
        )
        .map_err(|e| Error::Adapter {
            package: "change-detection".to_string(),
            message: format!("Failed to iterate over git diff: {}", e),
        })?;

        Ok(changed_files)
    }
}
