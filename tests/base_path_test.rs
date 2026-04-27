// SPDX-License-Identifier: MIT
//! Integration tests for --base-path reverse proxy support.
//! Verifies that all routes, redirects, hrefs, WebDAV XML, and search
//! work correctly when the application is configured with a base path prefix.
//!
//! IMPORTANT: Because `BASE_PATH` uses `OnceLock` (set-once-per-process),
//! every test in this file **must** use the same `--base-path` value: `/bp`.

use irondrop::cli::Cli;
use irondrop::server::run_server;
use reqwest::Method;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use std::fs::{self, File, create_dir_all};
use std::io::{BufRead, Read, Write};
use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use tempfile::{TempDir, tempdir};

const BASE: &str = "/bp";

// ---------------------------------------------------------------------------
// Test server helpers
// ---------------------------------------------------------------------------

struct TestServer {
    addr: SocketAddr,
    shutdown_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<()>>,
    root: std::path::PathBuf,
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

/// Start a server with `--base-path /bp`.
fn setup_bp_server<F>(enable_webdav: bool, populate: F) -> TestServer
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
        enable_upload: Some(true),
        max_upload_size: Some(10240),
        enable_webdav: Some(enable_webdav),
        disable_rate_limit: Some(false),
        config_file: None,
        log_dir: None,
        ssl_cert: None,
        ssl_key: None,
        base_path: Some(BASE.to_string()),
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

// ===========================================================================
// Phase 1 — Routing: base-path stripping and rejection
// ===========================================================================

#[test]
fn test_bp_root_returns_404() {
    let server = setup_bp_server(false, |root| {
        let mut f = File::create(root.join("hello.txt")).unwrap();
        write!(f, "hi").unwrap();
    });
    let client = Client::new();

    let res = client
        .get(format!("http://{}/", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_bp_wrong_prefix_returns_404() {
    let server = setup_bp_server(false, |root| {
        let mut f = File::create(root.join("hello.txt")).unwrap();
        write!(f, "hi").unwrap();
    });
    let client = Client::new();

    let res = client
        .get(format!("http://{}/other/hello.txt", server.addr))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_bp_root_listing_works() {
    let server = setup_bp_server(false, |root| {
        let mut f = File::create(root.join("readme.txt")).unwrap();
        write!(f, "contents").unwrap();
    });
    let client = Client::new();

    let res = client
        .get(format!("http://{}{}/", server.addr, BASE))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert!(
        body.contains("readme.txt"),
        "listing should contain the file"
    );
}

#[test]
fn test_bp_file_download() {
    let server = setup_bp_server(false, |root| {
        let mut f = File::create(root.join("data.txt")).unwrap();
        write!(f, "payload-123").unwrap();
    });
    let client = Client::new();

    let res = client
        .get(format!("http://{}{}/data.txt", server.addr, BASE))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().unwrap(), "payload-123");
}

#[test]
fn test_bp_nested_directory_download() {
    let server = setup_bp_server(false, |root| {
        create_dir_all(root.join("a/b/c")).unwrap();
        let mut f = File::create(root.join("a/b/c/deep.txt")).unwrap();
        write!(f, "deep-content").unwrap();
    });
    let client = Client::new();

    let res = client
        .get(format!("http://{}{}/a/b/c/deep.txt", server.addr, BASE))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().unwrap(), "deep-content");
}

// ===========================================================================
// Phase 2 — Redirects and HTML href generation
// ===========================================================================

#[test]
fn test_bp_directory_trailing_slash_redirect() {
    let server = setup_bp_server(false, |root| {
        create_dir_all(root.join("subdir")).unwrap();
    });
    let no_redirect = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let res = no_redirect
        .get(format!("http://{}{}/subdir", server.addr, BASE))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::MOVED_PERMANENTLY);
    let loc = res.headers().get("location").unwrap().to_str().unwrap();
    assert_eq!(loc, "/bp/subdir/", "redirect must include base-path prefix");
}

#[test]
fn test_bp_listing_links_have_prefix() {
    let server = setup_bp_server(false, |root| {
        create_dir_all(root.join("photos")).unwrap();
        let mut f = File::create(root.join("photos/pic.jpg")).unwrap();
        write!(f, "img").unwrap();
        let mut f2 = File::create(root.join("notes.txt")).unwrap();
        write!(f2, "n").unwrap();
    });
    let client = Client::new();

    // Root listing
    let res = client
        .get(format!("http://{}{}/", server.addr, BASE))
        .send()
        .unwrap();
    let body = res.text().unwrap();
    assert!(
        body.contains("/bp/photos/"),
        "dir entry must have /bp/ prefix"
    );
    assert!(
        body.contains("/bp/notes.txt"),
        "file entry must have /bp/ prefix"
    );

    // Nested listing
    let res = client
        .get(format!("http://{}{}/photos/", server.addr, BASE))
        .send()
        .unwrap();
    let body = res.text().unwrap();
    assert!(
        body.contains("/bp/photos/pic.jpg"),
        "nested file must have /bp/ prefix"
    );
}

#[test]
fn test_bp_html_contains_js_global_and_asset_prefixes() {
    let server = setup_bp_server(false, |root| {
        let mut f = File::create(root.join("f.txt")).unwrap();
        write!(f, "f").unwrap();
    });
    let client = Client::new();

    let res = client
        .get(format!("http://{}{}/", server.addr, BASE))
        .send()
        .unwrap();
    let body = res.text().unwrap();

    assert!(
        body.contains(r#"window.__BASE_PATH = "/bp""#),
        "JS global must be set"
    );
    assert!(
        body.contains("/bp/_irondrop/static/"),
        "static asset links must be prefixed"
    );
    assert!(body.contains("/bp/favicon.ico"), "favicon must be prefixed");
}

#[test]
fn test_bp_static_assets_reachable() {
    let server = setup_bp_server(false, |root| {
        let mut f = File::create(root.join("f.txt")).unwrap();
        write!(f, "f").unwrap();
    });
    let client = Client::new();

    let res = client
        .get(format!(
            "http://{}{}/_irondrop/static/common/base.css",
            server.addr, BASE
        ))
        .send()
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "base.css must be reachable through base path"
    );
    let ct = res.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("text/css"));
}

#[test]
fn test_bp_upload_link_has_prefix() {
    let server = setup_bp_server(false, |_root| {});
    let client = Client::new();

    let res = client
        .get(format!("http://{}{}/", server.addr, BASE))
        .send()
        .unwrap();
    let body = res.text().unwrap();
    assert!(
        body.contains("/bp/_irondrop/upload"),
        "upload link must have base path"
    );
}

// ===========================================================================
// Phase 3 — Search API
// ===========================================================================

#[test]
fn test_bp_search_api_works() {
    let server = setup_bp_server(false, |root| {
        let mut f = File::create(root.join("document.txt")).unwrap();
        write!(f, "contents").unwrap();
    });
    let client = Client::new();

    let res = client
        .get(format!(
            "http://{}{}/_irondrop/search?q=document&path=/",
            server.addr, BASE
        ))
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.text().unwrap();
    assert!(body.contains("document.txt"), "search must find the file");
}

// ===========================================================================
// Phase 4 — WebDAV with base-path
// ===========================================================================

#[test]
fn test_bp_webdav_options() {
    let server = setup_bp_server(true, |_root| {});
    let client = Client::new();

    let res = client
        .request(
            Method::from_bytes(b"OPTIONS").unwrap(),
            format!("http://{}{}/", server.addr, BASE),
        )
        .send()
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let dav = res.headers().get("DAV").unwrap().to_str().unwrap();
    assert!(dav.contains("1"));
    let allow = res.headers().get("Allow").unwrap().to_str().unwrap();
    assert!(allow.contains("PROPFIND"));
    assert!(allow.contains("COPY"));
    assert!(allow.contains("MOVE"));
}

#[test]
fn test_bp_webdav_propfind_root_href() {
    let server = setup_bp_server(true, |root| {
        create_dir_all(root.join("docs")).unwrap();
        let mut f = File::create(root.join("docs/report.txt")).unwrap();
        write!(f, "data").unwrap();
    });
    let client = Client::new();

    let res = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}{}/", server.addr, BASE),
        )
        .header("Depth", "1")
        .send()
        .unwrap();

    assert_eq!(res.status().as_u16(), 207);
    let body = res.text().unwrap();
    assert!(
        body.contains("<D:href>/bp/</D:href>"),
        "root href must be /bp/, got:\n{body}"
    );
    assert!(
        body.contains("<D:href>/bp/docs/</D:href>"),
        "child dir href must be /bp/docs/"
    );
}

#[test]
fn test_bp_webdav_propfind_nested_hrefs() {
    let server = setup_bp_server(true, |root| {
        create_dir_all(root.join("a/b")).unwrap();
        let mut f = File::create(root.join("a/b/file.txt")).unwrap();
        write!(f, "nested").unwrap();
    });
    let client = Client::new();

    let res = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}{}/a/b/", server.addr, BASE),
        )
        .header("Depth", "1")
        .send()
        .unwrap();

    assert_eq!(res.status().as_u16(), 207);
    let body = res.text().unwrap();
    assert!(
        body.contains("<D:href>/bp/a/b/</D:href>"),
        "nested collection href"
    );
    assert!(
        body.contains("<D:href>/bp/a/b/file.txt</D:href>"),
        "nested file href"
    );
}

#[test]
fn test_bp_webdav_propfind_infinity_all_hrefs_prefixed() {
    let server = setup_bp_server(true, |root| {
        create_dir_all(root.join("x/y")).unwrap();
        let mut f = File::create(root.join("x/y/z.txt")).unwrap();
        write!(f, "deep").unwrap();
    });
    let client = Client::new();

    let res = client
        .request(
            Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://{}{}/", server.addr, BASE),
        )
        .header("Depth", "infinity")
        .send()
        .unwrap();

    assert_eq!(res.status().as_u16(), 207);
    let body = res.text().unwrap();

    // Every <D:href> must start with /bp/
    for line in body.lines() {
        if let Some(start) = line.find("<D:href>") {
            let href_start = start + "<D:href>".len();
            if let Some(end) = line[href_start..].find("</D:href>") {
                let href = &line[href_start..href_start + end];
                assert!(
                    href.starts_with("/bp/"),
                    "every href must start with /bp/, got: {href}"
                );
            }
        }
    }
}

#[test]
fn test_bp_webdav_put() {
    let server = setup_bp_server(true, |_root| {});
    let client = Client::new();

    let res = client
        .request(
            Method::from_bytes(b"PUT").unwrap(),
            format!("http://{}{}/new-file.txt", server.addr, BASE),
        )
        .body("created-via-put")
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    assert_eq!(
        fs::read_to_string(server.root.join("new-file.txt")).unwrap(),
        "created-via-put"
    );
}

#[test]
fn test_bp_webdav_mkcol() {
    let server = setup_bp_server(true, |_root| {});
    let client = Client::new();

    let res = client
        .request(
            Method::from_bytes(b"MKCOL").unwrap(),
            format!("http://{}{}/newcol/", server.addr, BASE),
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    assert!(server.root.join("newcol").is_dir());
}

#[test]
fn test_bp_webdav_delete() {
    let server = setup_bp_server(true, |root| {
        let mut f = File::create(root.join("gone.txt")).unwrap();
        write!(f, "bye").unwrap();
    });
    let client = Client::new();

    let res = client
        .request(
            Method::from_bytes(b"DELETE").unwrap(),
            format!("http://{}{}/gone.txt", server.addr, BASE),
        )
        .send()
        .unwrap();

    assert_eq!(res.status().as_u16(), 204);
    assert!(!server.root.join("gone.txt").exists());
}

#[test]
fn test_bp_webdav_copy() {
    let server = setup_bp_server(true, |root| {
        let mut f = File::create(root.join("src.txt")).unwrap();
        write!(f, "copy-data").unwrap();
    });
    let client = Client::new();

    let res = client
        .request(
            Method::from_bytes(b"COPY").unwrap(),
            format!("http://{}{}/src.txt", server.addr, BASE),
        )
        .header(
            "Destination",
            format!("http://{}{}/dst.txt", server.addr, BASE),
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    assert_eq!(
        fs::read_to_string(server.root.join("src.txt")).unwrap(),
        "copy-data"
    );
    assert_eq!(
        fs::read_to_string(server.root.join("dst.txt")).unwrap(),
        "copy-data"
    );
}

#[test]
fn test_bp_webdav_copy_wrong_prefix_rejected() {
    let server = setup_bp_server(true, |root| {
        let mut f = File::create(root.join("src.txt")).unwrap();
        write!(f, "data").unwrap();
    });
    let client = Client::new();

    let res = client
        .request(
            Method::from_bytes(b"COPY").unwrap(),
            format!("http://{}{}/src.txt", server.addr, BASE),
        )
        .header(
            "Destination",
            format!("http://{}/wrong/dst.txt", server.addr),
        )
        .send()
        .unwrap();

    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "destination with wrong prefix must be rejected"
    );
}

#[test]
fn test_bp_webdav_move() {
    let server = setup_bp_server(true, |root| {
        let mut f = File::create(root.join("old.txt")).unwrap();
        write!(f, "move-data").unwrap();
    });
    let client = Client::new();

    let res = client
        .request(
            Method::from_bytes(b"MOVE").unwrap(),
            format!("http://{}{}/old.txt", server.addr, BASE),
        )
        .header(
            "Destination",
            format!("http://{}{}/new.txt", server.addr, BASE),
        )
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    assert!(!server.root.join("old.txt").exists());
    assert_eq!(
        fs::read_to_string(server.root.join("new.txt")).unwrap(),
        "move-data"
    );
}

#[test]
fn test_bp_webdav_move_nested_dirs() {
    let server = setup_bp_server(true, |root| {
        create_dir_all(root.join("from/sub")).unwrap();
        let mut f = File::create(root.join("from/sub/file.txt")).unwrap();
        write!(f, "nested-move").unwrap();
    });
    let client = Client::new();

    let res = client
        .request(
            Method::from_bytes(b"MOVE").unwrap(),
            format!("http://{}{}/from/", server.addr, BASE),
        )
        .header("Destination", format!("http://{}{}/to/", server.addr, BASE))
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    assert!(!server.root.join("from").exists());
    assert_eq!(
        fs::read_to_string(server.root.join("to/sub/file.txt")).unwrap(),
        "nested-move"
    );
}

#[test]
fn test_bp_webdav_lock_lockroot_has_prefix() {
    let server = setup_bp_server(true, |root| {
        let mut f = File::create(root.join("locked.txt")).unwrap();
        write!(f, "l").unwrap();
    });
    let client = Client::new();

    let lock_body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:lockinfo xmlns:D="DAV:">
  <D:lockscope><D:exclusive/></D:lockscope>
  <D:locktype><D:write/></D:locktype>
</D:lockinfo>"#;

    let res = client
        .request(
            Method::from_bytes(b"LOCK").unwrap(),
            format!("http://{}{}/locked.txt", server.addr, BASE),
        )
        .header("Timeout", "Second-3600")
        .body(lock_body)
        .send()
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let body = res.text().unwrap();
    assert!(
        body.contains("/bp/locked.txt"),
        "lockroot must include base path, got:\n{body}"
    );
}

#[test]
fn test_bp_webdav_proppatch_href_has_prefix() {
    let server = setup_bp_server(true, |root| {
        let mut f = File::create(root.join("prop.txt")).unwrap();
        write!(f, "p").unwrap();
    });
    let client = Client::new();

    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:" xmlns:Z="http://ns.example.com/z/">
  <D:set><D:prop><Z:Author>TestUser</Z:Author></D:prop></D:set>
</D:propertyupdate>"#;

    let res = client
        .request(
            Method::from_bytes(b"PROPPATCH").unwrap(),
            format!("http://{}{}/prop.txt", server.addr, BASE),
        )
        .body(body)
        .send()
        .unwrap();

    assert_eq!(res.status().as_u16(), 207);
    let resp_body = res.text().unwrap();
    assert!(
        resp_body.contains("/bp/prop.txt"),
        "PROPPATCH href must include base path"
    );
}

// ===========================================================================
// Phase 5 — Raw TCP edge cases
// ===========================================================================

#[test]
fn test_bp_raw_tcp_file_download() {
    let server = setup_bp_server(false, |root| {
        let mut f = File::create(root.join("tcp.txt")).unwrap();
        write!(f, "raw-tcp-content").unwrap();
    });

    let mut stream = std::net::TcpStream::connect(server.addr).unwrap();
    write!(
        stream,
        "GET /bp/tcp.txt HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    stream.flush().unwrap();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap();
    let text = String::from_utf8_lossy(&buf);
    assert!(text.contains("HTTP/1.1 200"), "expected 200, got: {text}");
    assert!(text.contains("raw-tcp-content"));
}

#[test]
fn test_bp_raw_tcp_without_prefix_is_404() {
    let server = setup_bp_server(false, |root| {
        let mut f = File::create(root.join("tcp.txt")).unwrap();
        write!(f, "content").unwrap();
    });

    let mut stream = std::net::TcpStream::connect(server.addr).unwrap();
    write!(
        stream,
        "GET /tcp.txt HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    stream.flush().unwrap();

    let mut reader = std::io::BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();
    assert!(
        status_line.contains("404"),
        "without prefix must be 404, got: {status_line}"
    );
}

// ===========================================================================
// Phase 6 — CLI validation unit tests
// ===========================================================================

#[test]
fn test_cli_base_path_normalization() {
    // Valid paths
    let cli = Cli {
        directory: std::path::PathBuf::from("."),
        listen: None,
        port: None,
        allowed_extensions: None,
        threads: None,
        chunk_size: None,
        verbose: None,
        detailed_logging: None,
        username: None,
        password: None,
        enable_upload: None,
        max_upload_size: None,
        enable_webdav: None,
        disable_rate_limit: None,
        config_file: None,
        log_dir: None,
        ssl_cert: None,
        ssl_key: None,
        base_path: Some("/webstorage".to_string()),
    };
    assert_eq!(cli.base_path.as_deref(), Some("/webstorage"));

    // With trailing slash — the server should strip it in validate_base_path
    let cli2 = Cli {
        directory: std::path::PathBuf::from("."),
        listen: None,
        port: None,
        allowed_extensions: None,
        threads: None,
        chunk_size: None,
        verbose: None,
        detailed_logging: None,
        username: None,
        password: None,
        enable_upload: None,
        max_upload_size: None,
        enable_webdav: None,
        disable_rate_limit: None,
        config_file: None,
        log_dir: None,
        ssl_cert: None,
        ssl_key: None,
        base_path: Some("/storage/".to_string()),
    };
    assert!(cli2.base_path.is_some());
}
