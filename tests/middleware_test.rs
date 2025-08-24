// SPDX-License-Identifier: MIT

use base64::Engine;
use irondrop::http::Request;
use irondrop::middleware::{AuthMiddleware, Middleware};
use std::collections::HashMap;

fn make_request_with_auth(header: Option<&str>) -> Request {
    let mut headers = HashMap::new();
    if let Some(h) = header {
        headers.insert("authorization".to_string(), h.to_string());
    }
    Request {
        method: "GET".to_string(),
        path: "/".to_string(),
        headers,
        body: None,
    }
}

#[test]
fn test_auth_middleware_disabled_allows() {
    let mw = AuthMiddleware::new(None, None);
    let req = make_request_with_auth(None);
    assert!(mw.handle(&req).is_ok());
}

#[test]
fn test_auth_middleware_missing_header_rejects() {
    let mw = AuthMiddleware::new(Some("user".into()), Some("pass".into()));
    let req = make_request_with_auth(None);
    assert!(mw.handle(&req).is_err());
}

#[test]
fn test_auth_middleware_invalid_scheme_rejects() {
    let mw = AuthMiddleware::new(Some("user".into()), Some("pass".into()));
    let req = make_request_with_auth(Some("Bearer abc"));
    assert!(mw.handle(&req).is_err());
}

#[test]
fn test_auth_middleware_invalid_base64_rejects() {
    let mw = AuthMiddleware::new(Some("user".into()), Some("pass".into()));
    let req = make_request_with_auth(Some("Basic !!!notbase64!!!"));
    assert!(mw.handle(&req).is_err());
}

#[test]
fn test_auth_middleware_success() {
    let mw = AuthMiddleware::new(Some("user".into()), Some("pass".into()));
    let creds = base64::engine::general_purpose::STANDARD.encode("user:pass");
    let header = format!("Basic {}", creds);
    let req = make_request_with_auth(Some(&header));
    assert!(mw.handle(&req).is_ok());
}
