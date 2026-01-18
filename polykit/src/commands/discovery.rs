//! Discovery and inspection commands.

use std::path::PathBuf;

use anyhow::Result;
use comfy_table::{Cell, Table};
use polykit_core::{ChangeDetector, DependencyGraph};

use crate::formatting::{print_key_value, print_package_list, print_package_table, print_section_header, print_success, print_warning, SectionStyle};

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
        print_section_header("Scanning packages", SectionStyle::Primary);

        if packages.is_empty() {
            print_warning("No packages found");
        } else {
            print_key_value(
                "Found",
                &format!("{} packages", packages.len()),
            );
            println!();
            let package_list: Vec<(String, String)> = packages
                .into_iter()
                .map(|p| (p.name, p.language.as_str().to_string()))
                .collect();
            print_package_table(&package_list);
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
        print_section_header("Dependency Graph", SectionStyle::Primary);

        if order.is_empty() {
            print_warning("No packages found");
        } else {
            print_key_value(
                "Topological order",
                &format!("{} packages", order.len()),
            );
            println!();
            let mut table = Table::new();
            table
                .set_header(vec![
                    Cell::new("#").add_attribute(comfy_table::Attribute::Bold),
                    Cell::new("Package").add_attribute(comfy_table::Attribute::Bold),
                ])
                .load_preset(comfy_table::presets::UTF8_FULL)
                .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                .set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

            for (idx, pkg) in order.iter().enumerate() {
                table.add_row(vec![
                    Cell::new((idx + 1).to_string()).fg(comfy_table::Color::DarkGrey),
                    Cell::new(pkg).fg(comfy_table::Color::White),
                ]);
            }
            println!("{}", table);
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

    print_section_header("Affected Packages", SectionStyle::Primary);

    if affected.is_empty() {
        print_success("No affected packages");
    } else {
        print_warning(&format!("{} packages affected", affected.len()));
        println!();
        let affected_vec: Vec<String> = affected.into_iter().collect();
        print_package_list(&affected_vec, "");
    }
    println!();

    if show_cache_stats {
        print_cache_stats(&scanner);
    }

    Ok(())
}
