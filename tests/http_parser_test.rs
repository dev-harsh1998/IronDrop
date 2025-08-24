// SPDX-License-Identifier: MIT

use irondrop::http::Request;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn serve_and_parse(request: &str) -> Result<Request, irondrop::error::AppError> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let req_owned = request.as_bytes().to_vec();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = stream.write_all(&req_owned);
        let _ = stream.flush();
        // Keep connection until parser finishes on client side
        std::thread::sleep(std::time::Duration::from_millis(50));
    });

    let mut client = TcpStream::connect(addr).unwrap();
    Request::from_stream(&mut client).map(|r| {
        handle.join().unwrap();
        r
    })
}

#[test]
fn test_invalid_http_version_is_bad_request() {
    let req = "GET / HTTP/2.0\r\nHost: x\r\n\r\n";
    let result = serve_and_parse(req);
    assert!(result.is_err());
}

#[test]
fn test_lf_only_headers_separator() {
    let req = b"GET /%2Fpath HTTP/1.1\nHost: x\nContent-Length: 0\n\n";
    let result = serve_and_parse(std::str::from_utf8(req).unwrap()).unwrap();
    assert_eq!(result.path, "//path");
}

#[test]
fn test_chunked_encoding_rejected() {
    let req = "POST / HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n";
    let result = serve_and_parse(req);
    assert!(result.is_err());
}
