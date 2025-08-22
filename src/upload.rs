// SPDX-License-Identifier: MIT

//! Direct file upload handler for IronDrop
//!
//! This module provides a simplified, efficient direct upload system that:
//! - Processes raw HTTP body data without multipart parsing
//! - Uses a 2MB threshold for memory vs disk streaming
//! - Provides comprehensive security validations
//! - Supports filename extraction from URL path or headers
//! - Implements atomic file operations with temporary files
//! - Includes progress tracking capabilities
//!
//! # Design Philosophy
//!
//! This implementation removes all multipart parsing complexity and focuses on:
//! - Direct binary data streaming
//! - Memory efficiency for large files
//! - Simple, robust error handling
//! - Security-first approach
//!
//! # Example Usage
//!
//! The direct upload handler processes raw binary uploads without multipart parsing,
//! providing constant memory usage regardless of file size.

use crate::cli::Cli;
use crate::error::AppError;
use crate::http::{Request, RequestBody};
use crate::response::{HttpResponse, get_mime_type};
use crate::templates::TemplateEngine;
use glob::Pattern;
use log::{debug, error, info, trace, warn};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Memory threshold: files <= 2MB processed in memory, >2MB streamed to disk
const MEMORY_THRESHOLD: u64 = 2 * 1024 * 1024; // 2MB

/// Temporary file prefix for atomic operations
const TEMP_FILE_PREFIX: &str = ".irondrop_temp_";

/// Buffer size for streaming operations
const STREAM_BUFFER_SIZE: usize = 64 * 1024; // 64KB

/// Progress tracking information for uploads
#[derive(Debug, Clone)]
pub struct UploadProgress {
    /// Total expected size in bytes
    pub total_size: u64,
    /// Bytes processed so far
    pub processed_size: u64,
    /// Current processing stage
    pub stage: UploadStage,
}

/// Different stages of upload processing
#[derive(Debug, Clone, PartialEq)]
pub enum UploadStage {
    /// Receiving upload data
    Receiving,
    /// Validating file
    Validating,
    /// Writing file to disk
    Writing,
    /// Finalizing upload
    Finalizing,
    /// Upload completed
    Completed,
}

/// Information about a successfully uploaded file
#[derive(Debug, Clone)]
pub struct UploadedFile {
    /// Original filename (from URL path or header)
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
    /// Successfully uploaded file
    pub uploaded_file: UploadedFile,
    /// Upload processing time in milliseconds
    pub processing_time_ms: u64,
    /// Any warnings during processing
    pub warnings: Vec<String>,
}

/// Direct upload handler with security and configuration
pub struct DirectUploadHandler {
    /// Target directory for uploads
    target_dir: PathBuf,
    /// Maximum upload size in bytes
    max_upload_size: u64,
    /// Allowed file extensions (glob patterns)
    allowed_extensions: Vec<Pattern>,
    /// Whether upload functionality is enabled
    upload_enabled: bool,
}

impl DirectUploadHandler {
    /// Create a new direct upload handler from CLI configuration
    pub fn new(cli: &Cli) -> Result<Self, AppError> {
        if !cli.enable_upload.unwrap_or(false) {
            return Err(AppError::upload_disabled());
        }

        // Always use the directory being served as the base for uploads
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

        Ok(Self {
            target_dir,
            max_upload_size: max_upload_bytes,
            allowed_extensions,
            upload_enabled: true,
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
            warn!(
                "Standard download directory {download_dir:?} does not exist, falling back to current directory"
            );
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

    /// Handle a direct file upload request with statistics tracking
    pub fn handle_upload_with_stats(
        &mut self,
        request: &Request,
        stats: Option<&crate::server::ServerStats>,
    ) -> Result<HttpResponse, AppError> {
        debug!(
            "Starting upload handling to directory: {}",
            self.target_dir.display()
        );
        trace!(
            "Upload request method: {}, path: {}",
            request.method, request.path
        );

        let result = self.handle_upload(request, stats);

        // If there was an error, record failure statistics
        if result.is_err() {
            if let Some(stats) = stats {
                stats.record_upload_request(false, 0, 0, 0, 0); // Record failure
                stats.finish_upload();
            }
            debug!("Upload error type: {:?}", result.as_ref().err());
        } else {
            trace!("Upload processing completed without errors");
        }

        result
    }

    /// Handle a direct file upload request
    pub fn handle_upload(
        &mut self,
        request: &Request,
        stats: Option<&crate::server::ServerStats>,
    ) -> Result<HttpResponse, AppError> {
        debug!(
            "Starting upload processing for request: {} {}",
            request.method, request.path
        );
        trace!(
            "Upload handler config - max_size: {} bytes, enabled: {}",
            self.max_upload_size, self.upload_enabled
        );

        if !self.upload_enabled {
            warn!("Upload attempt rejected - uploads are disabled");
            return Err(AppError::upload_disabled());
        }
        debug!("Upload enabled check passed");

        let start_time = std::time::Instant::now();

        // Track upload start
        if let Some(stats) = stats {
            stats.start_upload();
        }

        // Validate request method
        if request.method != "POST" && request.method != "PUT" {
            debug!(
                "Invalid method for upload: {}, expected POST or PUT",
                request.method
            );
            return Err(AppError::MethodNotAllowed);
        }

        trace!("Request method validation passed");

        // Get request body
        let body = request.body.as_ref().ok_or_else(|| {
            debug!("Missing request body in upload request");
            AppError::BadRequest
        })?;

        debug!(
            "Request body found: {} bytes",
            match body {
                RequestBody::Memory(data) => data.len(),
                RequestBody::File { size, .. } => *size as usize,
            }
        );
        trace!(
            "Body type: {}",
            match body {
                RequestBody::Memory(_) => "memory",
                RequestBody::File { .. } => "file",
            }
        );
        trace!("Request body validation passed");

        // Check total upload size
        let body_size = match body {
            RequestBody::Memory(data) => data.len() as u64,
            RequestBody::File { size, .. } => *size,
        };

        debug!(
            "Upload body size: {} bytes (limit: {} bytes)",
            body_size, self.max_upload_size
        );

        if body_size > self.max_upload_size {
            warn!(
                "Upload rejected - size {} exceeds limit of {} bytes",
                body_size, self.max_upload_size
            );
            return Err(AppError::payload_too_large(self.max_upload_size));
        }
        debug!(
            "Upload size check passed: {} bytes (limit: {})",
            body_size, self.max_upload_size
        );
        trace!("Upload size validation passed");

        // Extract filename from URL path or Content-Disposition header
        let filename = self.extract_filename(request)?;
        debug!("Extracted filename: '{}'", filename);

        // Validate filename
        debug!("Validating filename: '{}'", filename);
        self.validate_filename(&filename)?;
        trace!("Filename validation passed");

        // Validate file extension
        self.validate_file_extension(&filename)?;
        debug!("Filename validation passed");
        trace!("File extension validation passed");

        // Check available disk space
        debug!("Checking disk space for {} bytes", body_size);
        self.check_disk_space(body_size)?;
        debug!("Disk space check passed");

        // Process upload based on body type and size
        let uploaded_file = if body_size <= MEMORY_THRESHOLD {
            debug!(
                "Processing upload in memory (size: {} <= threshold: {})",
                body_size, MEMORY_THRESHOLD
            );
            self.handle_memory_upload(body, &filename)?
        } else {
            debug!(
                "Processing upload with streaming (size: {} > threshold: {})",
                body_size, MEMORY_THRESHOLD
            );
            self.handle_streaming_upload(body, &filename)?
        };

        let processing_time = start_time.elapsed().as_millis() as u64;

        debug!(
            "Upload result - renamed: {}, mime_type: {}, path: {}",
            uploaded_file.renamed,
            uploaded_file.mime_type,
            uploaded_file.saved_path.display()
        );

        let upload_result = UploadResult {
            uploaded_file,
            processing_time_ms: processing_time,
            warnings: Vec::new(),
        };

        // Record successful upload statistics
        if let Some(stats) = stats {
            stats.record_upload_request(
                true, // success
                1,    // file count
                upload_result.uploaded_file.size,
                processing_time,
                upload_result.uploaded_file.size, // largest file is the only file
            );
            stats.finish_upload();
        }

        // Generate appropriate response based on Accept header
        self.generate_upload_response(request, upload_result)
    }

    /// Extract filename from URL path or headers
    fn extract_filename(&self, request: &Request) -> Result<String, AppError> {
        // First, try to get filename from Content-Disposition header
        if let Some(content_disposition) = request.headers.get("content-disposition") {
            if let Some(filename) = Self::parse_filename_from_disposition(content_disposition) {
                return Ok(filename);
            }
        }

        // Next, try to get filename from custom X-Filename header
        if let Some(filename) = request.headers.get("x-filename") {
            if !filename.trim().is_empty() {
                return Ok(filename.trim().to_string());
            }
        }

        // Finally, extract from URL path (last segment after /, excluding query params)
        let path_without_query = request.path.split('?').next().unwrap_or(&request.path);
        let path_segments: Vec<&str> = path_without_query.split('/').collect();
        if let Some(last_segment) = path_segments.last() {
            if !last_segment.is_empty() && last_segment.contains('.') {
                return Ok(last_segment.to_string());
            }
        }

        // If no filename found anywhere, generate a default one
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(format!("upload_{}.bin", timestamp))
    }

    /// Parse filename from Content-Disposition header
    fn parse_filename_from_disposition(disposition: &str) -> Option<String> {
        for part in disposition.split(';') {
            let part = part.trim();
            if part.to_lowercase().starts_with("filename=") {
                let filename_part = &part[9..]; // Skip "filename="
                let filename = filename_part.trim_matches('"').trim();
                if !filename.is_empty() {
                    return Some(filename.to_string());
                }
            }
        }
        None
    }

    /// Handle uploads that fit in memory (≤2MB)
    fn handle_memory_upload(
        &mut self,
        body: &RequestBody,
        filename: &str,
    ) -> Result<UploadedFile, AppError> {
        debug!("Processing memory upload for file: {}", filename);

        let data = match body {
            RequestBody::Memory(data) => data,
            RequestBody::File { path, .. } => {
                // If body is in file but small enough for memory processing,
                // read it into memory for simpler handling
                return self.handle_file_based_upload(path, filename);
            }
        };

        // Generate unique filename to avoid conflicts
        let (final_filename, was_renamed) = self.generate_unique_filename(filename)?;
        debug!(
            "Generated filename: '{}' (renamed: {})",
            final_filename, was_renamed
        );
        let target_path = self.target_dir.join(&final_filename);
        trace!("Target path: {}", target_path.display());

        // Create temporary file for atomic write
        let temp_filename = format!(
            "{}{}_{}_{:x}.tmp",
            TEMP_FILE_PREFIX,
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            data.len() // Use data length as part of unique identifier
        );
        let temp_path = self.target_dir.join(&temp_filename);

        // Write data to temporary file
        debug!(
            "Writing {} bytes to temporary file: {}",
            data.len(),
            temp_path.display()
        );
        {
            let mut temp_file = File::create(&temp_path).map_err(|e| {
                error!("Failed to create temporary file {temp_path:?}: {e}");
                AppError::from(e)
            })?;

            temp_file.write_all(data).map_err(|e| {
                error!("Failed to write data to temporary file {temp_path:?}: {e}");
                let _ = fs::remove_file(&temp_path); // Cleanup on error
                AppError::from(e)
            })?;

            temp_file.sync_all().map_err(|e| {
                error!("Failed to sync temporary file {temp_path:?}: {e}");
                let _ = fs::remove_file(&temp_path); // Cleanup on error
                AppError::from(e)
            })?;
        }

        // Atomically rename temporary file to final location
        debug!("Atomically moving temporary file to final location");
        fs::rename(&temp_path, &target_path).map_err(|e| {
            error!("Failed to rename {temp_path:?} to {target_path:?}: {e}");
            let _ = fs::remove_file(&temp_path); // Cleanup on error
            AppError::from(e)
        })?;
        trace!("File successfully moved to: {}", target_path.display());

        // Determine MIME type
        let mime_type = get_mime_type(&target_path).to_string();
        trace!("Detected MIME type: {}", mime_type);

        info!(
            "Successfully uploaded file: {} ({} bytes) to {}",
            final_filename,
            data.len(),
            target_path.display()
        );

        Ok(UploadedFile {
            original_name: filename.to_string(),
            saved_name: final_filename,
            saved_path: target_path,
            size: data.len() as u64,
            mime_type,
            renamed: was_renamed,
        })
    }

    /// Handle uploads that are streamed to disk (>2MB)
    fn handle_streaming_upload(
        &mut self,
        body: &RequestBody,
        filename: &str,
    ) -> Result<UploadedFile, AppError> {
        debug!("Processing streaming upload for file: {}", filename);

        match body {
            RequestBody::Memory(_) => {
                // This shouldn't happen due to size checks, but handle gracefully
                return self.handle_memory_upload(body, filename);
            }
            RequestBody::File { path, size: _ } => self.handle_file_based_upload(path, filename),
        }
    }

    /// Handle uploads from file (used for both small files read from disk and large streaming files)
    fn handle_file_based_upload(
        &mut self,
        source_path: &PathBuf,
        filename: &str,
    ) -> Result<UploadedFile, AppError> {
        debug!(
            "Processing file-based upload: {} -> {}",
            source_path.display(),
            filename
        );

        // Generate unique filename to avoid conflicts
        let (final_filename, was_renamed) = self.generate_unique_filename(filename)?;
        debug!(
            "Generated filename: '{}' (renamed: {})",
            final_filename, was_renamed
        );
        let target_path = self.target_dir.join(&final_filename);
        trace!("Target path: {}", target_path.display());

        // Create temporary file for atomic operation
        let temp_filename = format!(
            "{}{}_{}_{:x}.tmp",
            TEMP_FILE_PREFIX,
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            source_path.to_string_lossy().len() // Use path length as part of unique identifier
        );
        let temp_path = self.target_dir.join(&temp_filename);

        // Stream copy from source to temporary file
        debug!(
            "Starting streaming copy from {} to {}",
            source_path.display(),
            temp_path.display()
        );
        let file_size = {
            let source_file = File::open(source_path).map_err(|e| {
                error!("Failed to open source file {source_path:?}: {e}");
                AppError::from(e)
            })?;

            let temp_file = File::create(&temp_path).map_err(|e| {
                error!("Failed to create temporary file {temp_path:?}: {e}");
                AppError::from(e)
            })?;

            // Use buffered streams for better performance
            let mut reader = BufReader::new(source_file);
            let mut writer = BufWriter::new(temp_file);

            let mut buffer = vec![0u8; STREAM_BUFFER_SIZE];
            let mut total_bytes = 0u64;
            trace!("Using buffer size: {} bytes", STREAM_BUFFER_SIZE);

            loop {
                let bytes_read = reader.read(&mut buffer).map_err(|e| {
                    error!("Failed to read from source file {source_path:?}: {e}");
                    let _ = fs::remove_file(&temp_path); // Cleanup on error
                    AppError::from(e)
                })?;

                if bytes_read == 0 {
                    break; // EOF
                }

                writer.write_all(&buffer[..bytes_read]).map_err(|e| {
                    error!("Failed to write to temporary file {temp_path:?}: {e}");
                    let _ = fs::remove_file(&temp_path); // Cleanup on error
                    AppError::from(e)
                })?;

                total_bytes += bytes_read as u64;

                // Log progress for large files
                if total_bytes % (1024 * 1024) == 0 || total_bytes < 1024 * 1024 {
                    trace!("Streamed {} bytes so far", total_bytes);
                }

                // Check size limit during streaming
                if total_bytes > self.max_upload_size {
                    warn!(
                        "Streaming upload exceeded size limit: {} > {}",
                        total_bytes, self.max_upload_size
                    );
                    let _ = fs::remove_file(&temp_path); // Cleanup
                    return Err(AppError::payload_too_large(self.max_upload_size));
                }
            }

            // Ensure all data is written
            writer.flush().map_err(|e| {
                error!("Failed to flush temporary file {temp_path:?}: {e}");
                let _ = fs::remove_file(&temp_path); // Cleanup on error
                AppError::from(e)
            })?;

            // Sync to disk
            writer
                .into_inner()
                .map_err(|e| {
                    error!("Failed to finalize temporary file {temp_path:?}: {e}");
                    let _ = fs::remove_file(&temp_path); // Cleanup on error
                    AppError::from(e.into_error())
                })?
                .sync_all()
                .map_err(|e| {
                    error!("Failed to sync temporary file {temp_path:?}: {e}");
                    let _ = fs::remove_file(&temp_path); // Cleanup on error
                    AppError::from(e)
                })?;

            total_bytes
        };

        // Atomically rename temporary file to final location
        fs::rename(&temp_path, &target_path).map_err(|e| {
            error!("Failed to rename {temp_path:?} to {target_path:?}: {e}");
            let _ = fs::remove_file(&temp_path); // Cleanup on error
            AppError::from(e)
        })?;

        // Determine MIME type
        let mime_type = get_mime_type(&target_path).to_string();

        info!(
            "Successfully uploaded file: {} ({} bytes) to {}",
            final_filename,
            file_size,
            target_path.display()
        );

        Ok(UploadedFile {
            original_name: filename.to_string(),
            saved_name: final_filename,
            saved_path: target_path,
            size: file_size,
            mime_type,
            renamed: was_renamed,
        })
    }

    /// Check available disk space
    fn check_disk_space(&self, required_bytes: u64) -> Result<(), AppError> {
        // Simple heuristic: Check if we can create a test file
        // In a production system, you might use platform-specific APIs to get actual disk space

        let test_size = std::cmp::min(required_bytes / 100, 1024 * 1024); // Test with 1% or max 1MB
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
        let file = &result.uploaded_file;

        let response_body = format!(
            r#"{{
    "status": "success",
    "message": "Upload completed successfully",
    "file": {{
        "name": "{}",
        "originalName": "{}",
        "size": {},
        "mimeType": "{}",
        "renamed": {}
    }},
    "statistics": {{
        "processingTimeMs": {}
    }},
    "warnings": []
}}"#,
            file.saved_name,
            file.original_name,
            file.size,
            file.mime_type,
            file.renamed,
            result.processing_time_ms
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
        let file = &result.uploaded_file;

        let rename_note = if file.renamed {
            format!(" <em>(renamed from {})</em>", file.original_name)
        } else {
            String::new()
        };

        let files_list = format!(
            r#"<li><strong>{}</strong>{} - {} bytes</li>"#,
            file.saved_name,
            rename_note,
            format_bytes(file.size)
        );

        // Use the template engine
        let template_engine = TemplateEngine::new();
        let response_body = template_engine.render_upload_success(
            1, // one file uploaded
            &format_bytes(file.size),
            result.processing_time_ms,
            &files_list,
            "", // no warnings
        )?;

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
        info.insert(
            "memory_threshold_mb".to_string(),
            (MEMORY_THRESHOLD / 1024 / 1024).to_string(),
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
            log_file: None,
        }
    }

    #[test]
    fn test_upload_handler_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cli = create_test_cli(temp_dir.path().to_path_buf());

        let handler = DirectUploadHandler::new(&cli);
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

        let result = DirectUploadHandler::new(&cli);
        assert!(matches!(result, Err(AppError::UploadDisabled)));
    }

    #[test]
    fn test_filename_validation() {
        let temp_dir = TempDir::new().unwrap();
        let cli = create_test_cli(temp_dir.path().to_path_buf());
        let handler = DirectUploadHandler::new(&cli).unwrap();

        // Valid filenames
        assert!(handler.validate_filename("document.txt").is_ok());
        assert!(
            handler
                .validate_filename("file_with_underscores.pdf")
                .is_ok()
        );
        assert!(handler.validate_filename("file-with-dashes.txt").is_ok());

        // Invalid filenames
        assert!(handler.validate_filename("../etc/passwd").is_err());
        assert!(handler.validate_filename("file/with/slashes.txt").is_err());
        assert!(
            handler
                .validate_filename("file\\with\\backslashes.txt")
                .is_err()
        );
        assert!(handler.validate_filename("file<with>brackets.txt").is_err());
        assert!(handler.validate_filename("").is_err());
    }

    #[test]
    fn test_unique_filename_generation() {
        let temp_dir = TempDir::new().unwrap();
        let cli = create_test_cli(temp_dir.path().to_path_buf());
        let handler = DirectUploadHandler::new(&cli).unwrap();

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
        let result = DirectUploadHandler::detect_os_download_directory();
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
        let handler = DirectUploadHandler::new(&cli).unwrap();

        // Allowed extensions (from CLI: *.txt,*.pdf)
        assert!(handler.validate_file_extension("document.txt").is_ok());
        assert!(handler.validate_file_extension("document.pdf").is_ok());

        // Not allowed extensions
        assert!(handler.validate_file_extension("document.exe").is_err());
        assert!(handler.validate_file_extension("document.jpg").is_err());
    }

    #[test]
    fn test_filename_extraction_from_disposition() {
        // Test various Content-Disposition formats
        assert_eq!(
            DirectUploadHandler::parse_filename_from_disposition("attachment; filename=test.txt"),
            Some("test.txt".to_string())
        );

        assert_eq!(
            DirectUploadHandler::parse_filename_from_disposition(
                "attachment; filename=\"quoted-file.pdf\""
            ),
            Some("quoted-file.pdf".to_string())
        );

        assert_eq!(
            DirectUploadHandler::parse_filename_from_disposition("inline"),
            None
        );

        assert_eq!(
            DirectUploadHandler::parse_filename_from_disposition("attachment; filename="),
            None
        );
    }
}
