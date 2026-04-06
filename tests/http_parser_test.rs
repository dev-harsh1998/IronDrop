// SPDX-License-Identifier: MIT

use irondrop::http::{ClientStream, Request};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread;

fn serve_and_parse(request: &str) -> Result<Request, irondrop::error::AppError> {
    serve_and_parse_bytes(request.as_bytes())
}

fn serve_and_parse_bytes(request: &[u8]) -> Result<Request, irondrop::error::AppError> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let req_owned = request.to_vec();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = stream.write_all(&req_owned);
        let _ = stream.flush();
        // Keep connection until parser finishes on client side
        std::thread::sleep(std::time::Duration::from_millis(50));
    });

    let client = TcpStream::connect(addr).unwrap();
    let mut client_stream = ClientStream::Plain(client);
    Request::from_stream(&mut client_stream).map(|r| {
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
fn test_chunked_encoding_is_accepted() {
    let req = concat!(
        "POST / HTTP/1.1\r\n",
        "Host: x\r\n",
        "Transfer-Encoding: chunked\r\n",
        "\r\n",
        "4\r\n",
        "Wiki\r\n",
        "5\r\n",
        "pedia\r\n",
        "0\r\n",
        "\r\n"
    );
    let result = serve_and_parse(req).expect("chunked request should parse");
    assert_eq!(result.method, "POST");
    match result.body {
        Some(irondrop::http::RequestBody::Memory(body)) => assert_eq!(body, b"Wikipedia"),
        _ => panic!("expected memory body for chunked payload"),
    }
}

#[test]
fn test_chunked_encoding_malformed_chunk_size_rejected() {
    let req = concat!(
        "POST / HTTP/1.1\r\n",
        "Host: x\r\n",
        "Transfer-Encoding: chunked\r\n",
        "\r\n",
        "ZZ\r\n",
        "abc\r\n",
        "0\r\n",
        "\r\n"
    );
    let result = serve_and_parse(req);
    assert!(result.is_err());
}

#[test]
fn test_chunked_encoding_invalid_terminator_rejected() {
    let req = concat!(
        "POST / HTTP/1.1\r\n",
        "Host: x\r\n",
        "Transfer-Encoding: chunked\r\n",
        "\r\n",
        "3\r\n",
        "abcX",
        "0\r\n",
        "\r\n"
    );
    let result = serve_and_parse(req);
    assert!(result.is_err());
}

#[test]
fn test_chunked_encoding_with_trailers_is_accepted() {
    let req = concat!(
        "POST / HTTP/1.1\r\n",
        "Host: x\r\n",
        "Transfer-Encoding: chunked\r\n",
        "\r\n",
        "3\r\n",
        "abc\r\n",
        "0\r\n",
        "X-Test: ok\r\n",
        "\r\n"
    );
    let result = serve_and_parse(req).expect("chunked request with trailer should parse");
    match result.body {
        Some(irondrop::http::RequestBody::Memory(body)) => assert_eq!(body, b"abc"),
        _ => panic!("expected memory body for chunked payload"),
    }
}

#[test]
fn test_chunked_and_content_length_conflict_rejected() {
    let req = concat!(
        "POST / HTTP/1.1\r\n",
        "Host: x\r\n",
        "Content-Length: 4\r\n",
        "Transfer-Encoding: chunked\r\n",
        "\r\n",
        "4\r\n",
        "Wiki\r\n",
        "0\r\n",
        "\r\n"
    );
    let result = serve_and_parse(req);
    assert!(result.is_err());
}

#[test]
fn test_missing_host_header() {
    let req = "GET / HTTP/1.1\r\n\r\n";
    let _result = serve_and_parse(req);
    // Should handle gracefully - either accept or reject with appropriate error
    // This test ensures no panic occurs
}

#[test]
fn test_malformed_method() {
    let test_cases = vec![
        "INVALID / HTTP/1.1\r\nHost: x\r\n\r\n",
        "GET@INVALID / HTTP/1.1\r\nHost: x\r\n\r\n",
        "\r\n / HTTP/1.1\r\nHost: x\r\n\r\n",
        "", // Empty method
    ];

    for req in test_cases {
        let result = serve_and_parse(req);
        assert!(result.is_err(), "Should reject malformed method: {}", req);
    }
}

#[test]
fn test_malformed_path() {
    let test_cases = vec![
        "GET  HTTP/1.1\r\nHost: x\r\n\r\n",          // Missing path
        "GET \r\n HTTP/1.1\r\nHost: x\r\n\r\n",      // Path with newline
        "GET \x00/path HTTP/1.1\r\nHost: x\r\n\r\n", // Path with null byte
    ];

    for req in test_cases {
        let result = serve_and_parse(req);
        assert!(result.is_err(), "Should reject malformed path: {:?}", req);
    }
}

#[test]
fn test_malformed_headers() {
    let test_cases = vec![
        "GET / HTTP/1.1\r\nInvalid-Header-No-Colon\r\n\r\n",
        "GET / HTTP/1.1\r\nHost: x\r\n: value-without-name\r\n\r\n",
        "GET / HTTP/1.1\r\nHost\r\n\r\n", // Header without colon or value
        "GET / HTTP/1.1\r\nHost: x\r\nContent-Length: not-a-number\r\n\r\n",
    ];

    for req in test_cases {
        let _result = serve_and_parse(req);
        // Should either parse successfully (ignoring bad headers) or return error
        // This test ensures no panic occurs
    }
}

#[test]
fn test_extremely_long_request_line() {
    let long_path = "/".to_string() + &"x".repeat(65536); // 64KB path
    let req = format!("GET {} HTTP/1.1\r\nHost: x\r\n\r\n", long_path);
    let _result = serve_and_parse(&req);
    // Should either accept or reject with appropriate error (414 URI Too Long)
    // This test ensures no panic or infinite loop occurs
}

#[test]
fn test_case_insensitive_headers() {
    let req = "GET / HTTP/1.1\r\nHOST: x\r\ncontent-length: 0\r\nContent-Type: text/plain\r\n\r\n";
    let result = serve_and_parse(req);
    if let Ok(request) = result {
        // Headers should be accessible regardless of case
        assert!(request.headers.contains_key("host") || request.headers.contains_key("HOST"));
    }
}

#[test]
fn test_multiple_content_length_headers() {
    let req = "POST / HTTP/1.1\r\nHost: x\r\nContent-Length: 5\r\nContent-Length: 10\r\n\r\n";
    let result = serve_and_parse(req);
    // Should reject multiple Content-Length headers as per RFC
    assert!(result.is_err());
}

#[test]
fn test_header_continuation_lines() {
    // HTTP/1.1 allows header continuation with leading whitespace
    let req = "GET / HTTP/1.1\r\nHost: x\r\nX-Custom: value1\r\n continuation\r\n\r\n";
    let _result = serve_and_parse(req);
    // Should handle header continuation or reject gracefully
}

#[test]
fn test_request_with_body_but_no_content_length() {
    let req = "POST / HTTP/1.1\r\nHost: x\r\n\r\nsome body data";
    let _result = serve_and_parse(req);
    // Should handle gracefully - either require Content-Length or read until connection close
}

#[test]
fn test_http_version_variations() {
    let test_cases = vec![
        "GET / HTTP/1.0\r\nHost: x\r\n\r\n", // HTTP/1.0
        "GET / HTTP/1.1\r\nHost: x\r\n\r\n", // HTTP/1.1
        "GET / HTTP/2.0\r\nHost: x\r\n\r\n", // HTTP/2.0 (should reject)
        "GET / HTTP/3.0\r\nHost: x\r\n\r\n", // HTTP/3.0 (should reject)
        "GET / HTTP/1.2\r\nHost: x\r\n\r\n", // Invalid minor version
    ];

    for req in test_cases {
        let _result = serve_and_parse(req);
        // Should accept HTTP/1.0 and HTTP/1.1, reject others
    }
}

#[test]
fn test_webdav_methods_are_accepted() {
    let methods = [
        "PROPFIND",
        "MKCOL",
        "COPY",
        "MOVE",
        "PROPPATCH",
        "LOCK",
        "UNLOCK",
    ];

    for method in methods {
        let req = format!("{method} /dav/path HTTP/1.1\r\nHost: x\r\n\r\n");
        let result = serve_and_parse(&req);
        assert!(result.is_ok(), "method {method} should be accepted");
    }
}

#[test]
fn test_chunked_body_split_across_tcp_frames() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .write_all(b"POST / HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nWi")
            .unwrap();
        stream.flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        stream.write_all(b"ki\r\n0\r\n\r\n").unwrap();
        stream.flush().unwrap();
    });

    let client = TcpStream::connect(addr).unwrap();
    let mut client_stream = ClientStream::Plain(client);
    let parsed = Request::from_stream(&mut client_stream).expect("request should parse");
    handle.join().unwrap();

    match parsed.body {
        Some(irondrop::http::RequestBody::Memory(body)) => assert_eq!(body, b"Wiki"),
        _ => panic!("expected in-memory body"),
    }
}
