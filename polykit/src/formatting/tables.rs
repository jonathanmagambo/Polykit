//! Table formatting utilities using comfy-table.

use comfy_table::{Cell, Table};
use owo_colors::OwoColorize;

/// Prints a table of packages with their languages.
pub fn print_package_table(packages: &[(String, String)]) {
    let mut table = Table::new();
    table
        .set_header(vec![
            Cell::new("Package").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Language").add_attribute(comfy_table::Attribute::Bold),
        ])
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

    for (name, language) in packages {
        table.add_row(vec![
            Cell::new(name).fg(comfy_table::Color::White),
            Cell::new(language).fg(comfy_table::Color::DarkGrey),
        ]);
    }

    println!("{}", table);
}

/// Prints a simple list of packages (one per line).
pub fn print_package_list(packages: &[String], _label: &str) {
    if packages.is_empty() {
        println!("  {} {}", "→".cyan(), "(none)".bright_black());
        return;
    }

    for pkg in packages {
        println!("  {} {}", "→".cyan(), pkg.bold().white());
    }
}

/// Prints a table of tasks grouped by package.
pub fn print_task_table(packages: &[(String, Vec<String>)]) {
    let mut table = Table::new();
    table
        .set_header(vec![
            Cell::new("Package").add_attribute(comfy_table::Attribute::Bold),
            Cell::new("Tasks").add_attribute(comfy_table::Attribute::Bold),
        ])
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

    for (pkg_name, tasks) in packages {
        let tasks_str = if tasks.is_empty() {
            "(no tasks)".bright_black().to_string()
        } else {
            tasks.join(", ")
        };
        table.add_row(vec![
            Cell::new(pkg_name).fg(comfy_table::Color::White),
            Cell::new(tasks_str),
        ]);
    }

    println!("{}", table);
}

/// Prints a table with custom headers and rows.
#[allow(dead_code)]
pub fn print_custom_table(headers: Vec<&str>, rows: Vec<Vec<String>>) {
    let mut table = Table::new();
    let header_cells: Vec<Cell> = headers
        .iter()
        .map(|h| Cell::new(*h).add_attribute(comfy_table::Attribute::Bold))
        .collect();
    table
        .set_header(header_cells)
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

    for row in rows {
        let row_cells: Vec<Cell> = row.iter().map(|cell| Cell::new(cell.as_str())).collect();
        table.add_row(row_cells);
    }

    println!("{}", table);
}
