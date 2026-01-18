//! Information and management commands.

use std::path::PathBuf;

use anyhow::Result;
use comfy_table::{Cell, Table};
use polykit_adapters::get_adapter;
use polykit_core::release::BumpType;
use polykit_core::{DependencyGraph, ReleaseEngine};

use crate::formatting::{print_key_value, print_package_list, print_section_header, print_separator_with_spacing, print_success, SectionStyle};

use super::release_reporter::CliReleaseReporter;
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

    let engine = ReleaseEngine::new(
        &packages_dir,
        graph,
        dry_run,
        get_adapter,
        CliReleaseReporter,
    );
    let plan = engine.plan_release(&package, bump_type)?;

    let title = if dry_run {
        "Release Plan (Dry Run)"
    } else {
        "Release Plan"
    };
    print_section_header(title, SectionStyle::Primary);

    if plan.packages.is_empty() {
        print_success("No packages need version bumps");
        println!();
        return Ok(());
    }

    print_key_value(
        "Packages to update",
        &plan.packages.len().to_string(),
    );
    println!();

    let mut table = Table::new();
    table
        .set_header(vec![
            Cell::new("Type").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Package").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Version").add_attribute(comfy_table::Attribute::Bold),
        ])
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

    for pkg in &plan.packages {
        let old_ver = pkg.old_version.as_deref().unwrap_or("(new)");
        let version_str = format!("{} → {}", old_ver, pkg.new_version);
        let (type_label, type_color) = match pkg.bump_type {
            BumpType::Major => ("MAJOR", comfy_table::Color::Red),
            BumpType::Minor => ("MINOR", comfy_table::Color::Yellow),
            BumpType::Patch => ("PATCH", comfy_table::Color::Green),
        };
        table.add_row(vec![
            Cell::new(type_label).fg(type_color),
            Cell::new(&pkg.name).fg(comfy_table::Color::White),
            Cell::new(version_str).fg(comfy_table::Color::Cyan),
        ]);
    }

    println!("{}", table);
    println!();

    engine.execute_release(&plan)?;

    if !dry_run {
        print_success("Release completed successfully");
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

    print_section_header("Package Dependencies", SectionStyle::Primary);
    print_key_value("Package", &package);
    print_separator_with_spacing();

    print_key_value(
        "Depends on",
        &format!("{} packages", deps.len()),
    );
    print_package_list(&deps, "");
    println!();

    print_key_value(
        "Depended on by",
        &format!("{} packages", dependents.len()),
    );
    print_package_list(&dependents, "");
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
        print_section_header("Validation", SectionStyle::Success);
        print_success("All packages are valid");
        print_success("No circular dependencies detected");
        print_success("Dependency graph is valid");
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
        print_section_header("Available Tasks", SectionStyle::Primary);
        println!();

        if packages.is_empty() {
            use crate::formatting::print_warning;
            print_warning("No packages found");
        } else {
            let package_tasks: Vec<(String, Vec<String>)> = packages
                .into_iter()
                .map(|p| {
                    let task_names: Vec<String> = p.tasks.iter().map(|t| t.name.clone()).collect();
                    (p.name, task_names)
                })
                .collect();
            use crate::formatting::print_task_table;
            print_task_table(&package_tasks);
        }
        println!();
    }

    if show_cache_stats {
        print_cache_stats(&scanner);
    }

    Ok(())
}
