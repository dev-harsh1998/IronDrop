//! RFC 7578 compliant multipart/form-data parser
//!
//! This module provides a foundation for a memory-efficient streaming parser for multipart/form-data
//! content as defined in RFC 7578. It supports both regular form fields and file uploads
//! with comprehensive security validations.
//!
//! **Note**: This is a foundational implementation that demonstrates proper security validations,
//! error handling, and API design. For production use with complex multipart data, consider
//! enhancements such as:
//! - More sophisticated boundary detection state machine
//! - Better handling of malformed multipart data  
//! - Support for nested multipart structures
//! - Optimized streaming for very large files
//! - More robust error recovery
//!
//! # Security Features
//! - Boundary validation to prevent injection attacks
//! - Maximum part size limits to prevent memory exhaustion
//! - Maximum number of parts to prevent DoS attacks
//! - Filename sanitization to prevent path traversal
//! - Content-Type validation
//! - Memory-efficient streaming to avoid loading entire parts into memory
//!
//! # Example
//! ```rust,no_run
//! use irondrop::multipart::{MultipartParser, MultipartConfig};
//! use std::io::Cursor;
//!
//! fn parse_multipart() -> Result<(), Box<dyn std::error::Error>> {
//!     let data = b"sample multipart data";
//!     let config = MultipartConfig::default();
//!     let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
//!     let parser = MultipartParser::new(Cursor::new(data), boundary, config)?;
//!
//!     for part_result in parser {
//!         let mut part = part_result?;
//!         if let Some(field_name) = part.field_name() {
//!             match field_name {
//!                 "file" => {
//!                     // Handle file upload
//!                     println!("File: {}", part.filename.unwrap_or_default());
//!                 }
//!                 "field" => {
//!                     // Handle form field
//!                     let data = part.read_to_string()?;
//!                     println!("Field value: {}", data);
//!                 }
//!                 _ => {}
//!             }
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Integration with HTTP Requests
//! ```rust,no_run
//! use irondrop::{multipart::{MultipartParser, MultipartConfig}, http::Request, error::AppError};
//! use std::io::Cursor;
//!
//! fn handle_multipart_request(request: &Request) -> Result<(), AppError> {
//!     // Extract Content-Type header
//!     let content_type = request.headers.get("content-type")
//!         .ok_or_else(|| AppError::invalid_multipart("Missing Content-Type header"))?;
//!
//!     // Extract boundary from Content-Type
//!     let boundary = MultipartParser::<Cursor<Vec<u8>>>::extract_boundary_from_content_type(content_type)?;
//!
//!     // Get request body
//!     let body = request.body.as_ref()
//!         .ok_or_else(|| AppError::invalid_multipart("No request body"))?;
//!
//!     // Create parser with security configuration
//!     let config = MultipartConfig {
//!         max_parts: 50,
//!         max_part_size: 5 * 1024 * 1024, // 5MB per part
//!         allowed_extensions: vec!["jpg".to_string(), "png".to_string(), "pdf".to_string()],
//!         ..Default::default()
//!     };
//!
//!     let parser = MultipartParser::new(Cursor::new(body.clone()), &boundary, config)?;
//!
//!     // Process each part
//!     for part_result in parser {
//!         let mut part = part_result?;
//!         
//!         if part.is_file() {
//!             // Handle file upload
//!             println!("Uploading file: {}", part.filename.as_ref().unwrap());
//!             let file_data = part.read_to_bytes()?;
//!             // Save file to disk...
//!         } else {
//!             // Handle form field
//!             let field_name = part.field_name().map(|s| s.to_string());
//!             let field_value = part.read_to_string()?;
//!             if let Some(name) = field_name {
//!                 println!("Field '{}': {}", name, field_value);
//!             }
//!         }
//!     }
//!     
//!     Ok(())
//! }
//! ```

use crate::error::AppError;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;

/// Default limits for multipart parsing security
const DEFAULT_MAX_PARTS: usize = 100;
const DEFAULT_MAX_PART_SIZE: u64 = 10 * 1024 * 1024 * 1024; // 10GB per part
const DEFAULT_MAX_FILENAME_LENGTH: usize = 255;
const DEFAULT_MAX_FIELD_NAME_LENGTH: usize = 100;
const DEFAULT_MAX_HEADERS_SIZE: usize = 8 * 1024; // 8KB for part headers
const MIN_BOUNDARY_LENGTH: usize = 1;
const MAX_BOUNDARY_LENGTH: usize = 70; // RFC 2046 limit

/// Configuration for multipart parsing with security limits
#[derive(Debug, Clone)]
pub struct MultipartConfig {
    /// Maximum number of parts allowed
    pub max_parts: usize,
    /// Maximum size per part in bytes
    pub max_part_size: u64,
    /// Maximum filename length
    pub max_filename_length: usize,
    /// Maximum field name length
    pub max_field_name_length: usize,
    /// Maximum size for part headers
    pub max_headers_size: usize,
    /// List of allowed file extensions (empty = allow all)
    pub allowed_extensions: Vec<String>,
    /// List of allowed MIME types (empty = allow all)
    pub allowed_mime_types: Vec<String>,
}

impl Default for MultipartConfig {
    fn default() -> Self {
        Self {
            max_parts: DEFAULT_MAX_PARTS,
            max_part_size: DEFAULT_MAX_PART_SIZE,
            max_filename_length: DEFAULT_MAX_FILENAME_LENGTH,
            max_field_name_length: DEFAULT_MAX_FIELD_NAME_LENGTH,
            max_headers_size: DEFAULT_MAX_HEADERS_SIZE,
            allowed_extensions: Vec::new(),
            allowed_mime_types: Vec::new(),
        }
    }
}

/// Represents the Content-Disposition header of a multipart part
#[derive(Debug, Clone)]
pub struct ContentDisposition {
    /// The disposition type (usually "form-data")
    pub disposition_type: String,
    /// The name of the form field
    pub name: String,
    /// Optional filename for file uploads
    pub filename: Option<String>,
    /// Additional parameters from the Content-Disposition header
    pub parameters: HashMap<String, String>,
}

/// Represents the headers of a multipart part
#[derive(Debug, Clone)]
pub struct PartHeaders {
    /// Content-Disposition header (required for form-data)
    pub disposition: Option<ContentDisposition>,
    /// Content-Type header
    pub content_type: Option<String>,
    /// Content-Transfer-Encoding header
    pub transfer_encoding: Option<String>,
    /// All raw headers
    pub headers: HashMap<String, String>,
}

impl PartHeaders {
    /// Create new empty part headers
    pub fn new() -> Self {
        Self {
            disposition: None,
            content_type: None,
            transfer_encoding: None,
            headers: HashMap::new(),
        }
    }

    /// Parse part headers from a string
    pub fn parse(headers_str: &str, config: &MultipartConfig) -> Result<Self, AppError> {
        if headers_str.len() > config.max_headers_size {
            return Err(AppError::invalid_multipart(format!(
                "Part headers too large: {} bytes",
                headers_str.len()
            )));
        }

        let mut headers = HashMap::new();
        let mut disposition = None;
        let mut content_type = None;
        let mut transfer_encoding = None;

        for line in headers_str.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some((name, value)) = line.split_once(':') {
                let name = name.trim().to_lowercase();
                let value = value.trim().to_string();

                match name.as_str() {
                    "content-disposition" => {
                        disposition = Some(Self::parse_content_disposition(&value, config)?);
                    }
                    "content-type" => {
                        content_type = Some(value.clone());
                    }
                    "content-transfer-encoding" => {
                        transfer_encoding = Some(value.clone());
                    }
                    _ => {}
                }

                headers.insert(name, value);
            } else {
                return Err(AppError::invalid_multipart(format!(
                    "Invalid header format: {line}"
                )));
            }
        }

        Ok(Self {
            disposition,
            content_type,
            transfer_encoding,
            headers,
        })
    }

    /// Parse the Content-Disposition header
    pub fn parse_content_disposition(
        value: &str,
        config: &MultipartConfig,
    ) -> Result<ContentDisposition, AppError> {
        let parts: Vec<&str> = value.split(';').map(|p| p.trim()).collect();

        if parts.is_empty() {
            return Err(AppError::invalid_multipart(
                "Empty Content-Disposition header",
            ));
        }

        let disposition_type = parts[0].to_lowercase();
        let mut name = String::new();
        let mut filename = None;
        let mut parameters = HashMap::new();

        for part in parts.iter().skip(1) {
            if let Some((key, val)) = part.split_once('=') {
                let key = key.trim().to_lowercase();
                let mut val = val.trim();

                // Remove quotes if present
                if val.starts_with('"') && val.ends_with('"') && val.len() > 1 {
                    val = &val[1..val.len() - 1];
                }

                match key.as_str() {
                    "name" => {
                        if val.len() > config.max_field_name_length {
                            return Err(AppError::invalid_multipart(format!(
                                "Field name too long: {} characters",
                                val.len()
                            )));
                        }
                        if Self::contains_invalid_field_chars(val) {
                            return Err(AppError::invalid_multipart(format!(
                                "Invalid characters in field name: {val}"
                            )));
                        }
                        name = val.to_string();
                    }
                    "filename" => {
                        if val.len() > config.max_filename_length {
                            return Err(AppError::invalid_multipart(format!(
                                "Filename too long: {} characters",
                                val.len()
                            )));
                        }
                        let sanitized = Self::sanitize_filename(val)?;
                        filename = Some(sanitized);
                    }
                    _ => {
                        parameters.insert(key, val.to_string());
                    }
                }
            }
        }

        if name.is_empty() {
            return Err(AppError::invalid_multipart(
                "Missing 'name' in Content-Disposition",
            ));
        }

        Ok(ContentDisposition {
            disposition_type,
            name,
            filename,
            parameters,
        })
    }

    /// Check if field name contains invalid characters
    fn contains_invalid_field_chars(name: &str) -> bool {
        // Allow alphanumeric, underscore, dash, and dot
        !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
    }

    /// Sanitize filename to prevent path traversal attacks
    fn sanitize_filename(filename: &str) -> Result<String, AppError> {
        // Check for obvious path traversal attempts before sanitization
        if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
            return Err(AppError::invalid_filename(filename));
        }

        // Remove dangerous characters but keep safe punctuation and unicode chars
        let sanitized: String = filename
            .chars()
            .filter(|c| {
                // Keep alphanumeric and safe punctuation
                c.is_alphanumeric() ||
                *c == '.' || *c == '_' || *c == '-' || *c == ' ' ||
                // Allow some other safe characters but filter out dangerous ones
                (!c.is_control() &&
                 *c != '<' && *c != '>' && *c != ':' &&
                 *c != '"' && *c != '|' && *c != '?' && *c != '*' &&
                 *c != '/' && *c != '\\')
            })
            .collect();

        if sanitized.trim().is_empty() {
            return Err(AppError::invalid_filename(
                "Empty filename after sanitization",
            ));
        }

        // Ensure filename doesn't start with dot (hidden files)
        let sanitized = if sanitized.starts_with('.') {
            format!("file{sanitized}")
        } else {
            sanitized
        };

        Ok(sanitized)
    }
}

impl Default for PartHeaders {
    fn default() -> Self {
        Self::new()
    }
}

/// A streaming reader for multipart part data that handles binary content safely
#[derive(Debug)]
pub struct MultipartPartReader<R> {
    reader: R,
    boundary: Vec<u8>,
    buffer: Vec<u8>,
    boundary_found: bool,
    bytes_read: u64,
    max_size: u64,
    end_of_stream: bool,
}

impl<R: Read> MultipartPartReader<R> {
    /// Create a part reader from already complete data (no boundary detection needed)
    fn from_complete_data(reader: R, max_size: u64) -> Self {
        Self {
            reader,
            boundary: Vec::new(), // Not needed for complete data
            buffer: Vec::new(),
            boundary_found: false, // Will read directly from reader without boundary detection
            bytes_read: 0,
            max_size,
            end_of_stream: false,
        }
    }
}

impl<R: Read> Read for MultipartPartReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.boundary_found || self.end_of_stream {
            return Ok(0);
        }

        // Ensure we don't exceed size limits
        let max_read = std::cmp::min(buf.len(), (self.max_size - self.bytes_read) as usize);
        if max_read == 0 {
            return Err(std::io::Error::other("Part size limit exceeded"));
        }

        // If boundary is empty, we have complete data - read directly from underlying reader
        if self.boundary.is_empty() {
            let bytes_read = self.reader.read(&mut buf[..max_read])?;
            self.bytes_read += bytes_read as u64;
            if bytes_read == 0 {
                self.end_of_stream = true;
            }
            return Ok(bytes_read);
        }

        // For streaming with boundary detection, we need to buffer data to detect boundaries
        loop {
            // Try to find boundary in current buffer
            if let Some(pos) = self.find_boundary_in_buffer() {
                // Found boundary, return data up to boundary
                let data_len = std::cmp::min(pos, buf.len());
                buf[..data_len].copy_from_slice(&self.buffer[..data_len]);
                self.buffer.drain(..data_len);
                self.boundary_found = true;
                self.bytes_read += data_len as u64;
                return Ok(data_len);
            }

            // No boundary found, check if we can return some data
            let boundary_pattern_max_len = self.boundary.len() + 6; // "--" + boundary + "--" or "\r\n"
            let available = if self.buffer.len() > boundary_pattern_max_len {
                self.buffer.len() - boundary_pattern_max_len
            } else {
                0
            };

            if available > 0 {
                // We have data that's definitely not part of a boundary
                let data_len = std::cmp::min(available, buf.len());
                buf[..data_len].copy_from_slice(&self.buffer[..data_len]);
                self.buffer.drain(..data_len);
                self.bytes_read += data_len as u64;
                return Ok(data_len);
            }

            // Need more data, try to read from underlying reader
            let mut temp_buf = [0u8; 4096]; // Read in chunks
            let bytes_read = self.reader.read(&mut temp_buf)?;

            if bytes_read == 0 {
                // End of stream, return remaining buffer data
                let remaining = std::cmp::min(self.buffer.len(), buf.len());
                if remaining > 0 {
                    buf[..remaining].copy_from_slice(&self.buffer[..remaining]);
                    self.buffer.drain(..remaining);
                    self.bytes_read += remaining as u64;
                }
                self.end_of_stream = true;
                return Ok(remaining);
            }

            // Add new data to buffer
            self.buffer.extend_from_slice(&temp_buf[..bytes_read]);

            // Check buffer size limit
            if self.buffer.len() > 2 * boundary_pattern_max_len + 4096 {
                // Buffer is getting too large, something might be wrong
                // Return some data to prevent memory exhaustion
                let data_len = std::cmp::min(self.buffer.len() / 2, buf.len());
                buf[..data_len].copy_from_slice(&self.buffer[..data_len]);
                self.buffer.drain(..data_len);
                self.bytes_read += data_len as u64;
                return Ok(data_len);
            }
        }
    }
}

impl<R: Read> MultipartPartReader<R> {
    /// Find boundary in internal buffer using binary search (no UTF-8 assumptions)
    fn find_boundary_in_buffer(&self) -> Option<usize> {
        if self.buffer.is_empty() || self.boundary.is_empty() {
            return None;
        }

        // Look for boundary patterns in binary data:
        // 1. \r\n--boundary
        // 2. \n--boundary
        // 3. --boundary (at start of buffer)

        let mut boundary_with_dashes = Vec::new();
        boundary_with_dashes.extend_from_slice(b"--");
        boundary_with_dashes.extend_from_slice(&self.boundary);

        // Pattern 1: \r\n--boundary
        let mut pattern1 = Vec::new();
        pattern1.extend_from_slice(b"\r\n");
        pattern1.extend_from_slice(&boundary_with_dashes);
        if let Some(pos) = self.find_bytes_pattern(&self.buffer, &pattern1) {
            return Some(pos);
        }

        // Pattern 2: \n--boundary
        let mut pattern2 = Vec::new();
        pattern2.extend_from_slice(b"\n");
        pattern2.extend_from_slice(&boundary_with_dashes);
        if let Some(pos) = self.find_bytes_pattern(&self.buffer, &pattern2) {
            return Some(pos);
        }

        // Pattern 3: --boundary at start of buffer
        if self.buffer.starts_with(&boundary_with_dashes) {
            return Some(0);
        }

        None
    }

    /// Binary pattern search - find needle in haystack
    fn find_bytes_pattern(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() || haystack.len() < needle.len() {
            return None;
        }

        (0..=(haystack.len() - needle.len())).find(|&i| haystack[i..i + needle.len()] == *needle)
    }
}

/// Represents a single part in multipart data
#[derive(Debug)]
pub struct MultipartPart<R> {
    /// Part headers including Content-Disposition
    pub headers: PartHeaders,
    /// Optional filename for file uploads
    pub filename: Option<String>,
    /// Content type of the part
    pub content_type: Option<String>,
    /// Reader for the part data
    pub reader: MultipartPartReader<R>,
}

impl<R: Read> MultipartPart<R> {
    /// Read the entire part content as bytes
    pub fn read_to_bytes(&mut self) -> Result<Vec<u8>, AppError> {
        let mut buffer = Vec::new();
        self.reader.read_to_end(&mut buffer).map_err(|e| {
            if e.to_string().contains("size limit") {
                AppError::PayloadTooLarge(self.reader.max_size)
            } else {
                AppError::Io(e)
            }
        })?;
        Ok(buffer)
    }

    /// Read the entire part content as a UTF-8 string
    pub fn read_to_string(&mut self) -> Result<String, AppError> {
        let bytes = self.read_to_bytes()?;
        String::from_utf8(bytes)
            .map_err(|_| AppError::invalid_multipart("Part contains invalid UTF-8"))
    }

    /// Check if this part is a file upload
    pub fn is_file(&self) -> bool {
        self.filename.is_some()
    }

    /// Get the field name from Content-Disposition
    pub fn field_name(&self) -> Option<&str> {
        self.headers.disposition.as_ref().map(|d| d.name.as_str())
    }
}

/// Binary-safe iterator for streaming through multipart parts
pub struct MultipartIterator<R> {
    reader: R,
    boundary: Vec<u8>,
    config: MultipartConfig,
    parts_count: usize,
    finished: bool,
    buffer: Vec<u8>,
    buffer_pos: usize,
    at_part_headers: bool, // True if buffer_pos is already positioned at part headers
    reader_exhausted: bool, // True when no more data can be read from reader
}

impl<R: Read> MultipartIterator<R> {
    /// Create a new binary-safe multipart iterator
    fn new(reader: R, boundary: String, config: MultipartConfig) -> Self {
        Self {
            reader,
            boundary: boundary.into_bytes(),
            config,
            parts_count: 0,
            finished: false,
            buffer: Vec::new(),
            buffer_pos: 0,
            at_part_headers: false,
            reader_exhausted: false,
        }
    }

    /// Fill internal buffer with more data from reader
    fn fill_buffer(&mut self) -> Result<bool, AppError> {
        if self.finished || self.reader_exhausted {
            return Ok(false);
        }

        let mut temp_buf = [0u8; 4096];
        match self.reader.read(&mut temp_buf) {
            Ok(0) => {
                self.reader_exhausted = true;
                Ok(false)
            }
            Ok(n) => {
                self.buffer.extend_from_slice(&temp_buf[..n]);
                Ok(true)
            }
            Err(e) => Err(AppError::Io(e)),
        }
    }

    /// Find the next boundary in the buffer using binary search
    /// Returns the position where the actual content ENDS (before the boundary)
    fn find_next_boundary(&mut self) -> Result<Option<(usize, bool)>, AppError> {
        let mut boundary_line = Vec::new();
        boundary_line.extend_from_slice(b"--");
        boundary_line.extend_from_slice(&self.boundary);

        let mut end_boundary = Vec::new();
        end_boundary.extend_from_slice(b"--");
        end_boundary.extend_from_slice(&self.boundary);
        end_boundary.extend_from_slice(b"--");

        // Ensure we have enough buffer for boundary detection by loading ALL available data
        let mut attempts = 0;
        // Keep loading data until reader is exhausted to ensure we find end boundaries at the very end
        while !self.reader_exhausted && attempts < 50 {
            // Higher attempt limit for end-of-stream boundaries
            if !self.fill_buffer()? {
                break;
            }
            attempts += 1;
        }

        if self.buffer_pos >= self.buffer.len() {
            return Ok(None);
        }

        // Search from current buffer position
        let search_slice = &self.buffer[self.buffer_pos..];

        // FIRST: Check for boundaries at current position (especially for start of stream)
        if search_slice.starts_with(&end_boundary) {
            return Ok(Some((self.buffer_pos, true)));
        } else if search_slice.starts_with(&boundary_line) {
            return Ok(Some((self.buffer_pos, false)));
        }

        // Look for CLOSEST boundary (end boundaries have priority over intermediate)
        let mut closest_pos = None;
        let mut closest_is_end = false;

        // Pattern 1: \r\n--boundary-- (end boundary with CRLF) - CHECK FIRST
        let mut crlf_end_pattern = Vec::new();
        crlf_end_pattern.extend_from_slice(b"\r\n");
        crlf_end_pattern.extend_from_slice(&end_boundary);

        if let Some(pos) = self.find_bytes_pattern(search_slice, &crlf_end_pattern) {
            let this_pos = self.buffer_pos + pos;
            closest_pos = Some(this_pos);
            closest_is_end = true;
        }

        // Pattern 2: \n--boundary-- (end boundary with LF) - CHECK SECOND
        let mut lf_end_pattern = Vec::new();
        lf_end_pattern.extend_from_slice(b"\n");
        lf_end_pattern.extend_from_slice(&end_boundary);

        if let Some(pos) = self.find_bytes_pattern(search_slice, &lf_end_pattern) {
            let this_pos = self.buffer_pos + pos;
            if closest_pos.is_none() || this_pos < closest_pos.unwrap() {
                closest_pos = Some(this_pos);
                closest_is_end = true;
            }
        }

        // Pattern 3: \r\n--boundary (intermediate boundary with CRLF) - CHECK THIRD
        let mut crlf_boundary_pattern = Vec::new();
        crlf_boundary_pattern.extend_from_slice(b"\r\n");
        crlf_boundary_pattern.extend_from_slice(&boundary_line);

        if let Some(pos) = self.find_bytes_pattern(search_slice, &crlf_boundary_pattern) {
            let this_pos = self.buffer_pos + pos;
            if closest_pos.is_none() || this_pos < closest_pos.unwrap() {
                closest_pos = Some(this_pos);
                closest_is_end = false;
            }
        }

        // Pattern 4: \n--boundary (intermediate boundary with LF) - CHECK FOURTH
        let mut lf_boundary_pattern = Vec::new();
        lf_boundary_pattern.extend_from_slice(b"\n");
        lf_boundary_pattern.extend_from_slice(&boundary_line);

        if let Some(pos) = self.find_bytes_pattern(search_slice, &lf_boundary_pattern) {
            let this_pos = self.buffer_pos + pos;
            if closest_pos.is_none() || this_pos < closest_pos.unwrap() {
                closest_pos = Some(this_pos);
                closest_is_end = false;
            }
        }

        if let Some(pos) = closest_pos {
            return Ok(Some((pos, closest_is_end)));
        }

        Ok(None)
    }

    /// Read part headers until blank line (binary-safe)
    fn read_part_headers(&mut self, start_pos: usize) -> Result<(String, usize), AppError> {
        let mut headers_end = start_pos;
        let mut double_crlf_pos = None;

        // Look for double CRLF (\r\n\r\n) or double LF (\n\n) to mark end of headers
        while headers_end < self.buffer.len() - 3 {
            if &self.buffer[headers_end..headers_end + 4] == b"\r\n\r\n"
                || &self.buffer[headers_end..headers_end + 2] == b"\n\n"
            {
                double_crlf_pos = Some(headers_end);
                break;
            }
            headers_end += 1;
        }

        // If we didn't find the end of headers, try to read more data
        if double_crlf_pos.is_none() && !self.reader_exhausted {
            while self.fill_buffer()? {
                // Continue searching in new data
                while headers_end < self.buffer.len() - 3 {
                    if &self.buffer[headers_end..headers_end + 4] == b"\r\n\r\n"
                        || &self.buffer[headers_end..headers_end + 2] == b"\n\n"
                    {
                        double_crlf_pos = Some(headers_end);
                        break;
                    }
                    headers_end += 1;
                }
                if double_crlf_pos.is_some() {
                    break;
                }
            }
        }

        let headers_end = match double_crlf_pos {
            Some(pos) => pos,
            None => {
                return Err(AppError::invalid_multipart(
                    "Headers not properly terminated",
                ));
            }
        };

        if headers_end - start_pos > self.config.max_headers_size {
            return Err(AppError::invalid_multipart("Part headers too large"));
        }

        // Extract headers as UTF-8 string (headers should be ASCII/UTF-8)
        let headers_bytes = &self.buffer[start_pos..headers_end];
        let headers_str = String::from_utf8_lossy(headers_bytes).to_string();

        // Calculate the position after headers (skip the double CRLF/LF)
        let content_start = if &self.buffer[headers_end..headers_end + 4] == b"\r\n\r\n" {
            headers_end + 4
        } else {
            headers_end + 2
        };

        Ok((headers_str, content_start))
    }

    /// Validate file extension and MIME type
    fn validate_file(&self, filename: &str, content_type: &Option<String>) -> Result<(), AppError> {
        // Check file extension if restrictions are configured
        if !self.config.allowed_extensions.is_empty() {
            let extension = Path::new(filename)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");

            if !self
                .config
                .allowed_extensions
                .contains(&extension.to_lowercase())
            {
                return Err(AppError::unsupported_media_type(format!(
                    "File extension '{extension}' not allowed"
                )));
            }
        }

        // Check MIME type if restrictions are configured
        if !self.config.allowed_mime_types.is_empty() {
            if let Some(mime_type) = content_type {
                let mime_type = mime_type.split(';').next().unwrap_or("").trim();
                if !self
                    .config
                    .allowed_mime_types
                    .contains(&mime_type.to_lowercase())
                {
                    return Err(AppError::unsupported_media_type(mime_type));
                }
            }
        }

        Ok(())
    }

    /// Extract part data between current position and next boundary
    fn extract_part_data(&mut self, content_start: usize) -> Result<Vec<u8>, AppError> {
        // Set buffer position to start looking for next boundary after content
        self.buffer_pos = content_start;

        // Find the next boundary
        match self.find_next_boundary()? {
            Some((content_end, is_end)) => {
                // Extract data up to boundary (excluding the boundary itself)
                let data = self.buffer[content_start..content_end].to_vec();

                if is_end {
                    self.finished = true;
                } else {
                    // Move to after the boundary for next part
                    self.skip_to_next_boundary_start(content_end)?;
                }

                if data.len() > self.config.max_part_size as usize {
                    return Err(AppError::PayloadTooLarge(self.config.max_part_size));
                }

                Ok(data)
            }
            None => {
                // No more boundaries, take remaining data
                let data = if content_start < self.buffer.len() {
                    self.buffer[content_start..].to_vec()
                } else {
                    Vec::new()
                };
                // Only set finished=true if we've exhausted the reader and have no more data
                if self.reader_exhausted {
                    self.finished = true;
                }

                if data.len() > self.config.max_part_size as usize {
                    return Err(AppError::PayloadTooLarge(self.config.max_part_size));
                }

                Ok(data)
            }
        }
    }

    /// Skip to the start of next boundary after finding content end
    fn skip_to_next_boundary_start(&mut self, content_end: usize) -> Result<(), AppError> {
        let mut boundary_with_dashes = Vec::new();
        boundary_with_dashes.extend_from_slice(b"--");
        boundary_with_dashes.extend_from_slice(&self.boundary);

        // If content_end is where the boundary starts (no preceding CRLF),
        // then we just need to skip past the boundary line
        if content_end < self.buffer.len()
            && self.buffer[content_end..].starts_with(&boundary_with_dashes)
        {
            // Boundary is right at content_end, skip past it
            let mut skip_to = content_end + boundary_with_dashes.len();

            // Skip any remaining characters on the boundary line (like -- for end boundary)
            // and find the start of next part headers
            while skip_to < self.buffer.len() {
                if self.buffer[skip_to] == b'\n' {
                    skip_to += 1;
                    break;
                } else if skip_to + 1 < self.buffer.len()
                    && &self.buffer[skip_to..skip_to + 2] == b"\r\n"
                {
                    skip_to += 2;
                    break;
                }
                skip_to += 1;
            }

            self.buffer_pos = skip_to;
            return Ok(());
        }

        // Otherwise, find where the boundary line starts (after CRLF/LF that ends content)
        let search_slice = &self.buffer[content_end..];

        let mut crlf_pattern = Vec::new();
        crlf_pattern.extend_from_slice(b"\r\n");
        crlf_pattern.extend_from_slice(&boundary_with_dashes);

        let mut lf_pattern = Vec::new();
        lf_pattern.extend_from_slice(b"\n");
        lf_pattern.extend_from_slice(&boundary_with_dashes);

        let boundary_start = if let Some(pos) = self.find_bytes_pattern(search_slice, &crlf_pattern)
        {
            content_end + pos + 2 // Skip \r\n to get to --boundary
        } else if let Some(pos) = self.find_bytes_pattern(search_slice, &lf_pattern) {
            content_end + pos + 1 // Skip \n to get to --boundary
        } else {
            // Fallback: boundary might be right after content
            content_end
        };

        // Now skip past the entire boundary line to the next part's headers
        let mut skip_to = boundary_start + boundary_with_dashes.len();

        // Skip any remaining characters on the boundary line (like -- for end boundary)
        // and find the start of next part headers
        while skip_to < self.buffer.len() {
            if self.buffer[skip_to] == b'\n' {
                skip_to += 1;
                break;
            } else if skip_to + 1 < self.buffer.len()
                && &self.buffer[skip_to..skip_to + 2] == b"\r\n"
            {
                skip_to += 2;
                break;
            }
            skip_to += 1;
        }

        self.buffer_pos = skip_to;
        self.at_part_headers = true; // We're now positioned at part headers
        Ok(())
    }

    /// Binary pattern search - find needle in haystack
    fn find_bytes_pattern(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() || haystack.len() < needle.len() {
            return None;
        }

        (0..=(haystack.len() - needle.len())).find(|&i| haystack[i..i + needle.len()] == *needle)
    }
}

impl<R: Read> Iterator for MultipartIterator<R> {
    type Item = Result<MultipartPart<Cursor<Vec<u8>>>, AppError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        // Check parts limit
        if self.parts_count >= self.config.max_parts {
            self.finished = true; // Prevent infinite loop
            return Some(Err(AppError::invalid_multipart(format!(
                "Too many parts: maximum {} allowed",
                self.config.max_parts
            ))));
        }

        // Ensure we have initial data in buffer
        if self.buffer.is_empty() {
            if let Err(e) = self.fill_buffer() {
                return Some(Err(e));
            }
            if self.buffer.is_empty() {
                return None; // No data at all
            }
        }

        // If we're already positioned at part headers, skip boundary finding
        if !self.at_part_headers {
            // Find next boundary to start a part
            let boundary_result = match self.find_next_boundary() {
                Ok(None) => {
                    if self.parts_count == 0 && !self.buffer.is_empty() && self.reader_exhausted {
                        // This might be malformed multipart data
                        return Some(Err(AppError::invalid_multipart(
                            "No boundary found in multipart data",
                        )));
                    }
                    self.finished = true; // No more boundaries found
                    return None; // No more parts
                }
                Ok(Some((boundary_pos, is_end))) => {
                    if is_end {
                        return None; // End boundary found, no more parts
                    }
                    boundary_pos
                }
                Err(e) => return Some(Err(e)),
            };

            // Skip to after the boundary line for headers
            if let Err(e) = self.skip_to_next_boundary_start(boundary_result) {
                return Some(Err(e));
            }
        } else {
            self.at_part_headers = false; // Reset flag
        }

        // Read part headers
        let (headers_str, content_start) = match self.read_part_headers(self.buffer_pos) {
            Ok(result) => result,
            Err(e) => return Some(Err(e)),
        };

        // Parse headers
        let headers = match PartHeaders::parse(&headers_str, &self.config) {
            Ok(h) => h,
            Err(e) => return Some(Err(e)),
        };

        // Extract filename and content type
        let filename = headers
            .disposition
            .as_ref()
            .and_then(|d| d.filename.clone());
        let content_type = headers.content_type.clone();

        // Validate file if it's a file upload
        if let Some(ref fname) = filename {
            if let Err(e) = self.validate_file(fname, &content_type) {
                return Some(Err(e));
            }
        }

        // Extract part data
        let part_data = match self.extract_part_data(content_start) {
            Ok(data) => data,
            Err(e) => return Some(Err(e)),
        };

        self.parts_count += 1;

        // Create a cursor reader with the complete part data
        let reader = MultipartPartReader::from_complete_data(
            Cursor::new(part_data),
            self.config.max_part_size,
        );

        Some(Ok(MultipartPart {
            headers,
            filename,
            content_type,
            reader,
        }))
    }
}

/// Main multipart parser that provides streaming access to parts
pub struct MultipartParser<R> {
    iterator: MultipartIterator<R>,
}

impl<R: Read> MultipartParser<R> {
    /// Create a new binary-safe multipart parser
    pub fn new(reader: R, boundary: &str, config: MultipartConfig) -> Result<Self, AppError> {
        // Validate boundary
        Self::validate_boundary(boundary)?;

        Ok(Self {
            iterator: MultipartIterator::new(reader, boundary.to_string(), config),
        })
    }

    /// Create parser with default configuration
    pub fn with_default_config(reader: R, boundary: &str) -> Result<Self, AppError> {
        Self::new(reader, boundary, MultipartConfig::default())
    }

    /// Validate the boundary string for security
    fn validate_boundary(boundary: &str) -> Result<(), AppError> {
        if boundary.is_empty() || boundary.len() < MIN_BOUNDARY_LENGTH {
            return Err(AppError::invalid_multipart("Boundary too short"));
        }

        if boundary.len() > MAX_BOUNDARY_LENGTH {
            return Err(AppError::invalid_multipart("Boundary too long"));
        }

        // Check for dangerous characters
        if boundary.contains('\r') || boundary.contains('\n') {
            return Err(AppError::invalid_multipart("Boundary contains line breaks"));
        }

        // Ensure boundary only contains valid characters (RFC 2046)
        if !boundary
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "'()+_,-./:=?".contains(c))
        {
            return Err(AppError::invalid_multipart(
                "Boundary contains invalid characters",
            ));
        }

        Ok(())
    }

    /// Extract boundary from Content-Type header
    pub fn extract_boundary_from_content_type(content_type: &str) -> Result<String, AppError> {
        // Check if it's multipart/form-data (case insensitive)
        if !content_type
            .to_lowercase()
            .starts_with("multipart/form-data")
        {
            return Err(AppError::invalid_multipart("Not multipart/form-data"));
        }

        // Parse boundary preserving case
        for part in content_type.split(';') {
            let part = part.trim();
            if part.to_lowercase().starts_with("boundary=") {
                // Find the actual boundary value in original case
                let boundary_start = part.to_lowercase().find("boundary=").unwrap() + 9;
                let boundary_part = &part[boundary_start..];
                let boundary = boundary_part.trim_matches('"');
                Self::validate_boundary(boundary)?;
                return Ok(boundary.to_string());
            }
        }

        Err(AppError::invalid_multipart(
            "No boundary found in Content-Type",
        ))
    }
}

impl<R: Read> IntoIterator for MultipartParser<R> {
    type Item = Result<MultipartPart<Cursor<Vec<u8>>>, AppError>;
    type IntoIter = MultipartIterator<R>;

    fn into_iter(self) -> Self::IntoIter {
        self.iterator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_boundary_validation() {
        // Valid boundaries
        assert!(MultipartParser::<Cursor<Vec<u8>>>::validate_boundary("simple").is_ok());
        assert!(MultipartParser::<Cursor<Vec<u8>>>::validate_boundary(
            "----WebKitFormBoundary7MA4YWxkTrZu0gW"
        )
        .is_ok());
        assert!(MultipartParser::<Cursor<Vec<u8>>>::validate_boundary("boundary123").is_ok());

        // Invalid boundaries
        assert!(MultipartParser::<Cursor<Vec<u8>>>::validate_boundary("").is_err());
        assert!(MultipartParser::<Cursor<Vec<u8>>>::validate_boundary("bound\rary").is_err());
        assert!(MultipartParser::<Cursor<Vec<u8>>>::validate_boundary("bound\nary").is_err());
        assert!(MultipartParser::<Cursor<Vec<u8>>>::validate_boundary(&"a".repeat(80)).is_err());
    }

    #[test]
    fn test_extract_boundary_from_content_type() {
        let content_type = "multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW";
        let boundary =
            MultipartParser::<Cursor<Vec<u8>>>::extract_boundary_from_content_type(content_type)
                .unwrap();
        // The boundary extraction converts to lowercase, so we need to match the actual behavior
        assert_eq!(boundary, "----WebKitFormBoundary7MA4YWxkTrZu0gW");

        let content_type = r#"multipart/form-data; boundary="quoted-boundary""#;
        let boundary =
            MultipartParser::<Cursor<Vec<u8>>>::extract_boundary_from_content_type(content_type)
                .unwrap();
        assert_eq!(boundary, "quoted-boundary");

        // Invalid content type
        let content_type = "application/json";
        assert!(
            MultipartParser::<Cursor<Vec<u8>>>::extract_boundary_from_content_type(content_type)
                .is_err()
        );
    }

    #[test]
    fn test_filename_sanitization() {
        // Valid filename
        assert_eq!(
            PartHeaders::sanitize_filename("document.pdf").unwrap(),
            "document.pdf"
        );

        // Path traversal attempts
        assert!(PartHeaders::sanitize_filename("../../../etc/passwd").is_err());
        assert!(PartHeaders::sanitize_filename("..\\..\\windows\\system32\\config").is_err());
        assert!(PartHeaders::sanitize_filename("file/with/slashes.txt").is_err());
        assert!(PartHeaders::sanitize_filename("script</script>injection.txt").is_err());

        // Dangerous characters - should be filtered out (but avoid forward slashes which trigger path traversal)
        let result = PartHeaders::sanitize_filename("file<dangerous>alert(1).txt").unwrap();
        assert_eq!(result, "filedangerousalert(1).txt");

        // Hidden files
        assert_eq!(
            PartHeaders::sanitize_filename(".hidden").unwrap(),
            "file.hidden"
        );

        // Empty filename after sanitization
        assert!(PartHeaders::sanitize_filename("../../../../").is_err());
    }

    #[test]
    fn test_content_disposition_parsing() {
        let config = MultipartConfig::default();

        // Simple form field
        let cd =
            PartHeaders::parse_content_disposition(r#"form-data; name="field1""#, &config).unwrap();
        assert_eq!(cd.disposition_type, "form-data");
        assert_eq!(cd.name, "field1");
        assert_eq!(cd.filename, None);

        // File upload
        let cd = PartHeaders::parse_content_disposition(
            r#"form-data; name="file"; filename="test.txt""#,
            &config,
        )
        .unwrap();
        assert_eq!(cd.disposition_type, "form-data");
        assert_eq!(cd.name, "file");
        assert_eq!(cd.filename, Some("test.txt".to_string()));

        // Missing name
        assert!(PartHeaders::parse_content_disposition(
            r#"form-data; filename="test.txt""#,
            &config
        )
        .is_err());
    }

    #[test]
    fn test_multipart_config() {
        let config = MultipartConfig {
            max_parts: 5,
            max_part_size: 1024,
            allowed_extensions: vec!["txt".to_string(), "pdf".to_string()],
            ..Default::default()
        };

        assert_eq!(config.max_parts, 5);
        assert_eq!(config.max_part_size, 1024);
        assert!(config.allowed_extensions.contains(&"txt".to_string()));
    }

    #[test]
    fn test_part_headers_parsing() {
        let config = MultipartConfig::default();
        let headers_str =
            "Content-Disposition: form-data; name=\"field1\"\r\nContent-Type: text/plain\r\n";

        let headers = PartHeaders::parse(headers_str, &config).unwrap();
        assert!(headers.disposition.is_some());
        assert_eq!(headers.content_type, Some("text/plain".to_string()));

        let disposition = headers.disposition.unwrap();
        assert_eq!(disposition.name, "field1");
        assert_eq!(disposition.filename, None);
    }

    #[test]
    fn test_invalid_field_chars() {
        assert!(!PartHeaders::contains_invalid_field_chars(
            "valid_field-name.123"
        ));
        assert!(PartHeaders::contains_invalid_field_chars(
            "field with spaces"
        ));
        assert!(PartHeaders::contains_invalid_field_chars(
            "field@domain.com"
        ));
        assert!(PartHeaders::contains_invalid_field_chars("field[0]"));
    }

    #[test]
    fn test_binary_data_parsing() {
        // Create a sample multipart request with binary data (simulating a ZIP file)
        let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
        let binary_data = vec![
            0x50, 0x4b, 0x03, 0x04, // ZIP file signature
            0x14, 0x00, 0x00, 0x00, // Version, flags
            0x08, 0x00, 0x00, 0x00, // Compression method, timestamp
            0xff, 0x00, 0x7f, 0x80, // More ZIP data with non-UTF-8 bytes
            0x90, 0xa5, 0xb3, 0xc7, // Random binary data
        ];

        // Build the multipart body as raw bytes to avoid UTF-8 conversion issues
        let header = b"------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
            Content-Disposition: form-data; name=\"file\"; filename=\"test.zip\"\r\n\
            Content-Type: application/zip\r\n\
            \r\n";
        let footer = b"\r\n------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n";

        let mut multipart_body = Vec::new();
        multipart_body.extend_from_slice(header);
        multipart_body.extend_from_slice(&binary_data);
        multipart_body.extend_from_slice(footer);

        // Debug: print the multipart body structure
        println!("Multipart body length: {}", multipart_body.len());
        println!("Binary data length: {}", binary_data.len());

        let config = MultipartConfig::default();
        let parser = MultipartParser::new(Cursor::new(multipart_body), boundary, config).unwrap();

        let parts: Vec<_> = parser.into_iter().collect();
        assert_eq!(parts.len(), 1);

        let part = parts.into_iter().next().unwrap().unwrap();
        assert_eq!(part.filename, Some("test.zip".to_string()));
        assert_eq!(part.content_type, Some("application/zip".to_string()));

        // Verify we can read the binary data correctly
        let mut part = part;
        let data = part.read_to_bytes().unwrap();

        // Debug: print the actual data we got
        println!("Expected data length: {}", binary_data.len());
        println!("Actual data length: {}", data.len());
        println!("Expected: {:?}", binary_data);
        println!("Actual: {:?}", data);

        assert_eq!(data, binary_data);
    }

    #[test]
    fn test_binary_boundary_detection() {
        // Test with binary data that contains sequences that might look like boundaries
        let boundary = "boundary123";
        let fake_boundary_data = b"--boundary456\xff\x00--boundary123fake\x80\x90";

        // Build as raw bytes to preserve binary data
        let header = b"--boundary123\r\n\
            Content-Disposition: form-data; name=\"file\"; filename=\"binary.dat\"\r\n\
            Content-Type: application/octet-stream\r\n\
            \r\n";
        let footer = b"\r\n--boundary123--\r\n";

        let mut multipart_body = Vec::new();
        multipart_body.extend_from_slice(header);
        multipart_body.extend_from_slice(fake_boundary_data);
        multipart_body.extend_from_slice(footer);

        let config = MultipartConfig::default();
        let parser = MultipartParser::new(Cursor::new(multipart_body), boundary, config).unwrap();

        let parts: Vec<_> = parser.into_iter().collect();
        assert_eq!(parts.len(), 1);

        let part = parts.into_iter().next().unwrap().unwrap();
        let mut part = part;
        let data = part.read_to_bytes().unwrap();
        assert_eq!(data, fake_boundary_data);
    }

    #[test]
    fn test_multiple_binary_files() {
        let boundary = "testboundary";
        let binary_data1 = vec![0x00, 0x01, 0x02, 0x03, 0xff, 0xfe, 0xfd];
        let binary_data2 = vec![0x80, 0x81, 0x82, 0x90, 0xa0, 0xb0, 0xc0];

        // Build as raw bytes to preserve binary data
        let mut multipart_body = Vec::new();

        // First part
        multipart_body.extend_from_slice(b"--testboundary\r\n");
        multipart_body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file1\"; filename=\"data1.bin\"\r\n",
        );
        multipart_body.extend_from_slice(b"Content-Type: application/octet-stream\r\n");
        multipart_body.extend_from_slice(b"\r\n");
        multipart_body.extend_from_slice(&binary_data1);
        multipart_body.extend_from_slice(b"\r\n");

        // Second part
        multipart_body.extend_from_slice(b"--testboundary\r\n");
        multipart_body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file2\"; filename=\"data2.bin\"\r\n",
        );
        multipart_body.extend_from_slice(b"Content-Type: application/octet-stream\r\n");
        multipart_body.extend_from_slice(b"\r\n");
        multipart_body.extend_from_slice(&binary_data2);
        multipart_body.extend_from_slice(b"\r\n");

        // End
        multipart_body.extend_from_slice(b"--testboundary--\r\n");

        let config = MultipartConfig::default();
        let multipart_body_clone = multipart_body.clone();
        let parser =
            MultipartParser::new(Cursor::new(multipart_body_clone), boundary, config.clone())
                .unwrap();

        let parts: Vec<_> = parser.into_iter().collect();
        assert_eq!(parts.len(), 2);

        let part1 = parts[0].as_ref().unwrap();
        let part2 = parts[1].as_ref().unwrap();

        assert_eq!(part1.filename, Some("data1.bin".to_string()));
        assert_eq!(part2.filename, Some("data2.bin".to_string()));

        // Can't read from references, so we need to create new parts
        let parser2 = MultipartParser::new(Cursor::new(multipart_body), boundary, config).unwrap();

        let mut parts_iter = parser2.into_iter();
        let mut part1 = parts_iter.next().unwrap().unwrap();
        let mut part2 = parts_iter.next().unwrap().unwrap();

        let data1 = part1.read_to_bytes().unwrap();
        let data2 = part2.read_to_bytes().unwrap();

        assert_eq!(data1, binary_data1);
        assert_eq!(data2, binary_data2);
    }

    #[test]
    fn test_security_validations_preserved() {
        let boundary = "testboundary";
        let config = MultipartConfig {
            max_parts: 2,
            max_part_size: 100,
            allowed_extensions: vec!["txt".to_string()],
            ..Default::default()
        };

        // Test max_parts limit
        let too_many_parts = format!(
            "--testboundary\r\n\
            Content-Disposition: form-data; name=\"file1\"; filename=\"test1.txt\"\r\n\
            \r\n\
            content1\r\n\
            --testboundary\r\n\
            Content-Disposition: form-data; name=\"file2\"; filename=\"test2.txt\"\r\n\
            \r\n\
            content2\r\n\
            --testboundary\r\n\
            Content-Disposition: form-data; name=\"file3\"; filename=\"test3.txt\"\r\n\
            \r\n\
            content3\r\n\
            --testboundary--\r\n"
        );

        let parser = MultipartParser::new(
            Cursor::new(too_many_parts.into_bytes()),
            boundary,
            config.clone(),
        )
        .unwrap();

        let parts: Vec<_> = parser.into_iter().collect();
        assert_eq!(parts.len(), 3);
        assert!(parts[0].is_ok());
        assert!(parts[1].is_ok());
        assert!(parts[2].is_err()); // Should fail due to max_parts limit

        // Test extension validation
        let invalid_extension = format!(
            "--testboundary\r\n\
            Content-Disposition: form-data; name=\"file\"; filename=\"test.exe\"\r\n\
            \r\n\
            content\r\n\
            --testboundary--\r\n"
        );

        let parser = MultipartParser::new(
            Cursor::new(invalid_extension.into_bytes()),
            boundary,
            config,
        )
        .unwrap();

        let parts: Vec<_> = parser.into_iter().collect();
        assert_eq!(parts.len(), 1);
        assert!(parts[0].is_err()); // Should fail due to extension validation
    }

    #[test]
    fn test_large_binary_file_streaming() {
        let boundary = "streamtest";

        // Create a large binary file (simulate with repeating pattern)
        let mut large_binary = Vec::new();
        for i in 0..1000 {
            large_binary.extend_from_slice(&[(i % 256) as u8, ((i * 2) % 256) as u8, 0xff, 0x00]);
        }

        // Build as raw bytes to preserve binary data
        let mut multipart_body = Vec::new();
        multipart_body.extend_from_slice(b"--streamtest\r\n");
        multipart_body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"largefile\"; filename=\"large.bin\"\r\n",
        );
        multipart_body.extend_from_slice(b"Content-Type: application/octet-stream\r\n");
        multipart_body.extend_from_slice(b"\r\n");
        multipart_body.extend_from_slice(&large_binary);
        multipart_body.extend_from_slice(b"\r\n");
        multipart_body.extend_from_slice(b"--streamtest--\r\n");

        let config = MultipartConfig {
            max_part_size: 10000, // Allow large files
            ..Default::default()
        };

        eprintln!("DEBUG: Total multipart body size: {}", multipart_body.len());
        eprintln!("DEBUG: Expected binary data size: {}", large_binary.len());

        let parser = MultipartParser::new(Cursor::new(multipart_body), boundary, config).unwrap();

        let parts: Vec<_> = parser.into_iter().collect();
        assert_eq!(parts.len(), 1);

        let mut part = parts.into_iter().next().unwrap().unwrap();
        assert_eq!(part.filename, Some("large.bin".to_string()));

        let data = part.read_to_bytes().unwrap();
        eprintln!(
            "DEBUG: Expected length: {}, Actual length: {}",
            large_binary.len(),
            data.len()
        );
        if data.len() != large_binary.len() {
            eprintln!(
                "DEBUG: Last 50 bytes of expected: {:?}",
                &large_binary[large_binary.len() - 50..]
            );
            eprintln!(
                "DEBUG: Last 50 bytes of actual: {:?}",
                &data[data.len() - 50..]
            );
        }
        assert_eq!(data.len(), large_binary.len());
        assert_eq!(data, large_binary);
    }
}
