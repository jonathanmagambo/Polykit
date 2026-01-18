//! Status indicators and message formatting.

use owo_colors::OwoColorize;

/// Status types for consistent formatting.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Status {
    Success,
    Error,
    Warning,
    Info,
}

impl Status {
    /// Returns the symbol for this status.
    pub fn symbol(&self) -> &'static str {
        match self {
            Status::Success => "✓",
            Status::Error => "✗",
            Status::Warning => "⚠",
            Status::Info => "→",
        }
    }

    /// Returns the colored symbol for this status.
    pub fn colored_symbol(&self) -> String {
        match self {
            Status::Success => self.symbol().green().to_string(),
            Status::Error => self.symbol().red().to_string(),
            Status::Warning => self.symbol().yellow().to_string(),
            Status::Info => self.symbol().cyan().to_string(),
        }
    }

    /// Formats a status message with symbol and color.
    pub fn format(&self, message: &str) -> String {
        format!("{} {}", self.colored_symbol(), self.colorize_text(message))
    }

    fn colorize_text(&self, text: &str) -> String {
        match self {
            Status::Success => text.green().bold().to_string(),
            Status::Error => text.red().bold().to_string(),
            Status::Warning => text.yellow().bold().to_string(),
            Status::Info => text.cyan().to_string(),
        }
    }
}

/// Prints a success message.
pub fn print_success(message: &str) {
    println!("  {}", Status::Success.format(message));
}

/// Prints a success banner for major achievements.
#[allow(dead_code)]
pub fn print_success_banner(message: &str) {
    println!();
    println!("  {}", "╔═══════════════════════════════════════════════════════════╗".green());
    println!("  {} {}", "║".green(), message.green().bold());
    println!("  {}", "╚═══════════════════════════════════════════════════════════╝".green());
    println!();
}

/// Prints an error message.
pub fn print_error(message: &str) {
    println!("  {}", Status::Error.format(message));
}

/// Prints a warning message.
pub fn print_warning(message: &str) {
    println!("  {}", Status::Warning.format(message));
}

/// Prints an info message.
#[allow(dead_code)]
pub fn print_info(message: &str) {
    println!("  {}", Status::Info.format(message));
}

/// Formats a status label (like "OK", "FAILED", etc.) with appropriate color.
#[allow(dead_code)]
pub fn format_status_label(label: &str, status: Status) -> String {
    match status {
        Status::Success => label.green().bold().to_string(),
        Status::Error => label.red().bold().to_string(),
        Status::Warning => label.yellow().bold().to_string(),
        Status::Info => label.cyan().bold().to_string(),
    }
}
