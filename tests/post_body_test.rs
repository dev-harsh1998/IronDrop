//! Test for POST request body parsing functionality.

use irondrop::http::{Request, RequestBody};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread;

#[test]
fn test_post_request_with_body() {
    // Start a listener to create a real TcpStream
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn a thread to accept the connection and parse the request
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(1)))
            .unwrap();

        // Try to parse the request
        match Request::from_stream(&mut stream) {
            Ok(request) => {
                assert_eq!(request.method, "POST");
                assert_eq!(request.path, "/test");
                assert!(request.body.is_some());
                let body = request.body.unwrap();
                match body {
                    RequestBody::Memory(data) => {
                        assert_eq!(data, b"Hello, world!");
                    }
                    RequestBody::File { .. } => {
                        panic!("Expected memory body, got file body");
                    }
                }
                assert_eq!(
                    request.headers.get("content-length"),
                    Some(&"13".to_string())
                );
                true
            }
            Err(e) => {
                eprintln!("Failed to parse request: {e}");
                false
            }
        }
    });

    // Connect and send a POST request with body
    thread::sleep(std::time::Duration::from_millis(10)); // Give listener time to start

    if let Ok(mut stream) = TcpStream::connect(addr) {
        let request =
            "POST /test HTTP/1.1\r\nHost: localhost\r\nContent-Length: 13\r\n\r\nHello, world!";
        let _ = stream.write_all(request.as_bytes());
        let _ = stream.flush();

        // Wait for the parsing thread to complete
        let success = handle.join().unwrap();
        assert!(success, "POST request parsing should succeed");
    } else {
        panic!("Failed to connect to test server");
    }
}

#[test]
fn test_post_request_without_body() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(1)))
            .unwrap();

        match Request::from_stream(&mut stream) {
            Ok(request) => {
                assert_eq!(request.method, "POST");
                assert_eq!(request.path, "/test");
                assert!(request.body.is_none());
                true
            }
            Err(e) => {
                eprintln!("Failed to parse request: {e}");
                false
            }
        }
    });

    thread::sleep(std::time::Duration::from_millis(10));

    if let Ok(mut stream) = TcpStream::connect(addr) {
        let request = "POST /test HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let _ = stream.write_all(request.as_bytes());
        let _ = stream.flush();

        let success = handle.join().unwrap();
        assert!(success, "POST request without body parsing should succeed");
    } else {
        panic!("Failed to connect to test server");
    }
}

#[test]
fn test_get_request_still_works() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(1)))
            .unwrap();

        match Request::from_stream(&mut stream) {
            Ok(request) => {
                assert_eq!(request.method, "GET");
                assert_eq!(request.path, "/test");
                assert!(request.body.is_none());
                true
            }
            Err(e) => {
                eprintln!("Failed to parse request: {e}");
                false
            }
        }
    });

    thread::sleep(std::time::Duration::from_millis(10));

    if let Ok(mut stream) = TcpStream::connect(addr) {
        let request = "GET /test HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let _ = stream.write_all(request.as_bytes());
        let _ = stream.flush();

        let success = handle.join().unwrap();
        assert!(success, "GET request parsing should still work");
    } else {
        panic!("Failed to connect to test server");
    }
}
