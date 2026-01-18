//! Beautiful CLI formatting utilities.
//!
//! This module provides a unified system for formatting CLI output with
//! consistent colors, visual hierarchy, and professional styling.

mod headers;
mod output;
mod progress;
mod status;
mod tables;

pub use headers::{print_section_header, SectionStyle};
pub use output::{format_duration, print_key_value, print_separator_with_spacing, print_summary_box};
pub use progress::create_progress_bar;
pub use status::{print_error, print_success, print_warning, Status};
pub use tables::{print_package_list, print_package_table, print_task_table};
