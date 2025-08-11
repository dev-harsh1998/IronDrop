use irondrop::http::{RequestBody, STREAM_TO_DISK_THRESHOLD};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Test that verifies HTTP layer streams large bodies to disk
#[test]
fn test_http_layer_streams_large_bodies_to_disk() {
    // Create a large request body that exceeds the streaming threshold
    let large_body_size = STREAM_TO_DISK_THRESHOLD + 1024; // Just over 128MB
    let large_body_data = vec![b'X'; large_body_size];

    // Create multipart data
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let mut multipart_data = Vec::new();

    // Add boundary and headers
    multipart_data.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    multipart_data.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"large_test.txt\"\r\n",
    );
    multipart_data.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");

    // Add file data
    multipart_data.extend_from_slice(&large_body_data);

    // Add closing boundary
    multipart_data.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    // Create HTTP request
    let http_request = format!(
        "POST /upload HTTP/1.1\r\n\
         Content-Type: multipart/form-data; boundary={}\r\n\
         Content-Length: {}\r\n\
         \r\n",
        boundary,
        multipart_data.len()
    );

    // Start a test server
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind test server");
    let server_addr = listener.local_addr().expect("Failed to get server address");

    let multipart_data_clone = multipart_data.clone();
    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");

        // Parse the request using our HTTP layer
        match irondrop::http::Request::from_stream(&mut stream) {
            Ok(request) => {
                // Verify that large body was streamed to disk
                match &request.body {
                    Some(RequestBody::File { path, size }) => {
                        assert_eq!(*size, multipart_data_clone.len() as u64);

                        // Verify the file exists and has the correct content
                        let file_content = std::fs::read(path).expect("Failed to read temp file");
                        assert_eq!(file_content.len(), multipart_data_clone.len());

                        // Clean up
                        request.cleanup();

                        println!(
                            "✓ Large HTTP body ({}MB) successfully streamed to disk",
                            multipart_data_clone.len() / (1024 * 1024)
                        );

                        // Send a simple response
                        let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
                        let _ = stream.write_all(response.as_bytes());
                    }
                    Some(RequestBody::Memory(_)) => {
                        panic!(
                            "Expected large body to be streamed to disk, but it was kept in memory"
                        );
                    }
                    None => {
                        panic!("Expected request body, but none found");
                    }
                }
            }
            Err(e) => {
                panic!("Failed to parse HTTP request: {:?}", e);
            }
        }
    });

    // Give the server a moment to start
    thread::sleep(Duration::from_millis(100));

    // Connect to the test server and send the request
    let mut client = TcpStream::connect(server_addr).expect("Failed to connect to test server");

    // Send headers
    client
        .write_all(http_request.as_bytes())
        .expect("Failed to send headers");

    // Send body
    client
        .write_all(&multipart_data)
        .expect("Failed to send body");

    // Read response
    let mut response = String::new();
    client
        .read_to_string(&mut response)
        .expect("Failed to read response");

    // Wait for server to finish
    server_handle.join().expect("Server thread panicked");

    assert!(response.contains("200 OK"), "Expected successful response");
}

/// Test that verifies HTTP layer keeps small bodies in memory
#[test]
fn test_http_layer_keeps_small_bodies_in_memory() {
    // Create a small request body that's under the streaming threshold
    let small_body_size = 1024; // 1KB - well under 128MB threshold
    let small_body_data = vec![b'Y'; small_body_size];

    // Create multipart data
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let mut multipart_data = Vec::new();

    // Add boundary and headers
    multipart_data.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    multipart_data.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"small_test.txt\"\r\n",
    );
    multipart_data.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");

    // Add file data
    multipart_data.extend_from_slice(&small_body_data);

    // Add closing boundary
    multipart_data.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    // Create HTTP request
    let http_request = format!(
        "POST /upload HTTP/1.1\r\n\
         Content-Type: multipart/form-data; boundary={}\r\n\
         Content-Length: {}\r\n\
         \r\n",
        boundary,
        multipart_data.len()
    );

    // Start a test server
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind test server");
    let server_addr = listener.local_addr().expect("Failed to get server address");

    let multipart_data_clone2 = multipart_data.clone();
    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");

        // Parse the request using our HTTP layer
        match irondrop::http::Request::from_stream(&mut stream) {
            Ok(request) => {
                // Verify that small body was kept in memory
                match &request.body {
                    Some(RequestBody::Memory(data)) => {
                        assert_eq!(data.len(), multipart_data_clone2.len());

                        println!(
                            "✓ Small HTTP body ({}KB) successfully kept in memory",
                            data.len() / 1024
                        );

                        // Send a simple response
                        let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
                        let _ = stream.write_all(response.as_bytes());
                    }
                    Some(RequestBody::File { .. }) => {
                        panic!(
                            "Expected small body to be kept in memory, but it was streamed to disk"
                        );
                    }
                    None => {
                        panic!("Expected request body, but none found");
                    }
                }
            }
            Err(e) => {
                panic!("Failed to parse HTTP request: {:?}", e);
            }
        }
    });

    // Give the server a moment to start
    thread::sleep(Duration::from_millis(100));

    // Connect to the test server and send the request
    let mut client = TcpStream::connect(server_addr).expect("Failed to connect to test server");

    // Send headers
    client
        .write_all(http_request.as_bytes())
        .expect("Failed to send headers");

    // Send body
    client
        .write_all(&multipart_data)
        .expect("Failed to send body");

    // Read response
    let mut response = String::new();
    client
        .read_to_string(&mut response)
        .expect("Failed to read response");

    // Wait for server to finish
    server_handle.join().expect("Server thread panicked");

    assert!(response.contains("200 OK"), "Expected successful response");
}
