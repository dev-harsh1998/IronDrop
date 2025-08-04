//! Handles HTTP request parsing, routing, and response generation.

use crate::error::AppError;
use crate::fs::{generate_directory_listing, FileDetails};
use crate::response::{create_error_response, get_mime_type};
use base64::Engine;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::io::prelude::*;
use std::net::TcpStream;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// Represents a parsed incoming HTTP request.
#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
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
        let headers_data = Self::read_headers(stream)?;

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

        debug!(
            "Parsed request: {} {} (headers: {})",
            method,
            path,
            headers.len()
        );
        Ok(Request {
            method,
            path,
            headers,
        })
    }

    /// Read HTTP headers efficiently in chunks
    fn read_headers(stream: &mut TcpStream) -> Result<String, AppError> {
        let mut buffer = vec![0; 8192]; // 8KB buffer for headers
        let mut headers_data = String::new();
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

                    // Convert bytes to string (up to what we've read)
                    match std::str::from_utf8(&buffer[0..total_read]) {
                        Ok(data) => {
                            // Look for the end of headers (\r\n\r\n or \n\n)
                            if data.contains("\r\n\r\n") {
                                let end_pos = data.find("\r\n\r\n").unwrap() + 4;
                                headers_data = data[0..end_pos - 4].to_string();
                                break;
                            } else if data.contains("\n\n") {
                                let end_pos = data.find("\n\n").unwrap() + 2;
                                headers_data = data[0..end_pos - 2].to_string();
                                break;
                            }
                        }
                        Err(_) => {
                            // Invalid UTF-8, continue reading
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

        Ok(headers_data)
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
pub fn handle_client(
    mut stream: TcpStream,
    base_dir: &Arc<PathBuf>,
    allowed_extensions: &Arc<Vec<glob::Pattern>>,
    username: &Arc<Option<String>>,
    password: &Arc<Option<String>>,
    chunk_size: usize,
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
    );

    match response_result {
        Ok(response) => {
            if let Err(e) = send_response(&mut stream, response, &log_prefix) {
                error!("{log_prefix} Failed to send response: {e}");
            }
        }
        Err(e) => {
            warn!("{log_prefix} Error processing request: {e}");
            send_error_response(&mut stream, e, &log_prefix);
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
    "version": "2.0.0",
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
fn route_request(
    request: &Request,
    base_dir: &Arc<PathBuf>,
    allowed_extensions: &Arc<Vec<glob::Pattern>>,
    username: &Arc<Option<String>>,
    password: &Arc<Option<String>>,
    chunk_size: usize,
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

    if request.method != "GET" {
        return Err(AppError::MethodNotAllowed);
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
) -> Result<(), std::io::Error> {
    info!(
        "{} {} {}",
        log_prefix, response.status_code, response.status_text
    );

    let mut response_str = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status_code, response.status_text
    );

    // Add standard server headers first
    response_str.push_str("Server: irondrop/2.0.0\r\n");
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

    // Send body
    match response.body {
        ResponseBody::Text(_) | ResponseBody::Binary(_) => {
            stream.write_all(&body_bytes)?;
        }
        ResponseBody::Stream(mut file_details) => {
            let mut buffer = vec![0; file_details.chunk_size];
            while let Ok(bytes_read) = file_details.file.read(&mut buffer) {
                if bytes_read == 0 {
                    break;
                }
                stream.write_all(&buffer[..bytes_read])?;
            }
        }
    }

    stream.flush()
}

/// Sends a pre-canned error response using the new response system.
fn send_error_response(stream: &mut TcpStream, error: AppError, log_prefix: &str) {
    let (status_code, status_text) = match error {
        AppError::NotFound => (404, "Not Found"),
        AppError::Forbidden => (403, "Forbidden"),
        AppError::BadRequest => (400, "Bad Request"),
        AppError::Unauthorized => (401, "Unauthorized"),
        AppError::MethodNotAllowed => (405, "Method Not Allowed"),
        _ => (500, "Internal Server Error"),
    };

    info!("{log_prefix} {status_code} {status_text}");

    let response = create_error_response(status_code, status_text);
    if let Err(e) = response.send(stream, log_prefix) {
        error!("{log_prefix} Failed to send error response: {e}");
    }
}
