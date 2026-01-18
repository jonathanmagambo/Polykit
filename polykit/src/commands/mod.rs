//! Command implementations for the CLI.

mod discovery;
mod execution;
mod info;
mod release_reporter;
mod watch;

use std::path::PathBuf;

use polykit_core::Scanner;

use crate::formatting::print_summary_box;

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
        print_summary_box(
            "Cache Statistics",
            &[
                ("Hit Rate", &format!("{:.0}%", hit_rate)),
                ("Hits", &stats.hits.to_string()),
                ("Misses", &stats.misses.to_string()),
            ],
        );
        println!();
    }
}
