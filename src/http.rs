//! Handles HTTP request parsing, routing, and response generation.

use crate::error::AppError;
use crate::fs::{generate_directory_listing, FileDetails};
use crate::response::{create_error_response, get_mime_type};
use crate::upload::UploadHandler;
use base64::Engine;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::io::prelude::*;
use std::net::TcpStream;
use std::path::{Component, Path, PathBuf};
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

/// A safe, manual path normalization function.
fn normalize_path(path: &Path) -> Result<PathBuf, AppError> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(name) => {
                components.push(name);
            }
            Component::ParentDir => {
                if components.pop().is_none() {
                    return Err(AppError::Forbidden);
                }
            }
            _ => {}
        }
    }
    Ok(components.iter().collect())
}

/// Handle static asset requests for CSS/JS files using embedded resources
fn handle_static_asset(path: &str) -> Result<Response, AppError> {
    use crate::templates::TemplateEngine;

    // Map /_static/ URLs to embedded templates
    let asset_path = path.strip_prefix("/_static/").unwrap_or("");

    let engine = TemplateEngine::new();
    let (content, content_type) = engine
        .get_static_asset(asset_path)
        .ok_or(AppError::NotFound)?;

    Ok(Response {
        status_code: 200,
        status_text: "OK".to_string(),
        headers: {
            let mut map = HashMap::new();
            map.insert("Content-Type".to_string(), content_type.to_string());
            map.insert(
                "Cache-Control".to_string(),
                "public, max-age=3600".to_string(),
            );
            map
        },
        body: ResponseBody::Text(content.to_string()),
    })
}

/// Handle favicon requests using embedded favicon files
fn handle_favicon_request(path: &str) -> Result<Response, AppError> {
    use crate::templates::TemplateEngine;

    // Strip leading slash for favicon lookup
    let favicon_path = path.strip_prefix('/').unwrap_or(path);

    let engine = TemplateEngine::new();
    let (content, content_type) = engine.get_favicon(favicon_path).ok_or(AppError::NotFound)?;

    Ok(Response {
        status_code: 200,
        status_text: "OK".to_string(),
        headers: {
            let mut map = HashMap::new();
            map.insert("Content-Type".to_string(), content_type.to_string());
            map.insert(
                "Cache-Control".to_string(),
                "public, max-age=86400".to_string(),
            ); // Cache for 24 hours
            map.insert("Content-Length".to_string(), content.len().to_string());
            map
        },
        body: ResponseBody::Binary(content.to_vec()),
    })
}

/// Handle GET requests for upload form
fn handle_upload_form_request(
    _request: &Request,
    cli_config: Option<&crate::cli::Cli>,
) -> Result<Response, AppError> {
    let cli = cli_config.ok_or_else(|| {
        AppError::InternalServerError(
            "CLI configuration not available for upload handling".to_string(),
        )
    })?;

    if !cli.enable_upload {
        return Err(AppError::upload_disabled());
    }

    // Load and render the upload template
    let template_engine = crate::templates::TemplateEngine::new();
    let mut variables = HashMap::new();
    variables.insert("PATH".to_string(), "/".to_string());

    let html_content = template_engine.render("upload_page", &variables)?;

    Ok(Response {
        status_code: 200,
        status_text: "OK".to_string(),
        headers: {
            let mut map = HashMap::new();
            map.insert(
                "Content-Type".to_string(),
                "text/html; charset=utf-8".to_string(),
            );
            map.insert("Cache-Control".to_string(), "no-cache".to_string());
            map
        },
        body: ResponseBody::Text(html_content),
    })
}

/// Handle file upload requests
fn handle_upload_request(
    request: &Request,
    cli_config: Option<&crate::cli::Cli>,
    stats: Option<&crate::server::ServerStats>,
) -> Result<Response, AppError> {
    let cli = cli_config.ok_or_else(|| {
        AppError::InternalServerError(
            "CLI configuration not available for upload handling".to_string(),
        )
    })?;

    if !cli.enable_upload {
        return Err(AppError::upload_disabled());
    }

    // Create upload handler
    let mut upload_handler = UploadHandler::new(cli)?;

    // Process the upload with statistics tracking
    let http_response = upload_handler.handle_upload_with_stats(request, stats)?;

    // Convert HttpResponse to Response
    let mut headers = HashMap::new();
    for (key, value) in http_response.headers {
        headers.insert(key, value);
    }

    let body = ResponseBody::Text(String::from_utf8_lossy(&http_response.body).to_string());

    Ok(Response {
        status_code: http_response.status_code,
        status_text: http_response.status_text,
        headers,
        body,
    })
}

/// Create a health check response with server status
fn create_health_check_response() -> Response {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let health_info = format!(
        r#"{{
    "status": "healthy",
    "service": "irondrop",
    "version": "2.5.0",
    "timestamp": {timestamp},
    "features": [
        "rate_limiting",
        "statistics", 
        "native_mime_detection",
        "enhanced_security",
        "beautiful_ui",
        "http11_compliance",
        "request_timeouts",
        "panic_recovery"
    ]
}}"#
    );

    Response {
        status_code: 200,
        status_text: "OK".to_string(),
        headers: {
            let mut map = HashMap::new();
            map.insert(
                "Content-Type".to_string(),
                "application/json; charset=utf-8".to_string(),
            );
            map.insert("Cache-Control".to_string(), "no-cache".to_string());
            map
        },
        body: ResponseBody::Text(health_info),
    }
}

/// Determines the correct response based on the request.
#[allow(clippy::too_many_arguments)]
fn route_request(
    request: &Request,
    base_dir: &Arc<PathBuf>,
    allowed_extensions: &Arc<Vec<glob::Pattern>>,
    username: &Arc<Option<String>>,
    password: &Arc<Option<String>>,
    chunk_size: usize,
    cli_config: Option<&crate::cli::Cli>,
    stats: Option<&crate::server::ServerStats>,
) -> Result<Response, AppError> {
    if let (Some(expected_user), Some(expected_pass)) = (username.as_ref(), password.as_ref()) {
        if !is_authenticated(
            request.headers.get("authorization"),
            expected_user,
            expected_pass,
        ) {
            return Err(AppError::Unauthorized);
        }
    }

    // /monitor endpoint (HTML or JSON if ?json=1)
    if request.path.starts_with("/monitor") {
        if request.path.contains("json=1") {
            return Ok(create_monitor_json(stats));
        } else {
            use crate::templates::TemplateEngine;
            let engine = TemplateEngine::new();
            if let Ok(html) = engine.render_monitor_page() {
                return Ok(Response {
                    status_code: 200,
                    status_text: "OK".into(),
                    headers: {
                        let mut h = HashMap::new();
                        h.insert("Content-Type".into(), "text/html; charset=utf-8".into());
                        h
                    },
                    body: ResponseBody::Text(html),
                });
            } else {
                return Ok(create_monitor_json(stats));
            }
        }
    }

    // Handle health check endpoint
    if request.path == "/_health" || request.path == "/_status" {
        return Ok(create_health_check_response());
    }

    // Handle static assets for templates
    if request.path.starts_with("/_static/") {
        return handle_static_asset(&request.path);
    }

    // Handle favicon requests
    if request.path == "/favicon.ico"
        || request.path == "/favicon-16x16.png"
        || request.path == "/favicon-32x32.png"
    {
        return handle_favicon_request(&request.path);
    }

    // Handle upload requests (strip query parameters for matching)
    let path_without_query = request.path.split('?').next().unwrap_or(&request.path);
    let normalized_path = path_without_query.trim_end_matches('/');
    if normalized_path == "/upload" {
        if request.method == "POST" {
            return handle_upload_request(request, cli_config, stats);
        } else if request.method == "GET" {
            return handle_upload_form_request(request, cli_config);
        }
    }

    // Handle different methods appropriately
    match request.method.as_str() {
        "GET" => {
            // GET requests are handled normally
        }
        "POST" => {
            // For now, POST requests are only accepted but not fully implemented
            // In a real implementation, this would handle file uploads
            // For the current implementation, we'll allow POST but treat it like GET for basic functionality
            debug!("POST request received, treating as GET for basic functionality");
        }
        _ => {
            return Err(AppError::MethodNotAllowed);
        }
    }

    let requested_path = PathBuf::from(request.path.strip_prefix('/').unwrap_or(&request.path));
    let safe_path = normalize_path(&requested_path)?;
    let full_path = base_dir.join(safe_path);

    if !full_path.starts_with(base_dir.as_ref()) {
        return Err(AppError::Forbidden);
    }

    if !full_path.exists() {
        return Err(AppError::NotFound);
    }

    if full_path.is_dir() {
        // Only serve directory listings for GET requests
        if request.method == "POST" {
            return Err(AppError::MethodNotAllowed);
        }

        let html_content = generate_directory_listing(&full_path, &request.path)?;
        Ok(Response {
            status_code: 200,
            status_text: "OK".to_string(),
            headers: {
                let mut map = HashMap::new();
                map.insert(
                    "Content-Type".to_string(),
                    "text/html; charset=utf-8".to_string(),
                );
                map
            },
            body: ResponseBody::Text(html_content),
        })
    } else if full_path.is_file() {
        if !allowed_extensions
            .iter()
            .any(|p| p.matches_path(&full_path))
        {
            return Err(AppError::Forbidden);
        }

        let file_details = FileDetails::new(full_path.clone(), chunk_size)?;
        let mime_type = get_mime_type(&full_path);
        Ok(Response {
            status_code: 200,
            status_text: "OK".to_string(),
            headers: {
                let mut map = HashMap::new();
                map.insert("Content-Type".to_string(), mime_type.to_string());
                map.insert("Content-Length".to_string(), file_details.size.to_string());
                map.insert("Accept-Ranges".to_string(), "bytes".to_string());
                map.insert(
                    "Cache-Control".to_string(),
                    "public, max-age=3600".to_string(),
                );
                map
            },
            body: ResponseBody::Stream(file_details),
        })
    } else {
        Err(AppError::NotFound)
    }
}

/// Build JSON stats snapshot for /monitor
fn create_monitor_json(stats: Option<&crate::server::ServerStats>) -> Response {
    if let Some(s) = stats {
        let (total, successful, errors, bytes, uptime) = s.get_stats();
        let up = s.get_upload_stats();
        let json = format!(
            r#"{{"requests":{{"total":{total},"successful":{successful},"errors":{errors}}},"downloads":{{"bytes_served":{bytes}}},"uptime_secs":{},"uploads":{{"total_uploads":{},"successful_uploads":{},"failed_uploads":{},"files_uploaded":{},"upload_bytes":{},"average_upload_size":{},"largest_upload":{},"concurrent_uploads":{},"average_processing_ms":{:.2},"success_rate":{:.2}}}}}"#,
            uptime.as_secs(),
            up.total_uploads,
            up.successful_uploads,
            up.failed_uploads,
            up.files_uploaded,
            up.upload_bytes,
            up.average_upload_size,
            up.largest_upload,
            up.concurrent_uploads,
            up.average_processing_time,
            up.success_rate
        );
        return Response {
            status_code: 200,
            status_text: "OK".into(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".into(), "application/json".into());
                h.insert("Cache-Control".into(), "no-cache".into());
                h
            },
            body: ResponseBody::Text(json),
        };
    }
    Response {
        status_code: 503,
        status_text: "Service Unavailable".into(),
        headers: {
            let mut h = HashMap::new();
            h.insert("Content-Type".into(), "application/json".into());
            h
        },
        body: ResponseBody::Text("{\"error\":\"stats unavailable\"}".into()),
    }
}

/// Checks the 'Authorization' header for valid credentials.
fn is_authenticated(auth_header: Option<&String>, user: &str, pass: &str) -> bool {
    let header = match auth_header {
        Some(h) => h,
        None => return false,
    };

    let credentials = match header.strip_prefix("Basic ") {
        Some(c) => c,
        None => return false,
    };

    let decoded = match base64::engine::general_purpose::STANDARD.decode(credentials) {
        Ok(d) => d,
        Err(_) => return false,
    };

    let decoded_str = match String::from_utf8(decoded) {
        Ok(s) => s,
        Err(_) => return false,
    };

    if let Some((provided_user, provided_pass)) = decoded_str.split_once(':') {
        provided_user == user && provided_pass == pass
    } else {
        false
    }
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
