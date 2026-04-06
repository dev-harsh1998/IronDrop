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
fn test_proppatch_set_property_then_propfind_reads_it() {
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

    let propfind_body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <Z:favorite xmlns:Z="urn:test"/>
  </D:prop>
</D:propfind>"#;
    let read_resp = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .body(propfind_body.to_string())
        .send()
        .unwrap();
    assert_eq!(read_resp.status().as_u16(), 207);
    let xml = read_resp.text().unwrap();
    assert!(xml.contains("favorite") && xml.contains(">blue</"));
}

#[test]
fn test_proppatch_remove_property() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("sample.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let set_body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
  <D:set>
    <D:prop>
      <Z:favorite xmlns:Z="urn:test">blue</Z:favorite>
    </D:prop>
  </D:set>
</D:propertyupdate>"#;
    client
        .request(
            Method::from_bytes(b"PROPPATCH").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Content-Type", "application/xml")
        .body(set_body.to_string())
        .send()
        .unwrap();

    let remove_body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
  <D:remove>
    <D:prop>
      <Z:favorite xmlns:Z="urn:test"/>
    </D:prop>
  </D:remove>
</D:propertyupdate>"#;
    let remove_resp = client
        .request(
            Method::from_bytes(b"PROPPATCH").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Content-Type", "application/xml")
        .body(remove_body.to_string())
        .send()
        .unwrap();
    assert_eq!(remove_resp.status().as_u16(), 207);

    let propfind_body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <Z:favorite xmlns:Z="urn:test"/>
  </D:prop>
</D:propfind>"#;
    let read_resp = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .body(propfind_body.to_string())
        .send()
        .unwrap();
    assert_eq!(read_resp.status().as_u16(), 207);
    let xml = read_resp.text().unwrap();
    assert!(xml.contains("HTTP/1.1 404 Not Found"));
}

#[test]
fn test_proppatch_empty_propertyupdate_is_bad_request() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("sample.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let patch_body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:"/>"#;

    let patch_resp = client
        .request(
            Method::from_bytes(b"PROPPATCH").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Content-Type", "application/xml")
        .body(patch_body.to_string())
        .send()
        .unwrap();
    assert_eq!(patch_resp.status().as_u16(), 400);
}

#[test]
fn test_proppatch_respects_document_order_remove_then_set() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("sample.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let seed_body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
  <D:set>
    <D:prop>
      <Z:favorite xmlns:Z="urn:test">blue</Z:favorite>
    </D:prop>
  </D:set>
</D:propertyupdate>"#;
    client
        .request(
            Method::from_bytes(b"PROPPATCH").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Content-Type", "application/xml")
        .body(seed_body.to_string())
        .send()
        .unwrap();

    let ordered_body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
  <D:remove>
    <D:prop>
      <Z:favorite xmlns:Z="urn:test"/>
    </D:prop>
  </D:remove>
  <D:set>
    <D:prop>
      <Z:favorite xmlns:Z="urn:test">green</Z:favorite>
    </D:prop>
  </D:set>
</D:propertyupdate>"#;
    let patch_resp = client
        .request(
            Method::from_bytes(b"PROPPATCH").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Content-Type", "application/xml")
        .body(ordered_body.to_string())
        .send()
        .unwrap();
    assert_eq!(patch_resp.status().as_u16(), 207);

    let propfind_body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <Z:favorite xmlns:Z="urn:test"/>
  </D:prop>
</D:propfind>"#;
    let read_resp = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .body(propfind_body.to_string())
        .send()
        .unwrap();
    assert_eq!(read_resp.status().as_u16(), 207);
    let xml = read_resp.text().unwrap();
    assert!(xml.contains("favorite") && xml.contains(">green</"));
}

#[test]
fn test_proppatch_protected_live_property_returns_403_propstat() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("sample.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
  <D:set>
    <D:prop>
      <D:getetag>"override"</D:getetag>
    </D:prop>
  </D:set>
</D:propertyupdate>"#;
    let resp = client
        .request(
            Method::from_bytes(b"PROPPATCH").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Content-Type", "application/xml")
        .body(body.to_string())
        .send()
        .unwrap();
    assert_eq!(resp.status().as_u16(), 207);
    let xml = resp.text().unwrap();
    assert!(xml.contains("HTTP/1.1 403 Forbidden"));
}

#[test]
fn test_proppatch_namespace_distinct_properties_do_not_collide() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("sample.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:" xmlns:A="urn:a" xmlns:B="urn:b">
  <D:set>
    <D:prop>
      <A:tag>one</A:tag>
      <B:tag>two</B:tag>
    </D:prop>
  </D:set>
</D:propertyupdate>"#;
    let resp = client
        .request(
            Method::from_bytes(b"PROPPATCH").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Content-Type", "application/xml")
        .body(body.to_string())
        .send()
        .unwrap();
    assert_eq!(resp.status().as_u16(), 207);

    let propfind_body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:" xmlns:A="urn:a" xmlns:B="urn:b">
  <D:prop>
    <A:tag/>
    <B:tag/>
  </D:prop>
</D:propfind>"#;
    let read_resp = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/sample.txt", server.addr),
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .body(propfind_body.to_string())
        .send()
        .unwrap();
    assert_eq!(read_resp.status().as_u16(), 207);
    let xml = read_resp.text().unwrap();
    assert!(xml.contains("urn:a"));
    assert!(xml.contains("urn:b"));
    assert!(xml.contains(">one</"));
    assert!(xml.contains(">two</"));
}
