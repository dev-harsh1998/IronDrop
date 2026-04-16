// SPDX-License-Identifier: MIT

use irondrop::cli::Cli;
use irondrop::server::run_server;
use reqwest::Method;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use std::fs::File;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::{Arc, Barrier, mpsc};
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

fn acquire_lock_token(client: &Client, addr: SocketAddr, path: &str) -> String {
    let response = client
        .request(
            Method::from_bytes(b"LOCK").unwrap(),
            format!("http://{addr}{path}"),
        )
        .header("Timeout", "Second-600")
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
        .to_string()
}

#[test]
fn test_lock_refresh_with_if_header() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let lock_token_header = acquire_lock_token(&client, server.addr, "/doc.txt");
    let token = lock_token_header
        .trim_matches(|c| c == '<' || c == '>')
        .to_string();

    let refresh = client
        .request(
            Method::from_bytes(b"LOCK").unwrap(),
            format!("http://{}/doc.txt", server.addr),
        )
        .header("If", format!("(<{token}>)"))
        .send()
        .unwrap();
    assert_eq!(refresh.status(), StatusCode::OK);
}

#[test]
fn test_lock_response_includes_lockroot_and_depth_zero_for_file() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"LOCK").unwrap(),
            format!("http://{}/doc.txt", server.addr),
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
    let body = response.text().unwrap();
    assert!(body.contains("<D:depth>0</D:depth>"));
    assert!(body.contains("<D:lockroot><D:href>/doc.txt</D:href></D:lockroot>"));
}

#[test]
fn test_new_lock_without_lockinfo_body_is_bad_request() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"LOCK").unwrap(),
            format!("http://{}/doc.txt", server.addr),
        )
        .send()
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_new_lock_with_shared_scope_is_conflict() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"LOCK").unwrap(),
            format!("http://{}/doc.txt", server.addr),
        )
        .body(
            r#"<?xml version="1.0" encoding="utf-8"?>
<D:lockinfo xmlns:D="DAV:">
  <D:lockscope><D:shared/></D:lockscope>
  <D:locktype><D:write/></D:locktype>
</D:lockinfo>"#,
        )
        .send()
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[test]
fn test_file_lock_rejects_depth_infinity() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"LOCK").unwrap(),
            format!("http://{}/doc.txt", server.addr),
        )
        .header("Depth", "infinity")
        .body(
            r#"<?xml version="1.0" encoding="utf-8"?>
<D:lockinfo xmlns:D="DAV:">
  <D:lockscope><D:exclusive/></D:lockscope>
  <D:locktype><D:write/></D:locktype>
</D:lockinfo>"#,
        )
        .send()
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_collection_lock_accepts_case_insensitive_depth_infinity() {
    let server = setup_test_server_with_tree(|root| {
        std::fs::create_dir_all(root.join("dir")).unwrap();
    });
    let client = Client::new();

    let response = client
        .request(
            Method::from_bytes(b"LOCK").unwrap(),
            format!("http://{}/dir/", server.addr),
        )
        .header("Depth", "InFiNiTy")
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
}

#[test]
fn test_if_not_condition_does_not_satisfy_lock_requirement() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let lock_token_header = acquire_lock_token(&client, server.addr, "/doc.txt");
    let token = lock_token_header
        .trim_matches(|c| c == '<' || c == '>')
        .to_string();

    let put = client
        .request(Method::PUT, format!("http://{}/doc.txt", server.addr))
        .header("If", format!("(Not <{token}>)"))
        .body("blocked".to_string())
        .send()
        .unwrap();
    assert_eq!(put.status().as_u16(), 423);
}

#[test]
fn test_if_not_wrong_token_does_not_bypass_lock() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();
    let _token = acquire_lock_token(&client, server.addr, "/doc.txt");

    let put = client
        .request(Method::PUT, format!("http://{}/doc.txt", server.addr))
        .header("If", "(Not <opaquelocktoken:wrong-token>)")
        .body("blocked".to_string())
        .send()
        .unwrap();
    assert_eq!(put.status(), StatusCode::LOCKED);
}

#[test]
fn test_if_tagged_list_with_correct_token_allows_write() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let lock_token_header = acquire_lock_token(&client, server.addr, "/doc.txt");
    let token = lock_token_header
        .trim_matches(|c| c == '<' || c == '>')
        .to_string();

    let put = client
        .request(Method::PUT, format!("http://{}/doc.txt", server.addr))
        .header(
            "If",
            format!("<http://{}/doc.txt> (<{}>)", server.addr, token),
        )
        .body("updated".to_string())
        .send()
        .unwrap();
    assert!(put.status() == StatusCode::NO_CONTENT || put.status() == StatusCode::CREATED);
}

#[test]
fn test_if_tag_for_other_resource_does_not_authorize() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
        let mut other = File::create(root.join("other.txt")).unwrap();
        writeln!(other, "other").unwrap();
    });
    let client = Client::new();
    let lock_token_header = acquire_lock_token(&client, server.addr, "/doc.txt");
    let token = lock_token_header
        .trim_matches(|c| c == '<' || c == '>')
        .to_string();

    let put = client
        .request(Method::PUT, format!("http://{}/doc.txt", server.addr))
        .header(
            "If",
            format!("<http://{}/other.txt> (<{}>)", server.addr, token),
        )
        .body("blocked".to_string())
        .send()
        .unwrap();
    assert_eq!(put.status(), StatusCode::LOCKED);
}

#[test]
fn test_collection_lock_blocks_mutation_of_child_without_token() {
    let server = setup_test_server_with_tree(|root| {
        std::fs::create_dir_all(root.join("dir")).unwrap();
        let mut file = File::create(root.join("dir").join("child.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();
    let _token = acquire_lock_token(&client, server.addr, "/dir/");

    let put = client
        .request(Method::PUT, format!("http://{}/dir/child.txt", server.addr))
        .body("blocked".to_string())
        .send()
        .unwrap();
    assert_eq!(put.status(), StatusCode::LOCKED);
}

#[test]
fn test_if_header_with_token_and_etag_state_token_is_accepted() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();
    let lock_token_header = acquire_lock_token(&client, server.addr, "/doc.txt");
    let token = lock_token_header
        .trim_matches(|c| c == '<' || c == '>')
        .to_string();

    let put = client
        .request(Method::PUT, format!("http://{}/doc.txt", server.addr))
        .header("If", format!("(<{}> [\"etag-state\"]) ", token))
        .body("updated".to_string())
        .send()
        .unwrap();
    assert!(put.status() == StatusCode::NO_CONTENT || put.status() == StatusCode::CREATED);
}

#[test]
fn test_unlock_with_wrong_token_is_conflict() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();
    let _token = acquire_lock_token(&client, server.addr, "/doc.txt");

    let unlock = client
        .request(
            Method::from_bytes(b"UNLOCK").unwrap(),
            format!("http://{}/doc.txt", server.addr),
        )
        .header("Lock-Token", "<opaquelocktoken:wrong-token>")
        .send()
        .unwrap();
    assert_eq!(unlock.status(), StatusCode::CONFLICT);
}

#[test]
fn test_unlock_missing_lock_token_is_bad_request() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();
    let _token = acquire_lock_token(&client, server.addr, "/doc.txt");

    let unlock = client
        .request(
            Method::from_bytes(b"UNLOCK").unwrap(),
            format!("http://{}/doc.txt", server.addr),
        )
        .send()
        .unwrap();
    assert_eq!(unlock.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_unlock_without_active_lock_is_conflict() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();

    let unlock = client
        .request(
            Method::from_bytes(b"UNLOCK").unwrap(),
            format!("http://{}/doc.txt", server.addr),
        )
        .header("Lock-Token", "<opaquelocktoken:does-not-exist>")
        .send()
        .unwrap();
    assert_eq!(unlock.status(), StatusCode::CONFLICT);
}

#[test]
fn test_lock_on_unmapped_url_creates_resource() {
    let server = setup_test_server_with_tree(|_root| {});
    let client = Client::new();

    let lock = client
        .request(
            Method::from_bytes(b"LOCK").unwrap(),
            format!("http://{}/new-doc.txt", server.addr),
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
    assert_eq!(lock.status(), StatusCode::CREATED);

    let get = client
        .get(format!("http://{}/new-doc.txt", server.addr))
        .send()
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
}

#[test]
fn test_delete_with_lock_token_clears_lock_state() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();
    let lock_token_header = acquire_lock_token(&client, server.addr, "/doc.txt");
    let token = lock_token_header
        .trim_matches(|c| c == '<' || c == '>')
        .to_string();

    let delete = client
        .request(Method::DELETE, format!("http://{}/doc.txt", server.addr))
        .header("If", format!("(<{}>)", token))
        .send()
        .unwrap();
    assert_eq!(delete.status(), StatusCode::NO_CONTENT);

    let recreate = client
        .request(Method::PUT, format!("http://{}/doc.txt", server.addr))
        .body("new".to_string())
        .send()
        .unwrap();
    assert!(
        recreate.status() == StatusCode::CREATED || recreate.status() == StatusCode::NO_CONTENT
    );
}

#[test]
fn test_move_transfers_lock_to_destination() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("src.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let client = Client::new();
    let lock_token_header = acquire_lock_token(&client, server.addr, "/src.txt");
    let token = lock_token_header
        .trim_matches(|c| c == '<' || c == '>')
        .to_string();

    let mv = client
        .request(
            Method::from_bytes(b"MOVE").unwrap(),
            format!("http://{}/src.txt", server.addr),
        )
        .header("Destination", format!("http://{}/dst.txt", server.addr))
        .header("If", format!("(<{}>)", token))
        .send()
        .unwrap();
    assert!(mv.status() == StatusCode::CREATED || mv.status() == StatusCode::NO_CONTENT);

    let blocked = client
        .request(Method::PUT, format!("http://{}/dst.txt", server.addr))
        .body("blocked".to_string())
        .send()
        .unwrap();
    assert_eq!(blocked.status(), StatusCode::LOCKED);

    let allowed = client
        .request(Method::PUT, format!("http://{}/dst.txt", server.addr))
        .header("If", format!("(<{}>)", token))
        .body("updated".to_string())
        .send()
        .unwrap();
    assert!(allowed.status() == StatusCode::NO_CONTENT || allowed.status() == StatusCode::CREATED);
}

#[test]
fn test_concurrent_lock_attempts_only_one_succeeds() {
    let server = setup_test_server_with_tree(|root| {
        let mut file = File::create(root.join("doc.txt")).unwrap();
        writeln!(file, "hello").unwrap();
    });
    let barrier = Arc::new(Barrier::new(6));
    let mut handles = Vec::new();
    for _ in 0..5usize {
        let barrier_cloned = barrier.clone();
        let addr = server.addr;
        handles.push(thread::spawn(move || {
            let client = Client::new();
            barrier_cloned.wait();
            client
                .request(
                    Method::from_bytes(b"LOCK").unwrap(),
                    format!("http://{addr}/doc.txt"),
                )
                .body(
                    r#"<?xml version="1.0" encoding="utf-8"?>
<D:lockinfo xmlns:D="DAV:">
  <D:lockscope><D:exclusive/></D:lockscope>
  <D:locktype><D:write/></D:locktype>
</D:lockinfo>"#,
                )
                .send()
                .unwrap()
                .status()
        }));
    }
    barrier.wait();
    let mut ok = 0usize;
    let mut locked = 0usize;
    for handle in handles {
        let status = handle.join().unwrap();
        if status == StatusCode::OK || status == StatusCode::CREATED {
            ok += 1;
        } else if status == StatusCode::LOCKED {
            locked += 1;
        }
    }
    assert_eq!(ok, 1, "exactly one lock acquisition should succeed");
    assert_eq!(locked, 4, "other concurrent lock attempts should be locked");
}
