// SPDX-License-Identifier: MIT

use crate::error::AppError;
use log::{debug, trace};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

// Helper function to percent-encode path segments for URLs. 🌐
pub fn percent_encode_path(path: &Path) -> String {
    let path_str = path.to_string_lossy();

    // Handle empty path
    if path_str.is_empty() {
        return String::new();
    }

    // Handle root path
    if path_str == "/" {
        return "/".to_string();
    }

    // Percent-encode the path
    path_str
        .chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '"' => "%22".to_string(),
            '#' => "%23".to_string(),
            '%' => "%25".to_string(),
            '<' => "%3C".to_string(),
            '>' => "%3E".to_string(),
            '?' => "%3F".to_string(),
            // Encode non-ASCII characters
            c if !c.is_ascii() => {
                let mut buf = [0; 4];
                let encoded = c.encode_utf8(&mut buf);
                encoded
                    .bytes()
                    .map(|b| format!("%{:02X}", b))
                    .collect::<String>()
            }
            c => c.to_string(),
        })
        .collect()
}

// Extracts the requested path from the HTTP request line. 🗺️
pub fn get_request_path(request_line: &str) -> &str {
    // Check if the request line starts with "GET ". 🔍
    if let Some(after_get) = request_line.strip_prefix("GET ") {
        // Skip "GET " and find the path

        // Skip any additional spaces after GET
        let after_get = after_get.trim_start();

        if after_get.is_empty() {
            return "/";
        }

        // Find the path by looking for the space before "HTTP/" version
        let path = if let Some(http_pos) = after_get.find(" HTTP/") {
            after_get[..http_pos].trim()
        } else {
            // No HTTP version, take the rest as path
            after_get.trim()
        };

        if path.is_empty() {
            return "/";
        }

        // Handle paths that start with "/".
        if let Some(relative_path) = path.strip_prefix("/") {
            // Remove the leading "/".
            if relative_path.is_empty() {
                // If it's just "/", return root path.
                return "/";
            } else {
                // Otherwise, return the relative path.
                return relative_path;
            }
        } else {
            // If it doesn't start with "/", return the path as is.
            return path;
        }
    }
    "/" // Default to root path if request line parsing fails - safer fallback. 🗺️
}

/// Parse query parameters from a URL
pub fn parse_query_params(url: &str) -> HashMap<String, String> {
    trace!("Parsing query parameters from URL: {}", url);
    let mut params = HashMap::new();

    if let Some(query_start) = url.find('?') {
        let query = &url[query_start + 1..];

        for param in query.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                // Simple URL decoding for common characters
                let decoded_value = url_decode(value);
                params.insert(key.to_string(), decoded_value);
            }
        }
    }

    params
}

/// Simple URL decoding for common percent-encoded characters
fn url_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            // Try to decode percent-encoded character
            if let (Some(hex1), Some(hex2)) = (chars.next(), chars.next()) {
                if let Ok(byte_val) = u8::from_str_radix(&format!("{hex1}{hex2}"), 16)
                    && let Some(decoded_char) = char::from_u32(byte_val as u32)
                {
                    result.push(decoded_char);
                    continue;
                }
                // If decoding failed, keep the original characters
                result.push('%');
                result.push(hex1);
                result.push(hex2);
            } else {
                result.push(ch);
            }
        } else if ch == '+' {
            // Handle + as space in query parameters
            result.push(' ');
        } else {
            result.push(ch);
        }
    }

    result
}

/// Resolve upload directory based on base directory and optional upload_to parameter
pub fn resolve_upload_directory(
    base_dir: &Path,
    upload_to: Option<&str>,
) -> Result<PathBuf, AppError> {
    debug!(
        "Resolving upload directory: base_dir={:?}, upload_to={:?}",
        base_dir, upload_to
    );
    match upload_to {
        Some(path_str) => {
            // Parse and validate the upload path
            let requested_path = PathBuf::from(path_str.strip_prefix('/').unwrap_or(path_str));
            let safe_path = normalize_path(&requested_path)?;
            let target_dir = base_dir.join(safe_path);

            // Security: Ensure target is within base directory
            if !target_dir.starts_with(base_dir) {
                return Err(AppError::Forbidden);
            }

            // Ensure target directory exists and is a directory
            if !target_dir.exists() {
                return Err(AppError::NotFound);
            }

            if !target_dir.is_dir() {
                return Err(AppError::NotFound);
            }

            Ok(target_dir)
        }
        None => {
            // Fall back to base directory
            Ok(base_dir.to_path_buf())
        }
    }
}

/// Safe path normalization to prevent directory traversal
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
            _ => {} // Ignore root, current dir, etc.
        }
    }
    Ok(components.iter().collect())
}
