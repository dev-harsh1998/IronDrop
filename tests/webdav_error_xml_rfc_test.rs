// SPDX-License-Identifier: MIT

use irondrop::cli::Cli;
use irondrop::server::run_server;
use reqwest::Method;
use reqwest::blocking::Client;
use std::fs::File;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use tempfile::{TempDir, tempdir};

struct TestServer {
    addr: SocketAddr,
    shutdown_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<()>>,
    _temp_dir: TempDir,
}

fn setup_test_server_with_tree<F>(populate: F) -> TestServer
where
    F: FnOnce(&std::path::Path),
{
    let dir = tempdir().unwrap();
    populate(dir.path());

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
        enable_webdav: Some(true),
        config_file: None,
        log_dir: None,
        ssl_cert: None,
        ssl_key: None,
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
            self.shutdown_tx.send(()).ok();
            handle.join().unwrap();
        }
    }
}

#[test]
fn test_locked_write_returns_dav_error_body() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("sample.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let lock_response = client
        .request(
            Method::from_bytes(b"LOCK").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .body(
            r#"<?xml version="1.0" encoding="utf-8"?>
<D:lockinfo xmlns:D="DAV:">
  <D:lockscope><D:exclusive/></D:lockscope>
  <D:locktype><D:write/></D:locktype>
</D:lockinfo>"#,
        )
        .send()
        .unwrap();
    assert!(
        lock_response.status() == reqwest::StatusCode::CREATED
            || lock_response.status().as_u16() == 200
    );

    let put_response = client
        .request(Method::PUT, format!("http://{}/sample.txt", server.addr))
        .body("blocked".to_string())
        .send()
        .unwrap();
    assert_eq!(put_response.status().as_u16(), 423);

    let body = put_response.text().unwrap();
    assert!(body.contains("<D:error"));
    assert!(body.contains("lock-token-submitted"));
}
