#![allow(clippy::uninlined_format_args)]
#![allow(clippy::expect_fun_call)]

use irondrop::cli::Cli;
use irondrop::http::Request;
use irondrop::upload::UploadHandler;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_cli(upload_dir: PathBuf) -> Cli {
    Cli {
        directory: upload_dir,
        listen: Some("127.0.0.1".to_string()),
        port: Some(8080),
        allowed_extensions: Some("*".to_string()), // Allow all files for testing
        threads: Some(4),
        chunk_size: Some(1024),
        verbose: Some(false),
        detailed_logging: Some(false),
        username: None,
        password: None,
        enable_upload: Some(true),
        max_upload_size: Some(10), // 10MB
        config_file: None,
    }
}

fn create_multipart_request_body(file_data: &[u8], filename: &str, boundary: &str) -> Vec<u8> {
    let header = format!(
        "--{}\r\n\
        Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n\
        Content-Type: application/octet-stream\r\n\
        \r\n",
        boundary, filename
    );
    let footer = format!("\r\n--{}--\r\n", boundary);

    let mut body = Vec::new();
    body.extend_from_slice(header.as_bytes());
    body.extend_from_slice(file_data);
    body.extend_from_slice(footer.as_bytes());
    body
}

#[test]
fn test_realistic_large_file_upload() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test sizes that previously failed
    let test_cases = vec![
        (404 * 1024, "404kb_file.bin"),
        (500 * 1024, "500kb_file.bin"),
        (1024 * 1024, "1mb_file.bin"),
        (2 * 1024 * 1024, "2mb_file.bin"),
    ];

    for (size, filename) in test_cases {
        println!(
            "Testing realistic upload of {} bytes file: {}",
            size, filename
        );

        // Create test data with a verifiable pattern
        let test_data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

        // Create multipart request
        let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
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

        // Process the upload
        let response = upload_handler.handle_upload(&request, None);
        match response {
            Ok(http_response) => {
                assert_eq!(http_response.status_code, 200, "Upload should succeed");

                // Verify the file was written correctly to disk
                let uploaded_file_path = temp_dir.path().join(filename);
                assert!(
                    uploaded_file_path.exists(),
                    "Uploaded file should exist on disk"
                );

                let uploaded_data =
                    fs::read(&uploaded_file_path).expect("Should be able to read uploaded file");

                assert_eq!(
                    uploaded_data.len(),
                    size,
                    "Uploaded file size should match original for {}",
                    filename
                );

                assert_eq!(
                    uploaded_data, test_data,
                    "Uploaded file content should match original for {}",
                    filename
                );

                println!(
                    "✅ Successfully uploaded and verified {} ({} bytes)",
                    filename, size
                );
            }
            Err(e) => panic!("Upload failed for {}: {:?}", filename, e),
        }
    }
}

#[test]
fn test_realistic_binary_file_upload() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Create binary data that includes all byte values including null bytes
    let mut binary_data = Vec::new();
    for _ in 0..2000 {
        // 2000 repetitions = 512KB of data
        for byte_val in 0..=255u8 {
            binary_data.push(byte_val);
        }
    }

    let filename = "binary_test.bin";
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = create_multipart_request_body(&binary_data, filename, boundary);

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
    match response {
        Ok(http_response) => {
            assert_eq!(
                http_response.status_code, 200,
                "Binary upload should succeed"
            );

            // Verify the binary file was written correctly
            let uploaded_file_path = temp_dir.path().join(filename);
            let uploaded_data =
                fs::read(&uploaded_file_path).expect("Should be able to read uploaded binary file");

            assert_eq!(
                uploaded_data.len(),
                binary_data.len(),
                "Binary file size should match"
            );

            assert_eq!(
                uploaded_data, binary_data,
                "Binary file content should be exactly preserved"
            );

            println!(
                "✅ Successfully uploaded and verified binary file ({} bytes)",
                binary_data.len()
            );
        }
        Err(e) => panic!("Binary upload failed: {:?}", e),
    }
}

#[test]
fn test_realistic_multiple_large_files() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Create multiple files with different sizes
    let files = vec![
        (300 * 1024, "file1.bin"),
        (500 * 1024, "file2.bin"),
        (800 * 1024, "file3.bin"),
    ];

    let boundary = "multifile_test_boundary";
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
    match response {
        Ok(http_response) => {
            assert_eq!(
                http_response.status_code, 200,
                "Multiple file upload should succeed"
            );

            // Verify all files were written correctly
            for (filename, expected_data) in expected_files {
                let uploaded_file_path = temp_dir.path().join(filename);
                assert!(
                    uploaded_file_path.exists(),
                    "File {} should exist",
                    filename
                );

                let uploaded_data =
                    fs::read(&uploaded_file_path).expect(&format!("Should read {}", filename));

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

                println!("✅ Verified {} ({} bytes)", filename, uploaded_data.len());
            }

            println!("✅ All multiple files uploaded and verified successfully");
        }
        Err(e) => panic!("Multiple file upload failed: {:?}", e),
    }
}

#[test]
fn test_exact_boundary_cases() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test exact sizes around the problematic boundary
    let boundary_test_sizes = vec![
        403 * 1024,       // 403KB - previously worked
        403 * 1024 + 512, // 403.5KB
        404 * 1024,       // 404KB - previously failed
        404 * 1024 + 1,   // 404KB + 1 byte
        413543,           // Exact previous truncation point
        413544,           // One byte past truncation
        500 * 1024,       // 500KB
    ];

    for size in boundary_test_sizes {
        let filename = format!("boundary_test_{}.bin", size);
        println!("Testing boundary case: {} bytes", size);

        // Create test data
        let test_data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

        let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
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
        match response {
            Ok(http_response) => {
                assert_eq!(
                    http_response.status_code, 200,
                    "Boundary test {} should succeed",
                    size
                );

                let uploaded_file_path = temp_dir.path().join(&filename);
                let uploaded_data = fs::read(&uploaded_file_path)
                    .expect(&format!("Should read boundary test file {}", filename));

                assert_eq!(
                    uploaded_data.len(),
                    size,
                    "Boundary test {} size should be exact",
                    size
                );

                assert_eq!(
                    uploaded_data, test_data,
                    "Boundary test {} content should be exact",
                    size
                );

                println!("✅ Boundary case {} bytes passed", size);
            }
            Err(e) => panic!("Boundary test {} failed: {:?}", size, e),
        }
    }
}
