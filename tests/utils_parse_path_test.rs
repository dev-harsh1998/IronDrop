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
