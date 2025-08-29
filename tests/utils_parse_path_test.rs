// SPDX-License-Identifier: MIT

use irondrop::utils::get_request_path;

#[test]
fn test_get_request_path_variants() {
    // Standard GET
    assert_eq!(get_request_path("GET /abc HTTP/1.1"), "abc");
    // Root
    assert_eq!(get_request_path("GET / HTTP/1.1"), "/");
    // No leading slash
    assert_eq!(get_request_path("GET abc HTTP/1.1"), "abc");
    // Missing version
    assert_eq!(get_request_path("GET /abc"), "abc");
    // Non-GET still returns fallback
    assert_eq!(get_request_path("POST /p HTTP/1.1"), "/");
}

#[test]
fn test_get_request_path_edge_cases() {
    // Empty string
    assert_eq!(get_request_path(""), "/");

    // Only method
    assert_eq!(get_request_path("GET"), "/");

    // Method with space but no path
    assert_eq!(get_request_path("GET "), "/");

    // Malformed request line
    assert_eq!(get_request_path("INVALID REQUEST"), "/");

    // Multiple spaces
    assert_eq!(get_request_path("GET    /path    HTTP/1.1"), "path");
}

#[test]
fn test_get_request_path_special_characters() {
    // Path with query parameters
    assert_eq!(
        get_request_path("GET /path?param=value HTTP/1.1"),
        "path?param=value"
    );

    // Path with fragment
    assert_eq!(
        get_request_path("GET /path#fragment HTTP/1.1"),
        "path#fragment"
    );

    // Path with encoded characters
    assert_eq!(
        get_request_path("GET /path%20with%20spaces HTTP/1.1"),
        "path%20with%20spaces"
    );

    // Path with special characters
    assert_eq!(
        get_request_path("GET /path/with-special_chars.html HTTP/1.1"),
        "path/with-special_chars.html"
    );
}

#[test]
fn test_get_request_path_different_methods() {
    // Various HTTP methods (should all return fallback "/")
    assert_eq!(get_request_path("POST /upload HTTP/1.1"), "/");
    assert_eq!(get_request_path("PUT /resource HTTP/1.1"), "/");
    assert_eq!(get_request_path("DELETE /item HTTP/1.1"), "/");
    assert_eq!(get_request_path("HEAD /info HTTP/1.1"), "/");
    assert_eq!(get_request_path("OPTIONS /options HTTP/1.1"), "/");
    assert_eq!(get_request_path("PATCH /update HTTP/1.1"), "/");
}

#[test]
fn test_get_request_path_http_versions() {
    // Different HTTP versions
    assert_eq!(get_request_path("GET /path HTTP/1.0"), "path");
    assert_eq!(get_request_path("GET /path HTTP/1.1"), "path");
    assert_eq!(get_request_path("GET /path HTTP/2.0"), "path");

    // Invalid HTTP version
    assert_eq!(get_request_path("GET /path HTTP/INVALID"), "path");

    // Missing HTTP version
    assert_eq!(get_request_path("GET /path"), "path");
}

#[test]
fn test_get_request_path_case_sensitivity() {
    // Method case variations (only GET should work)
    assert_eq!(get_request_path("get /path HTTP/1.1"), "/");
    assert_eq!(get_request_path("Get /path HTTP/1.1"), "/");
    assert_eq!(get_request_path("GEt /path HTTP/1.1"), "/");
    assert_eq!(get_request_path("GET /path HTTP/1.1"), "path"); // Only exact match works
}

#[test]
fn test_get_request_path_long_paths() {
    // Very long path
    let long_path = "/".to_string() + &"a".repeat(1000);
    let request = format!("GET {} HTTP/1.1", long_path);
    let expected = long_path.trim_start_matches('/');
    assert_eq!(get_request_path(&request), expected);
}

#[test]
fn test_get_request_path_unicode() {
    // Unicode characters in path
    assert_eq!(get_request_path("GET /файл.txt HTTP/1.1"), "файл.txt");
    assert_eq!(get_request_path("GET /文件.html HTTP/1.1"), "文件.html");
    assert_eq!(get_request_path("GET /café/résumé HTTP/1.1"), "café/résumé");
}

#[test]
fn test_get_request_path_whitespace_variations() {
    // Tab characters
    assert_eq!(get_request_path("GET\t/path\tHTTP/1.1"), "/"); // Tabs should not work

    // Leading/trailing whitespace in path
    assert_eq!(get_request_path("GET / path / HTTP/1.1"), " path /");

    // Newlines (should not work)
    assert_eq!(get_request_path("GET\n/path\nHTTP/1.1"), "/");
}
