//! Enhanced progress bar formatting.

use indicatif::{ProgressBar, ProgressStyle};

/// Creates a styled progress bar with enhanced visual appearance.
pub fn create_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    let style = ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/.blue}] {pos}/{len} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .progress_chars("█▉▊▋▌▍▎▏  ");
    pb.set_style(style);
    pb
}

/// Creates a progress bar with custom message template.
#[allow(dead_code)]
pub fn create_progress_bar_with_template(total: u64, template: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    let style = ProgressStyle::default_bar()
        .template(template)
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .progress_chars("█▉▊▋▌▍▎▏  ");
    pb.set_style(style);
    pb
}
