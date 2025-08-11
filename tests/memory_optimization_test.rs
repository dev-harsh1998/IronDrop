use irondrop::cli::Cli;
use irondrop::http::Request;
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
fn test_memory_efficient_small_files() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test multiple small files to verify buffer pooling
    let test_files = vec![
        (1024, "small1.txt"), // 1KB
        (2048, "small2.txt"), // 2KB
        (4096, "small3.txt"), // 4KB
        (512, "small4.txt"),  // 512B
    ];

    for (size, filename) in test_files {
        let test_data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let boundary = "memory_test_boundary";
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
            body: Some(body),
        };

        let response = upload_handler.handle_upload(&request, None);
        assert!(
            response.is_ok(),
            "Small file upload should succeed for {}",
            filename
        );

        // Verify file was written correctly
        let uploaded_file_path = temp_dir.path().join(filename);
        assert!(
            uploaded_file_path.exists(),
            "File {} should exist",
            filename
        );

        let uploaded_data = fs::read(&uploaded_file_path).unwrap();
        assert_eq!(
            uploaded_data.len(),
            size,
            "File {} size should match",
            filename
        );
        assert_eq!(
            uploaded_data, test_data,
            "File {} content should match",
            filename
        );
    }
}

#[test]
fn test_memory_efficient_medium_files() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test medium-sized files that would benefit from buffer optimization
    let test_files = vec![
        (64 * 1024, "medium1.bin"),   // 64KB - old buffer size
        (128 * 1024, "medium2.bin"),  // 128KB
        (256 * 1024, "medium3.bin"),  // 256KB
        (512 * 1024, "medium4.bin"),  // 512KB
        (1024 * 1024, "medium5.bin"), // 1MB
    ];

    for (size, filename) in test_files {
        println!("Testing medium file: {} ({} bytes)", filename, size);

        let test_data: Vec<u8> = (0..size).map(|i| ((i + size) % 256) as u8).collect();
        let boundary = "medium_test_boundary";
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
            body: Some(body),
        };

        let response = upload_handler.handle_upload(&request, None);
        assert!(
            response.is_ok(),
            "Medium file upload should succeed for {}",
            filename
        );

        // Verify file integrity
        let uploaded_file_path = temp_dir.path().join(filename);
        let uploaded_data = fs::read(&uploaded_file_path).unwrap();
        assert_eq!(
            uploaded_data.len(),
            size,
            "File {} size should match",
            filename
        );
        assert_eq!(
            uploaded_data, test_data,
            "File {} content should match",
            filename
        );

        println!("Medium file {} verified successfully", filename);
    }
}

#[test]
fn test_buffer_boundary_conditions() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test sizes around buffer boundaries to ensure proper handling
    let boundary_sizes = vec![
        8 * 1024 - 1, // Just under 8KB buffer
        8 * 1024,     // Exactly 8KB buffer
        8 * 1024 + 1, // Just over 8KB buffer
        16 * 1024,    // 2x buffer size
        24 * 1024,    // 3x buffer size
        2048 - 1,     // Just under multipart threshold
        2048,         // Exactly multipart threshold
        2048 + 1,     // Just over multipart threshold
    ];

    for (i, size) in boundary_sizes.iter().enumerate() {
        let filename = format!("boundary_{}.bin", size);
        println!("Testing boundary condition: {} bytes", size);

        let test_data: Vec<u8> = (0..*size).map(|j| ((j + i) % 256) as u8).collect();
        let boundary = "boundary_test";
        let body = create_multipart_request_body(&test_data, &filename, boundary);

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
            body: Some(body),
        };

        let response = upload_handler.handle_upload(&request, None);
        assert!(
            response.is_ok(),
            "Boundary test should succeed for {} bytes",
            size
        );

        // Verify exact content
        let uploaded_file_path = temp_dir.path().join(&filename);
        let uploaded_data = fs::read(&uploaded_file_path).unwrap();
        assert_eq!(
            uploaded_data.len(),
            *size,
            "Boundary test {} size should be exact",
            size
        );
        assert_eq!(
            uploaded_data, test_data,
            "Boundary test {} content should be exact",
            size
        );
    }
}

#[test]
fn test_multiple_files_single_request_memory_efficiency() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test multiple files in a single request to verify buffer pooling
    let files = vec![
        (50 * 1024, "multi1.bin"),  // 50KB
        (75 * 1024, "multi2.bin"),  // 75KB
        (100 * 1024, "multi3.bin"), // 100KB
        (25 * 1024, "multi4.bin"),  // 25KB
        (150 * 1024, "multi5.bin"), // 150KB
    ];

    let boundary = "multi_file_memory_test";
    let mut body = Vec::new();
    let mut expected_files = Vec::new();

    // Build multipart request with multiple files
    for (size, filename) in &files {
        let file_data: Vec<u8> = (0..*size)
            .map(|i| ((i + filename.len()) % 256) as u8)
            .collect();
        expected_files.push((*filename, file_data.clone()));

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
        body.extend_from_slice(&file_data);
        body.extend_from_slice(b"\r\n");
    }
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
        body: Some(body),
    };

    // Process the upload
    let response = upload_handler.handle_upload(&request, None);
    assert!(response.is_ok(), "Multiple file upload should succeed");

    // Verify all files were written correctly
    for (filename, expected_data) in expected_files {
        let uploaded_file_path = temp_dir.path().join(filename);
        assert!(
            uploaded_file_path.exists(),
            "File {} should exist",
            filename
        );

        let uploaded_data = fs::read(&uploaded_file_path).unwrap();
        assert_eq!(
            uploaded_data.len(),
            expected_data.len(),
            "File {} size should match",
            filename
        );
        assert_eq!(
            uploaded_data, expected_data,
            "File {} content should match",
            filename
        );
    }

    println!("Multiple files uploaded efficiently in single request");
}

#[test]
fn test_streaming_with_various_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test different data patterns that might affect streaming
    let test_patterns = vec![
        ("zeros", vec![0u8; 32 * 1024]),
        ("ones", vec![255u8; 32 * 1024]),
        (
            "alternating",
            (0..32 * 1024)
                .map(|i| if i % 2 == 0 { 0 } else { 255 })
                .collect(),
        ),
        (
            "sequential",
            (0..32 * 1024).map(|i| (i % 256) as u8).collect(),
        ),
        (
            "random_pattern",
            (0..32 * 1024)
                .map(|i| ((i * 17 + 42) % 256) as u8)
                .collect(),
        ),
    ];

    for (pattern_name, test_data) in test_patterns {
        let filename = format!("pattern_{}.bin", pattern_name);
        println!("Testing streaming pattern: {}", pattern_name);

        let boundary = "pattern_test_boundary";
        let body = create_multipart_request_body(&test_data, &filename, boundary);

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
            body: Some(body),
        };

        let response = upload_handler.handle_upload(&request, None);
        assert!(
            response.is_ok(),
            "Pattern {} upload should succeed",
            pattern_name
        );

        // Verify pattern integrity
        let uploaded_file_path = temp_dir.path().join(&filename);
        let uploaded_data = fs::read(&uploaded_file_path).unwrap();
        assert_eq!(
            uploaded_data.len(),
            test_data.len(),
            "Pattern {} size should match",
            pattern_name
        );
        assert_eq!(
            uploaded_data, test_data,
            "Pattern {} should be preserved exactly",
            pattern_name
        );

        println!("Pattern {} streamed correctly", pattern_name);
    }
}

#[test]
fn test_memory_cleanup_after_errors() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test that memory is properly cleaned up even when uploads fail
    let test_data = vec![0u8; 64 * 1024]; // 64KB

    // Test with invalid filename (should fail but not leak memory)
    let invalid_filename = "../../../etc/passwd";
    let boundary = "error_test_boundary";
    let body = create_multipart_request_body(&test_data, invalid_filename, boundary);

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
        body: Some(body),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(
        response.is_err(),
        "Invalid filename should cause upload to fail"
    );

    // After error, try a valid upload to ensure system is still working
    let valid_filename = "valid_after_error.bin";
    let body = create_multipart_request_body(&test_data, valid_filename, boundary);

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
        body: Some(body),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(response.is_ok(), "Valid upload after error should succeed");

    // Verify the valid file was uploaded correctly
    let uploaded_file_path = temp_dir.path().join(valid_filename);
    let uploaded_data = fs::read(&uploaded_file_path).unwrap();
    assert_eq!(
        uploaded_data, test_data,
        "File after error should be correct"
    );

    println!("Memory cleanup after errors verified");
}
