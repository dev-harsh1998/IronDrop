// SPDX-License-Identifier: MIT

use irondrop::response::{create_error_response, get_mime_type};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;

#[test]
fn test_get_mime_type_common() {
    assert_eq!(get_mime_type(Path::new("a.html")), "text/html");
    assert_eq!(get_mime_type(Path::new("a.css")), "text/css");
    assert_eq!(get_mime_type(Path::new("a.js")), "application/javascript");
    assert_eq!(get_mime_type(Path::new("a.png")), "image/png");
    assert_eq!(
        get_mime_type(Path::new("a.unknown")),
        "application/octet-stream"
    );
}

#[test]
fn test_create_error_response_headers_and_body() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut resp = create_error_response(401, "Unauthorized");
        // Should include WWW-Authenticate for 401
        resp.send(&mut stream, "[test]").unwrap();
    });

    let mut client = TcpStream::connect(addr).unwrap();
    // Read what server wrote
    let mut buf = Vec::new();
    use std::io::Read;
    client.read_to_end(&mut buf).unwrap();
    let text = String::from_utf8_lossy(&buf);
    assert!(text.contains("HTTP/1.1 401 Unauthorized"));
    assert!(text.contains("WWW-Authenticate: Basic realm=\"Restricted\""));
    assert!(text.contains("Content-Length:"));
    assert!(text.contains("text/html"));
    handle.join().unwrap();
}
