// SPDX-License-Identifier: MIT

use crate::config::Config;
use crate::error::AppError;
use crate::templates::TemplateEngine;
use crate::utils::is_hidden_file;
use log::{debug, trace};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

/// Enhanced directory listing using modular templates - dark mode only
pub fn generate_directory_listing(
    path: &Path,
    request_path: &str,
    config: Option<&Config>,
    page: usize,
) -> Result<String, AppError> {
    debug!("Generating directory listing for: '{}'", path.display());
    trace!("Request path: '{}'", request_path);

    let mut entries = Vec::new();

    // Collect and sort entries using lazy metadata lookup (super fast, low memory)
    trace!("Reading directory entries from: {}", path.display());
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let file_name = entry.file_name().into_string().unwrap_or_default();

        if is_hidden_file(&file_name) {
            continue;
        }

        entries.push((entry.path(), file_name, file_type.is_dir()));
    }

    debug!("Found {} entries, sorting...", entries.len());
    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| {
        let a_is_dir = a.2;
        let b_is_dir = b.2;

        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => cmp_case_insensitive_ascii(&a.1, &b.1),
        }
    });

    let display_path = if request_path.is_empty() || request_path == "/" {
        "/"
    } else {
        request_path
    };

    debug!("Preparing {} entries for template rendering", entries.len());
    let total_count = entries.len();
    let limit = 1000;
    let total_pages = total_count.div_ceil(limit);
    let safe_page = page.max(1).min(total_pages.max(1));
    let offset = (safe_page - 1) * limit;

    let page_entries: Vec<_> = entries.into_iter().skip(offset).take(limit).collect();
    let mut template_entries = Vec::with_capacity(page_entries.len());

    for (entry_path, file_name, is_dir) in page_entries {
        let link_name = if is_dir {
            format!("{file_name}/")
        } else {
            file_name.clone()
        };

        // Lazy metadata fetch for only the current page's files
        let metadata_res = std::fs::metadata(&entry_path);

        let size = if is_dir {
            "-".to_string()
        } else {
            metadata_res
                .as_ref()
                .map(|m| format_file_size(m.len()))
                .unwrap_or_else(|_| "-".to_string())
        };

        let modified = metadata_res
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|duration| format_timestamp(duration.as_secs()))
            .unwrap_or_else(|| "-".to_string());

        template_entries.push((link_name, size, modified));
    }

    debug!("Creating template engine and rendering directory listing");
    let engine = TemplateEngine::global();

    let upload_enabled = config.map(|c| c.enable_upload).unwrap_or(false);
    engine.render_directory_listing(
        display_path,
        &template_entries,
        total_count,
        upload_enabled,
        request_path,
        safe_page,
        total_pages,
    )
}

/// Format file size in human-readable format
fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: u64 = 1024;

    if size == 0 {
        return "0 B".to_string();
    }

    let mut size_f = size as f64;
    let mut unit_index = 0;

    while size_f >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
        size_f /= THRESHOLD as f64;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size_f, UNITS[unit_index])
    }
}

fn cmp_case_insensitive_ascii(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_it = a.bytes();
    let mut b_it = b.bytes();
    loop {
        match (a_it.next(), b_it.next()) {
            (Some(x), Some(y)) => {
                let xl = x.to_ascii_lowercase();
                let yl = y.to_ascii_lowercase();
                match xl.cmp(&yl) {
                    std::cmp::Ordering::Equal => {}
                    other => return other,
                }
            }
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, None) => return std::cmp::Ordering::Equal,
        }
    }
}

/// Format Unix timestamp to human-readable date
fn format_timestamp(timestamp: u64) -> String {
    // Simple date formatting without external dependencies
    let seconds_per_minute = 60;
    let seconds_per_hour = 3600;
    let seconds_per_day = 86400;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let age = now.saturating_sub(timestamp);

    if age < seconds_per_minute {
        "Just now".to_string()
    } else if age < seconds_per_hour {
        let minutes = age / seconds_per_minute;
        format!("{minutes} min ago")
    } else if age < seconds_per_day {
        let hours = age / seconds_per_hour;
        format!("{hours} hr ago")
    } else if age < seconds_per_day * 30 {
        let days = age / seconds_per_day;
        format!("{days} days ago")
    } else {
        // Rough date calculation for older files
        let days_since_epoch = timestamp / seconds_per_day;
        let year = 1970 + days_since_epoch / 365;
        let day_of_year = days_since_epoch % 365;
        let month = (day_of_year / 30) + 1;
        let day = (day_of_year % 30) + 1;
        format!("{:04}-{:02}-{:02}", year, month.min(12), day.min(31))
    }
}
