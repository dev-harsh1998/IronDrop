//! Tests for /monitor endpoint and bytes_served accounting.

use irondrop::cli::Cli;
use irondrop::server::run_server;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use std::fs::File;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use tempfile::{tempdir, TempDir};

struct TestServer {
    addr: SocketAddr,
    shutdown_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<()>>,
    _temp_dir: TempDir,
}

fn setup_test_server() -> TestServer {
    let dir = tempdir().unwrap();
    // Create a test file with known content length
    let file_path = dir.path().join("monitor_test.txt");
    let mut f = File::create(&file_path).unwrap();
    // Exact content (keep simple ASCII)
    write!(f, "This is a monitor test file.").unwrap(); // 29 bytes

    let cli = Cli {
        directory: dir.path().to_path_buf(),
        listen: "127.0.0.1".to_string(),
        port: 0,
        allowed_extensions: "*.txt".to_string(),
        threads: 4,
        chunk_size: 1024,
        verbose: false,
        detailed_logging: false,
        username: None,
        password: None,
        enable_upload: false,
        max_upload_size: 10240,
        upload_dir: None,
    };

    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let (addr_tx, addr_rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        if let Err(e) = run_server(cli, Some(shutdown_rx), Some(addr_tx)) {
            eprintln!("Server thread failed: {e}");
        }
    });

    let addr = addr_rx.recv().unwrap();

    TestServer {
        addr,
        shutdown_tx,
        handle: Some(handle),
        _temp_dir: dir,
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = self.shutdown_tx.send(());
            let _ = handle.join();
        }
    }
}

fn extract_bytes_served(json: &str) -> u64 {
    // Look for "bytes_served":<number>
    if let Some(idx) = json.find("\"bytes_served\":") {
        let slice = &json[idx + 15..];
        let mut digits = String::new();
        for ch in slice.chars() {
            if ch.is_ascii_digit() {
                digits.push(ch);
            } else {
                break;
            }
        }
        if let Ok(v) = digits.parse::<u64>() {
            return v;
        }
    }
    panic!("bytes_served not found in json: {json}");
}

#[test]
fn test_monitor_json_and_bytes_served_accounting() {
    let server = setup_test_server();
    let client = Client::new();

    // First monitor fetch (baseline)
    let res1 = client
        .get(format!("http://{}/monitor?json=1", server.addr))
        .send()
        .unwrap();
    assert_eq!(res1.status(), StatusCode::OK);
    let body1 = res1.text().unwrap();
    let bytes1 = extract_bytes_served(&body1);
    let monitor1_len = body1.as_bytes().len() as u64;
    // Initial bytes_served may be 0 if first response hasn't been recorded yet when fetched.
    assert!(
        bytes1 <= monitor1_len * 2,
        "unexpectedly large initial bytes_served: {bytes1}"
    );

    // Download file
    let res_file = client
        .get(format!("http://{}/monitor_test.txt", server.addr))
        .send()
        .unwrap();
    assert_eq!(res_file.status(), StatusCode::OK);
    let file_body = res_file.text().unwrap();
    let file_len = file_body.as_bytes().len() as u64; // Should be 29

    // Second monitor fetch
    let res2 = client
        .get(format!("http://{}/monitor?json=1", server.addr))
        .send()
        .unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
    let body2 = res2.text().unwrap();
    let bytes2 = extract_bytes_served(&body2);
    let monitor2_len = body2.as_bytes().len() as u64;

    // File bytes should appear in delta between monitor fetches minus monitor response body itself.
    let delta = bytes2.saturating_sub(bytes1);
    // Allow small variance if headers counted; require at least file_len and not wildly larger.
    assert!(
        delta >= file_len,
        "bytes_served delta {delta} did not include file bytes {file_len}"
    );
    assert!(
        delta < file_len + 4096,
        "bytes_served delta {delta} unreasonably larger than file {file_len}"
    );

    // Third monitor fetch to ensure monotonic increase
    let res3 = client
        .get(format!("http://{}/monitor?json=1", server.addr))
        .send()
        .unwrap();
    assert_eq!(res3.status(), StatusCode::OK);
    let body3 = res3.text().unwrap();
    let bytes3 = extract_bytes_served(&body3);
    assert!(bytes3 >= bytes2, "bytes_served should be non-decreasing");
}

#[test]
fn test_monitor_html_served() {
    let server = setup_test_server();
    let client = Client::new();

    let res = client
        .get(format!("http://{}/monitor", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert!(body.contains("<html"));
    assert!(body.to_lowercase().contains("monitor"));
}
