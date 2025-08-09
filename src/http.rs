//! Handles HTTP request parsing, routing, and response generation.

use crate::error::AppError;
use crate::fs::FileDetails;
use crate::response::create_error_response;
use crate::router::Router;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::io::prelude::*;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;

/// Maximum size for request body (10GB) to prevent memory exhaustion attacks
const MAX_REQUEST_BODY_SIZE: usize = 10 * 1024 * 1024 * 1024;

/// Maximum size for request headers (8KB) to prevent header buffer overflow
const MAX_HEADERS_SIZE: usize = 8 * 1024;

/// Represents a parsed incoming HTTP request.
#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
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
    Binary(Vec<u8>),
    Stream(FileDetails),
}

impl Request {
    /// Enhanced HTTP request parser with better performance and compliance
    pub fn from_stream(stream: &mut TcpStream) -> Result<Self, AppError> {
        // Set a reasonable timeout for reading requests
        stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;

        // Read the entire HTTP headers in chunks for better performance
        let (headers_data, remaining_bytes) = Self::read_headers_with_remaining(stream)?;

        // Parse the headers
        let mut lines = headers_data.lines();

        // Parse request line
        let request_line = lines.next().ok_or(AppError::BadRequest)?;
        let parts: Vec<&str> = request_line.split_whitespace().collect();

        if parts.len() != 3 {
            return Err(AppError::BadRequest);
        }

        let method = parts[0].to_string();
        let path = Self::decode_url(parts[1])?;
        let version = parts[2];

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
    fn read_request_body(
        stream: &mut TcpStream,
        headers: &HashMap<String, String>,
        remaining_bytes: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, AppError> {
        // Check if we have a Content-Length header
        let content_length = match headers.get("content-length") {
            Some(length_str) => match length_str.parse::<usize>() {
                Ok(length) => length,
                Err(_) => return Err(AppError::BadRequest),
            },
            None => {
                // Check for Transfer-Encoding: chunked (not fully implemented but detected)
                if let Some(encoding) = headers.get("transfer-encoding") {
                    if encoding.to_lowercase().contains("chunked") {
                        warn!("Chunked transfer encoding not yet supported");
                        return Err(AppError::BadRequest);
                    }
                }
                // No body expected
                return Ok(None);
            }
        };

        // Validate content length against security limits
        if content_length == 0 {
            return Ok(Some(Vec::new()));
        }

        if content_length > MAX_REQUEST_BODY_SIZE {
            return Err(AppError::PayloadTooLarge(MAX_REQUEST_BODY_SIZE as u64));
        }

        let mut body = Vec::with_capacity(content_length);

        // Use any remaining bytes from header parsing
        let bytes_from_headers = remaining_bytes.len().min(content_length);
        body.extend_from_slice(&remaining_bytes[..bytes_from_headers]);

        // Calculate how many more bytes we need to read
        let bytes_needed = content_length - bytes_from_headers;

        if bytes_needed > 0 {
            // Read the remaining body in chunks to avoid large allocations
            let mut bytes_read = 0;
            let chunk_size = 8192; // 8KB chunks
            let mut buffer = vec![0; chunk_size];

            while bytes_read < bytes_needed {
                let to_read = (bytes_needed - bytes_read).min(chunk_size);

                match stream.read(&mut buffer[..to_read]) {
                    Ok(0) => {
                        // Unexpected end of stream
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

        debug!("Successfully read request body: {} bytes", body.len());
        Ok(Some(body))
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

    let request = match Request::from_stream(&mut stream) {
        Ok(req) => req,
        Err(e) => {
            warn!("{log_prefix} Failed to parse request: {e}");
            send_error_response(&mut stream, e, &log_prefix);
            return;
        }
    };

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

    match response_result {
        Ok(response) => match send_response(&mut stream, response, &log_prefix) {
            Ok(body_bytes) => {
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
        },
        Err(e) => {
            warn!("{log_prefix} Error processing request: {e}");
            send_error_response(&mut stream, e, &log_prefix);
            if let Some(stats) = stats {
                stats.record_request(false, 0);
            }
        }
    }
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
    // Authentication is now handled by middleware in the router
    // First check if router handles the request (internal routes)
    if let Some(router_response) = router.route(request) {
        return router_response;
    }

    // All non-internal paths (not starting with /_irondrop/) are treated as file / directory lookup
    if request.path.starts_with("/_irondrop/") {
        return Err(AppError::NotFound);
    }

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

    let mut response_str = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status_code, response.status_text
    );

    // Add standard server headers first
    response_str.push_str("Server: irondrop/2.5.0\r\n");
    response_str.push_str("Connection: close\r\n");

    // Add response-specific headers
    for (key, value) in response.headers {
        response_str.push_str(&format!("{key}: {value}\r\n"));
    }

    // Calculate and add content length for text and binary responses
    let body_bytes = match &response.body {
        ResponseBody::Text(text) => {
            let bytes = text.as_bytes();
            response_str.push_str(&format!("Content-Length: {}\r\n", bytes.len()));
            bytes.to_vec()
        }
        ResponseBody::Binary(bytes) => {
            response_str.push_str(&format!("Content-Length: {}\r\n", bytes.len()));
            bytes.clone()
        }
        ResponseBody::Stream(file_details) => {
            response_str.push_str(&format!("Content-Length: {}\r\n", file_details.size));
            Vec::new() // Will be handled separately
        }
    };

    response_str.push_str("\r\n");

    stream.write_all(response_str.as_bytes())?;

    // Send body and count only body bytes (exclude headers for stats)
    let mut body_sent: u64 = 0;
    match response.body {
        ResponseBody::Text(_) | ResponseBody::Binary(_) => {
            stream.write_all(&body_bytes)?;
            body_sent += body_bytes.len() as u64;
        }
        ResponseBody::Stream(mut file_details) => {
            let mut buffer = vec![0; file_details.chunk_size];
            loop {
                let bytes_read = file_details.file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                stream.write_all(&buffer[..bytes_read])?;
                body_sent += bytes_read as u64;
            }
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
        AppError::InvalidMultipart(_) => (400, "Bad Request"),
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
