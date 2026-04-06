// SPDX-License-Identifier: MIT

use irondrop::cli::Cli;
use irondrop::server::run_server;
use reqwest::Method;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use std::fs::{File, create_dir_all};
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

fn setup_test_server_with_tree_and_webdav<F>(populate: F, enable_webdav: bool) -> TestServer
where
    F: FnOnce(&std::path::Path),
{
    let dir = tempdir().unwrap();
    populate(dir.path());

    let file_path = dir.path().join("test.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "hello from test file").unwrap();

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
        enable_webdav: Some(enable_webdav),
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

fn setup_test_server_with_tree<F>(populate: F) -> TestServer
where
    F: FnOnce(&std::path::Path),
{
    setup_test_server_with_tree_and_webdav(populate, true)
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
fn test_options_advertises_webdav_v1_capability() {
    let server = setup_test_server_with_tree(|_| {});
    let client = Client::new();

    let response = client
        .request(Method::OPTIONS, format!("http://{}/", server.addr))
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let dav = response.headers().get("dav").unwrap().to_str().unwrap();
    assert!(dav.contains('1'));

    let allow = response.headers().get("allow").unwrap().to_str().unwrap();
    assert!(allow.contains("PROPFIND"));
    assert!(allow.contains("MKCOL"));
    assert!(allow.contains("PUT"));
    assert!(allow.contains("DELETE"));
    assert!(allow.contains("COPY"));
    assert!(allow.contains("MOVE"));
}

#[test]
fn test_webdav_methods_disabled_without_flag() {
    let server = setup_test_server_with_tree_and_webdav(|_| {}, false);
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/", server.addr),
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .body(r#"<?xml version="1.0" encoding="utf-8"?><D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#.to_string())
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[test]
fn test_propfind_depth_zero_on_file_returns_multistatus() {
    let server = setup_test_server_with_tree(|_| {});
    let client = Client::new();
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#;

    let response = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/test.txt", server.addr),
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .body(body.to_string())
        .send()
        .unwrap();

    assert_eq!(response.status().as_u16(), 207);
    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("application/xml"));

    let text = response.text().unwrap();
    assert!(text.contains("<D:multistatus"));
    assert!(text.contains("<D:href>/test.txt</D:href>"));
    assert!(text.contains("<D:getcontentlength>"));
    assert!(text.contains("<D:getlastmodified>"));
}

#[test]
fn test_propfind_depth_one_on_collection_includes_children() {
    let server = setup_test_server_with_tree(|root| {
        create_dir_all(root.join("dav").join("nested")).unwrap();
        let mut f = File::create(root.join("dav").join("child.txt")).unwrap();
        writeln!(f, "child").unwrap();
    });
    let client = Client::new();
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#;

    let response = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/dav/", server.addr),
        )
        .header("Depth", "1")
        .header("Content-Type", "application/xml")
        .body(body.to_string())
        .send()
        .unwrap();

    assert_eq!(response.status().as_u16(), 207);
    let text = response.text().unwrap();
    assert!(text.contains("<D:href>/dav/</D:href>"));
    assert!(text.contains("<D:href>/dav/child.txt</D:href>"));
    assert!(text.contains("<D:href>/dav/nested/</D:href>"));
}

#[test]
fn test_propfind_infinite_depth_rejected_with_finite_depth_precondition() {
    let server = setup_test_server_with_tree(|_| {});
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/", server.addr),
        )
        .header("Depth", "infinity")
        .header("Content-Type", "application/xml")
        .body(String::new())
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response.text().unwrap();
    assert!(body.contains("propfind-finite-depth"));
}
