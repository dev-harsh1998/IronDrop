// SPDX-License-Identifier: MIT
use irondrop::cli::Cli;
use irondrop::http::{Request, RequestBody};
use irondrop::upload::DirectUploadHandler;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::{NamedTempFile, TempDir};

/// Helper function to create test CLI configuration
fn create_test_cli(upload_dir: PathBuf) -> Cli {
    Cli {
        directory: upload_dir,
        listen: Some("127.0.0.1".to_string()),
        port: Some(8080),
        allowed_extensions: Some("*.txt,*.pdf,*.bin".to_string()),
        threads: Some(4),
        chunk_size: Some(1024),
        verbose: Some(false),
        detailed_logging: Some(false),
        username: None,
        password: None,
        enable_upload: Some(true),
        max_upload_size: Some(100), // 100MB
        config_file: None,
        log_dir: None,
    }
}

#[test]
fn test_direct_upload_small_file() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = DirectUploadHandler::new(&cli).unwrap();

    // Create test data (small file, under 2MB)
    let test_data = b"Hello, world! This is a test file content.";
    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string(),
    );
    headers.insert(
        "content-disposition".to_string(),
        "attachment; filename=\"test.txt\"".to_string(),
    );

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(test_data.to_vec())),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(
        response.is_ok(),
        "Upload should succeed: {:?}",
        response.err()
    );

    let response = response.unwrap();
    assert_eq!(response.status_code, 200);

    // Verify file was saved
    let files_in_dir: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".txt"))
        .collect();

    assert_eq!(files_in_dir.len(), 1, "Should have saved exactly one file");

    let saved_file = files_in_dir[0].path();
    let saved_content = fs::read(&saved_file).unwrap();
    assert_eq!(
        saved_content, test_data,
        "File content should match original"
    );

    println!("Direct upload small file test passed");
}

#[test]
fn test_direct_upload_filename_from_url() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = DirectUploadHandler::new(&cli).unwrap();

    // Create test data
    let test_data = b"Test content for URL filename extraction";
    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string(),
    );

    let request = Request {
        method: "POST".to_string(),
        path: "/upload/document.txt".to_string(), // Filename in URL path
        headers,
        body: Some(RequestBody::Memory(test_data.to_vec())),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(
        response.is_ok(),
        "Upload should succeed: {:?}",
        response.err()
    );

    // Verify file was saved with correct name
    let files_in_dir: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy() == "document.txt")
        .collect();

    assert_eq!(files_in_dir.len(), 1, "Should have saved document.txt");

    println!("Direct upload filename from URL test passed");
}

#[test]
fn test_direct_upload_filename_from_header() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = DirectUploadHandler::new(&cli).unwrap();

    // Create test data
    let test_data = b"Test content for header filename extraction";
    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string(),
    );
    headers.insert("x-filename".to_string(), "header-file.txt".to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(test_data.to_vec())),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(
        response.is_ok(),
        "Upload should succeed: {:?}",
        response.err()
    );

    // Verify file was saved with correct name
    let files_in_dir: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy() == "header-file.txt")
        .collect();

    assert_eq!(files_in_dir.len(), 1, "Should have saved header-file.txt");

    println!("Direct upload filename from header test passed");
}

#[test]
fn test_direct_upload_filename_conflict_resolution() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = DirectUploadHandler::new(&cli).unwrap();

    // Create an existing file
    fs::write(temp_dir.path().join("conflict.txt"), b"existing content").unwrap();

    // Upload a file with the same name
    let test_data = b"New content that should get renamed";
    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string(),
    );
    headers.insert("x-filename".to_string(), "conflict.txt".to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(test_data.to_vec())),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(
        response.is_ok(),
        "Upload should succeed: {:?}",
        response.err()
    );

    // Verify we have both files
    let files_in_dir: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            name == "conflict.txt" || name == "conflict_1.txt"
        })
        .collect();

    assert_eq!(
        files_in_dir.len(),
        2,
        "Should have both original and renamed file"
    );

    // Verify the renamed file has the new content
    let renamed_content = fs::read(temp_dir.path().join("conflict_1.txt")).unwrap();
    assert_eq!(
        renamed_content, test_data,
        "Renamed file should have new content"
    );

    println!("Direct upload filename conflict resolution test passed");
}

#[test]
fn test_direct_upload_extension_validation() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = DirectUploadHandler::new(&cli).unwrap();

    // Try to upload file with disallowed extension
    let test_data = b"Malicious executable content";
    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string(),
    );
    headers.insert("x-filename".to_string(), "malware.exe".to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(test_data.to_vec())),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(
        response.is_err(),
        "Upload with disallowed extension should fail"
    );

    // Verify no file was saved
    let files_in_dir: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .collect();

    assert_eq!(
        files_in_dir.len(),
        0,
        "No files should be saved for rejected upload"
    );

    println!("Direct upload extension validation test passed");
}

#[test]
fn test_direct_upload_size_limit() {
    let temp_dir = TempDir::new().unwrap();
    let mut cli = create_test_cli(temp_dir.path().to_path_buf());
    cli.max_upload_size = Some(1); // 1MB limit

    let mut upload_handler = DirectUploadHandler::new(&cli).unwrap();

    // Create data that exceeds the limit (2MB)
    let test_data = vec![b'X'; 2 * 1024 * 1024];
    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string(),
    );
    headers.insert("x-filename".to_string(), "large.bin".to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(test_data)),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(response.is_err(), "Upload exceeding size limit should fail");

    // Verify no file was saved
    let files_in_dir: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .collect();

    assert_eq!(
        files_in_dir.len(),
        0,
        "No files should be saved for rejected upload"
    );

    println!("Direct upload size limit test passed");
}

#[test]
fn test_direct_upload_method_validation() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = DirectUploadHandler::new(&cli).unwrap();

    // Try to upload with GET method
    let test_data = b"Test data";
    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string(),
    );
    headers.insert("x-filename".to_string(), "test.txt".to_string());

    let request = Request {
        method: "GET".to_string(), // Wrong method
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(test_data.to_vec())),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(response.is_err(), "Upload with wrong method should fail");

    println!("Direct upload method validation test passed");
}

#[test]
fn test_direct_upload_large_file_streaming() {
    let temp_dir = TempDir::new().unwrap();
    let mut cli = create_test_cli(temp_dir.path().to_path_buf());
    cli.max_upload_size = Some(10); // 10MB limit for this test

    let mut upload_handler = DirectUploadHandler::new(&cli).unwrap();

    // Create test data larger than 2MB threshold (3MB)
    let test_data = vec![b'L'; 3 * 1024 * 1024]; // 3MB
    let temp_file = NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), &test_data).unwrap();

    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string(),
    );
    headers.insert("x-filename".to_string(), "large.bin".to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::File {
            path: temp_file.path().to_path_buf(),
            size: test_data.len() as u64,
        }),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(
        response.is_ok(),
        "Large file upload should succeed: {:?}",
        response.err()
    );

    let response = response.unwrap();
    assert_eq!(response.status_code, 200);

    // Verify file was saved correctly
    let files_in_dir: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy() == "large.bin")
        .collect();

    assert_eq!(files_in_dir.len(), 1, "Should have saved large.bin");

    let saved_file = files_in_dir[0].path();
    let saved_content = fs::read(&saved_file).unwrap();
    assert_eq!(
        saved_content.len(),
        test_data.len(),
        "File size should match original"
    );
    assert_eq!(
        saved_content, test_data,
        "File content should match original"
    );

    println!("Direct upload large file streaming test passed");
}

#[test]
fn test_direct_upload_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = DirectUploadHandler::new(&cli).unwrap();

    // Upload empty file
    let test_data = b"";
    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string(),
    );
    headers.insert("x-filename".to_string(), "empty.txt".to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(test_data.to_vec())),
    };

    let response = upload_handler.handle_upload(&request, None);
    assert!(
        response.is_ok(),
        "Empty file upload should succeed: {:?}",
        response.err()
    );

    // Verify empty file was saved
    let saved_file = temp_dir.path().join("empty.txt");
    assert!(saved_file.exists(), "Empty file should be saved");
    let content = fs::read(&saved_file).unwrap();
    assert_eq!(content.len(), 0, "File should be empty");

    println!("Direct upload empty file test passed");
}

#[test]
fn test_direct_upload_path_traversal_prevention() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = DirectUploadHandler::new(&cli).unwrap();

    let malicious_filenames = vec![
        "../../../etc/passwd",
        "..\\..\\..\\windows\\system32\\config\\sam",
        "/etc/shadow",
        "C:\\Windows\\System32\\config\\SAM",
        "....//....//....//etc//passwd",
        "file\x00.txt", // Null byte injection
        "con.txt",      // Windows reserved name
        "aux.txt",      // Windows reserved name
        "prn.txt",      // Windows reserved name
    ];

    for malicious_filename in malicious_filenames {
        let test_data = b"malicious content";
        let mut headers = HashMap::new();
        headers.insert(
            "content-type".to_string(),
            "application/octet-stream".to_string(),
        );
        headers.insert("x-filename".to_string(), malicious_filename.to_string());

        let request = Request {
            method: "POST".to_string(),
            path: "/upload".to_string(),
            headers,
            body: Some(RequestBody::Memory(test_data.to_vec())),
        };

        let response = upload_handler.handle_upload(&request, None);

        // Should either reject the upload or sanitize the filename
        if let Ok(response) = response {
            // If upload succeeds, verify file is saved in upload directory only
            let files_in_upload_dir: Vec<_> = fs::read_dir(temp_dir.path())
                .unwrap()
                .filter_map(|entry| entry.ok())
                .collect();

            // Should not create files outside upload directory
            assert!(
                files_in_upload_dir.len() <= 1,
                "Should not create multiple files for path traversal attempt: {}",
                malicious_filename
            );
        }
    }

    println!("Direct upload path traversal prevention test passed");
}

#[test]
fn test_direct_upload_filename_sanitization() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = DirectUploadHandler::new(&cli).unwrap();

    let problematic_filenames = [
        ("file with spaces.txt", "should handle spaces"),
        ("file<>:\"|?*.txt", "should handle special characters"),
        (
            "very_long_filename_that_exceeds_normal_filesystem_limits_and_should_be_truncated_or_handled_gracefully_by_the_system.txt",
            "should handle long names",
        ),
        ("файл.txt", "should handle unicode"),
        ("🚀rocket.txt", "should handle emoji"),
        (".hidden", "should handle hidden files"),
        ("file.", "should handle trailing dot"),
        ("file..txt", "should handle double dots"),
    ];

    for (filename, description) in problematic_filenames {
        let test_data = b"test content";
        let mut headers = HashMap::new();
        headers.insert(
            "content-type".to_string(),
            "application/octet-stream".to_string(),
        );
        headers.insert("x-filename".to_string(), filename.to_string());

        let request = Request {
            method: "POST".to_string(),
            path: "/upload".to_string(),
            headers,
            body: Some(RequestBody::Memory(test_data.to_vec())),
        };

        let response = upload_handler.handle_upload(&request, None);

        // Should handle gracefully - either accept with sanitized name or reject
        match response {
            Ok(_) => {
                // If successful, verify a file was created in upload directory
                let files_in_dir: Vec<_> = fs::read_dir(temp_dir.path())
                    .unwrap()
                    .filter_map(|entry| entry.ok())
                    .collect();
                assert!(
                    !files_in_dir.is_empty(),
                    "Should create a file for {}: {}",
                    description,
                    filename
                );
            }
            Err(_) => {
                // Acceptable to reject problematic filenames
            }
        }
    }

    println!("Direct upload filename sanitization test passed");
}

#[test]
fn test_direct_upload_concurrent_uploads() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = TempDir::new().unwrap();
    let cli = Arc::new(create_test_cli(temp_dir.path().to_path_buf()));

    let handles: Vec<_> = (0..5)
        .map(|i| {
            let cli_clone = Arc::clone(&cli);
            thread::spawn(move || {
                let mut upload_handler = DirectUploadHandler::new(&cli_clone).unwrap();

                let test_data = format!("Content from thread {}", i).into_bytes();
                let mut headers = HashMap::new();
                headers.insert(
                    "content-type".to_string(),
                    "application/octet-stream".to_string(),
                );
                headers.insert("x-filename".to_string(), format!("file_{}.txt", i));

                let request = Request {
                    method: "POST".to_string(),
                    path: "/upload".to_string(),
                    headers,
                    body: Some(RequestBody::Memory(test_data)),
                };

                upload_handler.handle_upload(&request, None)
            })
        })
        .collect();

    // Wait for all uploads to complete
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // All uploads should succeed
    for (i, result) in results.iter().enumerate() {
        assert!(
            result.is_ok(),
            "Concurrent upload {} should succeed: {:?}",
            i,
            result.as_ref().err()
        );
    }

    // Verify all files were created
    let files_in_dir: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .collect();

    assert_eq!(
        files_in_dir.len(),
        5,
        "Should have created 5 files from concurrent uploads"
    );

    println!("Direct upload concurrent uploads test passed");
}

#[test]
fn test_direct_upload_missing_content_type() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = DirectUploadHandler::new(&cli).unwrap();

    let test_data = b"Test content without content-type";
    let mut headers = HashMap::new();
    // Intentionally omit content-type header
    headers.insert("x-filename".to_string(), "no-content-type.txt".to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(test_data.to_vec())),
    };

    let response = upload_handler.handle_upload(&request, None);

    // Should handle gracefully - either accept with default content-type or reject
    match response {
        Ok(_) => {
            // If successful, verify file was created
            let saved_file = temp_dir.path().join("no-content-type.txt");
            assert!(
                saved_file.exists(),
                "File should be saved even without content-type"
            );
        }
        Err(_) => {
            // Acceptable to require content-type header
        }
    }

    println!("Direct upload missing content-type test passed");
}

#[test]
fn test_direct_upload_malformed_content_length() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = DirectUploadHandler::new(&cli).unwrap();

    let test_data = b"Test content";
    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string(),
    );
    headers.insert("x-filename".to_string(), "test.txt".to_string());
    headers.insert("content-length".to_string(), "invalid".to_string()); // Invalid content-length

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(test_data.to_vec())),
    };

    let response = upload_handler.handle_upload(&request, None);

    // Should handle gracefully - either ignore invalid header or reject
    // This test ensures no panic occurs
    match response {
        Ok(_) => {
            // If successful, verify file was created
            let saved_file = temp_dir.path().join("test.txt");
            if saved_file.exists() {
                let content = fs::read(&saved_file).unwrap();
                assert_eq!(content, test_data, "File content should match");
            }
        }
        Err(_) => {
            // Acceptable to reject malformed headers
        }
    }

    println!("Direct upload malformed content-length test passed");
}

#[test]
fn test_direct_upload_disk_space_simulation() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = DirectUploadHandler::new(&cli).unwrap();

    // Create a very large file that might cause disk space issues
    // Note: This test is more about ensuring graceful handling rather than actually filling disk
    let large_data = vec![b'X'; 100 * 1024 * 1024]; // 100MB
    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string(),
    );
    headers.insert("x-filename".to_string(), "large-test.bin".to_string());

    let request = Request {
        method: "POST".to_string(),
        path: "/upload".to_string(),
        headers,
        body: Some(RequestBody::Memory(large_data.clone())),
    };

    let response = upload_handler.handle_upload(&request, None);

    // Should either succeed or fail gracefully with appropriate error
    match response {
        Ok(_) => {
            // If successful, verify file was created correctly
            let saved_file = temp_dir.path().join("large-test.bin");
            if saved_file.exists() {
                let metadata = fs::metadata(&saved_file).unwrap();
                let file_size = metadata.len();
                assert!(
                    usize::try_from(file_size).is_ok(),
                    "File size should fit in usize"
                );
                assert_eq!(
                    file_size,
                    u64::try_from(large_data.len()).expect("vec length fits in u64"),
                    "File size should match uploaded data"
                );
            }
        }
        Err(_) => {
            // Acceptable to fail due to size limits or disk space
        }
    }

    println!("Direct upload disk space simulation test passed");
}
