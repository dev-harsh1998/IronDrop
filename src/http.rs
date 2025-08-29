// SPDX-License-Identifier: MIT

//! Handles HTTP request parsing, routing, and response generation.

use crate::error::AppError;
use crate::fs::FileDetails;
use crate::response::create_error_response;
use crate::router::Router;
use log::{debug, error, info, trace, warn};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;

/// Maximum size for request body (10GB) to prevent memory exhaustion attacks
const MAX_REQUEST_BODY_SIZE: usize = 10 * 1024 * 1024 * 1024;

/// Maximum size for request headers (8KB) to prevent header buffer overflow
const MAX_HEADERS_SIZE: usize = 8 * 1024;

/// Threshold for streaming request bodies to disk (64MB)
/// This ensures total memory usage stays well below 128MB
pub const STREAM_TO_DISK_THRESHOLD: usize = 64 * 1024 * 1024;

/// Represents a parsed incoming HTTP request.
#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<RequestBody>,
}

/// Request body can be either in memory or streamed to disk for large uploads
#[derive(Debug)]
pub enum RequestBody {
    /// Small bodies stored in memory
    Memory(Vec<u8>),
    /// Large bodies streamed to temporary file
    File { path: PathBuf, size: u64 },
}

impl RequestBody {
    /// Get the size of the request body in bytes
    pub fn len(&self) -> usize {
        let size = match self {
            RequestBody::Memory(data) => data.len(),
            RequestBody::File { size, .. } => *size as usize,
        };
        trace!("RequestBody size: {} bytes", size);
        size
    }

    /// Check if the request body is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Represents an outgoing HTTP response.
pub struct Response {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: ResponseBody,
}

pub enum ResponseBody {
    Text(String),
    StaticText(&'static str),
    Binary(Vec<u8>),
    StaticBinary(&'static [u8]),
    Stream(FileDetails),
}

impl Request {
    /// Validates if the given method is a valid HTTP method
    fn is_valid_http_method(method: &str) -> bool {
        matches!(
            method,
            "GET" | "POST" | "PUT" | "DELETE" | "HEAD" | "OPTIONS" | "PATCH" | "TRACE" | "CONNECT"
        )
    }

    /// Enhanced HTTP request parser with better performance and compliance
    pub fn from_stream(stream: &mut TcpStream) -> Result<Self, AppError> {
        trace!("Starting HTTP request parsing from stream");
        // Set a reasonable timeout for reading requests
        stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;

        // Read the entire HTTP headers in chunks for better performance
        let (headers_data, remaining_bytes) = Self::read_headers_with_remaining(stream)?;
        debug!(
            "Received headers ({} bytes), remaining buffer: {} bytes",
            headers_data.len(),
            remaining_bytes.len()
        );

        // Parse the headers
        let mut lines = headers_data.lines();

        // Parse request line
        let request_line = lines.next().ok_or(AppError::BadRequest)?;
        trace!("Request line: {}", request_line);
        let parts: Vec<&str> = request_line.split_whitespace().collect();

        if parts.len() != 3 {
            debug!("Invalid request line format: {}", request_line);
            return Err(AppError::BadRequest);
        }

        let method = parts[0].to_string();
        let raw_path = parts[1];
        let version = parts[2];

        // Validate HTTP method
        if !Self::is_valid_http_method(&method) {
            debug!("Invalid HTTP method: {}", method);
            return Err(AppError::BadRequest);
        }

        // Validate path doesn't contain null bytes or other invalid characters
        if raw_path.contains('\0') || raw_path.is_empty() {
            debug!("Invalid path: contains null byte or is empty");
            return Err(AppError::BadRequest);
        }

        let path = Self::decode_url(raw_path)?;

        debug!("Parsed request: {} {}", method, path);
        trace!("Raw path before decoding: {}", raw_path);
        trace!("HTTP version: {}", version);

        // Validate HTTP version
        if !version.starts_with("HTTP/1.") {
            return Err(AppError::BadRequest);
        }

        // Parse headers
        let mut headers = HashMap::new();
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                break;
            }

            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_lowercase();
                let value = value.trim().to_string();
                trace!("Header: {} = {}", key, value);

                // Handle multiple header values (comma-separated)
                if let Some(existing) = headers.get(&key) {
                    headers.insert(key, format!("{existing}, {value}"));
                } else {
                    headers.insert(key, value);
                }
            }
        }

        // Read request body if present
        let body = Self::read_request_body(stream, &headers, remaining_bytes)?;

        if let Some(ref body) = body {
            debug!("Request body parsed: {} bytes", body.len());
            match body {
                RequestBody::Memory(_) => trace!("Body stored in memory"),
                RequestBody::File { path, size } => {
                    trace!("Body streamed to file: {} ({} bytes)", path.display(), size);
                }
            }
        } else {
            trace!("No request body");
        }

        debug!(
            "Parsed request: {} {} (headers: {}, body_size: {})",
            method,
            path,
            headers.len(),
            body.as_ref().map(|b| b.len()).unwrap_or(0)
        );

        Ok(Request {
            method,
            path,
            headers,
            body,
        })
    }

    /// Read HTTP headers efficiently in chunks and return remaining bytes from body
    fn read_headers_with_remaining(stream: &mut TcpStream) -> Result<(String, Vec<u8>), AppError> {
        let mut buffer = vec![0; MAX_HEADERS_SIZE];
        let mut total_read = 0;

        loop {
            match stream.read(&mut buffer[total_read..]) {
                Ok(0) => {
                    if total_read == 0 {
                        return Err(AppError::BadRequest);
                    }
                    break;
                }
                Ok(bytes_read) => {
                    total_read += bytes_read;

                    // Look for the end of headers (\r\n\r\n or \n\n) in raw bytes
                    let double_crlf = b"\r\n\r\n";
                    let double_lf = b"\n\n";

                    if let Some(pos) = buffer[0..total_read]
                        .windows(4)
                        .position(|window| window == double_crlf)
                    {
                        let headers_end = pos;
                        let body_start = pos + 4;

                        // Only convert headers portion to UTF-8
                        match std::str::from_utf8(&buffer[0..headers_end]) {
                            Ok(headers_data) => {
                                let remaining_bytes = buffer[body_start..total_read].to_vec();
                                return Ok((headers_data.to_string(), remaining_bytes));
                            }
                            Err(_) => {
                                return Err(AppError::BadRequest);
                            }
                        }
                    } else if let Some(pos) = buffer[0..total_read]
                        .windows(2)
                        .position(|window| window == double_lf)
                    {
                        let headers_end = pos;
                        let body_start = pos + 2;

                        // Only convert headers portion to UTF-8
                        match std::str::from_utf8(&buffer[0..headers_end]) {
                            Ok(headers_data) => {
                                let remaining_bytes = buffer[body_start..total_read].to_vec();
                                return Ok((headers_data.to_string(), remaining_bytes));
                            }
                            Err(_) => {
                                return Err(AppError::BadRequest);
                            }
                        }
                    }

                    // Prevent header buffer overflow attacks
                    if total_read >= buffer.len() {
                        return Err(AppError::BadRequest);
                    }
                }
                Err(e) => return Err(AppError::Io(e)),
            }
        }

        // No body separator found, return all as headers with empty remaining bytes
        match std::str::from_utf8(&buffer[0..total_read]) {
            Ok(data) => Ok((data.to_string(), Vec::new())),
            Err(_) => Err(AppError::BadRequest),
        }
    }

    /// Read request body based on Content-Length header with security validations
    /// Large bodies are streamed to disk to prevent memory exhaustion
    fn read_request_body(
        stream: &mut TcpStream,
        headers: &HashMap<String, String>,
        remaining_bytes: Vec<u8>,
    ) -> Result<Option<RequestBody>, AppError> {
        // Check if we have a Content-Length header
        let content_length = match headers.get("content-length") {
            Some(length_str) => match length_str.parse::<usize>() {
                Ok(length) => length,
                Err(_) => return Err(AppError::BadRequest),
            },
            None => {
                // Check for Transfer-Encoding: chunked (not fully implemented but detected)
                if let Some(encoding) = headers.get("transfer-encoding")
                    && encoding.to_lowercase().contains("chunked")
                {
                    warn!("Chunked transfer encoding not yet supported");
                    return Err(AppError::BadRequest);
                }
                // No body expected
                return Ok(None);
            }
        };

        // Validate content length against security limits
        if content_length == 0 {
            return Ok(Some(RequestBody::Memory(Vec::new())));
        }

        if content_length > MAX_REQUEST_BODY_SIZE {
            return Err(AppError::PayloadTooLarge(MAX_REQUEST_BODY_SIZE as u64));
        }

        // Decide whether to use memory or disk based on size
        if content_length <= STREAM_TO_DISK_THRESHOLD {
            // Small body - use memory
            Self::read_body_to_memory(stream, content_length, remaining_bytes)
                .map(|body| Some(RequestBody::Memory(body)))
        } else {
            // Large body - stream to disk
            Self::read_body_to_disk(stream, content_length, remaining_bytes)
                .map(|(path, size)| Some(RequestBody::File { path, size }))
        }
    }

    /// Read small request body into memory
    fn read_body_to_memory(
        stream: &mut TcpStream,
        content_length: usize,
        remaining_bytes: Vec<u8>,
    ) -> Result<Vec<u8>, AppError> {
        let mut body = Vec::with_capacity(content_length);

        // Use any remaining bytes from header parsing
        let bytes_from_headers = remaining_bytes.len().min(content_length);
        body.extend_from_slice(&remaining_bytes[..bytes_from_headers]);

        // Calculate how many more bytes we need to read
        let bytes_needed = content_length - bytes_from_headers;

        if bytes_needed > 0 {
            // Read the remaining body in chunks
            let mut bytes_read = 0;
            let chunk_size = 8192; // 8KB chunks
            let mut buffer = vec![0; chunk_size];

            while bytes_read < bytes_needed {
                let to_read = (bytes_needed - bytes_read).min(chunk_size);

                match stream.read(&mut buffer[..to_read]) {
                    Ok(0) => {
                        return Err(AppError::BadRequest);
                    }
                    Ok(n) => {
                        body.extend_from_slice(&buffer[..n]);
                        bytes_read += n;
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::TimedOut {
                            warn!("Request body read timeout");
                        }
                        return Err(AppError::Io(e));
                    }
                }
            }
        }

        // Verify we read exactly the expected amount
        if body.len() != content_length {
            return Err(AppError::BadRequest);
        }

        debug!(
            "Successfully read request body to memory: {} bytes",
            body.len()
        );
        Ok(body)
    }

    /// Read large request body directly to disk to prevent memory exhaustion
    fn read_body_to_disk(
        stream: &mut TcpStream,
        content_length: usize,
        remaining_bytes: Vec<u8>,
    ) -> Result<(PathBuf, u64), AppError> {
        // Create temporary file for the request body
        let temp_filename = format!(
            "irondrop_request_{}_{:x}.tmp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );

        // Use system temp directory
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(&temp_filename);

        let mut temp_file = File::create(&temp_path).map_err(|e| {
            error!("Failed to create temporary file {temp_path:?}: {e}");
            AppError::from(e)
        })?;

        let mut total_written = 0;

        // Write any remaining bytes from header parsing
        if !remaining_bytes.is_empty() {
            let bytes_to_write = remaining_bytes.len().min(content_length);
            temp_file
                .write_all(&remaining_bytes[..bytes_to_write])
                .map_err(|e| {
                    let _ = std::fs::remove_file(&temp_path);
                    AppError::from(e)
                })?;
            total_written += bytes_to_write;
        }

        // Stream remaining bytes directly to disk
        let bytes_needed = content_length - total_written;
        if bytes_needed > 0 {
            let mut bytes_read = 0;
            let chunk_size = 64 * 1024; // 64KB chunks for better disk I/O
            let mut buffer = vec![0; chunk_size];

            while bytes_read < bytes_needed {
                let to_read = (bytes_needed - bytes_read).min(chunk_size);

                match stream.read(&mut buffer[..to_read]) {
                    Ok(0) => {
                        let _ = std::fs::remove_file(&temp_path);
                        return Err(AppError::BadRequest);
                    }
                    Ok(n) => {
                        temp_file.write_all(&buffer[..n]).map_err(|e| {
                            let _ = std::fs::remove_file(&temp_path);
                            AppError::from(e)
                        })?;
                        bytes_read += n;
                        total_written += n;
                    }
                    Err(e) => {
                        let _ = std::fs::remove_file(&temp_path);
                        if e.kind() == std::io::ErrorKind::TimedOut {
                            warn!("Request body read timeout");
                        }
                        return Err(AppError::Io(e));
                    }
                }
            }
        }

        // Ensure all data is written to disk
        temp_file.sync_all().map_err(|e| {
            let _ = std::fs::remove_file(&temp_path);
            AppError::from(e)
        })?;

        // Verify we read exactly the expected amount
        if total_written != content_length {
            let _ = std::fs::remove_file(&temp_path);
            return Err(AppError::BadRequest);
        }

        debug!(
            "Successfully streamed request body to disk: {} bytes at {temp_path:?}",
            total_written
        );
        Ok((temp_path, total_written as u64))
    }

    /// Simple URL decoding for percent-encoded paths
    fn decode_url(path: &str) -> Result<String, AppError> {
        let mut decoded = String::with_capacity(path.len());
        let mut chars = path.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '%' {
                // Try to decode percent-encoded character
                let hex1 = chars.next().ok_or(AppError::BadRequest)?;
                let hex2 = chars.next().ok_or(AppError::BadRequest)?;

                if let Ok(byte_val) = u8::from_str_radix(&format!("{hex1}{hex2}"), 16) {
                    if let Some(decoded_char) = char::from_u32(byte_val as u32) {
                        decoded.push(decoded_char);
                    } else {
                        // Invalid character, keep as-is
                        decoded.push(ch);
                        decoded.push(hex1);
                        decoded.push(hex2);
                    }
                } else {
                    // Invalid hex, keep as-is
                    decoded.push(ch);
                    decoded.push(hex1);
                    decoded.push(hex2);
                }
            } else {
                decoded.push(ch);
            }
        }

        Ok(decoded)
    }

    /// Clean up any temporary files associated with this request
    pub fn cleanup(&self) {
        if let Some(RequestBody::File { path, .. }) = &self.body {
            if let Err(e) = std::fs::remove_file(path) {
                warn!("Failed to clean up temporary file {path:?}: {e}");
            } else {
                debug!("Cleaned up temporary file: {path:?}");
            }
        }
    }
}

/// Top-level function to handle a client connection.
#[allow(clippy::too_many_arguments)]
pub fn handle_client(
    mut stream: TcpStream,
    base_dir: &Arc<PathBuf>,
    allowed_extensions: &Arc<Vec<glob::Pattern>>,
    username: &Arc<Option<String>>,
    password: &Arc<Option<String>>,
    chunk_size: usize,
    cli_config: Option<&crate::cli::Cli>,
    stats: Option<&crate::server::ServerStats>,
    router: &Arc<Router>,
) {
    let log_prefix = format!("[{}]", stream.peer_addr().unwrap());
    debug!("{} Handling client connection", log_prefix);
    trace!(
        "{} Client connection established, starting request processing",
        log_prefix
    );

    let request = match Request::from_stream(&mut stream) {
        Ok(req) => {
            debug!(
                "{} Successfully parsed request: {} {}",
                log_prefix, req.method, req.path
            );
            trace!(
                "{} Request headers count: {}",
                log_prefix,
                req.headers.len()
            );
            req
        }
        Err(e) => {
            warn!("{log_prefix} Failed to parse request: {e}");
            debug!("{} Sending error response for parse failure", log_prefix);
            send_error_response(&mut stream, e, &log_prefix);
            return;
        }
    };

    let start_time = std::time::Instant::now();
    let response_result = route_request(
        &request,
        base_dir,
        allowed_extensions,
        username,
        password,
        chunk_size,
        cli_config,
        stats,
        router,
    );
    let processing_time = start_time.elapsed();
    debug!("{} Request processed in {:?}", log_prefix, processing_time);

    match response_result {
        Ok(response) => {
            trace!("{} Response status: {}", log_prefix, response.status_code);
            match send_response(&mut stream, response, &log_prefix) {
                Ok(body_bytes) => {
                    trace!(
                        "{} Response sent successfully, {} bytes",
                        log_prefix, body_bytes
                    );
                    if let Some(stats) = stats {
                        stats.record_request(true, body_bytes);
                    }
                }
                Err(e) => {
                    error!("{log_prefix} Failed to send response: {e}");
                    if let Some(stats) = stats {
                        stats.record_request(false, 0);
                    }
                }
            }
        }
        Err(e) => {
            warn!("{log_prefix} Error processing request: {e}");
            debug!(
                "{} Sending error response for processing failure",
                log_prefix
            );
            send_error_response(&mut stream, e, &log_prefix);
            if let Some(stats) = stats {
                stats.record_request(false, 0);
            }
        }
    }

    // Clean up any temporary files created during request processing
    request.cleanup();
}

// Static asset, favicon, upload, and health handlers moved to handlers.rs

/// Determines the correct response based on the request.
#[allow(clippy::too_many_arguments)]
fn route_request(
    request: &Request,
    base_dir: &Arc<PathBuf>,
    allowed_extensions: &Arc<Vec<glob::Pattern>>,
    _username: &Arc<Option<String>>,
    _password: &Arc<Option<String>>,
    chunk_size: usize,
    cli_config: Option<&crate::cli::Cli>,
    _stats: Option<&crate::server::ServerStats>,
    router: &Arc<Router>,
) -> Result<Response, AppError> {
    trace!("Routing {} {} through router", request.method, request.path);
    // Authentication is now handled by middleware in the router
    // First check if router handles the request (internal routes)
    if let Some(router_response) = router.route(request) {
        debug!(
            "Route found in router for {} {}",
            request.method, request.path
        );
        trace!("Router handler execution starting");
        return router_response;
    }

    // All non-internal paths (not starting with /_irondrop/) are treated as file / directory lookup
    if request.path.starts_with("/_irondrop/") {
        debug!("Internal path {} not found in router", request.path);
        return Err(AppError::NotFound);
    }

    debug!("Handling file request for path: {}", request.path);
    trace!("Using file handler for non-internal path");
    // Handle file and directory serving via dedicated handler
    crate::handlers::handle_file_request(
        request,
        base_dir,
        allowed_extensions,
        chunk_size,
        cli_config,
    )
}

/// Sends a fully formed `Response` to the client with enhanced headers.
fn send_response(
    stream: &mut TcpStream,
    response: Response,
    log_prefix: &str,
) -> Result<u64, std::io::Error> {
    info!(
        "{} {} {}",
        log_prefix, response.status_code, response.status_text
    );
    debug!(
        "{} Preparing response headers ({} custom headers)",
        log_prefix,
        response.headers.len()
    );

    let mut response_str = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status_code, response.status_text
    );

    // Add standard server headers first
    response_str.push_str(&format!("Server: irondrop/{}\r\n", crate::VERSION));
    response_str.push_str("Connection: close\r\n");

    // Add response-specific headers
    for (key, value) in response.headers {
        trace!("{} Response header: {}: {}", log_prefix, key, value);
        response_str.push_str(&format!("{key}: {value}\r\n"));
    }

    // Calculate and add content length for text and binary responses without copying
    match &response.body {
        ResponseBody::Text(text) => {
            let bytes = text.as_bytes();
            response_str.push_str(&format!("Content-Length: {}\r\n", bytes.len()));
        }
        ResponseBody::StaticText(text) => {
            let bytes = text.as_bytes();
            response_str.push_str(&format!("Content-Length: {}\r\n", bytes.len()));
        }
        ResponseBody::Binary(bytes) => {
            response_str.push_str(&format!("Content-Length: {}\r\n", bytes.len()));
        }
        ResponseBody::StaticBinary(bytes) => {
            response_str.push_str(&format!("Content-Length: {}\r\n", bytes.len()));
        }
        ResponseBody::Stream(file_details) => {
            response_str.push_str(&format!("Content-Length: {}\r\n", file_details.size));
        }
    }

    response_str.push_str("\r\n");

    debug!(
        "{} Sending response headers ({} bytes)",
        log_prefix,
        response_str.len()
    );
    stream.write_all(response_str.as_bytes())?;

    // Send body and count only body bytes (exclude headers for stats)
    let mut body_sent: u64 = 0;
    debug!("{} Starting body transmission", log_prefix);
    match response.body {
        ResponseBody::Text(text) => {
            let bytes = text.as_bytes();
            trace!("{} Sending {} bytes of text data", log_prefix, bytes.len());
            stream.write_all(bytes)?;
            body_sent += bytes.len() as u64;
        }
        ResponseBody::StaticText(text) => {
            let bytes = text.as_bytes();
            trace!(
                "{} Sending {} bytes of static text",
                log_prefix,
                bytes.len()
            );
            stream.write_all(bytes)?;
            body_sent += bytes.len() as u64;
        }
        ResponseBody::Binary(bytes) => {
            trace!(
                "{} Sending {} bytes of binary data",
                log_prefix,
                bytes.len()
            );
            stream.write_all(&bytes)?;
            body_sent += bytes.len() as u64;
        }
        ResponseBody::StaticBinary(bytes) => {
            trace!(
                "{} Sending {} bytes of static binary data",
                log_prefix,
                bytes.len()
            );
            stream.write_all(bytes)?;
            body_sent += bytes.len() as u64;
        }
        ResponseBody::Stream(mut file_details) => {
            trace!(
                "{} Streaming file: {} bytes, chunk size: {}",
                log_prefix, file_details.size, file_details.chunk_size
            );
            let mut buffer = vec![0; file_details.chunk_size];
            let mut chunks_sent = 0;
            loop {
                let bytes_read = file_details.file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                stream.write_all(&buffer[..bytes_read])?;
                body_sent += bytes_read as u64;
                chunks_sent += 1;
                if chunks_sent % 100 == 0 {
                    trace!(
                        "{} Streamed {} chunks ({} bytes so far)",
                        log_prefix, chunks_sent, body_sent
                    );
                }
            }
            debug!(
                "{} File streaming completed: {} chunks, {} bytes total",
                log_prefix, chunks_sent, body_sent
            );
        }
    }

    stream.flush()?;
    Ok(body_sent)
}

/// Sends a pre-canned error response using the new response system.
fn send_error_response(stream: &mut TcpStream, error: AppError, log_prefix: &str) {
    let (status_code, status_text) = match error {
        AppError::NotFound => (404, "Not Found"),
        AppError::Forbidden => (403, "Forbidden"),
        AppError::BadRequest => (400, "Bad Request"),
        AppError::Unauthorized => (401, "Unauthorized"),
        AppError::MethodNotAllowed => (405, "Method Not Allowed"),
        // Upload-specific error mappings
        AppError::PayloadTooLarge(_) => (413, "Payload Too Large"),
        AppError::InvalidFilename(_) => (400, "Bad Request"),
        AppError::UploadDiskFull(_) => (507, "Insufficient Storage"),
        AppError::UnsupportedMediaType(_) => (415, "Unsupported Media Type"),
        AppError::UploadDisabled => (403, "Forbidden"),
        _ => (500, "Internal Server Error"),
    };

    info!("{log_prefix} {status_code} {status_text}");

    let response = create_error_response(status_code, status_text);
    if let Err(e) = response.send(stream, log_prefix) {
        error!("{log_prefix} Failed to send error response: {e}");
    }
}
