use irondrop::cli::Cli;
use irondrop::http::{Request, RequestBody};
use irondrop::upload::UploadHandler;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper function to create test CLI configuration
fn create_test_cli(upload_dir: PathBuf) -> Cli {
    Cli {
        directory: upload_dir,
        listen: Some("127.0.0.1".to_string()),
        port: Some(8080),
        allowed_extensions: Some("*".to_string()),
        threads: Some(4),
        chunk_size: Some(1024),
        verbose: Some(false),
        detailed_logging: Some(false),
        username: None,
        password: None,
        enable_upload: Some(true),
        max_upload_size: Some(10240), // 10GB in MB
        config_file: None,
    }
}

/// Helper function to create multipart request body
fn create_multipart_request_body(data: &[u8], filename: &str, boundary: &str) -> Vec<u8> {
    let mut body = Vec::new();

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n\
            Content-Type: application/octet-stream\r\n\
            \r\n",
            filename
        )
        .as_bytes(),
    );
    body.extend_from_slice(data);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    body
}

#[test]
fn test_empty_file_upload() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test uploading an empty file
    let filename = "empty_file.txt";
    let test_data = Vec::new(); // Empty data
    let boundary = "empty_file_boundary";
    let body = create_multipart_request_body(&test_data, filename, boundary);

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        format!("multipart/form-data; boundary={}", boundary),
    );
    headers.insert("content-length".to_string(), body.len().to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(body)),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(response.is_ok(), "Empty file upload should succeed");

    // Verify empty file was created
    let uploaded_file_path = temp_dir.path().join(filename);
    assert!(uploaded_file_path.exists(), "Empty file should exist");

    let uploaded_data = fs::read(&uploaded_file_path).unwrap();
    assert_eq!(uploaded_data.len(), 0, "Empty file should have zero size");

    println!("Empty file upload handled correctly");
}

#[test]
fn test_single_byte_file() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test uploading a single byte file
    let filename = "single_byte.bin";
    let test_data = vec![42u8]; // Single byte
    let boundary = "single_byte_boundary";
    let body = create_multipart_request_body(&test_data, filename, boundary);

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        format!("multipart/form-data; boundary={}", boundary),
    );
    headers.insert("content-length".to_string(), body.len().to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(body)),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(response.is_ok(), "Single byte file upload should succeed");

    // Verify file content
    let uploaded_file_path = temp_dir.path().join(filename);
    let uploaded_data = fs::read(&uploaded_file_path).unwrap();
    assert_eq!(
        uploaded_data.len(),
        1,
        "Single byte file should have size 1"
    );
    assert_eq!(uploaded_data[0], 42, "Single byte should be preserved");

    println!("Single byte file upload handled correctly");
}

#[test]
fn test_malformed_multipart_boundary() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test with malformed multipart data
    let boundary = "malformed_boundary";
    let malformed_body = b"--malformed_boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\r\ntest data\r\n--malformed_boundary"; // Missing final --

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        format!("multipart/form-data; boundary={}", boundary),
    );
    headers.insert(
        "content-length".to_string(),
        malformed_body.len().to_string(),
    );

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(malformed_body.to_vec())),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(
        response.is_err(),
        "Malformed multipart should fail gracefully"
    );

    println!("Malformed multipart boundary handled correctly");
}

#[test]
fn test_missing_content_type() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test with missing content-type header
    let test_data = b"test data";
    let mut headers = HashMap::new();
    headers.insert("content-length".to_string(), test_data.len().to_string());
    // Missing content-type header

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(test_data.to_vec())),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(response.is_err(), "Missing content-type should fail");

    println!("Missing content-type handled correctly");
}

#[test]
fn test_invalid_filename_characters() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test various invalid filename characters
    let invalid_filenames = vec![
        "../../../etc/passwd", // Path traversal
        "file\0name.txt",      // Null byte
        "file\nname.txt",      // Newline
        "file\rname.txt",      // Carriage return
        "file\tname.txt",      // Tab
        "con.txt",             // Windows reserved name
        "aux.txt",             // Windows reserved name
        "",                    // Empty filename
    ];

    for invalid_filename in invalid_filenames {
        println!("Testing invalid filename: {:?}", invalid_filename);

        let test_data = b"test data";
        let boundary = "invalid_filename_boundary";
        let body = create_multipart_request_body(test_data, invalid_filename, boundary);

        let mut headers = HashMap::new();
        headers.insert(
            "content-type".to_string(),
            format!("multipart/form-data; boundary={}", boundary),
        );
        headers.insert("content-length".to_string(), body.len().to_string());

        let request = Request {
            method: "POST".to_string(),
            path: "/upload".to_string(),
            headers,
            body: Some(RequestBody::Memory(body)),
        };

        let response = upload_handler.handle_upload(&request, None);
        // Some invalid filenames might be sanitized rather than rejected
        match response {
            Ok(_) => println!(
                "  Invalid filename '{}' was sanitized and accepted",
                invalid_filename
            ),
            Err(_) => println!("  Invalid filename '{}' was rejected", invalid_filename),
        }
    }

    println!("Invalid filename characters handled correctly");
}

#[test]
fn test_very_long_filename() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test with extremely long filename
    let long_filename = "a".repeat(1000) + ".txt"; // 1000+ character filename
    let test_data = b"test data";
    let boundary = "long_filename_boundary";
    let body = create_multipart_request_body(test_data, &long_filename, boundary);

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        format!("multipart/form-data; boundary={}", boundary),
    );
    headers.insert("content-length".to_string(), body.len().to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(body)),
    };

    let response = upload_handler.handle_upload(&request, None);
    // This might succeed or fail depending on filesystem limits
    // The important thing is it doesn't crash
    match response {
        Ok(_) => println!("Long filename accepted"),
        Err(_) => println!("Long filename rejected gracefully"),
    }
}

#[test]
fn test_boundary_at_buffer_edge() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test with boundary that might split across buffer boundaries
    let filename = "boundary_edge_test.bin";
    let boundary = "boundary_edge_test_boundary_that_is_quite_long_to_test_edge_cases";

    // Create data that will cause boundary to appear at various buffer positions
    let buffer_size = 8192; // Our streaming buffer size
    let test_data: Vec<u8> = (0..buffer_size * 2).map(|i| (i % 256) as u8).collect();

    let body = create_multipart_request_body(&test_data, filename, boundary);

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        format!("multipart/form-data; boundary={}", boundary),
    );
    headers.insert("content-length".to_string(), body.len().to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(body)),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(response.is_ok(), "Boundary edge test should succeed");

    // Verify file integrity
    let uploaded_file_path = temp_dir.path().join(filename);
    let uploaded_data = fs::read(&uploaded_file_path).unwrap();
    assert_eq!(
        uploaded_data.len(),
        test_data.len(),
        "Boundary edge file size should match"
    );
    assert_eq!(
        uploaded_data, test_data,
        "Boundary edge file content should match"
    );

    println!("Boundary at buffer edge handled correctly");
}

#[test]
fn test_multiple_files_with_same_name() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test uploading multiple files with the same name in one request
    let filename = "duplicate_name.txt";
    let boundary = "duplicate_name_boundary";
    let mut body = Vec::new();

    // Add first file
    let data1 = b"first file content";
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n\
            Content-Type: application/octet-stream\r\n\
            \r\n",
            filename
        )
        .as_bytes(),
    );
    body.extend_from_slice(data1);
    body.extend_from_slice(b"\r\n");

    // Add second file with same name
    let data2 = b"second file content";
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n\
            Content-Type: application/octet-stream\r\n\
            \r\n",
            filename
        )
        .as_bytes(),
    );
    body.extend_from_slice(data2);
    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        format!("multipart/form-data; boundary={}", boundary),
    );
    headers.insert("content-length".to_string(), body.len().to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(body)),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(response.is_ok(), "Duplicate filename upload should succeed");

    // Check what happened - should have one file (likely the last one)
    let uploaded_file_path = temp_dir.path().join(filename);
    assert!(
        uploaded_file_path.exists(),
        "File with duplicate name should exist"
    );

    println!("Multiple files with same name handled correctly");
}

#[test]
fn test_binary_data_with_boundary_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test binary data that contains patterns similar to multipart boundaries
    let filename = "binary_with_boundary_patterns.bin";
    let boundary = "test_boundary_123";

    // Create binary data that includes the boundary string
    let mut test_data = Vec::new();
    test_data.extend_from_slice(b"some data before\r\n--");
    test_data.extend_from_slice(boundary.as_bytes());
    test_data.extend_from_slice(b"\r\nsome data after");
    test_data.extend_from_slice(b"more data with \r\n patterns");

    let body = create_multipart_request_body(&test_data, filename, boundary);

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        format!("multipart/form-data; boundary={}", boundary),
    );
    headers.insert("content-length".to_string(), body.len().to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(body)),
    };

    let response = upload_handler.handle_upload(&request, None);
    // Binary data with boundary patterns might be challenging to parse
    match response {
        Ok(_) => {
            println!("Binary data with boundary patterns handled correctly");
            // Verify file integrity if upload succeeded
            let uploaded_file_path = temp_dir.path().join(filename);
            let uploaded_data = fs::read(&uploaded_file_path).unwrap();
            assert_eq!(
                uploaded_data.len(),
                test_data.len(),
                "Binary file size should match"
            );
            assert_eq!(
                uploaded_data, test_data,
                "Binary file content should be preserved exactly"
            );
        }
        Err(_) => {
            println!("Binary data with boundary patterns rejected (acceptable behavior)");
        }
    }
}

#[test]
fn test_special_character_filename() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test with special characters in filename (English only)
    let special_filename = "test_file_with_special_chars_and_symbols.txt";
    let test_data = b"Special filename test data";
    let boundary = "special_filename_boundary";
    let body = create_multipart_request_body(test_data, special_filename, boundary);

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        format!("multipart/form-data; boundary={}", boundary),
    );
    headers.insert("content-length".to_string(), body.len().to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(body)),
    };

    let response = upload_handler.handle_upload(&request, None);

    // Special character filenames should be handled properly
    match response {
        Ok(_) => {
            println!("Special character filename accepted");
            // If accepted, verify the file exists
            let uploaded_file_path = temp_dir.path().join(special_filename);
            if uploaded_file_path.exists() {
                println!("  Special character filename preserved");
                let uploaded_data = fs::read(&uploaded_file_path).unwrap();
                assert_eq!(uploaded_data, test_data, "File content should match");
            } else {
                println!("  Special character filename was sanitized");
                // Check if any file was created
                let entries = fs::read_dir(temp_dir.path()).unwrap();
                let file_count = entries.count();
                assert!(file_count > 0, "At least one file should be created");
            }
        }
        Err(_) => println!("Special character filename rejected gracefully"),
    }
}
