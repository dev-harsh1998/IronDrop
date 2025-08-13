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
