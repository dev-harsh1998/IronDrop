// SPDX-License-Identifier: MIT

use irondrop::cli::Cli;
use irondrop::server::run_server;
use reqwest::Method;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use std::fs::File;
use std::io::{BufRead, Write};
use std::net::{SocketAddr, TcpStream};
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
        disable_rate_limit: Some(false),
        config_file: None,
        log_dir: None,
        ssl_cert: None,
        ssl_key: None,
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

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.shutdown_tx.send(()).ok();
            handle.join().unwrap();
        }
    }
}

#[test]
fn test_copy_destination_parent_missing_returns_conflict() {
    let server = setup_test_server_with_tree(|root| {
        let mut source = File::create(root.join("src.txt")).unwrap();
        write!(source, "src").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"COPY").unwrap(),
            format!("http://{}/src.txt", server.addr),
        )
        .header(
            "Destination",
            format!("http://{}/missing/path/dst.txt", server.addr),
        )
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[test]
fn test_move_default_overwrite_is_true() {
    let server = setup_test_server_with_tree(|root| {
        let mut source = File::create(root.join("src.txt")).unwrap();
        write!(source, "source").unwrap();
        let mut destination = File::create(root.join("dst.txt")).unwrap();
        write!(destination, "old").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"MOVE").unwrap(),
            format!("http://{}/src.txt", server.addr),
        )
        .header("Destination", format!("http://{}/dst.txt", server.addr))
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[test]
fn test_copy_destination_host_mismatch_rejected() {
    let server = setup_test_server_with_tree(|root| {
        let mut source = File::create(root.join("src.txt")).unwrap();
        write!(source, "src").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"COPY").unwrap(),
            format!("http://{}/src.txt", server.addr),
        )
        .header("Destination", "http://example.com/dst.txt")
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_put_path_traversal_blocked() {
    let server = setup_test_server_with_tree(|_| {});
    let mut stream = TcpStream::connect(server.addr).unwrap();
    let request = concat!(
        "PUT /../../../../etc/passwd HTTP/1.1\r\n",
        "Host: localhost\r\n",
        "Content-Length: 4\r\n",
        "\r\n",
        "evil"
    );
    stream.write_all(request.as_bytes()).unwrap();
    stream.flush().unwrap();

    let mut reader = std::io::BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();
    assert!(status_line.starts_with("HTTP/1.1 403 Forbidden"));
}

#[cfg(unix)]
#[test]
fn test_put_through_symlink_outside_root_is_forbidden() {
    use std::os::unix::fs::symlink;

    let outside = tempdir().unwrap();
    let server = setup_test_server_with_tree(|root| {
        symlink(outside.path(), root.join("escape")).unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::PUT,
            format!("http://{}/escape/outside.txt", server.addr),
        )
        .body("blocked".to_string())
        .send()
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[test]
fn test_options_advertises_class2_locking() {
    let server = setup_test_server_with_tree(|_| {});
    let client = Client::new();

    let response = client
        .request(Method::OPTIONS, format!("http://{}/", server.addr))
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let dav = response.headers().get("dav").unwrap().to_str().unwrap();
    assert!(dav.contains('2'));
    let allow = response.headers().get("allow").unwrap().to_str().unwrap();
    assert!(allow.contains("LOCK"));
    assert!(allow.contains("UNLOCK"));
}

#[test]
fn test_lock_unlock_roundtrip_with_put_if_header() {
    let server = setup_test_server_with_tree(|_| {});
    let client = Client::new();

    let lock_response = client
        .request(
            Method::from_bytes(b"LOCK").unwrap(),
            format!("http://{}/locked.txt", server.addr),
        )
        .header("Timeout", "Second-300")
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
        lock_response.status() == StatusCode::CREATED || lock_response.status() == StatusCode::OK
    );
    let lock_token_header = lock_response
        .headers()
        .get("lock-token")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let lock_token = lock_token_header
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .to_string();

    let blocked_put = client
        .request(Method::PUT, format!("http://{}/locked.txt", server.addr))
        .body("no-token".to_string())
        .send()
        .unwrap();
    assert_eq!(blocked_put.status().as_u16(), 423);

    let allowed_put = client
        .request(Method::PUT, format!("http://{}/locked.txt", server.addr))
        .header("If", format!("(<{lock_token}>)"))
        .body("with-token".to_string())
        .send()
        .unwrap();
    assert!(
        allowed_put.status() == StatusCode::CREATED
            || allowed_put.status() == StatusCode::NO_CONTENT
    );

    let unlock_response = client
        .request(
            Method::from_bytes(b"UNLOCK").unwrap(),
            format!("http://{}/locked.txt", server.addr),
        )
        .header("Lock-Token", lock_token_header)
        .send()
        .unwrap();
    assert_eq!(unlock_response.status(), StatusCode::NO_CONTENT);
}
