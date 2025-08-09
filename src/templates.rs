//! Template loading and rendering system for modular HTML

use crate::error::AppError;
use std::collections::HashMap;

// Embed templates at compile time
const DIRECTORY_INDEX_HTML: &str = include_str!("../templates/directory/index.html");
const DIRECTORY_STYLES_CSS: &str = include_str!("../templates/directory/styles.css");
const DIRECTORY_SCRIPT_JS: &str = include_str!("../templates/directory/script.js");
const ERROR_PAGE_HTML: &str = include_str!("../templates/error/page.html");
const ERROR_STYLES_CSS: &str = include_str!("../templates/error/styles.css");
const ERROR_SCRIPT_JS: &str = include_str!("../templates/error/script.js");
const UPLOAD_PAGE_HTML: &str = include_str!("../templates/upload/page.html");
const UPLOAD_STYLES_CSS: &str = include_str!("../templates/upload/styles.css");
const UPLOAD_SCRIPT_JS: &str = include_str!("../templates/upload/script.js");
const UPLOAD_FORM_HTML: &str = include_str!("../templates/upload/form.html");

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
    templates: HashMap<String, String>,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine {
    /// Create a new template engine with embedded templates
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        // Load embedded templates
        templates.insert(
            "directory_index".to_string(),
            DIRECTORY_INDEX_HTML.to_string(),
        );
        templates.insert("error_page".to_string(), ERROR_PAGE_HTML.to_string());
        templates.insert("upload_page".to_string(), UPLOAD_PAGE_HTML.to_string());
        templates.insert("upload_form".to_string(), UPLOAD_FORM_HTML.to_string());

        Self { templates }
    }

    /// Get appropriate icon SVG based on file extension
    fn get_file_icon(filename: &str) -> &'static str {
        let extension = filename.split('.').last().unwrap_or("").to_lowercase();
        match extension.as_str() {
            // Archive formats
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => ZIP_ICON_SVG,
            // Image formats
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "svg" | "ico" | "tiff" => IMAGE_ICON_SVG,
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

    /// Render a template with variables, supporting conditionals
    pub fn render(
        &self,
        template_name: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String, AppError> {
        let template = self.templates.get(template_name).ok_or_else(|| {
            AppError::InternalServerError(format!("Template '{template_name}' not found"))
        })?;

        let mut rendered = template.clone();

        // Handle conditional blocks {{#if VARIABLE}}...{{/if}}
        rendered = self.process_conditionals(&rendered, variables);

        // Replace variables in the format {{VARIABLE_NAME}}
        for (key, value) in variables {
            let placeholder = format!("{{{{{key}}}}}");
            rendered = rendered.replace(&placeholder, value);
        }

        Ok(rendered)
    }

    /// Process conditional blocks in templates
    fn process_conditionals(&self, template: &str, variables: &HashMap<String, String>) -> String {
        let mut result = template.to_string();

        // Find and process {{#if VARIABLE}}...{{/if}} blocks
        while let Some(start) = result.find("{{#if ") {
            if let Some(var_end) = result[start..].find("}}") {
                let var_start = start + 6; // "{{#if ".len()
                let variable = &result[var_start..start + var_end];

                if let Some(block_end) = result.find("{{/if}}") {
                    let block_start = start + var_end + 2; // "}}"
                    let block_content = &result[block_start..block_end];

                    // Check if variable is true
                    let should_include = variables
                        .get(variable)
                        .map(|v| v == "true")
                        .unwrap_or(false);

                    let replacement = if should_include {
                        block_content.to_string()
                    } else {
                        String::new()
                    };

                    // Replace entire conditional block
                    let full_block = &result[start..block_end + 7]; // "{{/if}}".len()
                    result = result.replace(full_block, &replacement);
                } else {
                    break; // Malformed template
                }
            } else {
                break; // Malformed template
            }
        }

        result
    }

    /// Generate directory listing HTML using template
    pub fn render_directory_listing(
        &self,
        path: &str,
        entries: &[(String, String, String)], // (name, size, date)
        entry_count: usize,
        upload_enabled: bool,
        current_path: &str,
    ) -> Result<String, AppError> {
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

        // Add template variables for icons
        variables.insert("FOLDER_ICON".to_string(), FOLDER_ICON_SVG.to_string());
        variables.insert("FILE_ICON".to_string(), FILE_ICON_SVG.to_string());
        variables.insert("BACK_ICON".to_string(), BACK_ICON_SVG.to_string());

        // Generate entries data as JSON-like structure for template
        let mut entries_data = Vec::new();
        
        // Add parent directory link if not at root
        if path != "/" && !path.is_empty() {
            entries_data.push(format!(
                r#"{{"href": "../", "type": "back", "name": "Back", "size": "", "date": ""}}"#
            ));
        }

        // Add file/directory entries
        for (name, size, date) in entries {
            let is_directory = name.ends_with('/');
            let entry_type = if is_directory { "directory" } else { "file" };
            let display_name = if is_directory {
                name.trim_end_matches('/')
            } else {
                name
            };

            entries_data.push(format!(
                r#"{{"href": "{}", "type": "{}", "name": "{}", "size": "{}", "date": "{}"}}"#,
                percent_encode(name),
                entry_type,
                html_escape(display_name),
                size,
                date
            ));
        }

        // For now, still generate HTML but use template variables for icons
        let mut entries_html = String::new();
        
        // Add parent directory link if not at root (as table row)
        if path != "/" && !path.is_empty() {
            entries_html.push_str(&format!(
                r#"<tr>
                    <td>
                        <a href="../" class="file-link">
                            <span class="file-type directory">{}</span>
                            <span class="name">Back</span>
                        </a>
                    </td>
                    <td class="size" colspan="2"></td>
                </tr>"#,
                BACK_ICON_SVG
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

        self.render("directory_index", &variables)
    }

    /// Generate error page HTML using template
    pub fn render_error_page(
        &self,
        status_code: u16,
        status_text: &str,
        description: &str,
    ) -> Result<String, AppError> {
        let mut variables = HashMap::new();
        variables.insert("ERROR_CODE".to_string(), status_code.to_string());
        variables.insert("ERROR_MESSAGE".to_string(), status_text.to_string());
        variables.insert("ERROR_DESCRIPTION".to_string(), description.to_string());

        // Add additional variables for new template
        variables.insert(
            "REQUEST_ID".to_string(),
            format!(
                "REQ-{:08X}",
                std::ptr::addr_of!(variables) as usize & 0xFFFFFFFF
            ),
        );
        variables.insert(
            "TIMESTAMP".to_string(),
            format!("{:?}", std::time::SystemTime::now()),
        );

        self.render("error_page", &variables)
    }

    /// Generate upload page HTML using template
    pub fn render_upload_page(&self, path: &str) -> Result<String, AppError> {
        let mut variables = HashMap::new();
        variables.insert("PATH".to_string(), path.to_string());

        self.render("upload_page", &variables)
    }

    /// Get upload form component HTML
    pub fn get_upload_form(&self) -> Result<String, AppError> {
        self.render("upload_form", &HashMap::new())
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
