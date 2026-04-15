// SPDX-License-Identifier: MIT

use irondrop::cli::Cli;
use irondrop::server::run_server;
use reqwest::Method;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use std::fs::{self, File, create_dir_all};
use std::io::Write;
use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use tempfile::{TempDir, tempdir};

struct TestServer {
    addr: SocketAddr,
    shutdown_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<()>>,
    root: std::path::PathBuf,
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
    let root = dir.path().to_path_buf();

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
        root,
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
fn test_mkcol_creates_collection() {
    let server = setup_test_server_with_tree(|_| {});
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"MKCOL").unwrap(),
            format!("http://{}/newdir/", server.addr),
        )
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    assert!(server.root.join("newdir").is_dir());
}

#[test]
fn test_mkcol_parent_missing_conflict() {
    let server = setup_test_server_with_tree(|_| {});
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"MKCOL").unwrap(),
            format!("http://{}/missing/child/", server.addr),
        )
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[test]
fn test_put_creates_and_updates_file() {
    let server = setup_test_server_with_tree(|_| {});
    let client = Client::new();

    let create = client
        .request(Method::PUT, format!("http://{}/notes.txt", server.addr))
        .body("hello".to_string())
        .send()
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    assert_eq!(
        fs::read_to_string(server.root.join("notes.txt")).unwrap(),
        "hello"
    );

    let replace = client
        .request(Method::PUT, format!("http://{}/notes.txt", server.addr))
        .body("updated".to_string())
        .send()
        .unwrap();
    assert_eq!(replace.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        fs::read_to_string(server.root.join("notes.txt")).unwrap(),
        "updated"
    );
}

#[test]
fn test_delete_removes_file_and_collection() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("to-delete.txt")).unwrap();
        writeln!(file, "erase").unwrap();
        create_dir_all(root.join("to-delete-dir").join("nested")).unwrap();
        let mut nested =
            File::create(root.join("to-delete-dir").join("nested").join("file.txt")).unwrap();
        writeln!(nested, "nested").unwrap();
    });
    let client = Client::new();

    let delete_file = client
        .request(
            Method::DELETE,
            format!("http://{}/to-delete.txt", server.addr),
        )
        .send()
        .unwrap();
    assert_eq!(delete_file.status(), StatusCode::NO_CONTENT);
    assert!(!server.root.join("to-delete.txt").exists());

    let delete_dir = client
        .request(
            Method::DELETE,
            format!("http://{}/to-delete-dir/", server.addr),
        )
        .send()
        .unwrap();
    assert_eq!(delete_dir.status(), StatusCode::NO_CONTENT);
    assert!(!server.root.join("to-delete-dir").exists());
}

#[test]
fn test_delete_missing_resource_not_found() {
    let server = setup_test_server_with_tree(|_| {});
    let client = Client::new();

    let response = client
        .request(Method::DELETE, format!("http://{}/nope.txt", server.addr))
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
