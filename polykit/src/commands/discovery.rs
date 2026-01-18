//! Discovery and inspection commands.

use std::path::PathBuf;

use anyhow::Result;
use owo_colors::OwoColorize;
use polykit_core::{ChangeDetector, DependencyGraph};

use super::{create_scanner, print_cache_stats};

pub fn cmd_scan(
    packages_dir: PathBuf,
    json: bool,
    no_cache: bool,
    show_cache_stats: bool,
) -> Result<()> {
    let mut scanner = create_scanner(&packages_dir, no_cache);
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

    if show_cache_stats {
        print_cache_stats(&scanner);
    }

    Ok(())
}

pub fn cmd_graph(
    packages_dir: PathBuf,
    json: bool,
    no_cache: bool,
    show_cache_stats: bool,
) -> Result<()> {
    let mut scanner = create_scanner(&packages_dir, no_cache);
    let packages = scanner.scan()?;
    let graph = DependencyGraph::new(packages)?;

    let order = graph.topological_order();

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

    if show_cache_stats {
        print_cache_stats(&scanner);
    }

    Ok(())
}

pub fn cmd_affected(
    packages_dir: PathBuf,
    files: Vec<String>,
    git: bool,
    base: Option<String>,
    no_cache: bool,
    show_cache_stats: bool,
) -> Result<()> {
    let mut scanner = create_scanner(&packages_dir, no_cache);
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

    if show_cache_stats {
        print_cache_stats(&scanner);
    }

    Ok(())
}
