// SPDX-License-Identifier: MIT

//! Centralized request handlers for internal (/_irondrop/...) routes.
//! Keeps `http.rs` minimal and focused on parsing and fallback file serving.

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::AppError;
use crate::http::{Request, Response, ResponseBody};
use crate::search::{perform_search, SearchParams, SearchResult};
use crate::upload::DirectUploadHandler;
use crate::utils::parse_query_params;
use log::{debug, error, trace};
use std::time::Instant;

/// Register all internal routes under /_irondrop/.
pub fn register_internal_routes(
    router: &mut crate::router::Router,
    cli: Option<Arc<crate::cli::Cli>>,
    stats: Option<Arc<crate::server::ServerStats>>,
    base_dir: Option<Arc<std::path::PathBuf>>,
) {
    // Health & status
    router.register_exact(
        "GET",
        "/_irondrop/health",
        Box::new(|_| Ok(create_health_check_response())),
    );
    router.register_exact(
        "GET",
        "/_irondrop/status",
        Box::new(|_| Ok(create_health_check_response())),
    );

    // Compatibility routes for legacy endpoints
    router.register_exact(
        "GET",
        "/_health",
        Box::new(|_| Ok(create_health_check_response())),
    );

    // Legacy monitor endpoint compatibility
    if let Some(stats_arc) = stats.clone() {
        router.register_exact(
            "GET",
            "/monitor",
            Box::new(move |req: &Request| handle_monitor_request(req, Some(stats_arc.as_ref()))),
        );
    }

    // Static assets (new namespace)
    router.register_prefix(
        "GET",
        "/_irondrop/static/",
        Box::new(|req: &Request| handle_static_asset(&req.path)),
    );

    // Logo route (binary PNG)
    router.register_exact(
        "GET",
        "/_irondrop/logo",
        Box::new(|_| handle_logo_request()),
    );

    // Favicons (kept at root for browser compatibility)
    for icon in ["/favicon.ico", "/favicon-16x16.png", "/favicon-32x32.png"] {
        let path = icon.to_string();
        router.register_exact(
            "GET",
            path.clone(),
            Box::new(move |req: &Request| handle_favicon_request(&req.path)),
        );
    }

    // Upload endpoints
    if let Some(cli_arc) = cli.clone() {
        let cli_for_get = cli_arc.clone();
        let base_for_get = base_dir.clone();
        router.register_exact(
            "GET",
            "/_irondrop/upload",
            Box::new(move |req: &Request| {
                handle_upload_form_request(req, Some(cli_for_get.as_ref()), base_for_get.as_deref())
            }),
        );
        let cli_for_post = cli_arc.clone();
        let stats_for_post = stats.clone();
        let base_for_post = base_dir.clone();
        router.register_exact(
            "POST",
            "/_irondrop/upload",
            Box::new(move |req: &Request| {
                handle_upload_request(
                    req,
                    Some(cli_for_post.as_ref()),
                    stats_for_post.as_deref(),
                    base_for_post.as_deref(),
                )
            }),
        );
    }

    // Monitor endpoint (server metrics)
    if let Some(stats_arc) = stats {
        router.register_exact(
            "GET",
            "/_irondrop/monitor",
            Box::new(move |req: &Request| handle_monitor_request(req, Some(stats_arc.as_ref()))),
        );
    }

    // Search endpoint
    if let Some(base_arc) = base_dir {
        router.register_exact(
            "GET",
            "/_irondrop/search",
            Box::new(move |req: &Request| handle_search_api_request(req, &base_arc)),
        );
    }
}

pub fn create_health_check_response() -> Response {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let health_info = format!(
        r#"{{
    "status": "healthy",
    "service": "irondrop",
    "version": "{}",
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
}}"#,
        crate::VERSION
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

pub fn handle_static_asset(path: &str) -> Result<Response, AppError> {
    use crate::templates::TemplateEngine;
    let asset_path = path.strip_prefix("/_irondrop/static/").unwrap_or("");
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

pub fn handle_favicon_request(path: &str) -> Result<Response, AppError> {
    use crate::templates::TemplateEngine;
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
            );
            map.insert("Content-Length".to_string(), content.len().to_string());
            map
        },
        body: ResponseBody::Binary(content.to_vec()),
    })
}

fn handle_logo_request() -> Result<Response, AppError> {
    use crate::templates::TemplateEngine;
    let engine = TemplateEngine::new();
    let (content, content_type) = engine
        .get_favicon("irondrop-logo.png")
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
            map.insert("Content-Length".to_string(), content.len().to_string());
            map
        },
        body: ResponseBody::Binary(content.to_vec()),
    })
}

pub fn handle_upload_form_request(
    request: &Request,
    cli_config: Option<&crate::cli::Cli>,
    _base_dir: Option<&std::path::PathBuf>,
) -> Result<Response, AppError> {
    debug!("Handling upload form request for path: {}", request.path);
    let cli = cli_config.ok_or_else(|| {
        AppError::InternalServerError(
            "CLI configuration not available for upload handling".to_string(),
        )
    })?;
    if !cli.enable_upload.unwrap_or(false) {
        return Err(AppError::upload_disabled());
    }

    // Parse query parameters to get upload directory
    let query_params = parse_query_params(&request.path);
    let upload_to = query_params.get("upload_to").map(String::as_str);

    let engine = crate::templates::TemplateEngine::new();
    let path = upload_to.unwrap_or("/");

    let html = engine.render_upload_page(path)?;
    Ok(Response {
        status_code: 200,
        status_text: "OK".into(),
        headers: {
            let mut m = HashMap::new();
            m.insert("Content-Type".into(), "text/html; charset=utf-8".into());
            m.insert("Cache-Control".into(), "no-cache".into());
            m
        },
        body: ResponseBody::Text(html),
    })
}

pub fn handle_upload_request(
    request: &Request,
    cli_config: Option<&crate::cli::Cli>,
    stats: Option<&crate::server::ServerStats>,
    base_dir: Option<&std::path::PathBuf>,
) -> Result<Response, AppError> {
    debug!(
        "Processing upload request: method={}, path={}",
        request.method, request.path
    );
    trace!("Upload request headers: {:?}", request.headers);
    debug!(
        "Handling upload request from {}",
        request.headers.get("host").map_or("unknown", |v| v)
    );
    trace!(
        "Upload request headers: {:?}",
        request.headers.keys().collect::<Vec<_>>()
    );

    let cli = cli_config.ok_or_else(|| {
        error!("CLI configuration not available for upload handling");
        AppError::InternalServerError(
            "CLI configuration not available for upload handling".to_string(),
        )
    })?;
    if !cli.enable_upload.unwrap_or(false) {
        debug!("Upload disabled in configuration");
        return Err(AppError::upload_disabled());
    }

    // Parse query parameters to get upload directory
    let query_params = parse_query_params(&request.path);
    let upload_to = query_params.get("upload_to").map(String::as_str);

    // Resolve target directory
    let upload_handler = if let Some(base) = base_dir {
        debug!(
            "Resolving upload directory - base: {}, upload_to: {:?}",
            base.display(),
            upload_to
        );
        let target_dir = crate::utils::resolve_upload_directory(base, upload_to)?;
        debug!("Target upload directory: {}", target_dir.display());
        trace!("Target directory exists: {}", target_dir.exists());
        DirectUploadHandler::new_with_directory(cli, target_dir)?
    } else {
        debug!("Using default upload handler without base directory");
        DirectUploadHandler::new(cli)?
    };

    let mut upload_handler = upload_handler;
    let start_time = std::time::Instant::now();

    match upload_handler.handle_upload_with_stats(request, stats) {
        Ok(http_response) => {
            let upload_time = start_time.elapsed();
            debug!("Upload completed successfully in {:?}", upload_time);
            trace!("Upload response status: {}", http_response.status_code);

            let mut headers = HashMap::new();
            for (k, v) in http_response.headers {
                headers.insert(k, v);
            }
            let body = ResponseBody::Text(String::from_utf8_lossy(&http_response.body).to_string());
            Ok(Response {
                status_code: http_response.status_code,
                status_text: http_response.status_text,
                headers,
                body,
            })
        }
        Err(e) => {
            let upload_time = start_time.elapsed();
            error!("Upload failed after {:?}: {}", upload_time, e);
            debug!("Upload error details: {:?}", e);
            Err(e)
        }
    }
}

pub fn handle_monitor_request(
    request: &Request,
    stats: Option<&crate::server::ServerStats>,
) -> Result<Response, AppError> {
    debug!("Handling monitor request for path: {}", request.path);
    trace!(
        "Monitor request query params: {:?}",
        parse_query_params(&request.path)
    );
    // Check if JSON response is requested
    if request.path.contains("json=1") {
        return Ok(create_monitor_json(stats));
    }

    // Return HTML response
    let engine = crate::templates::TemplateEngine::new();
    match engine.render_monitor_page() {
        Ok(html) => Ok(Response {
            status_code: 200,
            status_text: "OK".into(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".into(), "text/html; charset=utf-8".into());
                h.insert("Cache-Control".into(), "no-cache".into());
                h
            },
            body: ResponseBody::Text(html),
        }),
        Err(_) => {
            // Fallback to JSON if HTML rendering fails
            Ok(create_monitor_json(stats))
        }
    }
}

fn create_monitor_json(stats: Option<&crate::server::ServerStats>) -> Response {
    if let Some(s) = stats {
        let (total, successful, errors, bytes, uptime) = s.get_stats();
        let up = s.get_upload_stats();
        let (current_memory, peak_memory, memory_available) = s.get_memory_usage();

        // Build memory section based on availability
        let memory_section = if memory_available {
            let current_bytes = current_memory.unwrap_or(0);
            let peak_bytes = peak_memory.unwrap_or(0);
            format!(
                r#""memory":{{"available":true,"current_bytes":{},"peak_bytes":{},"current_mb":{:.2},"peak_mb":{:.2}}}"#,
                current_bytes,
                peak_bytes,
                current_bytes as f64 / 1024.0 / 1024.0,
                peak_bytes as f64 / 1024.0 / 1024.0
            )
        } else {
            r#""memory":{"available":false,"current_bytes":null,"peak_bytes":null,"current_mb":null,"peak_mb":null}"#.to_string()
        };

        let json = format!(
            r#"{{"requests":{{"total":{total},"successful":{successful},"errors":{errors}}},"downloads":{{"bytes_served":{bytes}}},"uptime_secs":{},{},"uploads":{{"total_uploads":{},"successful_uploads":{},"failed_uploads":{},"files_uploaded":{},"upload_bytes":{},"average_upload_size":{},"largest_upload":{},"concurrent_uploads":{},"average_processing_ms":{:.2},"success_rate":{:.2}}}}}"#,
            uptime.as_secs(),
            memory_section,
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

/// Handle file and directory serving requests
/// This moves the file serving logic from http.rs to handlers.rs for better separation of concerns
pub fn handle_file_request(
    request: &Request,
    base_dir: &std::path::PathBuf,
    allowed_extensions: &[glob::Pattern],
    chunk_size: usize,
    cli_config: Option<&crate::cli::Cli>,
) -> Result<Response, AppError> {
    debug!(
        "Handling file request: method={}, path={}",
        request.method, request.path
    );
    trace!("Base directory: {:?}, chunk size: {}", base_dir, chunk_size);
    use crate::fs::{generate_directory_listing, FileDetails};
    use crate::response::get_mime_type;
    use log::debug;
    use std::path::PathBuf;

    debug!("Handling file request for path: {}", request.path);
    trace!(
        "Base directory: {}, chunk_size: {}",
        base_dir.display(),
        chunk_size
    );
    trace!(
        "Allowed extensions: {:?}",
        allowed_extensions
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
    );

    // Handle different methods appropriately
    match request.method.as_str() {
        "GET" => {
            // GET requests are handled normally
            trace!("Processing GET request");
        }
        "POST" => {
            // For now, POST requests are only accepted but not fully implemented
            // In a real implementation, this would handle file uploads
            // For the current implementation, we'll allow POST but treat it like GET for basic functionality
            debug!("POST request received, treating as GET for basic functionality");
        }
        _ => {
            debug!("Method not allowed: {}", request.method);
            return Err(AppError::MethodNotAllowed);
        }
    }

    let requested_path = PathBuf::from(request.path.strip_prefix('/').unwrap_or(&request.path));
    debug!("Requested path: {}", requested_path.display());

    let safe_path = normalize_path(&requested_path)?;
    let full_path = base_dir.join(safe_path);

    debug!("Full resolved path: {}", full_path.display());
    trace!(
        "Path components - requested: '{}', full: '{}'",
        requested_path.display(),
        full_path.display()
    );

    if !full_path.starts_with(base_dir) {
        debug!("Path traversal attempt blocked: {}", full_path.display());
        return Err(AppError::Forbidden);
    }

    if !full_path.exists() {
        debug!("Path does not exist: {}", full_path.display());
        trace!("File system check failed for path");
        return Err(AppError::NotFound);
    }

    trace!("Path exists, checking if directory or file");

    if full_path.is_dir() {
        debug!("Serving directory listing for: {}", full_path.display());
        trace!("Directory listing requested for path: {}", request.path);

        // Only serve directory listings for GET requests
        if request.method == "POST" {
            debug!("POST method not allowed for directory listings");
            return Err(AppError::MethodNotAllowed);
        }

        // Create a config from CLI if available
        let config = cli_config.map(|cli| crate::config::Config {
            listen: "127.0.0.1".to_string(),
            port: 8080,
            threads: 8,
            chunk_size: 1024,
            directory: cli.directory.clone(),
            enable_upload: cli.enable_upload.unwrap_or(false),
            max_upload_size: cli.max_upload_size_bytes(),
            username: cli.username.clone(),
            password: cli.password.clone(),
            allowed_extensions: cli
                .allowed_extensions
                .as_ref()
                .unwrap_or(&"*".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            verbose: cli.verbose.unwrap_or(false),
            detailed_logging: cli.detailed_logging.unwrap_or(false),
            log_file: cli.log_file.clone(),
        });

        let html_content = generate_directory_listing(&full_path, &request.path, config.as_ref())?;
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
        debug!("Serving file: {}", full_path.display());

        if !allowed_extensions
            .iter()
            .any(|p| p.matches_path(&full_path))
        {
            debug!("File extension not allowed for: {}", full_path.display());
            trace!("Extension validation failed, returning Forbidden");
            return Err(AppError::Forbidden);
        }

        trace!("File extension validation passed");

        let file_details = FileDetails::new(full_path.clone(), chunk_size)?;
        let mime_type = get_mime_type(&full_path);

        debug!(
            "File details - size: {} bytes, mime_type: {}",
            file_details.size, mime_type
        );
        trace!("Chunk size for streaming: {}", chunk_size);
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

/// A safe, manual path normalization function.
fn normalize_path(path: &std::path::Path) -> Result<std::path::PathBuf, AppError> {
    use std::path::Component;

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

/// URL decode function for parsing query parameters
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push(ch);
            }
        } else if ch == '+' {
            result.push(' ');
        } else {
            result.push(ch);
        }
    }
    result
}

/// Handle search API requests with optimizations
pub fn handle_search_api_request(
    request: &Request,
    base_dir: &Arc<std::path::PathBuf>,
) -> Result<Response, AppError> {
    debug!("Processing search API request for path: {}", request.path);
    trace!("Search base directory: {:?}", base_dir);
    let start_time = Instant::now();
    debug!("Handling search API request: {}", request.path);
    trace!("Search base directory: {}", base_dir.display());

    // Parse query parameters manually
    let query_params: HashMap<String, String> =
        if let Some(query_string) = request.path.split('?').nth(1) {
            query_string
                .split('&')
                .filter_map(|param| {
                    let mut parts = param.splitn(2, '=');
                    match (parts.next(), parts.next()) {
                        (Some(key), Some(value)) => Some((url_decode(key), url_decode(value))),
                        _ => None,
                    }
                })
                .collect()
        } else {
            HashMap::new()
        };

    let search_query = query_params.get("q").ok_or_else(|| {
        debug!("Search query parameter 'q' missing");
        AppError::BadRequest
    })?;

    debug!("Search query: '{}'", search_query);

    // Validate query length for performance
    if search_query.len() < 2 {
        debug!("Search query too short: {} characters", search_query.len());
        return Err(AppError::BadRequest);
    }
    if search_query.len() > 100 {
        debug!("Search query too long: {} characters", search_query.len());
        return Err(AppError::BadRequest);
    }

    let search_path = query_params.get("path").map_or("/", |v| v);
    let limit = query_params
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(50)
        .min(200); // Cap at 200 results
    let offset = query_params
        .get("offset")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);

    debug!(
        "Search parameters - path: '{}', limit: {}, offset: {}",
        search_path, limit, offset
    );
    trace!("Search query validation passed");

    let params = SearchParams {
        query: search_query.clone(),
        path: search_path.to_string(),
        limit,
        offset,
        case_sensitive: false,
    };

    // Perform optimized search with caching and indexing
    debug!("Performing search with parameters: {:?}", params);
    let mut results = perform_search(base_dir, &params)?;
    debug!("Search returned {} results", results.len());

    // Sort by relevance score
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Apply pagination
    let total_count = results.len();
    let paginated_results: Vec<SearchResult> =
        results.into_iter().skip(offset).take(limit).collect();

    let elapsed_ms = start_time.elapsed().as_millis();
    debug!(
        "Search completed in {}ms, returning {} of {} results",
        elapsed_ms,
        paginated_results.len(),
        total_count
    );
    trace!("Pagination applied - offset: {}, limit: {}", offset, limit);

    // Create simple JSON manually to avoid serde dependency
    let json_items: Vec<String> = paginated_results
        .iter()
        .map(|result| {
            format!(
                r#"{{"name":"{}","path":"{}","size":"{}","type":"{}"}}"#,
                result.name.replace('"', r#"\""#),
                result.path.replace('"', r#"\""#),
                result.size,
                result.file_type
            )
        })
        .collect();

    let json_response = format!("[{}]", json_items.join(","));

    Ok(Response {
        status_code: 200,
        status_text: "OK".to_string(),
        headers: {
            let mut map = HashMap::new();
            map.insert("Content-Type".to_string(), "application/json".to_string());
            map.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
            map
        },
        body: ResponseBody::Text(json_response),
    })
}
