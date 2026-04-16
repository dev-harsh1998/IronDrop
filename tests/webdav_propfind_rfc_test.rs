// SPDX-License-Identifier: MIT

use irondrop::cli::Cli;
use irondrop::server::run_server;
use reqwest::Method;
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
fn test_propfind_propname_returns_only_property_names() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("sample.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:propname/>
</D:propfind>"#;

    let response = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .body(body.to_string())
        .send()
        .unwrap();

    assert_eq!(response.status().as_u16(), 207);
    let xml = response.text().unwrap();
    assert!(xml.contains("<D:getlastmodified/>"));
    assert!(!xml.contains("<D:getcontentlength>6</D:getcontentlength>"));
}

#[test]
fn test_propfind_named_unknown_property_returns_404_propstat() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("sample.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:getetag/>
    <D:totallymissing/>
  </D:prop>
</D:propfind>"#;

    let response = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .body(body.to_string())
        .send()
        .unwrap();

    assert_eq!(response.status().as_u16(), 207);
    let xml = response.text().unwrap();
    assert!(xml.contains("<D:status>HTTP/1.1 200 OK</D:status>"));
    assert!(xml.contains("<D:status>HTTP/1.1 404 Not Found</D:status>"));
    assert!(xml.contains("<D:totallymissing/>"));
    assert!(xml.contains("<D:getetag>"));
}

#[test]
fn test_propfind_depth_infinity_on_collection_includes_recursive_descendants() {
    let server = setup_test_server_with_tree(|root| {
        create_dir_all(root.join("dir").join("nested")).unwrap();
        create_dir_all(root.join("dir").join("nested").join("leaf")).unwrap();
        let mut file = File::create(root.join("dir").join("nested").join("x.txt")).unwrap();
        writeln!(file, "x").unwrap();
    });
    let client = Client::new();
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#;

    let response = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/dir/", server.addr),
        )
        .header("Depth", "infinity")
        .header("Content-Type", "application/xml")
        .body(body.to_string())
        .send()
        .unwrap();

    assert_eq!(response.status().as_u16(), 207);
    let xml = response.text().unwrap();
    assert!(xml.contains("<D:href>/dir/</D:href>"));
    assert!(xml.contains("<D:href>/dir/nested/</D:href>"));
    assert!(xml.contains("<D:href>/dir/nested/x.txt</D:href>"));
    assert!(xml.contains("<D:href>/dir/nested/leaf/</D:href>"));
}

#[test]
fn test_propfind_missing_depth_defaults_to_infinity_on_collection() {
    let server = setup_test_server_with_tree(|root| {
        create_dir_all(root.join("dir").join("nested")).unwrap();
        let mut file = File::create(root.join("dir").join("nested").join("x.txt")).unwrap();
        writeln!(file, "x").unwrap();
    });
    let client = Client::new();
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#;

    let response = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/dir/", server.addr),
        )
        .header("Content-Type", "application/xml")
        .body(body.to_string())
        .send()
        .unwrap();

    assert_eq!(response.status().as_u16(), 207);
    let xml = response.text().unwrap();
    assert!(xml.contains("<D:href>/dir/</D:href>"));
    assert!(xml.contains("<D:href>/dir/nested/</D:href>"));
    assert!(xml.contains("<D:href>/dir/nested/x.txt</D:href>"));
}

#[test]
fn test_propfind_depth_header_is_case_insensitive_for_infinity() {
    let server = setup_test_server_with_tree(|root| {
        create_dir_all(root.join("dir").join("nested")).unwrap();
        let mut file = File::create(root.join("dir").join("nested").join("x.txt")).unwrap();
        writeln!(file, "x").unwrap();
    });
    let client = Client::new();
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#;

    let response = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/dir/", server.addr),
        )
        .header("Depth", "InFiNiTy")
        .header("Content-Type", "application/xml")
        .body(body.to_string())
        .send()
        .unwrap();

    assert_eq!(response.status().as_u16(), 207);
    let xml = response.text().unwrap();
    assert!(xml.contains("<D:href>/dir/</D:href>"));
    assert!(xml.contains("<D:href>/dir/nested/</D:href>"));
    assert!(xml.contains("<D:href>/dir/nested/x.txt</D:href>"));
}

#[test]
fn test_propfind_empty_body_defaults_to_allprop() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("sample.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Depth", "0")
        .header("Content-Length", "0")
        .send()
        .unwrap();

    assert_eq!(response.status().as_u16(), 207);
    let xml = response.text().unwrap();
    assert!(xml.contains("<D:getlastmodified>"));
    assert!(xml.contains("<D:resourcetype"));
    assert!(xml.contains("<D:creationdate>"));
    assert!(xml.contains('T'));
    assert!(xml.contains('Z'));
}

#[test]
fn test_propfind_allprop_includes_dead_properties() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("sample.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let patch_body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
  <D:set>
    <D:prop>
      <Z:favorite xmlns:Z="urn:test">blue</Z:favorite>
    </D:prop>
  </D:set>
</D:propertyupdate>"#;
    let patch_resp = client
        .request(
            Method::from_bytes(b"PROPPATCH").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Content-Type", "application/xml")
        .body(patch_body.to_string())
        .send()
        .unwrap();
    assert_eq!(patch_resp.status().as_u16(), 207);

    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#;
    let response = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .body(body.to_string())
        .send()
        .unwrap();
    assert_eq!(response.status().as_u16(), 207);
    let xml = response.text().unwrap();
    assert!(xml.contains("favorite") && xml.contains(">blue</"));
}

#[test]
fn test_propfind_wrapper_without_child_defaults_to_allprop() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("sample.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:"></D:propfind>"#;

    let response = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .body(body.to_string())
        .send()
        .unwrap();
    assert_eq!(response.status().as_u16(), 207);
    let xml = response.text().unwrap();
    assert!(xml.contains("<D:getlastmodified>"));
    assert!(xml.contains("<D:getetag>"));
}
