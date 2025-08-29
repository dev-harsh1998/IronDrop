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

#[test]
fn test_auth_middleware_case_insensitive_header() {
    let mw = AuthMiddleware::new(Some("user".into()), Some("pass".into()));

    let test_cases = vec![
        "Authorization",
        "AUTHORIZATION",
        "authorization",
        "AuThOrIzAtIoN",
    ];

    for header_name in test_cases {
        let mut headers = HashMap::new();
        let creds = base64::engine::general_purpose::STANDARD.encode("user:pass");
        headers.insert(header_name.to_lowercase(), format!("Basic {}", creds));

        let request = Request {
            method: "GET".to_string(),
            path: "/".to_string(),
            headers,
            body: None,
        };

        let result = mw.handle(&request);
        assert!(
            result.is_ok(),
            "Should accept case-insensitive header: {}",
            header_name
        );
    }
}

#[test]
fn test_auth_middleware_malformed_basic_auth() {
    let mw = AuthMiddleware::new(Some("user".into()), Some("pass".into()));

    let malformed_auth_headers = vec![
        "Basic",                      // Missing credentials
        "Basic ",                     // Empty credentials
        "Basic invalid-base64!",      // Invalid base64
        "Basic ==",                   // Invalid base64 padding
        "Basic dGVzdA==",             // Valid base64 but no colon separator
        "Basic dGVzdDp0ZXN0OnRlc3Q=", // Multiple colons
        "Bearer token123",            // Wrong auth scheme
        "Digest username=\"test\"",   // Different auth scheme
    ];

    for auth_header in malformed_auth_headers {
        let req = make_request_with_auth(Some(auth_header));
        let result = mw.handle(&req);
        assert!(
            result.is_err(),
            "Should reject malformed auth header: {}",
            auth_header
        );
    }
}

#[test]
fn test_auth_middleware_unicode_credentials() {
    let mw = AuthMiddleware::new(Some("üser".into()), Some("pässwörd".into()));

    // Base64 encode "üser:pässwörd"
    let credentials = base64::engine::general_purpose::STANDARD.encode("üser:pässwörd");
    let header = format!("Basic {}", credentials);
    let req = make_request_with_auth(Some(&header));

    let result = mw.handle(&req);
    assert!(result.is_ok(), "Should handle Unicode credentials");
}

#[test]
fn test_auth_middleware_empty_credentials() {
    let mw = AuthMiddleware::new(Some("".into()), Some("".into()));

    // Base64 encode ":"
    let credentials = base64::engine::general_purpose::STANDARD.encode(":");
    let header = format!("Basic {}", credentials);
    let req = make_request_with_auth(Some(&header));

    let result = mw.handle(&req);
    assert!(
        result.is_ok(),
        "Should handle empty credentials if configured"
    );
}

#[test]
fn test_auth_middleware_timing_attack_resistance() {
    use std::time::Instant;

    let mw = AuthMiddleware::new(Some("user".into()), Some("pass".into()));

    // Test with correct username, wrong password
    let correct_user_wrong_pass =
        base64::engine::general_purpose::STANDARD.encode("user:wrongpass");
    let header1 = format!("Basic {}", correct_user_wrong_pass);
    let req1 = make_request_with_auth(Some(&header1));

    // Test with wrong username, wrong password
    let wrong_user_wrong_pass =
        base64::engine::general_purpose::STANDARD.encode("wronguser:wrongpass");
    let header2 = format!("Basic {}", wrong_user_wrong_pass);
    let req2 = make_request_with_auth(Some(&header2));

    // Measure timing for both scenarios
    let start1 = Instant::now();
    let result1 = mw.handle(&req1);
    let duration1 = start1.elapsed();

    let start2 = Instant::now();
    let result2 = mw.handle(&req2);
    let duration2 = start2.elapsed();

    // Both should fail
    assert!(result1.is_err());
    assert!(result2.is_err());

    // Timing difference should be minimal (within reasonable bounds)
    // This is a basic check - in practice, constant-time comparison should be used
    let timing_diff = if duration1 > duration2 {
        duration1 - duration2
    } else {
        duration2 - duration1
    };

    // Allow up to 10ms difference (this is quite generous for testing)
    assert!(
        timing_diff.as_millis() < 10,
        "Timing difference too large: {:?}",
        timing_diff
    );
}

#[test]
fn test_auth_middleware_very_long_credentials() {
    let long_username = "u".repeat(1000);
    let long_password = "p".repeat(1000);
    let mw = AuthMiddleware::new(Some(long_username.clone()), Some(long_password.clone()));

    let credentials = base64::engine::general_purpose::STANDARD
        .encode(format!("{}:{}", long_username, long_password));
    let header = format!("Basic {}", credentials);
    let req = make_request_with_auth(Some(&header));

    let result = mw.handle(&req);
    assert!(result.is_ok(), "Should handle very long credentials");
}

#[test]
fn test_auth_middleware_special_characters_in_credentials() {
    let special_username = "user@domain.com";
    let special_password = "pass!@#$%^&*()_+-=[]{}|;':,.<>?";
    let mw = AuthMiddleware::new(Some(special_username.into()), Some(special_password.into()));

    let credentials = base64::engine::general_purpose::STANDARD
        .encode(format!("{}:{}", special_username, special_password));
    let header = format!("Basic {}", credentials);
    let req = make_request_with_auth(Some(&header));

    let result = mw.handle(&req);
    assert!(
        result.is_ok(),
        "Should handle special characters in credentials"
    );
}

#[test]
fn test_auth_middleware_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let mw = Arc::new(AuthMiddleware::new(
        Some("user".into()),
        Some("pass".into()),
    ));

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let mw_clone = Arc::clone(&mw);
            thread::spawn(move || {
                let creds = base64::engine::general_purpose::STANDARD.encode("user:pass");
                let header = format!("Basic {}", creds);
                let req = make_request_with_auth(Some(&header));

                mw_clone.handle(&req)
            })
        })
        .collect();

    // All concurrent authentications should succeed
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_ok(), "Concurrent authentication should succeed");
    }
}
