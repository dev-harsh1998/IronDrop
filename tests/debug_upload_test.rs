//! Debug test for upload functionality
//! This test attempts to isolate and debug the upload functionality issues.

use irondrop::cli::Cli;
use irondrop::http::Request;
use irondrop::multipart::{MultipartConfig, MultipartParser};
use irondrop::upload::UploadHandler;
use std::collections::HashMap;
use std::io::Cursor;
use tempfile::tempdir;

#[test]
fn test_multipart_parser_direct() {
    // Test the multipart parser directly to see if it can handle our data
    let boundary = "----IronDropTestBoundary12345";
    let multipart_data = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\nContent-Type: text/plain\r\n\r\nHello, World!\r\n--{boundary}--\r\n"
    );

    println!("Testing multipart data:\n{multipart_data}");

    let config = MultipartConfig::default();
    let cursor = Cursor::new(multipart_data.as_bytes());

    let parser_result = MultipartParser::new(cursor, boundary, config);
    assert!(
        parser_result.is_ok(),
        "Failed to create multipart parser: {:?}",
        parser_result.err()
    );

    let parser = parser_result.unwrap();
    let mut part_count = 0;

    for part_result in parser {
        match part_result {
            Ok(mut part) => {
                part_count += 1;
                println!(
                    "Found part: field_name = {:?}, filename = {:?}",
                    part.field_name(),
                    part.filename
                );
                let content = part.read_to_string().unwrap();
                println!("Part content: {content}");
                assert_eq!(content, "Hello, World!");
            }
            Err(e) => {
                panic!("Error parsing multipart data: {e:?}");
            }
        }
    }

    assert_eq!(part_count, 1, "Expected 1 part, found {part_count}");
}

#[test]
fn test_boundary_extraction() {
    // Test boundary extraction from Content-Type header
    let content_type = "multipart/form-data; boundary=----IronDropTestBoundary12345";
    let boundary_result =
        MultipartParser::<Cursor<Vec<u8>>>::extract_boundary_from_content_type(content_type);

    assert!(
        boundary_result.is_ok(),
        "Failed to extract boundary: {:?}",
        boundary_result.err()
    );
    let boundary = boundary_result.unwrap();
    println!("Extracted boundary: {boundary}");
    assert_eq!(boundary, "----IronDropTestBoundary12345");
}

#[test]
fn test_upload_handler_creation() {
    // Test that we can create an upload handler
    let temp_dir = tempdir().unwrap();

    let cli = Cli {
        directory: temp_dir.path().to_path_buf(),
        listen: "127.0.0.1".to_string(),
        port: 8080,
        allowed_extensions: "*.txt".to_string(),
        threads: 4,
        chunk_size: 1024,
        verbose: false,
        detailed_logging: false,
        username: None,
        password: None,
        enable_upload: true,
        max_upload_size: 10,
        upload_dir: Some(temp_dir.path().to_path_buf()),
        config_file: None,
    };

    let handler_result = UploadHandler::new(&cli);
    assert!(
        handler_result.is_ok(),
        "Failed to create upload handler: {:?}",
        handler_result.err()
    );

    let handler = handler_result.unwrap();
    let config = handler.get_config_info();
    println!("Upload handler config: {config:?}");
}

#[test]
fn test_upload_handler_direct() {
    // Initialize logger for this test
    let _ = env_logger::builder().is_test(true).try_init();

    // Test the upload handler directly with a multipart request
    let temp_dir = tempdir().unwrap();

    let cli = Cli {
        directory: temp_dir.path().to_path_buf(),
        listen: "127.0.0.1".to_string(),
        port: 8080,
        allowed_extensions: "*.txt".to_string(),
        threads: 4,
        chunk_size: 1024,
        verbose: false,
        detailed_logging: false,
        username: None,
        password: None,
        enable_upload: true,
        max_upload_size: 10,
        upload_dir: Some(temp_dir.path().to_path_buf()),
        config_file: None,
    };

    let mut handler = UploadHandler::new(&cli).unwrap();

    // Create a test multipart request
    let boundary = "----IronDropTestBoundary12345";
    let multipart_body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\nContent-Type: text/plain\r\n\r\nHello, World!\r\n--{boundary}--\r\n"
    );

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        format!("multipart/form-data; boundary={boundary}"),
    );
    headers.insert(
        "content-length".to_string(),
        multipart_body.len().to_string(),
    );

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(multipart_body.into_bytes()),
    };

    println!("Testing upload handler directly...");
    let result = handler.handle_upload(&request, None);

    match result {
        Ok(response) => {
            println!(
                "Success! Status: {} {}",
                response.status_code, response.status_text
            );
            assert_eq!(response.status_code, 200);
        }
        Err(e) => {
            println!("Error: {:?}", e);
            panic!("Upload handler failed: {:?}", e);
        }
    }
}

#[test]
fn test_upload_handler_no_extension_restrictions() {
    // Initialize logger for this test
    let _ = env_logger::builder().is_test(true).try_init();

    // Test the upload handler directly with no extension restrictions
    let temp_dir = tempdir().unwrap();

    let cli = Cli {
        directory: temp_dir.path().to_path_buf(),
        listen: "127.0.0.1".to_string(),
        port: 8080,
        allowed_extensions: "".to_string(), // Test with no extension restrictions
        threads: 4,
        chunk_size: 1024,
        verbose: false,
        detailed_logging: false,
        username: None,
        password: None,
        enable_upload: true,
        max_upload_size: 10,
        upload_dir: Some(temp_dir.path().to_path_buf()),
        config_file: None,
    };

    let mut handler = UploadHandler::new(&cli).unwrap();

    // Create a test multipart request
    let boundary = "----IronDropTestBoundary12345";
    let multipart_body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\nContent-Type: text/plain\r\n\r\nHello, World!\r\n--{boundary}--\r\n"
    );

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        format!("multipart/form-data; boundary={boundary}"),
    );
    headers.insert(
        "content-length".to_string(),
        multipart_body.len().to_string(),
    );

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(multipart_body.into_bytes()),
    };

    println!("Testing upload handler with no extension restrictions...");
    let result = handler.handle_upload(&request, None);

    match result {
        Ok(response) => {
            println!(
                "Success! Status: {} {}",
                response.status_code, response.status_text
            );
            assert_eq!(response.status_code, 200);
        }
        Err(e) => {
            println!("Error: {:?}", e);
            panic!("Upload handler failed: {:?}", e);
        }
    }
}

#[test]
fn test_multipart_parser_multiple_files() {
    // Test the multipart parser with multiple files
    let boundary = "----IronDropTestBoundary12345";
    let multipart_data = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file1\"; filename=\"test1.txt\"\r\nContent-Type: text/plain\r\n\r\nFirst file content\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file2\"; filename=\"test2.txt\"\r\nContent-Type: text/plain\r\n\r\nSecond file content\r\n--{boundary}--\r\n"
    );

    println!("Testing multipart parser with multiple files:\n{multipart_data}");

    let config = MultipartConfig::default();
    let cursor = Cursor::new(multipart_data.as_bytes());

    let parser_result = MultipartParser::new(cursor, boundary, config);
    assert!(
        parser_result.is_ok(),
        "Failed to create multipart parser: {:?}",
        parser_result.err()
    );

    let parser = parser_result.unwrap();
    let mut part_count = 0;

    for part_result in parser {
        match part_result {
            Ok(mut part) => {
                part_count += 1;
                println!(
                    "Found part {}: field_name = {:?}, filename = {:?}",
                    part_count,
                    part.field_name(),
                    part.filename
                );
                let content = part.read_to_string().unwrap();
                println!("Part {} content: {}", part_count, content);

                match part_count {
                    1 => {
                        assert_eq!(part.field_name(), Some("file1"));
                        assert_eq!(part.filename, Some("test1.txt".to_string()));
                        assert_eq!(content, "First file content");
                    }
                    2 => {
                        assert_eq!(part.field_name(), Some("file2"));
                        assert_eq!(part.filename, Some("test2.txt".to_string()));
                        assert_eq!(content, "Second file content");
                    }
                    _ => panic!("Unexpected part count: {}", part_count),
                }
            }
            Err(e) => {
                panic!("Error parsing multipart data: {e:?}");
            }
        }
    }

    assert_eq!(part_count, 2, "Expected 2 parts, found {part_count}");
}

#[test]
fn test_request_creation() {
    // Test creating a request manually like our test does
    let boundary = "----IronDropTestBoundary12345";
    let multipart_body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\nContent-Type: application/octet-stream\r\n\r\nHello, World!\r\n--{boundary}--\r\n"
    );

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        format!("multipart/form-data; boundary={boundary}"),
    );
    headers.insert(
        "content-length".to_string(),
        multipart_body.len().to_string(),
    );

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(multipart_body.into_bytes()),
    };

    println!(
        "Created request - method: {}, path: {}",
        request.method, request.path
    );
    println!("Content-Type: {:?}", request.headers.get("content-type"));
    println!("Body size: {} bytes", request.body.as_ref().unwrap().len());

    // Test that we can extract the boundary
    let content_type = request.headers.get("content-type").unwrap();
    let boundary_result =
        MultipartParser::<Cursor<Vec<u8>>>::extract_boundary_from_content_type(content_type);
    assert!(
        boundary_result.is_ok(),
        "Failed to extract boundary from request: {:?}",
        boundary_result.err()
    );
}
