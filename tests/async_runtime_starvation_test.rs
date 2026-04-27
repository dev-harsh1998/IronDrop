// SPDX-License-Identifier: MIT

use irondrop::cli::Cli;
use irondrop::server::run_server;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;

struct TestServer {
    addr: SocketAddr,
    shutdown_tx: mpsc::Sender<()>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = self.shutdown_tx.send(());
            let _ = handle.join();
        }
    }
}

fn start_server(dir: std::path::PathBuf, threads: usize) -> TestServer {
    let cli = Cli {
        directory: dir,
        listen: Some("127.0.0.1".to_string()),
        port: Some(0),
        allowed_extensions: Some("*.bin".to_string()),
        threads: Some(threads),
        chunk_size: Some(64 * 1024),
        verbose: Some(false),
        detailed_logging: Some(false),
        username: None,
        password: None,
        enable_upload: Some(false),
        max_upload_size: Some(10240),
        enable_webdav: Some(false),
        disable_rate_limit: Some(false),
        config_file: None,
        log_dir: None,
        ssl_cert: None,
        ssl_key: None,
        base_path: None,
    };

    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let (addr_tx, addr_rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        if let Err(e) = run_server(cli, Some(shutdown_rx), Some(addr_tx)) {
            eprintln!("server failed: {e}");
        }
    });

    let addr = addr_rx.recv().unwrap();
    TestServer {
        addr,
        shutdown_tx,
        handle: Some(handle),
    }
}

fn read_until_headers_end(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            return Ok(buf);
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.windows(2).any(|w| w == b"\n\n") {
            return Ok(buf);
        }
        if buf.len() > 64 * 1024 {
            return Ok(buf);
        }
    }
}

#[test]
fn test_large_downloads_do_not_starve_health_requests() {
    let dir = tempdir().unwrap();
    let big_path = dir.path().join("big.bin");
    let f = File::create(&big_path).unwrap();
    f.set_len(512 * 1024 * 1024).unwrap();

    let server = start_server(dir.path().to_path_buf(), 2);

    let mut slow1 = TcpStream::connect(server.addr).unwrap();
    slow1
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    slow1
        .write_all(b"GET /big.bin HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n")
        .unwrap();
    let _ = read_until_headers_end(&mut slow1).unwrap();

    let mut slow2 = TcpStream::connect(server.addr).unwrap();
    slow2
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    slow2
        .write_all(b"GET /big.bin HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n")
        .unwrap();
    let _ = read_until_headers_end(&mut slow2).unwrap();

    thread::sleep(Duration::from_millis(200));

    let start = Instant::now();
    let mut health = TcpStream::connect(server.addr).unwrap();
    health
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    health
        .write_all(
            b"GET /_irondrop/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .unwrap();

    let headers = read_until_headers_end(&mut health).unwrap();
    let elapsed = start.elapsed();

    let headers_str = String::from_utf8_lossy(&headers);
    assert!(headers_str.starts_with("HTTP/1.1 200"));
    assert!(
        elapsed < Duration::from_millis(500),
        "health took {elapsed:?}"
    );

    drop(slow1);
    drop(slow2);
}
