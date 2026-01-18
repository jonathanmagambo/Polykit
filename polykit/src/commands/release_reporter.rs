//! Release reporter implementation for CLI.

use polykit_core::release_reporter::ReleaseReporter;

/// CLI implementation of ReleaseReporter.
pub struct CliReleaseReporter;

impl ReleaseReporter for CliReleaseReporter {
    fn report_bump(&self, package: &str, old: Option<&str>, new: &str, dry_run: bool) {
        if dry_run {
            println!("[DRY RUN] Would bump {} from {:?} to {}", package, old, new);
        } else {
            println!("Bumped {} from {:?} to {}", package, old, new);
        }
    }
}
