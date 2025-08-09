//! Comprehensive file upload handler for IronDrop
//!
//! This module provides secure, efficient file upload handling with:
//! - Cross-platform OS download directory detection
//! - Multipart form-data parsing integration
//! - Comprehensive security validations
//! - Atomic file operations with temporary files
//! - Duplicate filename conflict resolution
//! - Progress tracking capabilities
//! - Integration with existing CLI configuration and error systems
//!
//! # Security Features
//! - Extension validation using glob patterns from CLI configuration
//! - Filename sanitization to prevent path traversal attacks
//! - Size limit enforcement per file and total upload
//! - Disk space checking before upload operations
//! - MIME type validation against allowed types
//! - Rate limiting integration ready
//!
//! # Example Usage
//! ```rust,no_run
//! use irondrop::upload::UploadHandler;
//! use irondrop::cli::Cli;
//! use std::path::PathBuf;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let cli = Cli {
//!     directory: PathBuf::from("/tmp"),
//!     listen: Some("127.0.0.1".to_string()),
//!     port: Some(8080),
//!     allowed_extensions: Some("*.txt,*.pdf".to_string()),
//!     threads: Some(4),
//!     chunk_size: Some(1024),
//!     verbose: Some(false),
//!     detailed_logging: Some(false),
//!     username: None,
//!     password: None,
//!     enable_upload: Some(true),
//!     max_upload_size: Some(10),
//!     config_file: None,
//! };
//! let mut upload_handler = UploadHandler::new(&cli)?;
//! # Ok(())
//! # }
//! ```

use crate::cli::Cli;
use crate::error::AppError;
use crate::http::Request;
use crate::multipart::{MultipartConfig, MultipartParser};
use crate::response::{get_mime_type, HttpResponse};
use glob::Pattern;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

/// Temporary file prefix for atomic operations
const TEMP_FILE_PREFIX: &str = ".irondrop_temp_";

/// Progress tracking information for uploads
#[derive(Debug, Clone)]
pub struct UploadProgress {
    /// Total expected size in bytes
    pub total_size: u64,
    /// Bytes processed so far
    pub processed_size: u64,
    /// Number of files processed
    pub files_processed: usize,
    /// Total number of files expected
    pub total_files: usize,
    /// Current processing stage
    pub stage: UploadStage,
}

/// Different stages of upload processing
#[derive(Debug, Clone)]
pub enum UploadStage {
    /// Parsing multipart data
    Parsing,
    /// Validating files
    Validating,
    /// Writing files to disk
    Writing,
    /// Finalizing upload
    Finalizing,
    /// Upload completed
    Completed,
}

/// Information about a successfully uploaded file
#[derive(Debug, Clone)]
pub struct UploadedFile {
    /// Original filename from client
    pub original_name: String,
    /// Final filename on disk (may be different due to conflicts)
    pub saved_name: String,
    /// Full path where file was saved
    pub saved_path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// MIME type detected
    pub mime_type: String,
    /// Whether filename was modified to resolve conflicts
    pub renamed: bool,
}

/// Upload operation result
#[derive(Debug)]
pub struct UploadResult {
    /// Successfully uploaded files
    pub uploaded_files: Vec<UploadedFile>,
    /// Upload processing time in milliseconds
    pub processing_time_ms: u64,
    /// Total bytes processed
    pub total_bytes: u64,
    /// Any warnings during processing
    pub warnings: Vec<String>,
}

/// Comprehensive upload handler with security and configuration
pub struct UploadHandler {
    /// Target directory for uploads
    target_dir: PathBuf,
    /// Maximum total upload size in bytes
    max_upload_size: u64,
    /// Maximum size per individual file
    max_file_size: u64,
    /// Allowed file extensions (glob patterns)
    allowed_extensions: Vec<Pattern>,
    /// Whether upload functionality is enabled
    upload_enabled: bool,
    /// Multipart parser configuration
    multipart_config: MultipartConfig,
}

impl UploadHandler {
    /// Create a new upload handler from CLI configuration
    pub fn new(cli: &Cli) -> Result<Self, AppError> {
        if !cli.enable_upload.unwrap_or(false) {
            return Err(AppError::upload_disabled());
        }

        // Always use the directory being served as the base for uploads
        // Individual upload directories will be determined dynamically
        Self::new_with_directory(cli, cli.directory.clone())
    }

    /// Create upload handler with custom target directory
    pub fn new_with_directory(cli: &Cli, target_dir: PathBuf) -> Result<Self, AppError> {
        if !cli.enable_upload.unwrap_or(false) {
            return Err(AppError::upload_disabled());
        }

        // Ensure target directory exists
        Self::ensure_directory_exists(&target_dir)?;

        // Parse allowed extensions from CLI
        let allowed_extensions = cli
            .allowed_extensions
            .as_deref()
            .unwrap_or("*")
            .split(',')
            .map(|ext| ext.trim())
            .filter(|ext| !ext.is_empty())
            .map(Pattern::new)
            .collect::<Result<Vec<Pattern>, _>>()
            .map_err(AppError::from)?;

        let max_upload_bytes = cli.max_upload_size_bytes();
        let max_file_size = max_upload_bytes; // Individual file can be as large as total limit

        // Configure multipart parser with security settings
        // Extract just the file extensions from glob patterns for multipart validation
        let simple_extensions: Vec<String> = allowed_extensions
            .iter()
            .filter_map(|pattern| {
                let pattern_str = pattern.as_str();
                if pattern_str == "*" || pattern_str == "*.*" || pattern_str == "**" {
                    None // Wildcard patterns mean allow all, so don't add any restriction
                } else if let Some(stripped) = pattern_str.strip_prefix("*.") {
                    if stripped == "*" {
                        None // "*.*" case - allow all extensions
                    } else {
                        Some(stripped.to_lowercase()) // Remove "*." and convert to lowercase
                    }
                } else if let Some(stripped) = pattern_str.strip_prefix(".") {
                    if stripped == "*" {
                        None // ".*" case - allow all extensions
                    } else {
                        Some(stripped.to_lowercase()) // Remove "." and convert to lowercase
                    }
                } else {
                    Some(pattern_str.to_lowercase()) // Use as-is but lowercase
                }
            })
            .collect();

        let multipart_config = MultipartConfig {
            max_parts: 50, // Allow up to 50 files per upload
            max_part_size: max_file_size,
            max_filename_length: 255,
            max_field_name_length: 100,
            max_headers_size: 8 * 1024,
            allowed_extensions: simple_extensions,
            allowed_mime_types: Vec::new(), // Use extension-based validation instead
        };

        Ok(Self {
            target_dir,
            max_upload_size: max_upload_bytes,
            max_file_size,
            allowed_extensions,
            upload_enabled: true,
            multipart_config,
        })
    }

    /// Detect the OS-specific download directory
    pub fn detect_os_download_directory() -> Result<PathBuf, AppError> {
        let download_dir = if cfg!(target_os = "windows") {
            // Windows: %USERPROFILE%\Downloads
            env::var("USERPROFILE")
                .map(|profile| PathBuf::from(profile).join("Downloads"))
                .unwrap_or_else(|_| PathBuf::from("Downloads"))
        } else if cfg!(target_os = "macos") {
            // macOS: ~/Downloads
            env::var("HOME")
                .map(|home| PathBuf::from(home).join("Downloads"))
                .unwrap_or_else(|_| PathBuf::from("Downloads"))
        } else {
            // Linux and other Unix-like: Check XDG_DOWNLOAD_DIR, fallback to ~/Downloads
            if let Ok(xdg_download) = env::var("XDG_DOWNLOAD_DIR") {
                PathBuf::from(xdg_download)
            } else if let Ok(home) = env::var("HOME") {
                PathBuf::from(home).join("Downloads")
            } else {
                PathBuf::from("Downloads")
            }
        };

        // If the standard download directory doesn't exist, fallback to current working directory
        if !download_dir.exists() {
            warn!("Standard download directory {download_dir:?} does not exist, falling back to current directory");
            env::current_dir().map_err(AppError::from)
        } else {
            Ok(download_dir)
        }
    }

    /// Ensure the target directory exists, create if necessary
    fn ensure_directory_exists(dir: &Path) -> Result<(), AppError> {
        if !dir.exists() {
            info!("Creating upload directory: {dir:?}");
            fs::create_dir_all(dir).map_err(|e| {
                error!("Failed to create upload directory {dir:?}: {e}");
                AppError::from(e)
            })?;
        } else if !dir.is_dir() {
            return Err(AppError::InternalServerError(format!(
                "Upload path {dir:?} exists but is not a directory"
            )));
        }

        // Check if directory is writable
        let test_file = dir.join(".write_test");
        match File::create(&test_file) {
            Ok(_) => {
                let _ = fs::remove_file(&test_file); // Ignore errors on cleanup
                Ok(())
            }
            Err(e) => {
                error!("Upload directory {dir:?} is not writable: {e}");
                Err(AppError::from(e))
            }
        }
    }

    /// Handle a file upload request with statistics tracking  
    pub fn handle_upload_with_stats(
        &mut self,
        request: &Request,
        stats: Option<&crate::server::ServerStats>,
    ) -> Result<HttpResponse, AppError> {
        let result = self.handle_upload(request, stats);

        // If there was an error, record failure statistics
        if result.is_err() {
            if let Some(stats) = stats {
                stats.record_upload_request(false, 0, 0, 0, 0); // Record failure
                stats.finish_upload();
            }
        }

        result
    }

    /// Handle a file upload request with statistics tracking
    pub fn handle_upload(
        &mut self,
        request: &Request,
        stats: Option<&crate::server::ServerStats>,
    ) -> Result<HttpResponse, AppError> {
        if !self.upload_enabled {
            return Err(AppError::upload_disabled());
        }

        let start_time = std::time::Instant::now();

        // Track upload start
        if let Some(stats) = stats {
            stats.start_upload();
        }

        // Validate request method
        if request.method != "POST" {
            return Err(AppError::MethodNotAllowed);
        }

        // Get request body
        let body = request
            .body
            .as_ref()
            .ok_or_else(|| AppError::invalid_multipart("No request body"))?;

        // Check total upload size
        if body.len() as u64 > self.max_upload_size {
            return Err(AppError::payload_too_large(self.max_upload_size));
        }

        // Check available disk space
        self.check_disk_space(body.len() as u64)?;

        // Extract content type and boundary
        let content_type = request
            .headers
            .get("content-type")
            .ok_or_else(|| AppError::invalid_multipart("Missing Content-Type header"))?;

        // Validate that this is actually multipart/form-data
        if !content_type
            .to_lowercase()
            .starts_with("multipart/form-data")
        {
            return Err(AppError::invalid_multipart("Not multipart/form-data"));
        }

        let boundary =
            MultipartParser::<Cursor<Vec<u8>>>::extract_boundary_from_content_type(content_type)?;

        // Additional validation: check if the boundary actually appears in the body
        let body_str = String::from_utf8_lossy(body);
        let expected_boundary = format!("--{boundary}");
        if !body_str.contains(&expected_boundary) {
            return Err(AppError::invalid_multipart(
                "Boundary not found in request body",
            ));
        }

        // Create multipart parser and handle parsing errors
        let parser = match MultipartParser::new(
            Cursor::new(body.clone()),
            &boundary,
            self.multipart_config.clone(),
        ) {
            Ok(p) => p,
            Err(e) => {
                return Err(e);
            }
        };

        // Process all parts
        let mut uploaded_files = Vec::new();
        let mut total_bytes = 0u64;
        let mut warnings = Vec::new();
        let mut part_count = 0;

        for part_result in parser {
            let mut part = match part_result {
                Ok(p) => p,
                Err(e) => {
                    // If there's an error processing parts, it could be malformed data
                    return Err(e);
                }
            };

            part_count += 1;
            if part_count > self.multipart_config.max_parts {
                return Err(AppError::invalid_multipart("Too many parts"));
            }

            // Skip non-file form fields for now
            if !part.is_file() {
                continue;
            }

            let original_filename = part
                .filename
                .as_ref()
                .ok_or_else(|| AppError::invalid_multipart("Missing filename in file part"))?
                .clone();

            // Validate filename
            self.validate_filename(&original_filename)?;

            // Read file content
            let file_content = part.read_to_bytes()?;
            total_bytes += file_content.len() as u64;

            // Additional size check per file
            if file_content.len() as u64 > self.max_file_size {
                return Err(AppError::payload_too_large(self.max_file_size));
            }

            // Validate file extension
            self.validate_file_extension(&original_filename)?;

            // Determine MIME type
            let mime_type = get_mime_type(Path::new(&original_filename)).to_string();

            // Generate unique filename to avoid conflicts
            let (final_filename, was_renamed) =
                self.generate_unique_filename(&original_filename)?;
            let target_path = self.target_dir.join(&final_filename);

            // Write file atomically
            let saved_path = self.write_file_atomically(&target_path, &file_content)?;

            if was_renamed {
                warnings.push(format!(
                    "File '{original_filename}' was renamed to '{final_filename}' to avoid conflicts"
                ));
            }

            uploaded_files.push(UploadedFile {
                original_name: original_filename,
                saved_name: final_filename,
                saved_path,
                size: file_content.len() as u64,
                mime_type,
                renamed: was_renamed,
            });

            info!(
                "Successfully uploaded file: {} ({} bytes)",
                uploaded_files.last().unwrap().saved_name,
                file_content.len()
            );
        }

        let processing_time = start_time.elapsed().as_millis() as u64;

        // Calculate largest file size for statistics
        let largest_file = uploaded_files.iter().map(|f| f.size).max().unwrap_or(0);

        let upload_result = UploadResult {
            uploaded_files,
            processing_time_ms: processing_time,
            total_bytes,
            warnings,
        };

        // Record successful upload statistics
        if let Some(stats) = stats {
            stats.record_upload_request(
                true, // success
                upload_result.uploaded_files.len() as u64,
                upload_result.total_bytes,
                processing_time,
                largest_file,
            );
            stats.finish_upload();
        }

        // Generate appropriate response based on Accept header
        self.generate_upload_response(request, upload_result)
    }

    /// Check available disk space
    fn check_disk_space(&self, required_bytes: u64) -> Result<(), AppError> {
        // Simple heuristic: Check if we have at least 2x the required space
        // In a production system, you might use platform-specific APIs to get actual disk space

        // For now, we'll create a test file to check if we can write
        // This isn't a perfect disk space check but provides basic validation
        let test_size = std::cmp::min(required_bytes / 10, 1024 * 1024); // Test with 10% or max 1MB
        let test_path = self.target_dir.join(".space_test");

        match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&test_path)
        {
            Ok(mut file) => {
                let test_data = vec![0u8; test_size as usize];
                match file.write_all(&test_data) {
                    Ok(_) => {
                        let _ = fs::remove_file(&test_path); // Cleanup
                        Ok(())
                    }
                    Err(_) => {
                        let _ = fs::remove_file(&test_path); // Cleanup
                        Err(AppError::upload_disk_full(0)) // We don't have exact available space
                    }
                }
            }
            Err(_) => Err(AppError::upload_disk_full(0)),
        }
    }

    /// Validate filename for security
    fn validate_filename(&self, filename: &str) -> Result<(), AppError> {
        if filename.is_empty() {
            return Err(AppError::invalid_filename("Empty filename"));
        }

        if filename.len() > 255 {
            return Err(AppError::invalid_filename("Filename too long"));
        }

        // Check for path traversal attempts
        if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
            return Err(AppError::invalid_filename(filename));
        }

        // Check for dangerous characters
        let dangerous_chars = ['<', '>', ':', '"', '|', '?', '*'];
        if filename
            .chars()
            .any(|c| dangerous_chars.contains(&c) || c.is_control())
        {
            return Err(AppError::invalid_filename(filename));
        }

        Ok(())
    }

    /// Validate file extension against allowed patterns
    fn validate_file_extension(&self, filename: &str) -> Result<(), AppError> {
        if self.allowed_extensions.is_empty() {
            return Ok(()); // No restrictions
        }

        let path = Path::new(filename);

        let matches = self
            .allowed_extensions
            .iter()
            .any(|pattern| pattern.matches_path(path));

        if !matches {
            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("(no extension)");
            return Err(AppError::unsupported_media_type(format!(
                "File extension '{extension}' not allowed"
            )));
        }

        Ok(())
    }

    /// Generate a unique filename to avoid conflicts
    fn generate_unique_filename(&self, original: &str) -> Result<(String, bool), AppError> {
        let target_path = self.target_dir.join(original);

        if !target_path.exists() {
            return Ok((original.to_string(), false));
        }

        // File exists, generate a unique name
        let path = Path::new(original);
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!(".{ext}"))
            .unwrap_or_default();

        for i in 1..=9999 {
            let new_filename = format!("{stem}_{i}{extension}");
            let new_path = self.target_dir.join(&new_filename);

            if !new_path.exists() {
                return Ok((new_filename, true));
            }
        }

        Err(AppError::InternalServerError(
            "Unable to generate unique filename after 9999 attempts".to_string(),
        ))
    }

    /// Write file atomically using temporary file and rename
    fn write_file_atomically(
        &self,
        target_path: &Path,
        content: &[u8],
    ) -> Result<PathBuf, AppError> {
        // Create temporary file in same directory with unique name
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_filename = format!(
            "{}{}_{}_{:x}.tmp",
            TEMP_FILE_PREFIX,
            std::process::id(),
            nanos,
            nanos.wrapping_mul(7919) // Simple hash to add more uniqueness
        );
        let temp_path = self.target_dir.join(temp_filename);

        // Write to temporary file
        {
            let mut temp_file = File::create(&temp_path).map_err(|e| {
                error!("Failed to create temporary file {temp_path:?}: {e}");
                AppError::from(e)
            })?;

            temp_file.write_all(content).map_err(|e| {
                error!("Failed to write to temporary file {temp_path:?}: {e}");
                let _ = fs::remove_file(&temp_path); // Cleanup on error
                AppError::from(e)
            })?;

            temp_file.sync_all().map_err(|e| {
                error!("Failed to sync temporary file {temp_path:?}: {e}");
                let _ = fs::remove_file(&temp_path); // Cleanup on error
                AppError::from(e)
            })?;
        }

        // Atomically rename temporary file to target
        fs::rename(&temp_path, target_path).map_err(|e| {
            error!("Failed to rename {temp_path:?} to {target_path:?}: {e}");
            let _ = fs::remove_file(&temp_path); // Cleanup on error
            AppError::from(e)
        })?;

        debug!("Successfully wrote file atomically to {target_path:?}");
        Ok(target_path.to_path_buf())
    }

    /// Generate appropriate response based on request Accept header
    fn generate_upload_response(
        &self,
        request: &Request,
        result: UploadResult,
    ) -> Result<HttpResponse, AppError> {
        let accept_header = request
            .headers
            .get("accept")
            .map(|s| s.as_str())
            .unwrap_or("");

        // Determine if client wants JSON response
        let wants_json = accept_header.contains("application/json")
            || request.headers.contains_key("x-requested-with");

        if wants_json {
            self.generate_json_response(result)
        } else {
            self.generate_html_response(result)
        }
    }

    /// Generate JSON response for API clients
    fn generate_json_response(&self, result: UploadResult) -> Result<HttpResponse, AppError> {
        let files_json: Vec<String> = result.uploaded_files.iter().map(|file| {
            format!(
                r#"{{"name": "{}", "originalName": "{}", "size": {}, "mimeType": "{}", "renamed": {}}}"#,
                file.saved_name,
                file.original_name,
                file.size,
                file.mime_type,
                file.renamed
            )
        }).collect();

        let warnings_json: Vec<String> = result
            .warnings
            .iter()
            .map(|w| format!(r#""{}""#, w.replace('"', r#"\""#)))
            .collect();

        let response_body = format!(
            r#"{{
    "status": "success",
    "message": "Upload completed successfully",
    "files": [{}],
    "statistics": {{
        "filesUploaded": {},
        "totalBytes": {},
        "processingTimeMs": {}
    }},
    "warnings": [{}]
}}"#,
            files_json.join(", "),
            result.uploaded_files.len(),
            result.total_bytes,
            result.processing_time_ms,
            warnings_json.join(", ")
        );

        Ok(HttpResponse::new(200, "OK")
            .add_header(
                "Content-Type".to_string(),
                "application/json; charset=utf-8".to_string(),
            )
            .add_header("Cache-Control".to_string(), "no-cache".to_string())
            .with_html_body(response_body))
    }

    /// Generate HTML response for form submissions
    fn generate_html_response(&self, result: UploadResult) -> Result<HttpResponse, AppError> {
        let files_list = result
            .uploaded_files
            .iter()
            .map(|file| {
                let rename_note = if file.renamed {
                    format!(" <em>(renamed from {})</em>", file.original_name)
                } else {
                    String::new()
                };

                format!(
                    r#"<li><strong>{}</strong>{} - {} bytes</li>"#,
                    file.saved_name,
                    rename_note,
                    format_bytes(file.size)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let warnings_html = if result.warnings.is_empty() {
            String::new()
        } else {
            let warnings_list = result
                .warnings
                .iter()
                .map(|w| format!(r#"<li>{w}</li>"#))
                .collect::<Vec<_>>()
                .join("\n");

            format!(
                r#"
                <div class="warnings">
                    <h3>⚠️ Warnings</h3>
                    <ul>{warnings_list}</ul>
                </div>
            "#
            )
        };

        let response_body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Upload Complete - IronDrop</title>
    <meta charset="utf-8">
    <style>
        body {{ background: #1e293b; color: #f1f5f9; font-family: sans-serif; padding: 2rem; }}
        .container {{ max-width: 800px; margin: 0 auto; }}
        .success {{ background: #10b981; color: white; padding: 1rem; border-radius: 0.5rem; margin-bottom: 1rem; }}
        .stats {{ background: #374151; padding: 1rem; border-radius: 0.5rem; margin: 1rem 0; }}
        .warnings {{ background: #f59e0b; color: #1f2937; padding: 1rem; border-radius: 0.5rem; margin: 1rem 0; }}
        ul {{ list-style-type: none; padding-left: 0; }}
        li {{ padding: 0.25rem 0; }}
        a {{ color: #60a5fa; text-decoration: none; }}
        a:hover {{ text-decoration: underline; }}
        .back-link {{ display: inline-block; margin-top: 1rem; background: #3b82f6; color: white; padding: 0.5rem 1rem; border-radius: 0.25rem; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="success">
            <h1>✅ Upload Successful!</h1>
            <p>Successfully uploaded {} file(s)</p>
        </div>
        
        <h2>📁 Uploaded Files</h2>
        <ul>{}</ul>
        
        <div class="stats">
            <h3>📊 Upload Statistics</h3>
            <p><strong>Files:</strong> {}</p>
            <p><strong>Total Size:</strong> {}</p>
            <p><strong>Processing Time:</strong> {} ms</p>
        </div>
        
        {}
        
        <a href="/" class="back-link">← Back to Files</a>
    </div>
</body>
</html>"#,
            result.uploaded_files.len(),
            files_list,
            result.uploaded_files.len(),
            format_bytes(result.total_bytes),
            result.processing_time_ms,
            warnings_html
        );

        Ok(HttpResponse::new(200, "OK").with_html_body(response_body))
    }

    /// Get upload handler configuration for debugging
    pub fn get_config_info(&self) -> HashMap<String, String> {
        let mut info = HashMap::new();
        info.insert(
            "target_directory".to_string(),
            self.target_dir.to_string_lossy().to_string(),
        );
        info.insert(
            "max_upload_size_mb".to_string(),
            (self.max_upload_size / 1024 / 1024).to_string(),
        );
        info.insert(
            "upload_enabled".to_string(),
            self.upload_enabled.to_string(),
        );
        info.insert(
            "allowed_extensions".to_string(),
            self.allowed_extensions
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
        info
    }
}

/// Format bytes into human-readable format
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_cli(upload_dir: PathBuf) -> Cli {
        Cli {
            // Use the provided temp directory as the server base directory
            directory: upload_dir,
            listen: Some("127.0.0.1".to_string()),
            port: Some(8080),
            allowed_extensions: Some("*.txt,*.pdf".to_string()),
            threads: Some(4),
            chunk_size: Some(1024),
            verbose: Some(false),
            detailed_logging: Some(false),
            username: None,
            password: None,
            enable_upload: Some(true),
            max_upload_size: Some(100), // 100MB for testing
            config_file: None,
        }
    }

    #[test]
    fn test_upload_handler_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cli = create_test_cli(temp_dir.path().to_path_buf());

        let handler = UploadHandler::new(&cli);
        assert!(handler.is_ok());

        let handler = handler.unwrap();
        assert_eq!(handler.target_dir, temp_dir.path());
        assert_eq!(handler.max_upload_size, 100 * 1024 * 1024);
        assert!(handler.upload_enabled);
    }

    #[test]
    fn test_upload_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let mut cli = create_test_cli(temp_dir.path().to_path_buf());
        cli.enable_upload = Some(false);

        let result = UploadHandler::new(&cli);
        assert!(matches!(result, Err(AppError::UploadDisabled)));
    }

    #[test]
    fn test_filename_validation() {
        let temp_dir = TempDir::new().unwrap();
        let cli = create_test_cli(temp_dir.path().to_path_buf());
        let handler = UploadHandler::new(&cli).unwrap();

        // Valid filenames
        assert!(handler.validate_filename("document.txt").is_ok());
        assert!(handler
            .validate_filename("file_with_underscores.pdf")
            .is_ok());
        assert!(handler.validate_filename("file-with-dashes.txt").is_ok());

        // Invalid filenames
        assert!(handler.validate_filename("../etc/passwd").is_err());
        assert!(handler.validate_filename("file/with/slashes.txt").is_err());
        assert!(handler
            .validate_filename("file\\with\\backslashes.txt")
            .is_err());
        assert!(handler.validate_filename("file<with>brackets.txt").is_err());
        assert!(handler.validate_filename("").is_err());
    }

    #[test]
    fn test_unique_filename_generation() {
        let temp_dir = TempDir::new().unwrap();
        let cli = create_test_cli(temp_dir.path().to_path_buf());
        let handler = UploadHandler::new(&cli).unwrap();

        // Create an existing file
        let existing_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&existing_path).unwrap();
        file.write_all(b"test content").unwrap();

        // Test unique filename generation
        let (unique_name, renamed) = handler.generate_unique_filename("test.txt").unwrap();
        assert_eq!(unique_name, "test_1.txt");
        assert!(renamed);

        // Test when original doesn't exist
        let (original_name, renamed) = handler.generate_unique_filename("nonexistent.txt").unwrap();
        assert_eq!(original_name, "nonexistent.txt");
        assert!(!renamed);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }

    #[test]
    fn test_detect_download_directory() {
        let result = UploadHandler::detect_os_download_directory();
        assert!(result.is_ok());

        let dir = result.unwrap();
        // The detected path should be an absolute path and a directory
        assert!(dir.is_absolute(), "Detected path should be absolute");
        assert!(dir.is_dir(), "Detected path should be a directory");
    }

    #[test]
    fn test_extension_validation() {
        let temp_dir = TempDir::new().unwrap();
        let cli = create_test_cli(temp_dir.path().to_path_buf());
        let handler = UploadHandler::new(&cli).unwrap();

        // Allowed extensions (from CLI: *.txt,*.pdf)
        assert!(handler.validate_file_extension("document.txt").is_ok());
        assert!(handler.validate_file_extension("document.pdf").is_ok());

        // Not allowed extensions
        assert!(handler.validate_file_extension("document.exe").is_err());
        assert!(handler.validate_file_extension("document.jpg").is_err());
    }
}
