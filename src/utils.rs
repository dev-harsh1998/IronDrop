use crate::error::AppError;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

// Helper function to percent-encode path segments for URLs. 🌐
pub fn percent_encode_path(path: &Path) -> String {
    path.components() // Iterate over path components. 🚶
        .filter_map(|component| match component {
            // Filter and map path components. 🗺️
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()), // For normal components (filenames/dirnames), convert to String.
            _ => None, // Skip RootDir, ParentDir, CurDir, Prefix components - we don't need to encode these special components.
        })
        .collect::<Vec<_>>() // Collect all String components into a vector.
        .join("/") // Join the components with "/" to form the path string.
        .replace(" ", "%20") // Replace spaces with "%20" for URL encoding - important for spaces in filenames!
}

// Extracts the requested path from the HTTP request line. 🗺️
pub fn get_request_path(request_line: &str) -> &str {
    // Check if the request line starts with "GET ". 🔍
    if request_line.starts_with("GET ") {
        // Find the first space after "GET " - this marks the start of the path.
        if let Some(path_start_index) = request_line.find(' ') {
            // Get the part of the request line after "GET ".
            let path_with_http_version = &request_line[path_start_index + 1..];
            // Find the next space - this marks the end of the path (before HTTP version).
            if let Some(path_end_index) = path_with_http_version.find(' ') {
                // Extract the path part.
                let path = &path_with_http_version[..path_end_index];
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
            } else {
                // If there's no second space (unusual HTTP request but handle it).
                let path = path_with_http_version; // Take the rest as path.
                                                   // Handle paths starting with "/".
                if let Some(relative_path) = path.strip_prefix("/") {
                    // Remove leading "/".
                    if relative_path.is_empty() {
                        // If it's just "/", return root path.
                        return "/";
                    } else {
                        // Otherwise return the relative path.
                        return relative_path;
                    }
                } else {
                    // If it doesn't start with "/", return the path as is.
                    return path;
                }
            }
        }
    }
    "/" // Default to root path if request line parsing fails - safer fallback. 🗺️
}

/// Parse query parameters from a URL
pub fn parse_query_params(url: &str) -> HashMap<String, String> {
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
                if let Ok(byte_val) = u8::from_str_radix(&format!("{hex1}{hex2}"), 16) {
                    if let Some(decoded_char) = char::from_u32(byte_val as u32) {
                        result.push(decoded_char);
                        continue;
                    }
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
