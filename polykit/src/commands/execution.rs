//! Task execution commands.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Result;
use comfy_table::{Cell, Table};
use owo_colors::OwoColorize;

use polykit_core::{DependencyGraph, RemoteCache, RemoteCacheConfig, TaskRunner};

use crate::formatting::{create_progress_bar, format_duration, print_section_header, print_separator_with_spacing, print_summary_box, print_success, print_warning, SectionStyle, Status};

use super::create_scanner;

fn create_remote_cache(
    url: Option<String>,
    read_only: bool,
    disabled: bool,
) -> Result<Option<Arc<RemoteCache>>> {
    if disabled || url.is_none() {
        return Ok(None);
    }

    let url = url.unwrap();
    if url.is_empty() {
        return Ok(None);
    }

    let config = RemoteCacheConfig::new(url)
        .read_only(read_only);

    let remote_cache = RemoteCache::from_config(config)?;
    Ok(Some(Arc::new(remote_cache)))
}

#[allow(clippy::too_many_arguments)]
fn run_task_with_progress(
    packages_dir: PathBuf,
    task_name: &str,
    packages_opt: Option<&[String]>,
    parallel: Option<usize>,
    no_stream: bool,
    graph: DependencyGraph,
    progress_msg: &str,
    remote_cache: Option<Arc<RemoteCache>>,
) -> Result<Vec<polykit_core::TaskResult>> {
    let packages_to_run = if let Some(names) = packages_opt {
        names.len()
    } else {
        graph.all_packages().len()
    };

    let pb = create_progress_bar(packages_to_run as u64);
    pb.set_message(progress_msg.to_string());

    let mut runner = TaskRunner::new(&packages_dir, graph).with_max_parallel(parallel);
    if let Some(ref rc) = remote_cache {
        runner = runner.with_remote_cache(Arc::clone(rc));
    }

    if no_stream {
        let results = runner.run_task(task_name, packages_opt)?;
        pb.finish_and_clear();
        Ok(results)
    } else {
        let pb = Arc::new(Mutex::new(pb));
        let pb_clone = Arc::clone(&pb);
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create tokio runtime: {}", e))?;
        let results = rt.block_on(runner.run_task_streaming(
            task_name,
            packages_opt,
            move |package_name, line, is_stderr| {
                let prefix = format!("[{}]", package_name);
                if is_stderr {
                    eprintln!("  {} {}", prefix.bright_black().bold(), line.bright_red());
                } else {
                    println!("  {} {}", prefix.bright_black().bold(), line);
                }
                if let Ok(pb_guard) = pb_clone.lock() {
                    pb_guard.tick();
                }
            },
        ))?;
        if let Ok(pb_guard) = pb.lock() {
            pb_guard.finish_and_clear();
        }
        Ok(results)
    }
}

fn print_task_results(
    results: Vec<polykit_core::TaskResult>,
    packages_to_run: usize,
    section_title: &str,
    success_msg: &str,
) -> bool {
    print_section_header(section_title, SectionStyle::Primary);
    println!();

    let mut failed = false;
    let mut succeeded = 0;
    let mut table = Table::new();
    table
        .set_header(vec![
            Cell::new("Status").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Package").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Details").add_attribute(comfy_table::Attribute::Bold),
        ])
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

    for result in results {
        if result.success {
            table.add_row(vec![
                Cell::new(Status::Success.symbol()).fg(comfy_table::Color::Green),
                Cell::new(&result.package_name).fg(comfy_table::Color::White),
                Cell::new(""),
            ]);
            succeeded += 1;
        } else {
            let error_msg = if result.stderr.is_empty() {
                "Task failed".to_string()
            } else {
                result.stderr.trim().to_string()
            };
            table.add_row(vec![
                Cell::new(Status::Error.symbol()).fg(comfy_table::Color::Red),
                Cell::new(&result.package_name).fg(comfy_table::Color::Red),
                Cell::new(error_msg).fg(comfy_table::Color::Red),
            ]);
            failed = true;
        }
    }

    println!("{}", table);
    println!();

    if failed {
        print_warning(&format!(
            "{} succeeded, {} failed",
            succeeded,
            packages_to_run - succeeded
        ));
    } else {
        let msg = success_msg.replace("{}", &succeeded.to_string());
        print_success(&msg);
    }

    failed
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_build(
    packages_dir: PathBuf,
    packages: Vec<String>,
    parallel: Option<usize>,
    continue_on_error: bool,
    no_cache: bool,
    no_stream: bool,
    show_cache_stats: bool,
    remote_cache_url: Option<String>,
    remote_cache_readonly: bool,
    no_remote_cache: bool,
) -> Result<()> {
    let start = Instant::now();
    let mut scanner = create_scanner(&packages_dir, no_cache);
    let scanned = scanner.scan()?;
    let graph = DependencyGraph::new(scanned)?;

    let packages_opt = if packages.is_empty() {
        None
    } else {
        Some(packages.as_slice())
    };

    let packages_to_run = if let Some(names) = packages_opt {
        names.len()
    } else {
        graph.all_packages().len()
    };

    print_section_header("Building packages", SectionStyle::Primary);

    let remote_cache = create_remote_cache(remote_cache_url, remote_cache_readonly, no_remote_cache)?;

    let results = run_task_with_progress(
        packages_dir,
        "build",
        packages_opt,
        parallel,
        no_stream,
        graph,
        "Building...",
        remote_cache,
    )?;

    let failed = print_task_results(
        results,
        packages_to_run,
        "Build Results",
        "All {} packages built successfully",
    );

    print_separator_with_spacing();

    let duration_str = format_duration(start.elapsed().as_secs_f64());
    
    if show_cache_stats {
        if let Some(stats) = scanner.cache_stats() {
            let hit_rate = stats.hit_rate() * 100.0;
            let cache_str = format!("{:.0}% ({} hits, {} misses)", hit_rate, stats.hits, stats.misses);
            print_summary_box("Summary", &[
                ("Duration", &duration_str),
                ("Cache Hit Rate", &cache_str),
            ]);
        } else {
            print_summary_box("Summary", &[("Duration", &duration_str)]);
        }
    } else {
        print_summary_box("Summary", &[("Duration", &duration_str)]);
    }
    println!();

    if failed && !continue_on_error {
        std::process::exit(1);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_test(
    packages_dir: PathBuf,
    packages: Vec<String>,
    parallel: Option<usize>,
    continue_on_error: bool,
    no_cache: bool,
    no_stream: bool,
    show_cache_stats: bool,
    remote_cache_url: Option<String>,
    remote_cache_readonly: bool,
    no_remote_cache: bool,
) -> Result<()> {
    let start = Instant::now();
    let mut scanner = create_scanner(&packages_dir, no_cache);
    let scanned = scanner.scan()?;
    let graph = DependencyGraph::new(scanned)?;

    let packages_opt = if packages.is_empty() {
        None
    } else {
        Some(packages.as_slice())
    };

    let packages_to_run = if let Some(names) = packages_opt {
        names.len()
    } else {
        graph.all_packages().len()
    };

    print_section_header("Running tests", SectionStyle::Primary);

    let remote_cache = create_remote_cache(remote_cache_url, remote_cache_readonly, no_remote_cache)?;

    let results = run_task_with_progress(
        packages_dir,
        "test",
        packages_opt,
        parallel,
        no_stream,
        graph,
        "Testing...",
        remote_cache,
    )?;

    let failed = print_task_results(
        results,
        packages_to_run,
        "Test Results",
        "All {} packages passed",
    );

    print_separator_with_spacing();

    let duration_str = format_duration(start.elapsed().as_secs_f64());
    
    if show_cache_stats {
        if let Some(stats) = scanner.cache_stats() {
            let hit_rate = stats.hit_rate() * 100.0;
            let cache_str = format!("{:.0}% ({} hits, {} misses)", hit_rate, stats.hits, stats.misses);
            print_summary_box("Summary", &[
                ("Duration", &duration_str),
                ("Cache Hit Rate", &cache_str),
            ]);
        } else {
            print_summary_box("Summary", &[("Duration", &duration_str)]);
        }
    } else {
        print_summary_box("Summary", &[("Duration", &duration_str)]);
    }
    println!();

    if failed && !continue_on_error {
        std::process::exit(1);
    }

    Ok(())
}
