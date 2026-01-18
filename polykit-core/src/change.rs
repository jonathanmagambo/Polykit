//! Change detection for determining affected packages.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

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
        let changed_files = Self::git_diff(base)?;
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

    fn git_diff(base: &str) -> Result<Vec<PathBuf>> {
        let output = Command::new("git")
            .arg("diff")
            .arg("--name-only")
            .arg(base)
            .output()
            .map_err(|e| Error::Adapter {
                package: "change-detection".to_string(),
                message: format!("Failed to run git diff: {}", e),
            })?;

        if !output.status.success() {
            return Err(Error::Adapter {
                package: "change-detection".to_string(),
                message: format!(
                    "git diff failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let files: Vec<PathBuf> = stdout
            .lines()
            .map(|line| PathBuf::from(line.trim()))
            .filter(|p| !p.as_os_str().is_empty())
            .collect();

        Ok(files)
    }
}
