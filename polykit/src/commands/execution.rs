//! Task execution commands.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use polykit_core::{DependencyGraph, TaskRunner};

use super::{create_scanner, print_cache_stats};

fn run_task_with_progress(
    packages_dir: PathBuf,
    task_name: &str,
    packages_opt: Option<&[String]>,
    parallel: Option<usize>,
    no_stream: bool,
    graph: DependencyGraph,
    progress_msg: &str,
) -> Result<Vec<polykit_core::TaskResult>> {
    let packages_to_run = if let Some(names) = packages_opt {
        names.len()
    } else {
        graph.all_packages().len()
    };

    let pb = ProgressBar::new(packages_to_run as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .expect("Progress bar template is valid")
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );
    pb.set_message(progress_msg.to_string());

    let runner = TaskRunner::new(&packages_dir, graph).with_max_parallel(parallel);

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
                let prefix = format!("[{}] ", package_name);
                if is_stderr {
                    eprintln!("{}{}", prefix.bright_black(), line.bright_red());
                } else {
                    println!("{}{}", prefix.bright_black(), line);
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
    println!("{}", section_title.bold().cyan());
    println!();

    let mut failed = false;
    let mut succeeded = 0;
    for result in results {
        if result.success {
            println!(
                "  {} {}",
                "OK".green(),
                result.package_name.to_string().bold().white()
            );
            succeeded += 1;
        } else {
            println!(
                "  {} {}",
                "FAILED".red(),
                result.package_name.to_string().bold().red()
            );
            if !result.stderr.is_empty() {
                println!("     {}", result.stderr.trim().bright_red());
            }
            failed = true;
        }
    }

    println!();
    if failed {
        println!(
            "  {} {} succeeded, {} failed",
            "WARNING:".yellow(),
            succeeded.to_string().bold().green(),
            (packages_to_run - succeeded).to_string().bold().red()
        );
    } else {
        let msg = success_msg.replace("{}", &succeeded.to_string());
        println!("  {} {}", "OK".green(), msg.bold().green());
    }

    failed
}

pub fn cmd_build(
    packages_dir: PathBuf,
    packages: Vec<String>,
    parallel: Option<usize>,
    continue_on_error: bool,
    no_cache: bool,
    no_stream: bool,
    show_cache_stats: bool,
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

    println!("{}", "[Building packages]".bold().cyan());
    println!();

    let results = run_task_with_progress(
        packages_dir,
        "build",
        packages_opt,
        parallel,
        no_stream,
        graph,
        "Building...",
    )?;

    let failed = print_task_results(
        results,
        packages_to_run,
        "[Build Results]",
        "All {} packages built successfully",
    );

    println!(
        "  {} Duration: {:.2}s",
        "TIME:".bright_black(),
        start.elapsed().as_secs_f64().to_string().bold()
    );
    if show_cache_stats {
        print_cache_stats(&scanner);
    }
    println!();

    if failed && !continue_on_error {
        std::process::exit(1);
    }

    Ok(())
}

pub fn cmd_test(
    packages_dir: PathBuf,
    packages: Vec<String>,
    parallel: Option<usize>,
    continue_on_error: bool,
    no_cache: bool,
    no_stream: bool,
    show_cache_stats: bool,
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

    println!("{}", "[Running tests]".bold().cyan());
    println!();

    let results = run_task_with_progress(
        packages_dir,
        "test",
        packages_opt,
        parallel,
        no_stream,
        graph,
        "Testing...",
    )?;

    let failed = print_task_results(
        results,
        packages_to_run,
        "[Test Results]",
        "All {} packages passed",
    );

    println!(
        "  {} Duration: {:.2}s",
        "TIME:".bright_black(),
        start.elapsed().as_secs_f64().to_string().bold()
    );
    if show_cache_stats {
        print_cache_stats(&scanner);
    }
    println!();

    if failed && !continue_on_error {
        std::process::exit(1);
    }

    Ok(())
}
