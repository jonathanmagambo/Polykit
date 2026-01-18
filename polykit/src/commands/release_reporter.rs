//! Release reporter implementation for CLI.

use owo_colors::OwoColorize;
use polykit_core::release_reporter::ReleaseReporter;

/// CLI implementation of ReleaseReporter.
pub struct CliReleaseReporter;

impl ReleaseReporter for CliReleaseReporter {
    fn report_bump(&self, package: &str, old: Option<&str>, new: &str, dry_run: bool) {
        let old_str = old.unwrap_or("(new)");
        if dry_run {
            println!(
                "  {} Would bump {} from {} to {}",
                "DRY RUN:".yellow().bold(),
                package.bold().white(),
                old_str.bright_black(),
                new.bold().cyan()
            );
        } else {
            println!(
                "  {} Bumped {} from {} to {}",
                "→".cyan(),
                package.bold().white(),
                old_str.bright_black(),
                new.bold().cyan()
            );
        }
    }
}
