//! Command implementations for the CLI.

mod discovery;
mod execution;
mod info;
mod watch;

use std::path::PathBuf;

use owo_colors::OwoColorize;
use polykit_core::Scanner;

pub use discovery::{cmd_affected, cmd_graph, cmd_scan};
pub use execution::{cmd_build, cmd_test};
pub use info::{cmd_list, cmd_release, cmd_validate, cmd_why};
pub use watch::cmd_watch;

fn create_scanner(packages_dir: &PathBuf, no_cache: bool) -> Scanner {
    if no_cache {
        Scanner::new(packages_dir)
    } else {
        Scanner::with_default_cache(packages_dir)
    }
}

fn print_cache_stats(scanner: &Scanner) {
    if let Some(stats) = scanner.cache_stats() {
        let hit_rate = stats.hit_rate() * 100.0;
        println!(
            "  {} Cache: {:.0}% hit rate ({} hits, {} misses)",
            "CACHE:".bright_black(),
            hit_rate,
            stats.hits.to_string().bold(),
            stats.misses.to_string().bold()
        );
        println!();
    }
}
