//! Comprehensive tests for the enhanced file server without external dependencies.

use irondrop::cli::Cli;
use irondrop::server::run_server;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tempfile::{tempdir, TempDir};

/// A helper struct to manage a running test server without external HTTP clients.
struct TestServer {
    addr: SocketAddr,
    shutdown_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<()>>,
    _temp_dir: TempDir,
}

impl TestServer {
    /// Sets up and runs a server in a background thread for testing.
    fn new(username: Option<String>, password: Option<String>) -> Self {
        let dir = tempdir().unwrap();

        // Create test files
        let test_file = dir.path().join("test.txt");
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, "Hello from test file!").unwrap();

        let binary_file = dir.path().join("test.pdf");
        File::create(&binary_file).unwrap();

        let large_file = dir.path().join("large.txt");
        let mut large = File::create(&large_file).unwrap();
        for i in 0..1000 {
            writeln!(large, "Line {i} of a large file for testing").unwrap();
        }

        // Create subdirectory
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        let subfile = subdir.join("nested.txt");
        let mut nested = File::create(&subfile).unwrap();
        writeln!(nested, "Nested file content").unwrap();

        let cli = Cli {
            directory: dir.path().to_path_buf(),
            listen: Some("127.0.0.1".to_string()),
            port: Some(0),
            allowed_extensions: Some("*.txt,*.pdf".to_string()),
            threads: Some(4),
            chunk_size: Some(1024),
            verbose: Some(false),
            detailed_logging: Some(false),
            username,
            password,
            enable_upload: Some(false),
            max_upload_size: Some(10240),
            config_file: None,
        };

        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        let (addr_tx, addr_rx) = mpsc::channel();

        let server_handle = thread::spawn(move || {
            if let Err(e) = run_server(cli, Some(shutdown_rx), Some(addr_tx)) {
                eprintln!("Server thread failed: {e}");
            }
        });

        let server_addr = addr_rx.recv().unwrap();

        TestServer {
            addr: server_addr,
            shutdown_tx,
            handle: Some(server_handle),
            _temp_dir: dir,
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            // Send shutdown signal
            if self.shutdown_tx.send(()).is_err() {
                eprintln!("Failed to send shutdown signal to server thread.");
            }

            // Wait for the server thread to finish
            if let Err(e) = handle.join() {
                eprintln!("Server thread panicked: {:?}", e);
            }
        }
    }
}

/// Native HTTP client implementation for testing
struct HttpClient;

impl HttpClient {
    fn get(url: &str) -> HttpResponse {
        Self::request("GET", url, None, None)
    }

    fn get_with_auth(url: &str, username: &str, password: &str) -> HttpResponse {
        let credentials = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("{username}:{password}"),
        );
        let auth_header = format!("Basic {credentials}");
        Self::request("GET", url, Some(&auth_header), None)
    }

    fn request(method: &str, url: &str, auth: Option<&str>, body: Option<&str>) -> HttpResponse {
        // Parse URL properly: http://127.0.0.1:8080/path
        let url = url.strip_prefix("http://").unwrap_or(url);
        let parts: Vec<&str> = url.splitn(2, '/').collect();
        let host_port = parts[0];
        let path = if parts.len() > 1 {
            format!("/{}", parts[1])
        } else {
            "/".to_string()
        };

        let mut stream = TcpStream::connect(host_port).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();

        let mut request = format!("{method} {path} HTTP/1.1\r\nHost: {host_port}\r\n");

        if let Some(auth_header) = auth {
            request.push_str(&format!("Authorization: {auth_header}\r\n"));
        }

        if let Some(body_content) = body {
            request.push_str(&format!("Content-Length: {}\r\n", body_content.len()));
        }

        request.push_str("\r\n");

        if let Some(body_content) = body {
            request.push_str(body_content);
        }

        stream.write_all(request.as_bytes()).unwrap();

        let mut reader = BufReader::new(stream);
        let mut status_line = String::new();
        reader.read_line(&mut status_line).unwrap();

        let status_code = status_line
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse::<u16>()
            .unwrap();

        let mut headers = std::collections::HashMap::new();
        let mut reading_headers = true;

        // Read headers line by line
        let mut header_line = String::new();
        while reading_headers {
            header_line.clear();
            reader.read_line(&mut header_line).unwrap();
            if header_line.trim().is_empty() {
                reading_headers = false;
                continue;
            }
            if let Some((key, value)) = header_line.trim().split_once(": ") {
                headers.insert(key.to_lowercase(), value.to_string());
            }
        }

        // Read body as bytes and convert to string if possible
        let mut body_bytes = Vec::new();
        reader.read_to_end(&mut body_bytes).unwrap();

        // Try to convert to string, but fall back to a placeholder for binary data
        let body_content = match String::from_utf8(body_bytes.clone()) {
            Ok(text) => text,
            Err(_) => {
                // For binary data, we'll create a placeholder string that includes the bytes
                // This allows tests to verify the binary content exists
                format!("BINARY_DATA_{}_BYTES", body_bytes.len())
            }
        };

        HttpResponse {
            status_code,
            headers,
            body: body_content,
            body_bytes,
        }
    }
}

struct HttpResponse {
    status_code: u16,
    headers: std::collections::HashMap<String, String>,
    body: String,
    body_bytes: Vec<u8>,
}

#[test]
fn test_enhanced_directory_listing() {
    let server = TestServer::new(None, None);
    let url = format!("http://{}/", server.addr);
    let response = HttpClient::get(&url);

    assert_eq!(response.status_code, 200);
    assert!(response
        .headers
        .get("content-type")
        .unwrap()
        .contains("text/html"));
    assert!(response.body.contains("test.txt"));
    assert!(response.body.contains("subdir/"));
    assert!(response.body.contains("Name"));

    // Check for modular template structure
    assert!(
        response.body.contains("/_irondrop/static/directory/styles.css"),
        "Should link to external CSS"
    );
    assert!(
        response.body.contains("/_irondrop/static/directory/script.js"),
        "Should link to external JS"
    );
    assert!(
        response.body.contains("class=\"container\""),
        "Should use proper CSS classes"
    );

    // Ensure no emoji icons are present
    assert!(!response.body.contains("📁"));
    assert!(!response.body.contains("📄"));
}

#[test]
fn test_beautiful_error_pages() {
    let server = TestServer::new(None, None);
    let url = format!("http://{}/nonexistent", server.addr);
    let response = HttpClient::get(&url);

    assert_eq!(response.status_code, 404);
    assert!(response
        .headers
        .get("content-type")
        .unwrap()
        .contains("text/html"));
    assert!(response.body.contains("404"));
    assert!(response.body.contains("Not Found"));
    assert!(response.body.contains("IronDrop v2.5.0"));

    // Check for modular error page template structure
    assert!(
        response.body.contains("/_irondrop/static/error/styles.css"),
        "Should link to external error CSS"
    );
    assert!(
        response.body.contains("/_irondrop/static/error/script.js"),
        "Should link to external error JS"
    );
    assert!(
        response.body.contains("class=\"error-container\""),
        "Should use proper error CSS classes"
    );

    // Check for modern interaction elements
    assert!(response.body.contains("error-button"));
    assert!(response.body.contains("Go Home"));
}

#[test]
fn test_static_asset_serving() {
    let server = TestServer::new(None, None);

    // Test CSS file serving
    let css_url = format!("http://{}/_irondrop/static/directory/styles.css", server.addr);
    let css_response = HttpClient::get(&css_url);

    assert_eq!(css_response.status_code, 200);
    assert!(css_response
        .headers
        .get("content-type")
        .unwrap()
        .contains("text/css"));
    assert!(
        css_response
            .body
            .contains("Professional Blackish Grey Design"),
        "Should contain design system comment"
    );
    assert!(
        css_response.body.contains("directory-header"),
        "Should contain directory-specific styles"
    );

    // Test JS file serving
    let js_url = format!("http://{}/_irondrop/static/directory/script.js", server.addr);
    let js_response = HttpClient::get(&js_url);

    assert_eq!(js_response.status_code, 200);
    assert!(js_response
        .headers
        .get("content-type")
        .unwrap()
        .contains("application/javascript"));
    assert!(
        js_response.body.contains("DOMContentLoaded"),
        "Should contain valid JavaScript"
    );

    // Test error CSS serving
    let error_css_url = format!("http://{}/_irondrop/static/error/styles.css", server.addr);
    let error_css_response = HttpClient::get(&error_css_url);

    assert_eq!(error_css_response.status_code, 200);
    assert!(error_css_response
        .headers
        .get("content-type")
        .unwrap()
        .contains("text/css"));
    assert!(
        error_css_response.body.contains("error-container"),
        "Should contain error page styles"
    );

    // Test 404 for non-existent static asset
    let missing_url = format!("http://{}/_irondrop/static/nonexistent.css", server.addr);
    let missing_response = HttpClient::get(&missing_url);

    assert_eq!(missing_response.status_code, 404);
}

#[test]
fn test_health_check_endpoint() {
    let server = TestServer::new(None, None);
    let url = format!("http://{}/_irondrop/health", server.addr);
    let response = HttpClient::get(&url);

    assert_eq!(response.status_code, 200);
    assert!(response
        .headers
        .get("content-type")
        .unwrap()
        .contains("application/json"));
    assert!(response.body.contains("\"status\": \"healthy\""));
    assert!(response.body.contains("\"service\": \"irondrop\""));
    assert!(response.body.contains("rate_limiting"));
    assert!(response.body.contains("enhanced_security"));
}

#[test]
fn test_mime_type_detection() {
    let server = TestServer::new(None, None);

    // Test text file
    let url = format!("http://{}/test.txt", server.addr);
    let response = HttpClient::get(&url);
    assert_eq!(response.status_code, 200);
    assert!(response
        .headers
        .get("content-type")
        .unwrap()
        .contains("text/plain"));

    // Test PDF file
    let url = format!("http://{}/test.pdf", server.addr);
    let response = HttpClient::get(&url);
    assert_eq!(response.status_code, 200);
    assert!(response
        .headers
        .get("content-type")
        .unwrap()
        .contains("application/pdf"));
}

#[test]
fn test_enhanced_security_headers() {
    let server = TestServer::new(None, None);
    let url = format!("http://{}/test.txt", server.addr);
    let response = HttpClient::get(&url);

    assert_eq!(response.status_code, 200);
    assert!(response.headers.contains_key("server"));
    assert!(response
        .headers
        .get("server")
        .unwrap()
        .contains("irondrop/2.5.0"));
    assert!(response.headers.contains_key("cache-control"));
    assert!(response.headers.contains_key("accept-ranges"));
}

#[test]
fn test_authentication_flow() {
    let server = TestServer::new(Some("user".to_string()), Some("pass".to_string()));

    // Test without credentials
    let url = format!("http://{}/", server.addr);
    let response = HttpClient::get(&url);
    assert_eq!(response.status_code, 401);
    assert!(response.headers.contains_key("www-authenticate"));
    assert!(response.body.contains("401"));

    // Test with correct credentials
    let response = HttpClient::get_with_auth(&url, "user", "pass");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("Name"));

    // Test with wrong credentials
    let response = HttpClient::get_with_auth(&url, "wrong", "credentials");
    assert_eq!(response.status_code, 401);
}

#[test]
fn test_rate_limiting_simulation() {
    let server = TestServer::new(None, None);
    let url = format!("http://{}/test.txt", server.addr);

    // Make several requests quickly to test rate limiting
    // Note: In real scenarios, rate limiting would kick in after many requests
    // This test verifies the server handles multiple concurrent requests gracefully
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let url = url.clone();
            thread::spawn(move || HttpClient::get(&url))
        })
        .collect();

    let mut success_count = 0;
    for handle in handles {
        let response = handle.join().unwrap();
        if response.status_code == 200 {
            success_count += 1;
        }
    }

    // All requests should succeed in this test scenario
    assert!(success_count >= 8); // Allow for some variance
}

#[test]
fn test_nested_directory_access() {
    let server = TestServer::new(None, None);

    // Test subdirectory listing
    let url = format!("http://{}/subdir/", server.addr);
    let response = HttpClient::get(&url);
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("nested.txt"));
    assert!(response.body.contains("/subdir/"));

    // Test nested file access
    let url = format!("http://{}/subdir/nested.txt", server.addr);
    let response = HttpClient::get(&url);
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("Nested file content"));
}

#[test]
fn test_path_traversal_security() {
    let server = TestServer::new(None, None);

    // Attempt path traversal attack
    let url = format!("http://{}/../../etc/passwd", server.addr);
    let response = HttpClient::get(&url);
    assert_eq!(response.status_code, 403); // Should be forbidden

    // Another traversal attempt
    let url = format!("http://{}/../../../", server.addr);
    let response = HttpClient::get(&url);
    assert_eq!(response.status_code, 403);
}

#[test]
fn test_malformed_requests() {
    use std::io::Write;

    let server = TestServer::new(None, None);

    // Send malformed HTTP request
    let mut stream = TcpStream::connect(server.addr).unwrap();
    stream.write_all(b"INVALID REQUEST\r\n\r\n").unwrap();

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();

    assert!(status_line.contains("400") || status_line.contains("Bad Request"));
}

#[test]
fn test_large_file_handling() {
    let server = TestServer::new(None, None);
    let url = format!("http://{}/large.txt", server.addr);
    let response = HttpClient::get(&url);

    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("Line 0 of a large file"));
    assert!(response.body.contains("Line 999 of a large file"));

    // Check proper content length (allow for small variations due to line endings)
    if let Some(content_length) = response.headers.get("content-length") {
        let length: usize = content_length.parse().unwrap();
        assert!(length > 0);
        let body_len = response.body.len();
        assert!(
            (length as i64 - body_len as i64).abs() <= 2,
            "Content length {} doesn't match body length {} (diff: {})",
            length,
            body_len,
            (length as i64 - body_len as i64).abs()
        );
    }
}

#[test]
fn test_http_compliance() {
    let server = TestServer::new(None, None);
    let url = format!("http://{}/test.txt", server.addr);
    let response = HttpClient::get(&url);

    assert_eq!(response.status_code, 200);

    // Check for required HTTP headers
    assert!(response.headers.contains_key("content-type"));
    assert!(response.headers.contains_key("content-length"));
    assert!(response.headers.contains_key("server"));

    // Verify server identification
    assert_eq!(response.headers.get("server").unwrap(), "irondrop/2.5.0");
}

#[test]
fn test_favicon_ico_serving() {
    let server = TestServer::new(None, None);
    let url = format!("http://{}/favicon.ico", server.addr);
    let response = HttpClient::get(&url);

    assert_eq!(response.status_code, 200);
    assert_eq!(
        response.headers.get("content-type").unwrap(),
        "image/x-icon"
    );
    assert!(response.headers.contains_key("content-length"));

    // Verify cache headers
    assert_eq!(
        response.headers.get("cache-control").unwrap(),
        "public, max-age=86400"
    );

    // Verify binary content exists (favicon should not be empty)
    assert!(!response.body.is_empty());

    // Basic validation - ICO files should have binary content
    assert!(response.body_bytes.len() > 4, "Favicon should have content");

    // For binary data, the body should be a placeholder
    assert!(
        response.body.starts_with("BINARY_DATA_"),
        "Should be binary data"
    );
}

#[test]
fn test_favicon_png_16x16() {
    let server = TestServer::new(None, None);
    let url = format!("http://{}/favicon-16x16.png", server.addr);
    let response = HttpClient::get(&url);

    assert_eq!(response.status_code, 200);
    assert_eq!(response.headers.get("content-type").unwrap(), "image/png");
    assert!(response.headers.contains_key("content-length"));

    // Verify cache headers for PNG
    assert_eq!(
        response.headers.get("cache-control").unwrap(),
        "public, max-age=86400"
    );

    // Verify binary content exists
    assert!(!response.body.is_empty());

    // Basic PNG validation - PNG files start with specific signature
    assert!(response.body_bytes.len() > 8, "PNG should have content");
    // PNG signature: 137 80 78 71 13 10 26 10
    assert_eq!(
        &response.body_bytes[0..4],
        &[137, 80, 78, 71],
        "Should be valid PNG signature"
    );
}

#[test]
fn test_favicon_png_32x32() {
    let server = TestServer::new(None, None);
    let url = format!("http://{}/favicon-32x32.png", server.addr);
    let response = HttpClient::get(&url);

    assert_eq!(response.status_code, 200);
    assert_eq!(response.headers.get("content-type").unwrap(), "image/png");
    assert!(response.headers.contains_key("content-length"));

    // Verify binary content exists
    assert!(!response.body.is_empty());

    // Basic PNG validation
    assert!(response.body_bytes.len() > 8, "PNG should have content");
    assert_eq!(
        &response.body_bytes[0..4],
        &[137, 80, 78, 71],
        "Should be valid PNG signature"
    );
}

#[test]
fn test_favicon_not_found() {
    let server = TestServer::new(None, None);
    let url = format!("http://{}/favicon-invalid.ico", server.addr);
    let response = HttpClient::get(&url);

    // Should return 404 for non-existent favicon variants
    assert_eq!(response.status_code, 404);
}

#[test]
fn test_html_includes_favicon_links() {
    let server = TestServer::new(None, None);
    let url = format!("http://{}/", server.addr);
    let response = HttpClient::get(&url);

    assert_eq!(response.status_code, 200);
    assert!(response
        .headers
        .get("content-type")
        .unwrap()
        .contains("text/html"));

    // Check that HTML includes favicon links
    assert!(
        response.body.contains("favicon.ico"),
        "HTML should link to favicon.ico"
    );
    assert!(
        response.body.contains("favicon-32x32.png"),
        "HTML should link to 32x32 PNG"
    );
    assert!(
        response.body.contains("favicon-16x16.png"),
        "HTML should link to 16x16 PNG"
    );

    // Check proper link tag structure
    assert!(response
        .body
        .contains(r#"<link rel="icon" type="image/x-icon" href="/favicon.ico">"#));
    assert!(response
        .body
        .contains(r#"<link rel="icon" type="image/png" sizes="32x32""#));
    assert!(response
        .body
        .contains(r#"<link rel="icon" type="image/png" sizes="16x16""#));
}

#[test]
fn test_error_page_includes_favicon_links() {
    let server = TestServer::new(None, None);
    let url = format!("http://{}/nonexistent-file.txt", server.addr);
    let response = HttpClient::get(&url);

    assert_eq!(response.status_code, 404);
    assert!(response
        .headers
        .get("content-type")
        .unwrap()
        .contains("text/html"));

    // Check that error pages also include favicon links
    assert!(
        response.body.contains("favicon.ico"),
        "Error page should link to favicon.ico"
    );
    assert!(
        response.body.contains("favicon-32x32.png"),
        "Error page should link to 32x32 PNG"
    );
    assert!(
        response.body.contains("favicon-16x16.png"),
        "Error page should link to 16x16 PNG"
    );
}
