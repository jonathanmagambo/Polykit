//! Command implementations for the CLI.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use polykit_adapters::get_adapter;
use polykit_core::release::BumpType;
use polykit_core::{
    ChangeDetector, DependencyGraph, FileWatcher, ReleaseEngine, Scanner, TaskRunner, WatcherConfig,
};

pub fn cmd_scan(packages_dir: PathBuf, json: bool, no_cache: bool) -> Result<()> {
    let mut scanner = if no_cache {
        Scanner::new(&packages_dir)
    } else {
        Scanner::with_default_cache(&packages_dir)
    };
    let packages = scanner.scan()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&packages)?);
    } else {
        println!("{}", "[Scanning packages...]".bold().cyan());
        println!();

        if packages.is_empty() {
            println!("  {} No packages found", "WARNING:".yellow());
        } else {
            println!(
                "  {} Found {} {}",
                "OK".green(),
                packages.len().to_string().bold().cyan(),
                "packages".bold()
            );
            println!();
            for pkg in packages {
                println!(
                    "  {} {}",
                    pkg.name.bold().white(),
                    format!("({})", pkg.language.as_str()).bright_black()
                );
            }
        }
        println!();
    }

    Ok(())
}

pub fn cmd_graph(packages_dir: PathBuf, json: bool, no_cache: bool) -> Result<()> {
    let mut scanner = if no_cache {
        Scanner::new(&packages_dir)
    } else {
        Scanner::with_default_cache(&packages_dir)
    };
    let packages = scanner.scan()?;
    let graph = DependencyGraph::new(packages)?;

    let order = graph.topological_order()?;

    if json {
        let graph_data = serde_json::json!({
            "packages": order,
            "edges": {
                // Could expand this to show actual edges
            }
        });
        println!("{}", serde_json::to_string_pretty(&graph_data)?);
    } else {
        println!("{}", "[Dependency Graph]".bold().cyan());
        println!();

        if order.is_empty() {
            println!("  {} No packages found", "WARNING:".yellow());
        } else {
            println!(
                "  {} Topological order ({} packages):",
                "OK".green(),
                order.len().to_string().bold().cyan()
            );
            println!();
            for (idx, pkg) in order.iter().enumerate() {
                println!(
                    "  {} {}",
                    format!("{:2}", idx + 1).bright_black(),
                    pkg.bold().white()
                );
            }
        }
        println!();
    }

    Ok(())
}

pub fn cmd_affected(
    packages_dir: PathBuf,
    files: Vec<String>,
    git: bool,
    base: Option<String>,
    no_cache: bool,
) -> Result<()> {
    let mut scanner = if no_cache {
        Scanner::new(&packages_dir)
    } else {
        Scanner::with_default_cache(&packages_dir)
    };
    let packages = scanner.scan()?;
    let graph = DependencyGraph::new(packages)?;

    let affected = if git {
        ChangeDetector::detect_from_git(&graph, &packages_dir, base.as_deref())?
    } else if files.is_empty() {
        return Err(anyhow::anyhow!(
            "No files specified. Use --git to detect from git or provide file paths."
        ));
    } else {
        let file_paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
        ChangeDetector::detect_affected_packages(&graph, &file_paths, &packages_dir)?
    };

    println!("{}", "[Affected Packages]".bold().cyan());
    println!();

    if affected.is_empty() {
        println!("  {} No affected packages", "OK".green());
    } else {
        println!(
            "  {} {} {}",
            "WARNING:".yellow(),
            affected.len().to_string().bold().yellow(),
            "packages affected".bold()
        );
        println!();
        for pkg in affected {
            println!("  - {}", pkg.bold().yellow());
        }
    }
    println!();

    Ok(())
}

pub fn cmd_build(
    packages_dir: PathBuf,
    packages: Vec<String>,
    parallel: Option<usize>,
    continue_on_error: bool,
    no_cache: bool,
    no_stream: bool,
) -> Result<()> {
    let start = Instant::now();
    let mut scanner = if no_cache {
        Scanner::new(&packages_dir)
    } else {
        Scanner::with_default_cache(&packages_dir)
    };
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

    let pb = ProgressBar::new(packages_to_run as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .expect("Progress bar template is valid")
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );
    pb.set_message("Building...".to_string());

    let runner = TaskRunner::new(&packages_dir, graph).with_max_parallel(parallel);

    let results = if no_stream {
        let results = runner.run_task("build", packages_opt)?;
        pb.finish_and_clear();
        results
    } else {
        use std::sync::{Arc, Mutex};
        let pb = Arc::new(Mutex::new(pb));
        let pb_clone = Arc::clone(&pb);
        let results = runner.run_task_streaming(
            "build",
            packages_opt,
            move |package_name, line, is_stderr| {
                let prefix = format!("[{}] ", package_name);
                if is_stderr {
                    eprintln!("{}{}", prefix.bright_black(), line.bright_red());
                } else {
                    println!("{}{}", prefix.bright_black(), line);
                }
                pb_clone.lock().unwrap().tick();
            },
        )?;
        pb.lock().unwrap().finish_and_clear();
        results
    };

    println!("{}", "[Build Results]".bold().cyan());
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
            if !continue_on_error {
                break;
            }
        }
    }

    println!();
    let duration = start.elapsed();
    if failed {
        println!(
            "  {} {} succeeded, {} failed",
            "WARNING:".yellow(),
            succeeded.to_string().bold().green(),
            (packages_to_run - succeeded).to_string().bold().red()
        );
    } else {
        println!(
            "  {} All {} packages built successfully",
            "OK".green(),
            succeeded.to_string().bold().green()
        );
    }
    println!(
        "  {} Duration: {:.2}s",
        "TIME:".bright_black(),
        duration.as_secs_f64().to_string().bold()
    );
    println!();

    if failed {
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
) -> Result<()> {
    let start = Instant::now();
    let mut scanner = if no_cache {
        Scanner::new(&packages_dir)
    } else {
        Scanner::with_default_cache(&packages_dir)
    };
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

    let pb = ProgressBar::new(packages_to_run as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .expect("Progress bar template is valid")
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );
    pb.set_message("Testing...".to_string());

    let runner = TaskRunner::new(&packages_dir, graph).with_max_parallel(parallel);

    let results = if no_stream {
        let results = runner.run_task("test", packages_opt)?;
        pb.finish_and_clear();
        results
    } else {
        use std::sync::{Arc, Mutex};
        let pb = Arc::new(Mutex::new(pb));
        let pb_clone = Arc::clone(&pb);
        let results = runner.run_task_streaming(
            "test",
            packages_opt,
            move |package_name, line, is_stderr| {
                let prefix = format!("[{}] ", package_name);
                if is_stderr {
                    eprintln!("{}{}", prefix.bright_black(), line.bright_red());
                } else {
                    println!("{}{}", prefix.bright_black(), line);
                }
                pb_clone.lock().unwrap().tick();
            },
        )?;
        pb.lock().unwrap().finish_and_clear();
        results
    };

    println!("{}", "[Test Results]".bold().cyan());
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
            if !continue_on_error {
                break;
            }
        }
    }

    println!();
    let duration = start.elapsed();
    if failed {
        println!(
            "  {} {} succeeded, {} failed",
            "WARNING:".yellow(),
            succeeded.to_string().bold().green(),
            (packages_to_run - succeeded).to_string().bold().red()
        );
    } else {
        println!(
            "  {} All {} packages passed",
            "OK".green(),
            succeeded.to_string().bold().green()
        );
    }
    println!(
        "  {} Duration: {:.2}s",
        "TIME:".bright_black(),
        duration.as_secs_f64().to_string().bold()
    );
    println!();

    if failed {
        std::process::exit(1);
    }

    Ok(())
}

pub fn cmd_release(
    packages_dir: PathBuf,
    package: String,
    bump_type: BumpType,
    dry_run: bool,
    no_cache: bool,
) -> Result<()> {
    let mut scanner = if no_cache {
        Scanner::new(&packages_dir)
    } else {
        Scanner::with_default_cache(&packages_dir)
    };
    let scanned = scanner.scan()?;
    let graph = DependencyGraph::new(scanned)?;

    let engine = ReleaseEngine::new(&packages_dir, graph, dry_run, get_adapter);
    let plan = engine.plan_release(&package, bump_type)?;

    if dry_run {
        println!("{}", "[Release Plan (Dry Run)]".bold().cyan());
    } else {
        println!("{}", "[Release Plan]".bold().cyan());
    }
    println!();

    if plan.packages.is_empty() {
        println!("  {} No packages need version bumps", "OK".green());
        println!();
        return Ok(());
    }

    println!(
        "  {} {} packages will be updated:",
        "PACKAGES:".bright_cyan(),
        plan.packages.len().to_string().bold().cyan()
    );
    println!();

    for pkg in &plan.packages {
        let old_ver = pkg.old_version.as_deref().unwrap_or("(new)");
        match pkg.bump_type {
            BumpType::Major => {
                println!(
                    "  [{}] {} {} → {}",
                    "MAJOR".red(),
                    pkg.name.bold().white(),
                    old_ver.bright_black(),
                    pkg.new_version.bold().cyan()
                );
            }
            BumpType::Minor => {
                println!(
                    "  [{}] {} {} → {}",
                    "MINOR".yellow(),
                    pkg.name.bold().white(),
                    old_ver.bright_black(),
                    pkg.new_version.bold().cyan()
                );
            }
            BumpType::Patch => {
                println!(
                    "  [{}] {} {} → {}",
                    "PATCH".green(),
                    pkg.name.bold().white(),
                    old_ver.bright_black(),
                    pkg.new_version.bold().cyan()
                );
            }
        }
    }
    println!();

    engine.execute_release(&plan)?;

    if !dry_run {
        println!("  {} Release completed successfully", "OK".green());
        println!();
    }

    Ok(())
}

pub fn cmd_why(packages_dir: PathBuf, package: String, no_cache: bool) -> Result<()> {
    let mut scanner = if no_cache {
        Scanner::new(&packages_dir)
    } else {
        Scanner::with_default_cache(&packages_dir)
    };
    let scanned = scanner.scan()?;
    let graph = DependencyGraph::new(scanned)?;

    let deps = graph.dependencies(&package)?;
    let dependents = graph.dependents(&package)?;

    println!("{}", "[Package Dependencies]".bold().cyan());
    println!();
    println!("  Package: {}", package.bold().white());
    println!();

    println!(
        "  {} Dependencies ({}):",
        "DEPENDS ON:".bright_cyan(),
        deps.len().to_string().bold().cyan()
    );
    if deps.is_empty() {
        println!("     {}", "(none)".bright_black());
    } else {
        for dep in deps {
            println!("     - {}", dep.bold().white());
        }
    }
    println!();

    println!(
        "  {} Dependents ({}):",
        "DEPENDED ON BY:".bright_cyan(),
        dependents.len().to_string().bold().cyan()
    );
    if dependents.is_empty() {
        println!("     {}", "(none)".bright_black());
    } else {
        for dep in dependents {
            println!("     - {}", dep.bold().white());
        }
    }
    println!();

    Ok(())
}

pub fn cmd_validate(packages_dir: PathBuf, json: bool, no_cache: bool) -> Result<()> {
    let mut scanner = if no_cache {
        Scanner::new(&packages_dir)
    } else {
        Scanner::with_default_cache(&packages_dir)
    };
    let packages = scanner.scan()?;
    let _ = DependencyGraph::new(packages)?;

    if json {
        println!("{{\"valid\": true}}");
    } else {
        println!("{}", "[Validation]".bold().cyan());
        println!();
        println!("  {} All packages are valid", "OK".green());
        println!("  {} No circular dependencies detected", "OK".green());
        println!("  {} Dependency graph is valid", "OK".green());
        println!();
    }

    Ok(())
}

pub fn cmd_list(packages_dir: PathBuf, json: bool, no_cache: bool) -> Result<()> {
    let mut scanner = if no_cache {
        Scanner::new(&packages_dir)
    } else {
        Scanner::with_default_cache(&packages_dir)
    };
    let packages = scanner.scan()?;

    if json {
        let tasks: std::collections::HashMap<String, Vec<String>> = packages
            .iter()
            .map(|p| {
                let task_names: Vec<String> = p.tasks.iter().map(|t| t.name.clone()).collect();
                (p.name.clone(), task_names)
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&tasks)?);
    } else {
        println!("{}", "[Available Tasks]".bold().cyan());
        println!();

        if packages.is_empty() {
            println!("  {} No packages found", "WARNING:".yellow());
        } else {
            for pkg in packages {
                println!("  {} {}", "PACKAGE:".bright_cyan(), pkg.name.bold().white());
                if pkg.tasks.is_empty() {
                    println!("     {}", "(no tasks)".bright_black());
                } else {
                    for task in pkg.tasks {
                        println!("     - {}", task.name.bold());
                    }
                }
                println!();
            }
        }
    }

    Ok(())
}

pub fn cmd_watch(
    packages_dir: PathBuf,
    task: String,
    packages: Vec<String>,
    debounce_ms: Option<u64>,
    no_cache: bool,
) -> Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
    })
    .map_err(|e| anyhow::anyhow!("Failed to set signal handler: {}", e))?;

    let mut scanner = if no_cache {
        Scanner::new(&packages_dir)
    } else {
        Scanner::with_default_cache(&packages_dir)
    };

    let mut last_run = Instant::now();
    let debounce_duration = Duration::from_millis(debounce_ms.unwrap_or(300));

    println!("{}", "[Watch Mode]".bold().cyan());
    println!("  Watching for changes in: {}", packages_dir.display());
    println!("  Task: {}", task.bold());
    if !packages.is_empty() {
        println!("  Packages: {}", packages.join(", ").bold());
    }
    println!("  Press Ctrl+C to stop");
    println!();

    let watcher_config = WatcherConfig {
        packages_dir: packages_dir.clone(),
        debounce_ms: debounce_ms.unwrap_or(300),
    };

    let mut watcher = FileWatcher::new(watcher_config)?;
    let mut last_affected = std::collections::HashSet::new();

    loop {
        if !running.load(Ordering::SeqCst) {
            println!("\n{}", "Stopping watch mode...".yellow());
            break;
        }

        if let Ok(Some(event)) = watcher.next_event() {
            let affected = watcher.get_affected_packages(&event);
            if !affected.is_empty() {
                last_affected.extend(affected);
                if last_run.elapsed() >= debounce_duration {
                    let packages_to_run = if packages.is_empty() {
                        None
                    } else {
                        Some(packages.as_slice())
                    };

                    println!("{}", "[Change detected, rebuilding...]".bold().yellow());
                    let scanned = scanner.scan()?;
                    let graph = DependencyGraph::new(scanned)?;

                    let runner = TaskRunner::new(&packages_dir, graph);
                    let results = runner.run_task(&task, packages_to_run)?;

                    let mut failed = false;
                    for result in results {
                        if !result.success {
                            println!(
                                "  {} {}",
                                "FAILED".red(),
                                result.package_name.to_string().bold().red()
                            );
                            failed = true;
                        }
                    }

                    if !failed {
                        println!("  {} Rebuild complete", "OK".green());
                    }
                    println!();

                    last_affected.clear();
                    last_run = Instant::now();
                }
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}
