//! Watch mode command.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use polykit_core::{DependencyGraph, FileWatcher, TaskRunner, WatcherConfig};

use crate::formatting::{print_key_value, print_section_header, print_success, print_warning, SectionStyle};

use super::create_scanner;

pub fn cmd_watch(
    packages_dir: PathBuf,
    task: String,
    packages: Vec<String>,
    debounce_ms: Option<u64>,
    no_cache: bool,
) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
    })
    .map_err(|e| anyhow::anyhow!("Failed to set signal handler: {}", e))?;

    let mut scanner = create_scanner(&packages_dir, no_cache);

    let debounce_duration = Duration::from_millis(debounce_ms.unwrap_or(300));

    print_section_header("Watch Mode", SectionStyle::Primary);
    print_key_value("Watching", &packages_dir.display().to_string());
    print_key_value("Task", &task);
    if !packages.is_empty() {
        print_key_value("Packages", &packages.join(", "));
    }
    println!("  Press Ctrl+C to stop");
    println!();

    let watcher_config = WatcherConfig {
        packages_dir: packages_dir.clone(),
        debounce_ms: debounce_ms.unwrap_or(300),
    };

    let mut watcher = FileWatcher::new(watcher_config)?;
    let mut affected_packages = std::collections::HashSet::new();
    let mut last_event_time = Instant::now();

    loop {
        if !running.load(Ordering::SeqCst) {
            println!();
            print_warning("Stopping watch mode...");
            break;
        }

        match watcher.next_event() {
            Ok(Some(event)) => {
                let affected = watcher.get_affected_packages(&event);
                if !affected.is_empty() {
                    affected_packages.extend(affected);
                    last_event_time = Instant::now();
                }
            }
            Ok(None) => {
                if !affected_packages.is_empty() && last_event_time.elapsed() >= debounce_duration {
                    print_warning("Change detected, rebuilding...");
                    let scanned = scanner.scan()?;
                    let graph = DependencyGraph::new(scanned)?;

                    let mut packages_to_rebuild = affected_packages.clone();
                    for pkg_name in &affected_packages {
                        if let Ok(dependents) = graph.dependents(pkg_name) {
                            packages_to_rebuild.extend(dependents);
                        }
                    }

                    let packages_to_run: Vec<String> = if packages.is_empty() {
                        packages_to_rebuild.into_iter().collect()
                    } else {
                        packages_to_rebuild
                            .into_iter()
                            .filter(|p| packages.contains(p))
                            .collect()
                    };

                    if !packages_to_run.is_empty() {
                        let runner = TaskRunner::new(&packages_dir, graph);
                        let results = runner.run_task(&task, Some(&packages_to_run))?;

                        let mut failed = false;
                        for result in results {
                            if !result.success {
                                use crate::formatting::print_error;
                                print_error(&format!("{} failed", result.package_name));
                                failed = true;
                            }
                        }

                        if !failed {
                            print_success("Rebuild complete");
                        }
                    } else {
                        print_success("No matching packages to rebuild");
                    }
                    println!();

                    affected_packages.clear();
                } else {
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
            Err(_) => {
                break;
            }
        }
    }

    Ok(())
}
