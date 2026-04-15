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
fn test_copy_creates_destination_and_preserves_source() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("source.txt")).unwrap();
        write!(file, "copy-me").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"COPY").unwrap(),
            format!("http://{}/source.txt", server.addr),
        )
        .header("Destination", format!("http://{}/dest.txt", server.addr))
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(
        fs::read_to_string(server.root.join("source.txt")).unwrap(),
        "copy-me"
    );
    assert_eq!(
        fs::read_to_string(server.root.join("dest.txt")).unwrap(),
        "copy-me"
    );
}

#[test]
fn test_copy_overwrite_false_returns_precondition_failed() {
    let server = setup_test_server_with_tree(|root| {
        let mut src = File::create(root.join("src.txt")).unwrap();
        write!(src, "src").unwrap();
        let mut dst = File::create(root.join("dst.txt")).unwrap();
        write!(dst, "dst").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"COPY").unwrap(),
            format!("http://{}/src.txt", server.addr),
        )
        .header("Destination", format!("http://{}/dst.txt", server.addr))
        .header("Overwrite", "F")
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::PRECONDITION_FAILED);
    assert_eq!(
        fs::read_to_string(server.root.join("dst.txt")).unwrap(),
        "dst"
    );
}

#[test]
fn test_move_renames_resource() {
    let server = setup_test_server_with_tree(|root| {
        create_dir_all(root.join("from")).unwrap();
        let mut src = File::create(root.join("from").join("file.txt")).unwrap();
        write!(src, "move-me").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"MOVE").unwrap(),
            format!("http://{}/from/file.txt", server.addr),
        )
        .header(
            "Destination",
            format!("http://{}/file-moved.txt", server.addr),
        )
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    assert!(!server.root.join("from").join("file.txt").exists());
    assert_eq!(
        fs::read_to_string(server.root.join("file-moved.txt")).unwrap(),
        "move-me"
    );
}

#[test]
fn test_copy_missing_destination_is_bad_request() {
    let server = setup_test_server_with_tree(|root| {
        let mut src = File::create(root.join("src.txt")).unwrap();
        write!(src, "src").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"COPY").unwrap(),
            format!("http://{}/src.txt", server.addr),
        )
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_copy_path_only_destination_is_bad_request() {
    let server = setup_test_server_with_tree(|root| {
        let mut src = File::create(root.join("src.txt")).unwrap();
        write!(src, "src").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"COPY").unwrap(),
            format!("http://{}/src.txt", server.addr),
        )
        .header("Destination", "/dst.txt")
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_move_finder_temp_source_creates_missing_destination_parent() {
    let server = setup_test_server_with_tree(|root| {
        create_dir_all(root.join(".AU.PCOUC")).unwrap();
        let mut src = File::create(root.join(".AU.PCOUC").join("a.txt")).unwrap();
        write!(src, "tmp").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"MOVE").unwrap(),
            format!("http://{}/.AU.PCOUC/", server.addr),
        )
        .header(
            "Destination",
            format!("http://{}/missing/parent/.AU.PCOUC/", server.addr),
        )
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    assert!(!server.root.join(".AU.PCOUC").exists());
    assert!(server.root.join("missing/parent/.AU.PCOUC").exists());
}

#[test]
fn test_move_into_missing_sb_container_creates_parent() {
    let server = setup_test_server_with_tree(|root| {
        create_dir_all(root.join("TaiwanStuff")).unwrap();
        let mut src = File::create(root.join("TaiwanStuff").join("a.txt")).unwrap();
        write!(src, "x").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"MOVE").unwrap(),
            format!("http://{}/TaiwanStuff/", server.addr),
        )
        .header(
            "Destination",
            format!(
                "http://{}/TaiwanStuff.sb-1234-AbCdEf/TaiwanStuff/",
                server.addr
            ),
        )
        .send()
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    assert!(!server.root.join("TaiwanStuff").exists());
    assert!(
        server
            .root
            .join("TaiwanStuff.sb-1234-AbCdEf/TaiwanStuff")
            .exists()
    );
}
