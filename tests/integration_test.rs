//! Integration tests for the file server.

use irondrop::cli::Cli;
use irondrop::server::run_server;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use std::fs::File;
use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use tempfile::{tempdir, TempDir};

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
        listen: "127.0.0.1".to_string(),
        port: 0, // Port 0 lets the OS pick a free port.
        allowed_extensions: "*.txt".to_string(),
        threads: 4,
        chunk_size: 1024,
        verbose: false,
        detailed_logging: false,
        username,
        password,
    };

    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let (addr_tx, addr_rx) = mpsc::channel();

    let server_handle = thread::spawn(move || {
        // The server will run here.
        if let Err(e) = run_server(cli, Some(shutdown_rx), Some(addr_tx)) {
            // Use eprintln so the error shows up in test output.
            eprintln!("Server thread failed: {}", e);
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
