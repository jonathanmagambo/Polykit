//! Information and management commands.

use std::path::PathBuf;

use anyhow::Result;
use owo_colors::OwoColorize;
use polykit_adapters::get_adapter;
use polykit_core::release::BumpType;
use polykit_core::{DependencyGraph, ReleaseEngine};

use super::{create_scanner, print_cache_stats};

pub fn cmd_release(
    packages_dir: PathBuf,
    package: String,
    bump_type: BumpType,
    dry_run: bool,
    no_cache: bool,
    show_cache_stats: bool,
) -> Result<()> {
    let mut scanner = create_scanner(&packages_dir, no_cache);
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
    }
    if show_cache_stats {
        print_cache_stats(&scanner);
    }
    println!();

    Ok(())
}

pub fn cmd_why(
    packages_dir: PathBuf,
    package: String,
    no_cache: bool,
    show_cache_stats: bool,
) -> Result<()> {
    let mut scanner = create_scanner(&packages_dir, no_cache);
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

    if show_cache_stats {
        print_cache_stats(&scanner);
    }

    Ok(())
}

pub fn cmd_validate(
    packages_dir: PathBuf,
    json: bool,
    no_cache: bool,
    show_cache_stats: bool,
) -> Result<()> {
    let mut scanner = create_scanner(&packages_dir, no_cache);
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

    if show_cache_stats {
        print_cache_stats(&scanner);
    }

    Ok(())
}

pub fn cmd_list(
    packages_dir: PathBuf,
    json: bool,
    no_cache: bool,
    show_cache_stats: bool,
) -> Result<()> {
    let mut scanner = create_scanner(&packages_dir, no_cache);
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

    if show_cache_stats {
        print_cache_stats(&scanner);
    }

    Ok(())
}
