//! Trait for reporting release operations.

/// Trait for reporting release version bumps.
///
/// This trait allows the core library to report release operations without
/// directly writing to stdout/stderr, maintaining separation of concerns.
pub trait ReleaseReporter: Send + Sync {
    /// Reports a version bump operation.
    ///
    /// # Arguments
    ///
    /// * `package` - The package name being bumped
    /// * `old` - The old version, if it existed
    /// * `new` - The new version after bump
    /// * `dry_run` - Whether this is a dry run (no actual changes made)
    fn report_bump(&self, package: &str, old: Option<&str>, new: &str, dry_run: bool);
}
