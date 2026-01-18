//! Shared path utilities for package discovery.

use std::path::Path;

/// Converts a file path to its corresponding package name.
///
/// This function extracts the package name from a file path relative to
/// the packages directory. It handles both direct file paths and paths
/// to polykit.toml configuration files.
///
/// # Arguments
///
/// * `file_path` - The file path to convert
/// * `packages_dir` - The base packages directory
///
/// # Returns
///
/// Returns `Some(package_name)` if a package can be determined, `None` otherwise.
pub fn file_to_package(file_path: &Path, packages_dir: &Path) -> Option<String> {
    let relative = file_path.strip_prefix(packages_dir).ok()?;

    for component in relative.components() {
        if let std::path::Component::Normal(name) = component {
            let name_str = name.to_string_lossy();
            if name_str == "polykit.toml" {
                return relative
                    .parent()
                    .and_then(|p| p.components().next())
                    .and_then(|c| {
                        if let std::path::Component::Normal(n) = c {
                            Some(n.to_string_lossy().to_string())
                        } else {
                            None
                        }
                    });
            }
        }
    }

    relative.components().next().and_then(|c| {
        if let std::path::Component::Normal(n) = c {
            Some(n.to_string_lossy().to_string())
        } else {
            None
        }
    })
}
