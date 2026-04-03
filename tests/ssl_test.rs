// SPDX-License-Identifier: MIT
//! SSL/TLS integration tests for the file server.

use irondrop::cli::Cli;
use irondrop::server::run_server;
use rcgen::generate_simple_self_signed;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use std::fs::{self, File};
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use tempfile::{TempDir, tempdir};

/// A helper struct to manage a running test server with SSL support.
struct TestServer {
    addr: SocketAddr,
    shutdown_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<()>>,
    _temp_dir: TempDir,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.shutdown_tx.send(()).ok();
            handle.join().unwrap();
        }
    }
}

/// Ensure the rustls crypto provider is installed (idempotent).
fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// Generate self-signed TLS certificates for testing.
fn generate_test_certs(dir: &std::path::Path) -> (PathBuf, PathBuf) {
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let cert = generate_simple_self_signed(subject_alt_names).unwrap();

    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");

    fs::write(&cert_path, cert.cert.pem()).unwrap();
    fs::write(&key_path, cert.signing_key.serialize_pem()).unwrap();

    (cert_path, key_path)
}

/// Build a reqwest client that accepts self-signed certificates.
fn https_client() -> Client {
    Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
}

/// Start an HTTPS test server with optional authentication.
fn setup_ssl_server(username: Option<String>, password: Option<String>) -> TestServer {
    install_crypto_provider();
    let dir = tempdir().unwrap();

    // Create a test file for downloads.
    let file_path = dir.path().join("test.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "hello from ssl test file").unwrap();

    // Create a subdirectory with a file for directory listing tests.
    let sub_dir = dir.path().join("subdir");
    fs::create_dir(&sub_dir).unwrap();
    let sub_file = sub_dir.join("nested.txt");
    let mut f = File::create(&sub_file).unwrap();
    writeln!(f, "nested content").unwrap();

    let (cert_path, key_path) = generate_test_certs(dir.path());

    let cli = Cli {
        directory: dir.path().to_path_buf(),
        listen: Some("127.0.0.1".to_string()),
        port: Some(0),
        allowed_extensions: Some("*".to_string()),
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
        ssl_cert: Some(cert_path),
        ssl_key: Some(key_path),
    };

    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let (addr_tx, addr_rx) = mpsc::channel();

    let server_handle = thread::spawn(move || {
        if let Err(e) = run_server(cli, Some(shutdown_rx), Some(addr_tx)) {
            eprintln!("SSL server thread failed: {e}");
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

// ---------------------------------------------------------------------------
// Test 1: Basic HTTPS request returns 200 OK
// ---------------------------------------------------------------------------
#[test]
fn test_https_basic_request() {
    let server = setup_ssl_server(None, None);
    let client = https_client();

    let res = client
        .get(format!("https://{}/", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Test 2: File download over HTTPS returns correct content
// ---------------------------------------------------------------------------
#[test]
fn test_https_file_download() {
    let server = setup_ssl_server(None, None);
    let client = https_client();

    let res = client
        .get(format!("https://{}/test.txt", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().unwrap(), "hello from ssl test file\n");
}

// ---------------------------------------------------------------------------
// Test 3: Directory listing works over HTTPS
// ---------------------------------------------------------------------------
#[test]
fn test_https_directory_listing() {
    let server = setup_ssl_server(None, None);
    let client = https_client();

    let res = client
        .get(format!("https://{}/", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert!(body.contains("test.txt"), "listing should contain test.txt");
    assert!(body.contains("subdir"), "listing should contain subdir");
}

// ---------------------------------------------------------------------------
// Test 4: HTTPS with basic authentication
// ---------------------------------------------------------------------------
#[test]
fn test_https_with_authentication() {
    let server = setup_ssl_server(Some("admin".to_string()), Some("secret".to_string()));
    let client = https_client();

    // Without credentials -> 401 Unauthorized
    let res = client
        .get(format!("https://{}/", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert!(res.headers().contains_key("www-authenticate"));

    // With correct credentials -> 200 OK
    let res = client
        .get(format!("https://{}/", server.addr))
        .basic_auth("admin", Some("secret"))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert!(body.contains("test.txt"));
}

// ---------------------------------------------------------------------------
// Test 5: Health endpoint works over HTTPS
// ---------------------------------------------------------------------------
#[test]
fn test_https_health_endpoint() {
    let server = setup_ssl_server(None, None);
    let client = https_client();

    let res = client
        .get(format!("https://{}/_irondrop/health", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Test 5b: Monitor endpoint works over HTTPS (HTML and JSON)
// ---------------------------------------------------------------------------
#[test]
fn test_https_monitor_endpoint() {
    let server = setup_ssl_server(None, None);
    let client = https_client();

    // HTML monitor page
    let res = client
        .get(format!("https://{}/_irondrop/monitor", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert!(
        body.contains("html") || body.contains("HTML"),
        "monitor should return an HTML page"
    );

    // JSON monitor endpoint
    let res = client
        .get(format!("https://{}/_irondrop/monitor?json=1", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert!(
        body.contains("total_requests") || body.contains("uptime"),
        "JSON monitor should contain stats fields"
    );

    // Legacy /monitor path
    let res = client
        .get(format!("https://{}/monitor", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Test 6: Server fails to start with non-existent cert file
// ---------------------------------------------------------------------------
#[test]
fn test_ssl_missing_cert_file() {
    install_crypto_provider();
    let dir = tempdir().unwrap();

    // Create a valid key but point cert to a non-existent file.
    let (_, key_path) = generate_test_certs(dir.path());
    let bogus_cert = dir.path().join("nonexistent_cert.pem");

    let cli = Cli {
        directory: dir.path().to_path_buf(),
        listen: Some("127.0.0.1".to_string()),
        port: Some(0),
        allowed_extensions: Some("*".to_string()),
        threads: Some(4),
        chunk_size: Some(1024),
        verbose: Some(false),
        detailed_logging: Some(false),
        username: None,
        password: None,
        enable_upload: Some(false),
        max_upload_size: Some(10240),
        config_file: None,
        log_dir: None,
        ssl_cert: Some(bogus_cert),
        ssl_key: Some(key_path),
    };

    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let result = run_server(cli, Some(shutdown_rx), None);
    assert!(
        result.is_err(),
        "run_server should fail with missing cert file"
    );
    drop(shutdown_tx);
}

// ---------------------------------------------------------------------------
// Test 7: Server fails to start with non-existent key file
// ---------------------------------------------------------------------------
#[test]
fn test_ssl_missing_key_file() {
    install_crypto_provider();
    let dir = tempdir().unwrap();

    // Create a valid cert but point key to a non-existent file.
    let (cert_path, _) = generate_test_certs(dir.path());
    let bogus_key = dir.path().join("nonexistent_key.pem");

    let cli = Cli {
        directory: dir.path().to_path_buf(),
        listen: Some("127.0.0.1".to_string()),
        port: Some(0),
        allowed_extensions: Some("*".to_string()),
        threads: Some(4),
        chunk_size: Some(1024),
        verbose: Some(false),
        detailed_logging: Some(false),
        username: None,
        password: None,
        enable_upload: Some(false),
        max_upload_size: Some(10240),
        config_file: None,
        log_dir: None,
        ssl_cert: Some(cert_path),
        ssl_key: Some(bogus_key),
    };

    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let result = run_server(cli, Some(shutdown_rx), None);
    assert!(
        result.is_err(),
        "run_server should fail with missing key file"
    );
    drop(shutdown_tx);
}

// ---------------------------------------------------------------------------
// Test 8: Validation error when only cert is provided (no key)
// ---------------------------------------------------------------------------
#[test]
fn test_ssl_cert_without_key() {
    let dir = tempdir().unwrap();
    let (cert_path, _) = generate_test_certs(dir.path());

    let cli = Cli {
        directory: dir.path().to_path_buf(),
        listen: Some("127.0.0.1".to_string()),
        port: Some(0),
        allowed_extensions: Some("*".to_string()),
        threads: Some(4),
        chunk_size: Some(1024),
        verbose: Some(false),
        detailed_logging: Some(false),
        username: None,
        password: None,
        enable_upload: Some(false),
        max_upload_size: Some(10240),
        config_file: None,
        log_dir: None,
        ssl_cert: Some(cert_path),
        ssl_key: None,
    };

    let result = cli.validate();
    assert!(
        result.is_err(),
        "validate() should fail when ssl_cert is set without ssl_key"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Both --ssl-cert and --ssl-key must be provided together"),
        "error message should mention both flags are required, got: {err_msg}"
    );
}

// ---------------------------------------------------------------------------
// Test 9: Validation error when only key is provided (no cert)
// ---------------------------------------------------------------------------
#[test]
fn test_ssl_key_without_cert() {
    let dir = tempdir().unwrap();
    let (_, key_path) = generate_test_certs(dir.path());

    let cli = Cli {
        directory: dir.path().to_path_buf(),
        listen: Some("127.0.0.1".to_string()),
        port: Some(0),
        allowed_extensions: Some("*".to_string()),
        threads: Some(4),
        chunk_size: Some(1024),
        verbose: Some(false),
        detailed_logging: Some(false),
        username: None,
        password: None,
        enable_upload: Some(false),
        max_upload_size: Some(10240),
        config_file: None,
        log_dir: None,
        ssl_cert: None,
        ssl_key: Some(key_path),
    };

    let result = cli.validate();
    assert!(
        result.is_err(),
        "validate() should fail when ssl_key is set without ssl_cert"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Both --ssl-cert and --ssl-key must be provided together"),
        "error message should mention both flags are required, got: {err_msg}"
    );
}

// ---------------------------------------------------------------------------
// Test 10: Plain HTTP still works when no SSL config is provided
// ---------------------------------------------------------------------------
#[test]
fn test_http_still_works_without_ssl() {
    let dir = tempdir().unwrap();

    let file_path = dir.path().join("hello.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "plain http content").unwrap();

    let cli = Cli {
        directory: dir.path().to_path_buf(),
        listen: Some("127.0.0.1".to_string()),
        port: Some(0),
        allowed_extensions: Some("*".to_string()),
        threads: Some(4),
        chunk_size: Some(1024),
        verbose: Some(false),
        detailed_logging: Some(false),
        username: None,
        password: None,
        enable_upload: Some(false),
        max_upload_size: Some(10240),
        config_file: None,
        log_dir: None,
        ssl_cert: None,
        ssl_key: None,
    };

    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let (addr_tx, addr_rx) = mpsc::channel();

    let server_handle = thread::spawn(move || {
        if let Err(e) = run_server(cli, Some(shutdown_rx), Some(addr_tx)) {
            eprintln!("HTTP server thread failed: {e}");
        }
    });

    let server_addr = addr_rx.recv().unwrap();

    let server = TestServer {
        addr: server_addr,
        shutdown_tx,
        handle: Some(server_handle),
        _temp_dir: dir,
    };

    let client = Client::new();
    let res = client
        .get(format!("http://{}/hello.txt", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().unwrap(), "plain http content\n");
}
