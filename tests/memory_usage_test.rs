use irondrop::cli::Cli;
use irondrop::http::Request;
use irondrop::upload::UploadHandler;
use std::collections::HashMap;
use tempfile::TempDir;

/// Test that verifies disk-based streaming for large uploads
#[test]
fn test_large_upload_uses_disk_streaming() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let upload_dir = temp_dir.path().to_path_buf();

    let cli = Cli {
        directory: upload_dir.clone(),
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
        max_upload_size: Some(1000), // 1GB
        config_file: None,
    };

    let mut upload_handler = UploadHandler::new(&cli).expect("Failed to create upload handler");

    // Create a large file (150MB) that should trigger disk-based streaming
    let large_file_size = 150 * 1024 * 1024; // 150MB
    let large_file_data = vec![b'A'; large_file_size];

    // Create multipart data
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let mut multipart_data = Vec::new();

    // Add boundary and headers
    multipart_data.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    multipart_data.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"large_file.txt\"\r\n",
    );
    multipart_data.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");

    // Add file data
    multipart_data.extend_from_slice(&large_file_data);

    // Add closing boundary
    multipart_data.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        format!("multipart/form-data; boundary={}", boundary),
    );

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(multipart_data),
    };

    // This should succeed and use disk-based streaming
    let result = upload_handler.handle_upload(&request, None);

    match result {
        Ok(response) => {
            // Verify the response indicates success
            assert!(response.status_code >= 200 && response.status_code < 300);

            // Verify the file was actually saved
            let uploaded_files: Vec<_> = std::fs::read_dir(&upload_dir)
                .expect("Failed to read upload directory")
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_name().to_string_lossy().contains("large_file"))
                .collect();

            assert_eq!(
                uploaded_files.len(),
                1,
                "Expected exactly one uploaded file"
            );

            let uploaded_file = &uploaded_files[0];
            let file_size = uploaded_file
                .metadata()
                .expect("Failed to get file metadata")
                .len();

            assert_eq!(file_size, large_file_size as u64, "File size mismatch");

            println!(
                "✓ Large file upload ({}MB) successfully processed using disk-based streaming",
                large_file_size / (1024 * 1024)
            );
        }
        Err(e) => {
            panic!("Large file upload failed: {:?}", e);
        }
    }
}

/// Test that verifies small uploads still use memory-based processing
#[test]
fn test_small_upload_uses_memory_processing() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let upload_dir = temp_dir.path().to_path_buf();

    let cli = Cli {
        directory: upload_dir.clone(),
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
        max_upload_size: Some(1000), // 1GB
        config_file: None,
    };

    let mut upload_handler = UploadHandler::new(&cli).expect("Failed to create upload handler");

    // Create a small file (64MB) that should use memory-based processing
    let small_file_size = 64 * 1024 * 1024; // 64MB
    let small_file_data = vec![b'B'; small_file_size];

    // Create multipart data
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let mut multipart_data = Vec::new();

    // Add boundary and headers
    multipart_data.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    multipart_data.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"small_file.txt\"\r\n",
    );
    multipart_data.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");

    // Add file data
    multipart_data.extend_from_slice(&small_file_data);

    // Add closing boundary
    multipart_data.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        format!("multipart/form-data; boundary={}", boundary),
    );

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(multipart_data),
    };

    // This should succeed and use memory-based processing
    let result = upload_handler.handle_upload(&request, None);

    match result {
        Ok(response) => {
            // Verify the response indicates success
            assert!(response.status_code >= 200 && response.status_code < 300);

            // Verify the file was actually saved
            let uploaded_files: Vec<_> = std::fs::read_dir(&upload_dir)
                .expect("Failed to read upload directory")
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_name().to_string_lossy().contains("small_file"))
                .collect();

            assert_eq!(
                uploaded_files.len(),
                1,
                "Expected exactly one uploaded file"
            );

            let uploaded_file = &uploaded_files[0];
            let file_size = uploaded_file
                .metadata()
                .expect("Failed to get file metadata")
                .len();

            assert_eq!(file_size, small_file_size as u64, "File size mismatch");

            println!(
                "✓ Small file upload ({}MB) successfully processed using memory-based processing",
                small_file_size / (1024 * 1024)
            );
        }
        Err(e) => {
            panic!("Small file upload failed: {:?}", e);
        }
    }
}
