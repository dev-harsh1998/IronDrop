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

fn lock_token(client: &Client, addr: SocketAddr, path: &str) -> String {
    let response = client
        .request(
            Method::from_bytes(b"LOCK").unwrap(),
            format!("http://{addr}{path}"),
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
    assert!(response.status() == StatusCode::CREATED || response.status() == StatusCode::OK);
    response
        .headers()
        .get("lock-token")
        .unwrap()
        .to_str()
        .unwrap()
        .trim_matches(|c| c == '<' || c == '>')
        .to_string()
}

#[test]
fn test_move_locked_destination_without_if_token_is_locked() {
    let server = setup_test_server_with_tree(|root| {
        let mut src = File::create(root.join("src.txt")).unwrap();
        writeln!(src, "src").unwrap();
        let mut dst = File::create(root.join("dst.txt")).unwrap();
        writeln!(dst, "dst").unwrap();
    });
    let client = Client::new();
    let _dst_lock = lock_token(&client, server.addr, "/dst.txt");

    let move_resp = client
        .request(
            Method::from_bytes(b"MOVE").unwrap(),
            format!("http://{}/src.txt", server.addr),
        )
        .header("Destination", format!("http://{}/dst.txt", server.addr))
        .send()
        .unwrap();

    assert_eq!(move_resp.status().as_u16(), 423);
}

#[test]
fn test_delete_collection_with_locked_child_returns_multistatus() {
    let server = setup_test_server_with_tree(|root| {
        create_dir_all(root.join("dir")).unwrap();
        let mut child = File::create(root.join("dir").join("child.txt")).unwrap();
        writeln!(child, "child").unwrap();
    });
    let client = Client::new();
    let _child_lock = lock_token(&client, server.addr, "/dir/child.txt");

    let delete_resp = client
        .request(Method::DELETE, format!("http://{}/dir/", server.addr))
        .send()
        .unwrap();

    assert_eq!(delete_resp.status().as_u16(), 207);
    let xml = delete_resp.text().unwrap();
    assert!(xml.contains("/dir/child.txt"));
    assert!(xml.contains("HTTP/1.1 423 Locked"));
    assert!(xml.contains("/dir/"));
    assert!(xml.contains("HTTP/1.1 424 Failed Dependency"));
}

#[test]
fn test_copy_depth_zero_on_collection_does_not_copy_members() {
    let server = setup_test_server_with_tree(|root| {
        create_dir_all(root.join("src").join("nested")).unwrap();
        let mut child = File::create(root.join("src").join("nested").join("child.txt")).unwrap();
        writeln!(child, "child").unwrap();
    });
    let client = Client::new();

    let copy_resp = client
        .request(
            Method::from_bytes(b"COPY").unwrap(),
            format!("http://{}/src/", server.addr),
        )
        .header("Depth", "0")
        .header("Destination", format!("http://{}/dst/", server.addr))
        .send()
        .unwrap();
    assert_eq!(copy_resp.status(), StatusCode::CREATED);

    let nested_resp = client
        .get(format!("http://{}/dst/nested/child.txt", server.addr))
        .send()
        .unwrap();
    assert_eq!(nested_resp.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_copy_destination_descendant_of_source_is_bad_request() {
    let server = setup_test_server_with_tree(|root| {
        create_dir_all(root.join("src").join("nested")).unwrap();
        let mut child = File::create(root.join("src").join("nested").join("child.txt")).unwrap();
        writeln!(child, "child").unwrap();
    });
    let client = Client::new();

    let copy_resp = client
        .request(
            Method::from_bytes(b"COPY").unwrap(),
            format!("http://{}/src/", server.addr),
        )
        .header(
            "Destination",
            format!("http://{}/src/nested/newcopy/", server.addr),
        )
        .send()
        .unwrap();
    assert_eq!(copy_resp.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_move_preserves_dead_property_at_destination() {
    let server = setup_test_server_with_tree(|root| {
        let mut src = File::create(root.join("src.txt")).unwrap();
        writeln!(src, "src").unwrap();
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
            format!("http://{}/src.txt", server.addr),
        )
        .header("Content-Type", "application/xml")
        .body(patch_body.to_string())
        .send()
        .unwrap();
    assert_eq!(patch_resp.status().as_u16(), 207);

    let move_resp = client
        .request(
            Method::from_bytes(b"MOVE").unwrap(),
            format!("http://{}/src.txt", server.addr),
        )
        .header("Destination", format!("http://{}/dst.txt", server.addr))
        .send()
        .unwrap();
    assert!(
        move_resp.status() == StatusCode::CREATED || move_resp.status() == StatusCode::NO_CONTENT
    );

    let propfind_body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <Z:favorite xmlns:Z="urn:test"/>
  </D:prop>
</D:propfind>"#;
    let read_resp = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}/dst.txt", server.addr),
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .body(propfind_body.to_string())
        .send()
        .unwrap();
    assert_eq!(read_resp.status().as_u16(), 207);
    let xml = read_resp.text().unwrap();
    assert!(
        xml.contains("favorite") && xml.contains(">blue</"),
        "unexpected xml: {xml}"
    );
}

#[test]
fn test_copy_invalid_depth_is_bad_request() {
    let server = setup_test_server_with_tree(|root| {
        let mut src = File::create(root.join("src.txt")).unwrap();
        writeln!(src, "src").unwrap();
    });
    let client = Client::new();

    let copy_resp = client
        .request(
            Method::from_bytes(b"COPY").unwrap(),
            format!("http://{}/src.txt", server.addr),
        )
        .header("Depth", "1")
        .header("Destination", format!("http://{}/dst.txt", server.addr))
        .send()
        .unwrap();
    assert_eq!(copy_resp.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_delete_collection_with_multiple_tokens_succeeds() {
    let server = setup_test_server_with_tree(|root| {
        create_dir_all(root.join("dir")).unwrap();
        let mut a = File::create(root.join("dir").join("a.txt")).unwrap();
        writeln!(a, "a").unwrap();
        let mut b = File::create(root.join("dir").join("b.txt")).unwrap();
        writeln!(b, "b").unwrap();
    });
    let client = Client::new();
    let token_a = lock_token(&client, server.addr, "/dir/a.txt");
    let token_b = lock_token(&client, server.addr, "/dir/b.txt");

    let delete_resp = client
        .request(Method::DELETE, format!("http://{}/dir/", server.addr))
        .header("If", format!("(<{}>) (<{}>)", token_a, token_b))
        .send()
        .unwrap();
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    let check = client
        .request(Method::GET, format!("http://{}/dir/", server.addr))
        .send()
        .unwrap();
    assert_eq!(check.status(), StatusCode::NOT_FOUND);
}
