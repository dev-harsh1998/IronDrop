//! Centralized request handlers for internal (/_irondrop/...) routes.
//! Keeps `http.rs` minimal and focused on parsing and fallback file serving.

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::AppError;
use crate::http::{Request, Response, ResponseBody};
use crate::upload::UploadHandler;
use crate::utils::parse_query_params;

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
}

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

fn handle_static_asset(path: &str) -> Result<Response, AppError> {
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

fn handle_favicon_request(path: &str) -> Result<Response, AppError> {
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

fn handle_upload_form_request(
    request: &Request,
    cli_config: Option<&crate::cli::Cli>,
    _base_dir: Option<&std::path::PathBuf>,
) -> Result<Response, AppError> {
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

fn handle_upload_request(
    request: &Request,
    cli_config: Option<&crate::cli::Cli>,
    stats: Option<&crate::server::ServerStats>,
    base_dir: Option<&std::path::PathBuf>,
) -> Result<Response, AppError> {
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

    // Resolve target directory
    let upload_handler = if let Some(base) = base_dir {
        let target_dir = crate::utils::resolve_upload_directory(base, upload_to)?;
        UploadHandler::new_with_directory(cli, target_dir)?
    } else {
        UploadHandler::new(cli)?
    };

    let mut upload_handler = upload_handler;
    let http_response = upload_handler.handle_upload_with_stats(request, stats)?;
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
