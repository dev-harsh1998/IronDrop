use crate::config::Config;
use crate::error::AppError;
use crate::templates::TemplateEngine;
use log::{debug, trace};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Enhanced directory listing using modular templates - dark mode only
pub fn generate_directory_listing(
    path: &Path,
    request_path: &str,
    config: Option<&Config>,
) -> Result<String, AppError> {
    debug!("Generating directory listing for: '{}'", path.display());
    trace!("Request path: '{}'", request_path);

    let mut entries = Vec::new();

    // Collect and sort entries
    trace!("Reading directory entries from: {}", path.display());
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let file_name = entry.file_name().into_string().unwrap_or_default();

        trace!(
            "Found entry: {} ({})",
            file_name,
            if metadata.is_dir() {
                "directory"
            } else {
                "file"
            }
        );
        entries.push((entry.path(), file_name, metadata));
    }

    debug!("Found {} entries, sorting...", entries.len());
    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| {
        let a_is_dir = a.2.is_dir();
        let b_is_dir = b.2.is_dir();

        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.1.to_lowercase().cmp(&b.1.to_lowercase()),
        }
    });

    let display_path = if request_path.is_empty() || request_path == "/" {
        "/"
    } else {
        request_path
    };

    debug!("Preparing {} entries for template rendering", entries.len());
    // Prepare entries data for template
    let mut template_entries = Vec::new();

    for (_entry_path, file_name, metadata) in entries {
        let is_dir = metadata.is_dir();
        let link_name = if is_dir {
            format!("{file_name}/")
        } else {
            file_name.clone()
        };

        let size = if is_dir {
            "-".to_string()
        } else {
            format_file_size(metadata.len())
        };

        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|duration| {
                let timestamp = duration.as_secs();
                format_timestamp(timestamp)
            })
            .unwrap_or_else(|| "-".to_string());

        template_entries.push((link_name, size, modified));
    }

    debug!("Creating template engine and rendering directory listing");
    // Create template engine with embedded templates
    let engine = TemplateEngine::new();

    // Render using template with conditional upload button
    trace!(
        "Upload enabled: {}",
        config.map(|c| c.enable_upload).unwrap_or(false)
    );
    engine.render_directory_listing(
        display_path,
        &template_entries,
        template_entries.len(),
        config.map(|c| c.enable_upload).unwrap_or(false),
        request_path,
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

/// Holds details about a file to be streamed.
pub struct FileDetails {
    pub path: PathBuf,
    pub file: File,
    pub size: u64,
    pub chunk_size: usize,
}

impl FileDetails {
    pub fn new(path: PathBuf, chunk_size: usize) -> Result<Self, io::Error> {
        debug!("Opening file for streaming: {}", path.display());
        trace!("Chunk size: {} bytes", chunk_size);

        let file = File::open(&path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        debug!(
            "File opened successfully: {} bytes, chunk size: {}",
            size, chunk_size
        );
        trace!(
            "File metadata - size: {}, is_file: {}",
            size,
            metadata.is_file()
        );

        Ok(FileDetails {
            path,
            file,
            size,
            chunk_size,
        })
    }
}
