// SPDX-License-Identifier: MIT

//! Template loading and rendering system for modular HTML

use crate::error::AppError;
use log::{debug, trace};
use std::collections::HashMap;
use std::sync::OnceLock;

// Embed templates at compile time
// Base template
const BASE_HTML: &str = include_str!("../templates/common/base.html");

// Content templates
const DIRECTORY_CONTENT_HTML: &str = include_str!("../templates/directory/content.html");
const ERROR_CONTENT_HTML: &str = include_str!("../templates/error/content.html");
const UPLOAD_CONTENT_HTML: &str = include_str!("../templates/upload/content.html");
const UPLOAD_SUCCESS_HTML: &str = include_str!("../templates/upload/success.html");

// CSS and JS assets
const DIRECTORY_STYLES_CSS: &str = include_str!("../templates/directory/styles.css");
const DIRECTORY_SCRIPT_JS: &str = include_str!("../templates/directory/script.js");
const ERROR_STYLES_CSS: &str = include_str!("../templates/error/styles.css");
const ERROR_SCRIPT_JS: &str = include_str!("../templates/error/script.js");
const UPLOAD_STYLES_CSS: &str = include_str!("../templates/upload/styles.css");
const UPLOAD_SCRIPT_JS: &str = include_str!("../templates/upload/script.js");
const UPLOAD_FORM_HTML: &str = include_str!("../templates/upload/form.html");

// Monitor templates
const MONITOR_CONTENT_HTML: &str = include_str!("../templates/monitor/content.html");
const MONITOR_STYLES_CSS: &str = include_str!("../templates/monitor/styles.css");
const MONITOR_SCRIPT_JS: &str = include_str!("../templates/monitor/script.js");

// Common base styles
const BASE_CSS: &str = include_str!("../templates/common/base.css");

// Embed favicon files at compile time
const FAVICON_ICO: &[u8] = include_bytes!("../favicon.ico");
const FAVICON_16X16_PNG: &[u8] = include_bytes!("../favicon-16x16.png");
const FAVICON_32X32_PNG: &[u8] = include_bytes!("../favicon-32x32.png");
// Logo image
const IRONDROP_LOGO_PNG: &[u8] = include_bytes!("../irondrop-logo.png");

// Icon partials
const FOLDER_ICON_SVG: &str = include_str!("../templates/directory/folder_icon.svg");
const FILE_ICON_SVG: &str = include_str!("../templates/directory/file_icon.svg");
const BACK_ICON_SVG: &str = include_str!("../templates/directory/back_icon.svg");
const ZIP_ICON_SVG: &str = include_str!("../templates/directory/zip_icon.svg");
const IMAGE_ICON_SVG: &str = include_str!("../templates/directory/image_icon.svg");
const VIDEO_ICON_SVG: &str = include_str!("../templates/directory/video_icon.svg");

/// Template loader and renderer for modular HTML templates
pub struct TemplateEngine {
    templates: HashMap<&'static str, &'static str>,
}

static TEMPLATE_ENGINE: OnceLock<TemplateEngine> = OnceLock::new();

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine {
    /// Create a new template engine with embedded templates
    pub fn new() -> Self {
        let mut templates: HashMap<&'static str, &'static str> = HashMap::new();

        // Load base template
        templates.insert("base", BASE_HTML);

        // Load content templates
        templates.insert("directory_content", DIRECTORY_CONTENT_HTML);
        templates.insert("error_content", ERROR_CONTENT_HTML);
        templates.insert("upload_content", UPLOAD_CONTENT_HTML);
        templates.insert("upload_success", UPLOAD_SUCCESS_HTML);
        templates.insert("upload_form", UPLOAD_FORM_HTML);
        templates.insert("monitor_content", MONITOR_CONTENT_HTML);

        Self { templates }
    }

    /// Global singleton instance to avoid per-request allocations
    pub fn global() -> &'static Self {
        TEMPLATE_ENGINE.get_or_init(Self::new)
    }

    /// Get appropriate icon SVG based on file extension
    fn get_file_icon(filename: &str) -> &'static str {
        let extension = filename.split('.').next_back().unwrap_or("").to_lowercase();
        match extension.as_str() {
            // Archive formats
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => ZIP_ICON_SVG,
            // Image formats
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "svg" | "ico" | "tiff" => {
                IMAGE_ICON_SVG
            }
            // Video formats
            "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" => VIDEO_ICON_SVG,
            // Default file icon
            _ => FILE_ICON_SVG,
        }
    }

    /// Load all templates - now uses embedded templates
    pub fn load_all_templates(&mut self) -> Result<(), AppError> {
        // Templates are already loaded in new(), this is kept for compatibility
        Ok(())
    }

    /// Get embedded static asset content
    pub fn get_static_asset(&self, path: &str) -> Option<(&'static str, &'static str)> {
        match path {
            // Common base styles
            "common/base.css" => Some((BASE_CSS, "text/css")),
            // Directory assets
            "directory/styles.css" => Some((DIRECTORY_STYLES_CSS, "text/css")),
            "directory/script.js" => Some((DIRECTORY_SCRIPT_JS, "application/javascript")),
            // Error assets
            "error/styles.css" => Some((ERROR_STYLES_CSS, "text/css")),
            "error/script.js" => Some((ERROR_SCRIPT_JS, "application/javascript")),
            // Upload assets
            "upload/styles.css" => Some((UPLOAD_STYLES_CSS, "text/css")),
            "upload/script.js" => Some((UPLOAD_SCRIPT_JS, "application/javascript")),
            // Monitor assets
            "monitor/styles.css" => Some((MONITOR_STYLES_CSS, "text/css")),
            "monitor/script.js" => Some((MONITOR_SCRIPT_JS, "application/javascript")),
            _ => None,
        }
    }

    /// Get embedded favicon as binary data
    pub fn get_favicon(&self, path: &str) -> Option<(&'static [u8], &'static str)> {
        match path {
            "favicon.ico" => Some((FAVICON_ICO, "image/x-icon")),
            "favicon-16x16.png" => Some((FAVICON_16X16_PNG, "image/png")),
            "favicon-32x32.png" => Some((FAVICON_32X32_PNG, "image/png")),
            "irondrop-logo.png" => Some((IRONDROP_LOGO_PNG, "image/png")),
            _ => None,
        }
    }

    /// Render a page using the base template system
    pub fn render_page(
        &self,
        content_template: &str,
        page_title: &str,
        page_styles: &str,
        page_scripts: &str,
        header_actions: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String, AppError> {
        // First render the content template
        let content = self.render(content_template, variables)?;

        // Create variables for the base template
        let mut base_variables = variables.clone();
        base_variables.insert("PAGE_TITLE".to_string(), page_title.to_string());
        base_variables.insert("PAGE_STYLES".to_string(), page_styles.to_string());
        base_variables.insert("PAGE_SCRIPTS".to_string(), page_scripts.to_string());
        base_variables.insert("HEADER_ACTIONS".to_string(), header_actions.to_string());
        base_variables.insert("PAGE_CONTENT".to_string(), content);
        base_variables.insert("VERSION".to_string(), crate::VERSION.to_string());

        // Render the base template
        self.render("base", &base_variables)
    }

    /// Helper method to render directory page
    pub fn render_directory_page(
        &self,
        variables: &HashMap<String, String>,
    ) -> Result<String, AppError> {
        let default_path = "/".to_string();
        let raw_path = variables.get("PATH").unwrap_or(&default_path);
        // Clean up path for display: remove leading/trailing slashes, show "Root" for empty
        let page_title = if raw_path == "/" || raw_path.is_empty() {
            "Root".to_string()
        } else {
            raw_path
                .trim_start_matches('/')
                .trim_end_matches('/')
                .to_string()
        };
        let page_styles =
            r#"<link rel="stylesheet" href="/_irondrop/static/directory/styles.css">"#;
        let page_scripts = r#"<script src="/_irondrop/static/directory/script.js"></script>"#;

        // Build header actions based on upload status
        let header_actions = if variables
            .get("UPLOAD_ENABLED")
            .map(|v| v == "true")
            .unwrap_or(false)
        {
            let suffix = variables
                .get("QUERY_UPLOAD_SUFFIX")
                .unwrap_or(&String::new())
                .clone();
            format!(
                r#"<a href="/_irondrop/upload{suffix}" class="btn btn-light">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                        <polyline points="17,8 12,3 7,8" />
                        <line x1="12" y1="3" x2="12" y2="15" />
                    </svg>
                    Upload Files
                </a>"#
            )
        } else {
            String::new()
        };

        // Add cleaned path for display in the directory header
        let display_title = if raw_path == "/" || raw_path.is_empty() {
            "Root".to_string()
        } else {
            raw_path.trim_end_matches('/').to_string()
        };

        // Create a mutable copy of variables and add the display title
        let mut enhanced_variables = variables.clone();
        enhanced_variables.insert("DISPLAY_TITLE".to_string(), display_title);

        self.render_page(
            "directory_content",
            &page_title,
            page_styles,
            page_scripts,
            &header_actions,
            &enhanced_variables,
        )
    }

    /// Helper method to render error page
    pub fn render_error_page_new(
        &self,
        error_code: u16,
        error_message: &str,
        error_description: &str,
    ) -> Result<String, AppError> {
        let mut variables = HashMap::new();
        variables.insert("ERROR_CODE".to_string(), error_code.to_string());
        variables.insert("ERROR_MESSAGE".to_string(), error_message.to_string());
        variables.insert(
            "ERROR_DESCRIPTION".to_string(),
            error_description.to_string(),
        );

        // Generate request ID and timestamp
        let request_id = format!(
            "req_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        let timestamp = {
            use std::time::{SystemTime, UNIX_EPOCH};
            let since_epoch = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            // Convert to UTC time components
            const SECONDS_IN_DAY: u64 = 86400;
            const SECONDS_IN_HOUR: u64 = 3600;
            const SECONDS_IN_MINUTE: u64 = 60;

            let days_since_epoch = since_epoch / SECONDS_IN_DAY;
            let remaining_seconds = since_epoch % SECONDS_IN_DAY;

            let hours = remaining_seconds / SECONDS_IN_HOUR;
            let minutes = (remaining_seconds % SECONDS_IN_HOUR) / SECONDS_IN_MINUTE;
            let seconds = remaining_seconds % SECONDS_IN_MINUTE;

            // Simple epoch to date conversion (approximate)
            // Start from 1970-01-01 and add days
            let mut year = 1970;
            let mut remaining_days = days_since_epoch;

            // Handle leap years (simplified)
            while remaining_days >= 365 {
                let days_in_year = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                    366
                } else {
                    365
                };

                if remaining_days >= days_in_year {
                    remaining_days -= days_in_year;
                    year += 1;
                } else {
                    break;
                }
            }

            // Simplified month/day calculation
            let month = (remaining_days / 31) + 1;
            let day = (remaining_days % 31) + 1;

            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
                year,
                month.min(12),
                day.max(1),
                hours,
                minutes,
                seconds
            )
        };
        variables.insert("REQUEST_ID".to_string(), request_id);
        variables.insert("TIMESTAMP".to_string(), timestamp);
        variables.insert("VERSION".to_string(), crate::VERSION.to_string());

        let page_title = format!("{error_code} {error_message}");
        let page_styles = r#"<link rel="stylesheet" href="/_irondrop/static/error/styles.css">"#;
        let page_scripts = r#"<script src="/_irondrop/static/error/script.js"></script>"#;
        let header_actions = ""; // No actions on error page

        self.render_page(
            "error_content",
            &page_title,
            page_styles,
            page_scripts,
            header_actions,
            &variables,
        )
    }

    /// Helper method to render upload page
    pub fn render_upload_page_new(&self, path: &str) -> Result<String, AppError> {
        let mut variables = HashMap::new();
        variables.insert("PATH".to_string(), path.to_string());

        let page_title = format!("Upload to {path}");
        let page_styles = r#"<link rel="stylesheet" href="/_irondrop/static/upload/styles.css">"#;
        let page_scripts = r#"<script src="/_irondrop/static/upload/script.js"></script>"#;

        // Header action is back to directory
        let header_actions = format!(
            r#"<a href="{path}" class="btn btn-light" id="backToDir">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="m12 19-7-7 7-7" />
                        <path d="m19 12H5" />
                    </svg>
                    Back to Directory
                </a>"#
        );

        self.render_page(
            "upload_content",
            &page_title,
            page_styles,
            page_scripts,
            &header_actions,
            &variables,
        )
    }

    /// Render a template with variables, supporting conditionals (optimized single-scan passes)
    pub fn render(
        &self,
        template_name: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String, AppError> {
        let template = self.templates.get(template_name).ok_or_else(|| {
            AppError::InternalServerError(format!("Template '{template_name}' not found"))
        })?;

        // First pass: process simple {{#if VAR}}...{{/if}} blocks (no nesting)
        let conditional_processed = self.process_conditionals_optimized(template, variables);

        // Second pass: substitute variables {{VAR}} in a single scan
        let rendered = Self::substitute_variables_single_pass(&conditional_processed, variables);

        Ok(rendered)
    }

    /// Optimized conditional processing with pre-reserved buffer (no nested ifs)
    fn process_conditionals_optimized(
        &self,
        template: &str,
        variables: &HashMap<String, String>,
    ) -> String {
        let mut out: Vec<u8> = Vec::with_capacity(template.len());
        let bytes = template.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            // Look for b"{{#if "
            if i + 6 <= bytes.len() && bytes[i..].starts_with(b"{{#if ") {
                let var_start = i + 6;
                // Find closing b"}}" for variable name
                if let Some(close_pos_rel) = bytes[var_start..].windows(2).position(|w| w == b"}}") {
                    let var_end = var_start + close_pos_rel;
                    // SAFETY: variable names are ASCII in our templates; validate UTF-8
                    let var_name = std::str::from_utf8(&bytes[var_start..var_end]).unwrap_or("");

                    // Find matching b"{{/if}}" from after }}
                    let block_search_start = var_end + 2;
                    if let Some(end_rel) = bytes[block_search_start..]
                        .windows(7)
                        .position(|w| w == b"{{/if}}")
                    {
                        let block_end = block_search_start + end_rel;
                        let include = variables
                            .get(var_name)
                            .map(|v| v == "true")
                            .unwrap_or(false);
                        if include {
                            out.extend_from_slice(&bytes[block_search_start..block_end]);
                        }
                        i = block_end + 7; // skip entire if-block
                        continue;
                    }
                }
            }
            // Default: copy one byte
            out.push(bytes[i]);
            i += 1;
        }
        String::from_utf8(out).unwrap_or_else(|_| template.to_string())
    }

    /// Single-pass variable substitution for placeholders {{VAR}}
    fn substitute_variables_single_pass(
        input: &str,
        variables: &HashMap<String, String>,
    ) -> String {
        // Heuristic capacity: base + total value sizes capped
        let extra: usize = variables.values().map(|v| v.len()).sum::<usize>().min(64 * 1024);
        let mut out: Vec<u8> = Vec::with_capacity(input.len() + extra);
        let bytes = input.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if i + 2 <= bytes.len() && bytes[i..].starts_with(b"{{") {
                // Find closing }}
                if let Some(close_rel) = bytes[i + 2..].windows(2).position(|w| w == b"}}") {
                    let name_bytes = &bytes[i + 2..i + 2 + close_rel];
                    if let Ok(name) = std::str::from_utf8(name_bytes)
                        && let Some(val) = variables.get(name)
                    {
                        out.extend_from_slice(val.as_bytes());
                    }
                    i = i + 2 + close_rel + 2;
                    continue;
                }
            }
            out.push(bytes[i]);
            i += 1;
        }
        String::from_utf8(out).unwrap_or_else(|_| input.to_string())
    }

    /// Generate directory listing HTML using base template system
    pub fn render_directory_listing(
        &self,
        path: &str,
        entries: &[(String, String, String)], // (name, size, date)
        entry_count: usize,
        upload_enabled: bool,
        current_path: &str,
    ) -> Result<String, AppError> {
        debug!(
            "Rendering directory listing: path='{}', entries={}, upload_enabled={}",
            path, entry_count, upload_enabled
        );
        trace!("Directory listing current path: {}", current_path);
        let mut variables = HashMap::new();
        variables.insert("PATH".to_string(), path.to_string());
        variables.insert("ENTRY_COUNT".to_string(), entry_count.to_string());
        variables.insert("UPLOAD_ENABLED".to_string(), upload_enabled.to_string());
        variables.insert("CURRENT_PATH".to_string(), current_path.to_string());

        // Build a clean query suffix for the upload link (omit for root)
        let clean = current_path.trim_start_matches('/').trim_end_matches('/');
        let query_suffix = if clean.is_empty() {
            String::new()
        } else {
            // Percent-encode minimal set for URLs
            let encoded = percent_encode(clean);
            format!("?upload_to={encoded}")
        };
        variables.insert("QUERY_UPLOAD_SUFFIX".to_string(), query_suffix);

        // Generate entries HTML
        let mut entries_html = String::new();

        // Add parent directory link if not at root (as table row)
        if path != "/" && !path.is_empty() {
            entries_html.push_str(&format!(
                r#"<tr>
                    <td>
                        <a href="../" class="file-link">
                            <span class="file-type directory">{BACK_ICON_SVG}</span>
                            <span class="name">Back</span>
                        </a>
                    </td>
                    <td class="size" colspan="2"></td>
                </tr>"#
            ));
        }

        // Add file/directory entries with template-based icons
        for (name, size, date) in entries {
            let is_directory = name.ends_with('/');
            let type_class = if is_directory { "directory" } else { "file" };
            let display_name = if is_directory {
                name.trim_end_matches('/')
            } else {
                name
            };

            let icon_svg = if is_directory {
                FOLDER_ICON_SVG
            } else {
                Self::get_file_icon(name)
            };

            entries_html.push_str(&format!(
                r#"<tr>
                    <td>
                        <a href="{}" class="file-link">
                            <span class="file-type {}">{}</span>
                            <span class="name">{}</span>
                        </a>
                    </td>
                    <td class="size">{}</td>
                    <td class="date">{}</td>
                </tr>"#,
                percent_encode(name),
                type_class,
                icon_svg,
                html_escape(display_name),
                size,
                date
            ));
        }

        variables.insert("ENTRIES".to_string(), entries_html);

        // Use the new base template system
        self.render_directory_page(&variables)
    }
    /// Generate error page HTML using base template system
    pub fn render_error_page(
        &self,
        status_code: u16,
        status_text: &str,
        description: &str,
    ) -> Result<String, AppError> {
        debug!(
            "Rendering error page: {} {} - {}",
            status_code, status_text, description
        );
        // Use the new base template system
        self.render_error_page_new(status_code, status_text, description)
    }

    /// Generate upload page HTML using base template system
    pub fn render_upload_page(&self, path: &str) -> Result<String, AppError> {
        debug!("Rendering upload page for path: {}", path);
        // Use the new base template system
        self.render_upload_page_new(path)
    }

    /// Render monitor page using the base template system
    pub fn render_monitor_page(&self) -> Result<String, AppError> {
        debug!("Rendering monitor page");
        let page_title = "Monitor";
        let page_styles = r#"<link rel="stylesheet" href="/_irondrop/static/monitor/styles.css">"#;
        let page_scripts = r#"
            <script src="https://cdn.jsdelivr.net/npm/chart.js@3.9.1/dist/chart.min.js"></script>
            <script src="/_irondrop/static/monitor/script.js"></script>
        "#;
        let header_actions = r#"<a href="/" class="btn btn-light">← Back to Files</a>"#;

        let variables = HashMap::new();

        self.render_page(
            "monitor_content",
            page_title,
            page_styles,
            page_scripts,
            header_actions,
            &variables,
        )
    }

    /// Get upload form component HTML
    pub fn get_upload_form(&self) -> Result<String, AppError> {
        self.render("upload_form", &HashMap::new())
    }

    /// Render upload success page
    pub fn render_upload_success(
        &self,
        file_count: usize,
        total_size: &str,
        processing_time: u64,
        files_list: &str,
        warnings: &str,
    ) -> Result<String, AppError> {
        let page_title = "Upload Successful";
        let page_styles = r#"<link rel="stylesheet" href="/_irondrop/static/upload/styles.css">"#;
        let page_scripts = "";
        let header_actions = r#"<a href="/" class="btn btn-light">← Back to Files</a>"#;

        let mut variables = HashMap::new();
        variables.insert("FILE_COUNT".to_string(), file_count.to_string());
        variables.insert("TOTAL_SIZE".to_string(), total_size.to_string());
        variables.insert("PROCESSING_TIME".to_string(), processing_time.to_string());
        variables.insert("FILES_LIST".to_string(), files_list.to_string());
        variables.insert("WARNINGS".to_string(), warnings.to_string());

        self.render_page(
            "upload_success",
            page_title,
            page_styles,
            page_scripts,
            header_actions,
            &variables,
        )
    }
}

/// Simple percent encoding for URLs
fn percent_encode(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '"' => "%22".to_string(),
            '#' => "%23".to_string(),
            '%' => "%25".to_string(),
            '<' => "%3C".to_string(),
            '>' => "%3E".to_string(),
            '?' => "%3F".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

/// Simple HTML entity escaping
fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Get human-friendly error descriptions
pub fn get_error_description(status_code: u16) -> &'static str {
    match status_code {
        400 => "The request could not be understood due to malformed syntax.",
        401 => "Authentication is required to access this resource.",
        403 => "Access to this resource is forbidden.",
        404 => "The requested file or directory could not be found.",
        405 => "The request method is not allowed for this resource.",
        500 => "An internal server error occurred while processing your request.",
        _ => "An unexpected error occurred while processing your request.",
    }
}
