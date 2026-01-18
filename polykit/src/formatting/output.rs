//! General output formatting utilities.

use owo_colors::OwoColorize;

/// Prints a visual separator line.
#[allow(dead_code)]
pub fn print_separator() {
    println!("{}", "─".repeat(60).bright_black());
}

/// Prints a visual separator with spacing.
pub fn print_separator_with_spacing() {
    println!();
    println!("{}", "─".repeat(60).bright_black());
    println!();
}

/// Prints information in a boxed format.
#[allow(dead_code)]
pub fn print_boxed_info(title: &str, content: &str) {
    let title_colored = title.cyan().bold().to_string();
    let separator = "─".repeat(50);
    println!("┌─ {} {}", title_colored, separator.bright_black());
    println!("│ {}", content);
    println!("└{}", "─".repeat(60).bright_black());
}

/// Prints a summary box with statistics.
pub fn print_summary_box(title: &str, items: &[(&str, &str)]) {
    let title_colored = title.cyan().bold().to_string();
    let separator = "─".repeat(50);
    println!("┌─ {} {}", title_colored, separator.bright_black());
    for (key, value) in items {
        println!("│ {} {}", key.bright_black().bold(), value.bold().white());
    }
    println!("└{}", "─".repeat(60).bright_black());
}

/// Prints a key-value pair with consistent formatting.
pub fn print_key_value(key: &str, value: &str) {
    println!("  {} {}", key.bright_black().bold(), value.bold().white());
}

/// Prints a key-value pair with colored value.
#[allow(dead_code)]
pub fn print_key_value_colored(key: &str, value: &str, color: fn(&str) -> String) {
    println!("  {} {}", key.bright_black().bold(), color(value));
}

/// Formats duration in a human-readable way.
pub fn format_duration(seconds: f64) -> String {
    if seconds < 1.0 {
        format!("{:.0}ms", seconds * 1000.0)
    } else if seconds < 60.0 {
        format!("{:.2}s", seconds)
    } else {
        let mins = (seconds / 60.0) as u64;
        let secs = seconds % 60.0;
        format!("{}m {:.1}s", mins, secs)
    }
}
