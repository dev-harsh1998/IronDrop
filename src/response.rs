use crate::error::AppError;
use crate::templates::{get_error_description, TemplateEngine};
use log::{debug, error};
use std::io::prelude::*;
use std::net::TcpStream;
use std::path::Path;

/// Native MIME type detection for common file types
pub fn get_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") | Some("htm") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("txt") => "text/plain",
        Some("md") => "text/markdown",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        Some("tar") => "application/x-tar",
        Some("gz") => "application/gzip",
        Some("mp4") => "video/mp4",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        _ => "application/octet-stream",
    }
}

/// Generate error pages using embedded templates - dark mode only
fn generate_error_page(status_code: u16, status_text: &str) -> String {
    let engine = TemplateEngine::new();
    let description = get_error_description(status_code);

    engine.render_error_page(status_code, status_text, description)
        .unwrap_or_else(|_| {
            // Fallback if template rendering fails
            format!(
                r#"<!DOCTYPE html>
<html>
<head><title>Error {status_code}</title>
<style>body{{background:#1e293b;color:#f1f5f9;font-family:sans-serif;text-align:center;padding:2rem}}</style>
</head>
<body><h1>{status_code}</h1><p>{status_text}</p><a href="/" style="color:#60a5fa">← Back to Files</a></body>
</html>"#
            )
        })
}

/// Enhanced HTTP response builder with proper headers and error handling
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn new(status_code: u16, status_text: &str) -> Self {
        Self {
            status_code,
            status_text: status_text.to_string(),
            headers: vec![
                ("Server".to_string(), "irondrop/2.0.0".to_string()),
                ("Connection".to_string(), "close".to_string()),
                ("Cache-Control".to_string(), "no-cache".to_string()),
            ],
            body: Vec::new(),
        }
    }

    pub fn with_html_body(mut self, body: String) -> Self {
        self.headers.push((
            "Content-Type".to_string(),
            "text/html; charset=utf-8".to_string(),
        ));
        self.body = body.into_bytes();
        self
    }

    pub fn with_file_body(mut self, body: Vec<u8>, mime_type: &str) -> Self {
        self.headers
            .push(("Content-Type".to_string(), mime_type.to_string()));
        self.body = body;
        self
    }

    pub fn with_auth_challenge(mut self) -> Self {
        self.headers.push((
            "WWW-Authenticate".to_string(),
            "Basic realm=\"Restricted\"".to_string(),
        ));
        self
    }

    pub fn add_header(mut self, name: String, value: String) -> Self {
        self.headers.push((name, value));
        self
    }

    pub fn send(self, stream: &mut TcpStream, log_prefix: &str) -> Result<(), AppError> {
        debug!(
            "{} Sending response - Status: {}, Body Length: {}",
            log_prefix,
            self.status_code,
            self.body.len()
        );

        let mut response = format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_text);

        // Add Content-Length header
        response.push_str(&format!("Content-Length: {}\r\n", self.body.len()));

        // Add all headers
        for (name, value) in &self.headers {
            response.push_str(&format!("{name}: {value}\r\n"));
        }

        response.push_str("\r\n");

        // Send headers
        stream.write_all(response.as_bytes()).map_err(|e| {
            error!("{log_prefix} Failed to write response headers: {e}");
            AppError::Io(e)
        })?;

        // Send body
        if !self.body.is_empty() {
            stream.write_all(&self.body).map_err(|e| {
                error!("{log_prefix} Failed to write response body: {e}");
                AppError::Io(e)
            })?;
        }

        stream.flush().map_err(|e| {
            error!("{log_prefix} Failed to flush response: {e}");
            AppError::Io(e)
        })?;

        Ok(())
    }
}

/// Create error response with beautiful HTML error page
pub fn create_error_response(status_code: u16, status_text: &str) -> HttpResponse {
    let error_page = generate_error_page(status_code, status_text);
    let mut response = HttpResponse::new(status_code, status_text).with_html_body(error_page);

    if status_code == 401 {
        response = response.with_auth_challenge();
    }

    response
}

/// Legacy function for compatibility - will be removed in refactor
pub fn send_response(
    stream: &mut TcpStream,
    status_code: u16,
    status_text: &str,
    body: &str,
    log_prefix: &str,
) -> Result<(), AppError> {
    let response = if status_code >= 400 {
        create_error_response(status_code, status_text)
    } else {
        HttpResponse::new(status_code, status_text).with_html_body(body.to_string())
    };

    response.send(stream, log_prefix)
}
