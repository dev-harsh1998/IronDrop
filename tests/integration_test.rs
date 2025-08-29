// SPDX-License-Identifier: MIT
//! Integration tests for the file server.

use irondrop::cli::Cli;
use irondrop::server::run_server;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use std::fs::File;
use std::io::{BufRead, Read, Write};
use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use tempfile::{TempDir, tempdir};

/// A helper struct to manage a running test server.
struct TestServer {
    addr: SocketAddr,
    shutdown_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<()>>,
    // Keep the tempdir alive for the duration of the test.
    _temp_dir: TempDir,
}

/// Sets up and runs a server in a background thread for testing.
fn setup_test_server(username: Option<String>, password: Option<String>) -> TestServer {
    let dir = tempdir().unwrap();
    // Create a dummy file for testing downloads.
    let file_path = dir.path().join("test.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "hello from test file").unwrap();
    // Create a forbidden file type.
    let forbidden_file_path = dir.path().join("test.zip");
    File::create(&forbidden_file_path).unwrap();

    let cli = Cli {
        directory: dir.path().to_path_buf(),
        listen: Some("127.0.0.1".to_string()),
        port: Some(0), // Port 0 lets the OS pick a free port.
        allowed_extensions: Some("*.txt".to_string()),
        threads: Some(4),
        chunk_size: Some(1024),
        verbose: Some(false),
        detailed_logging: Some(false),
        username,
        password,
        enable_upload: Some(false),
        max_upload_size: Some(10240),
        config_file: None,
        log_dir: None,
    };

    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let (addr_tx, addr_rx) = mpsc::channel();

    let server_handle = thread::spawn(move || {
        // The server will run here.
        if let Err(e) = run_server(cli, Some(shutdown_rx), Some(addr_tx)) {
            // Use eprintln so the error shows up in test output.
            eprintln!("Server thread failed: {e}");
        }
    });

    // Block until the server has started and sent us its address.
    let server_addr = addr_rx.recv().unwrap();

    TestServer {
        addr: server_addr,
        shutdown_tx,
        handle: Some(server_handle),
        _temp_dir: dir,
    }
}

/// When the TestServer is dropped, shut down the server thread.
impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            // An empty send signals the server to shut down.
            self.shutdown_tx.send(()).ok();
            handle.join().unwrap();
        }
    }
}

#[test]
fn test_unauthenticated_access() {
    let server = setup_test_server(None, None);
    let client = Client::new();

    // 1. Test directory listing
    let res = client
        .get(format!("http://{}/", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert!(body.contains("test.txt"));

    // 2. Test allowed file download
    let res = client
        .get(format!("http://{}/test.txt", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().unwrap(), "hello from test file\n");
}

#[test]
fn test_authentication_required() {
    let server = setup_test_server(Some("user".to_string()), Some("pass".to_string()));
    let client = Client::new();

    // 1. Test without credentials -> 401 Unauthorized
    let res = client
        .get(format!("http://{}/", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert!(res.headers().contains_key("www-authenticate"));

    // 2. Test with wrong credentials -> 401 Unauthorized
    let res = client
        .get(format!("http://{}/", server.addr))
        .basic_auth("wrong", Some("user"))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_successful_authentication() {
    let server = setup_test_server(Some("user".to_string()), Some("pass".to_string()));
    let client = Client::new();

    // Test with correct credentials -> 200 OK
    let res = client
        .get(format!("http://{}/", server.addr))
        .basic_auth("user", Some("pass"))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert!(body.contains("test.txt"));

    // Test file download with correct credentials
    let res = client
        .get(format!("http://{}/test.txt", server.addr))
        .basic_auth("user", Some("pass"))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().unwrap(), "hello from test file\n");
}

#[test]
fn test_error_responses() {
    let server = setup_test_server(None, None);
    let client = Client::new();

    // 1. Test Not Found
    let res = client
        .get(format!("http://{}/nonexistent.txt", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 2. Test Forbidden file type
    let res = client
        .get(format!("http://{}/test.zip", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 3. Test Method Not Allowed
    let res = client
        .post(format!("http://{}/", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[test]
fn test_http_range_request_partial_content() {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let server = setup_test_server(None, None);
    // Request an existing small file from the test directory and ask for first 10 bytes
    let mut stream = TcpStream::connect(server.addr).expect("connect ok");
    write!(
        stream,
        "GET /test.txt HTTP/1.1\r\nHost: localhost\r\nRange: bytes=0-9\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    stream.flush().unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap();
    let text = String::from_utf8_lossy(&buf);
    // Some servers may return 200 OK with Accept-Ranges and full body; accept either 206 with Content-Range
    // or 200 with Accept-Ranges present.
    let ok_206 = text.contains("HTTP/1.1 206") && text.contains("Content-Range: bytes 0-9/");
    let ok_200 = text.contains("HTTP/1.1 200") && text.contains("Accept-Ranges: bytes");
    assert!(
        ok_206 || ok_200,
        "expected 206 with Content-Range or 200 with Accept-Ranges, got: {}",
        &*text
    );
}

#[test]
fn test_static_asset_headers_and_lengths() {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let server = setup_test_server(None, None);
    let mut stream = TcpStream::connect(server.addr).expect("connect ok");
    write!(
        stream,
        "GET /_irondrop/static/common/base.css HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    stream.flush().unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap();
    let text = String::from_utf8_lossy(&buf);
    assert!(text.contains("HTTP/1.1 200"));
    assert!(text.contains("Content-Type: text/css"));
    assert!(text.contains("Content-Length:"));
}

#[test]
fn test_path_traversal_prevention() {
    let server = setup_test_server(None, None);
    let client = Client::new();

    // First, test a *valid* use of '..' that stays within the directory.
    let res = client
        .get(format!("http://{}/subdir/../test.txt", server.addr))
        .send()
        .unwrap();
    // The server should correctly resolve this to `/test.txt` and serve it.
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().unwrap(), "hello from test file\n");

    // Now, attempt a true traversal attack using a raw TCP stream
    // to bypass any client-side URL normalization.
    let mut stream = std::net::TcpStream::connect(server.addr).unwrap();
    let request = "GET /../../../../../../etc/passwd HTTP/1.1\r\n\r\n";
    stream.write_all(request.as_bytes()).unwrap();

    let mut reader = std::io::BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();

    // This should be caught and result in a 403 Forbidden.
    assert!(status_line.starts_with("HTTP/1.1 403 Forbidden"));
}

#[test]
fn test_malformed_request() {
    let server = setup_test_server(None, None);

    // Send a request that is syntactically incorrect.
    let request = "GET /not-a-valid-http-version\r\n\r\n";
    let mut stream = std::net::TcpStream::connect(server.addr).unwrap();
    stream.write_all(request.as_bytes()).unwrap();

    let mut reader = std::io::BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();

    // The server should gracefully handle this with a 400 Bad Request.
    assert!(status_line.starts_with("HTTP/1.1 400 Bad Request"));
}

#[test]
fn test_concurrent_requests() {
    let server = setup_test_server(None, None);
    let client = Client::new();

    // Test multiple concurrent requests
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let addr = server.addr;
            let client = client.clone();
            thread::spawn(move || {
                client
                    .get(format!("http://{}/test.txt", addr))
                    .send()
                    .unwrap()
            })
        })
        .collect();

    // All requests should succeed
    for handle in handles {
        let response = handle.join().unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[test]
fn test_large_header_handling() {
    let server = setup_test_server(None, None);

    // Test with very large headers
    let large_value = "x".repeat(8192); // 8KB header value
    let mut stream = std::net::TcpStream::connect(server.addr).unwrap();
    let request = format!(
        "GET /test.txt HTTP/1.1\r\nHost: localhost\r\nX-Large-Header: {}\r\n\r\n",
        large_value
    );
    stream.write_all(request.as_bytes()).unwrap();

    let mut reader = std::io::BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();

    // Should either accept (200) or reject with appropriate error (413/400)
    assert!(
        status_line.starts_with("HTTP/1.1 200")
            || status_line.starts_with("HTTP/1.1 413")
            || status_line.starts_with("HTTP/1.1 400")
    );
}

#[test]
fn test_empty_request_handling() {
    let server = setup_test_server(None, None);

    // Send completely empty request
    let mut stream = std::net::TcpStream::connect(server.addr).unwrap();
    stream.write_all(b"").unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();

    let mut reader = std::io::BufReader::new(stream);
    let mut response = String::new();
    let _ = reader.read_to_string(&mut response);

    // Server should handle gracefully (connection may close or return error)
    // This test ensures no panic occurs
}

#[test]
fn test_invalid_range_requests() {
    let server = setup_test_server(None, None);

    // Test invalid range header formats
    let test_cases = vec![
        "Range: bytes=invalid",
        "Range: bytes=100-50",  // End before start
        "Range: bytes=999999-", // Beyond file size
        "Range: units=0-10",    // Invalid unit
    ];

    for range_header in test_cases {
        let mut stream = std::net::TcpStream::connect(server.addr).unwrap();
        let request = format!(
            "GET /test.txt HTTP/1.1\r\nHost: localhost\r\n{}\r\n\r\n",
            range_header
        );
        stream.write_all(request.as_bytes()).unwrap();

        let mut reader = std::io::BufReader::new(stream);
        let mut status_line = String::new();
        reader.read_line(&mut status_line).unwrap();

        // Should return 200 (ignore invalid range) or 416 (range not satisfiable)
        assert!(status_line.starts_with("HTTP/1.1 200") || status_line.starts_with("HTTP/1.1 416"));
    }
}

#[test]
fn test_connection_timeout_handling() {
    let server = setup_test_server(None, None);

    // Connect but don't send complete request
    let mut stream = std::net::TcpStream::connect(server.addr).unwrap();
    stream.write_all(b"GET /test.txt HTTP/1.1\r\n").unwrap();
    // Don't send the rest of the request

    // Wait a bit then try to read
    thread::sleep(std::time::Duration::from_millis(100));

    let mut reader = std::io::BufReader::new(stream);
    let mut response = String::new();
    let result = reader.read_to_string(&mut response);

    // Connection should eventually close or timeout
    // This test ensures the server doesn't hang indefinitely
}

#[test]
fn test_special_characters_in_paths() {
    let server = setup_test_server(None, None);
    let client = Client::new();

    // Test various special characters that should be handled safely
    let test_paths = vec![
        "/test%20file.txt", // URL encoded space
        "/test%2Efile.txt", // URL encoded dot
        "/test%3Ffile.txt", // URL encoded question mark
        "/test%23file.txt", // URL encoded hash
    ];

    for path in test_paths {
        let res = client
            .get(format!("http://{}{}", server.addr, path))
            .send()
            .unwrap();

        // Should return 404 (file not found) rather than crash
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}
