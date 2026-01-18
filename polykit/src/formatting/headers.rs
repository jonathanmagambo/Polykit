//! Section header formatting with visual separators.

use owo_colors::OwoColorize;

/// Style options for section headers.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum SectionStyle {
    Primary,
    Secondary,
    Success,
    Warning,
    Error,
}

impl SectionStyle {
    fn colorize(&self, text: &str) -> String {
        match self {
            SectionStyle::Primary => text.cyan().bold().to_string(),
            SectionStyle::Secondary => text.bright_black().bold().to_string(),
            SectionStyle::Success => text.green().bold().to_string(),
            SectionStyle::Warning => text.yellow().bold().to_string(),
            SectionStyle::Error => text.red().bold().to_string(),
        }
    }
}

/// Prints a section header with visual separator.
pub fn print_section_header(title: &str, style: SectionStyle) {
    let colored_title = style.colorize(title);
    println!("{}", colored_title);
    println!();
}

/// Prints a section header with a subtitle.
#[allow(dead_code)]
pub fn print_section_header_with_subtitle(title: &str, subtitle: &str, style: SectionStyle) {
    let colored_title = style.colorize(title);
    let colored_subtitle = subtitle.bright_black();
    println!("{}", colored_title);
    println!("  {}", colored_subtitle);
    println!();
}
