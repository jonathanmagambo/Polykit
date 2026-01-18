//! Semantic versioning and release management.

use std::collections::HashMap;
use std::path::PathBuf;

use semver::Version;

use crate::adapter::LanguageAdapter;
use crate::error::{Error, Result};
use crate::graph::DependencyGraph;
use crate::release_reporter::ReleaseReporter;

type AdapterGetter =
    Box<dyn Fn(&crate::package::Language) -> Box<dyn LanguageAdapter> + Send + Sync>;

/// Engine for planning and executing releases.
pub struct ReleaseEngine {
    packages_dir: PathBuf,
    graph: DependencyGraph,
    dry_run: bool,
    adapter_getter: AdapterGetter,
    reporter: Box<dyn ReleaseReporter>,
}

/// A plan for releasing packages with version bumps.
#[derive(Debug, Clone)]
pub struct ReleasePlan {
    /// Packages that will be version-bumped.
    pub packages: Vec<ReleasePackage>,
}

/// A package that will be version-bumped as part of a release.
#[derive(Debug, Clone)]
pub struct ReleasePackage {
    /// Package name.
    pub name: String,
    /// Old version (if it existed).
    pub old_version: Option<String>,
    /// New version after bump.
    pub new_version: String,
    /// Type of version bump.
    pub bump_type: BumpType,
}

/// Type of semantic version bump.
#[derive(Debug, Clone, Copy)]
pub enum BumpType {
    /// Major version bump (1.0.0 -> 2.0.0).
    Major,
    /// Minor version bump (1.0.0 -> 1.1.0).
    Minor,
    /// Patch version bump (1.0.0 -> 1.0.1).
    Patch,
}

impl ReleaseEngine {
    /// Creates a new release engine.
    ///
    /// If `dry_run` is `true`, version bumps will be planned but not executed.
    ///
    /// The `adapter_getter` function is used to obtain language adapters for reading
    /// and updating package metadata.
    ///
    /// The `reporter` is used to report version bump operations without directly
    /// writing to stdout/stderr.
    pub fn new<F, R>(
        packages_dir: impl Into<PathBuf>,
        graph: DependencyGraph,
        dry_run: bool,
        adapter_getter: F,
        reporter: R,
    ) -> Self
    where
        F: Fn(&crate::package::Language) -> Box<dyn LanguageAdapter> + Send + Sync + 'static,
        R: ReleaseReporter + 'static,
    {
        Self {
            packages_dir: packages_dir.into(),
            graph,
            dry_run,
            adapter_getter: Box::new(adapter_getter),
            reporter: Box::new(reporter),
        }
    }

    /// Plans a release by bumping the specified package and updating dependents.
    ///
    /// The release plan includes:
    /// - The target package with the requested bump type
    /// - Dependent packages that need patch bumps
    ///
    /// # Errors
    ///
    /// Returns an error if the package is not found or version operations fail.
    pub fn plan_release(&self, package_name: &str, bump_type: BumpType) -> Result<ReleasePlan> {
        let order = self.graph.topological_order();
        let mut plan = ReleasePlan {
            packages: Vec::new(),
        };

        let available: Vec<String> = self
            .graph
            .all_packages()
            .iter()
            .map(|p| p.name.clone())
            .collect();
        let available_str = available.join(", ");
        let target_idx = order
            .iter()
            .position(|n| n == package_name)
            .ok_or_else(|| Error::PackageNotFound {
                name: package_name.to_string(),
                available: available_str.clone(),
            })?;

        let mut versions = HashMap::new();

        for (idx, package_name) in order.iter().enumerate() {
            let package =
                self.graph
                    .get_package(package_name)
                    .ok_or_else(|| Error::PackageNotFound {
                        name: package_name.clone(),
                        available: available_str.clone(),
                    })?;

            let adapter = (self.adapter_getter)(&package.language);
            let package_path = self.packages_dir.join(&package.path);
            let metadata = adapter.read_metadata(&package_path)?;

            let old_version = metadata.version.clone();
            let new_version = if idx == target_idx {
                self.bump_version(&old_version, bump_type)?
            } else if idx < target_idx {
                let deps = self.graph.dependencies(package_name)?;
                let current_version = old_version.as_deref().unwrap_or("0.1.0");
                let needs_bump = deps.iter().any(|dep| {
                    versions
                        .get(dep)
                        .map(|v: &String| v != current_version)
                        .unwrap_or(false)
                });

                if needs_bump {
                    self.bump_version(&old_version, BumpType::Patch)?
                } else {
                    old_version.clone().unwrap_or_else(|| "0.1.0".to_string())
                }
            } else {
                old_version.clone().unwrap_or_else(|| "0.1.0".to_string())
            };

            versions.insert(package_name.clone(), new_version.clone());

            if let Some(old) = &old_version {
                if old != &new_version {
                    plan.packages.push(ReleasePackage {
                        name: package_name.clone(),
                        old_version: old_version.clone(),
                        new_version,
                        bump_type: if idx == target_idx {
                            bump_type
                        } else {
                            BumpType::Patch
                        },
                    });
                }
            } else if idx <= target_idx {
                plan.packages.push(ReleasePackage {
                    name: package_name.clone(),
                    old_version: None,
                    new_version,
                    bump_type: if idx == target_idx {
                        bump_type
                    } else {
                        BumpType::Patch
                    },
                });
            }
        }

        Ok(plan)
    }

    /// Executes a release plan by updating version numbers in package files.
    ///
    /// If `dry_run` is enabled, this will only report what would be changed
    /// without actually modifying files.
    ///
    /// # Errors
    ///
    /// Returns an error if version bumping fails for any package.
    pub fn execute_release(&self, plan: &ReleasePlan) -> Result<()> {
        for release_pkg in &plan.packages {
            let available: Vec<String> = self
                .graph
                .all_packages()
                .iter()
                .map(|p| p.name.clone())
                .collect();
            let available_str = available.join(", ");
            let package = self.graph.get_package(&release_pkg.name).ok_or_else(|| {
                Error::PackageNotFound {
                    name: release_pkg.name.clone(),
                    available: available_str,
                }
            })?;

            let adapter = (self.adapter_getter)(&package.language);
            let package_path = self.packages_dir.join(&package.path);

            if !self.dry_run {
                adapter.bump_version(&package_path, &release_pkg.new_version)?;
            }

            self.reporter.report_bump(
                &release_pkg.name,
                release_pkg.old_version.as_deref(),
                &release_pkg.new_version,
                self.dry_run,
            );
        }

        Ok(())
    }

    fn bump_version(&self, current: &Option<String>, bump_type: BumpType) -> Result<String> {
        let version = if let Some(v) = current {
            Version::parse(v)
                .map_err(|e| Error::Release(format!("Invalid version {}: {}", v, e)))?
        } else {
            Version::parse("0.1.0")
                .map_err(|e| Error::Release(format!("Failed to parse default version: {}", e)))?
        };

        let new_version = match bump_type {
            BumpType::Major => Version::new(version.major + 1, 0, 0),
            BumpType::Minor => Version::new(version.major, version.minor + 1, 0),
            BumpType::Patch => Version::new(version.major, version.minor, version.patch + 1),
        };

        Ok(new_version.to_string())
    }
}
