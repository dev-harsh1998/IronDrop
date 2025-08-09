#![allow(clippy::uninlined_format_args)]
#![allow(clippy::expect_fun_call)]

use irondrop::cli::Cli;
use irondrop::http::Request;
use irondrop::upload::UploadHandler;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
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
        max_upload_size: Some(2048), // 2GB limit for large file testing
        config_file: None,
    }
}

fn run_command(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute {}: {}", cmd, e))?;

    if !output.status.success() {
        return Err(format!(
            "Command failed: {} {}\nStderr: {}",
            cmd,
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
fn test_large_file_upload_with_bash_verification() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Create a unique filename for this test
    let test_file_path = temp_dir.path().join("large_test_file.bin");
    let filename = "large_test_file.bin";

    // Test file sizes
    let test_sizes = vec![
        100 * 1024 * 1024, // 100MB - Large but manageable for CI
                           // Note: 1GB+ files would be too large for most CI systems and could cause timeouts
                           // In production, you could uncomment the line below for local testing:
                           // 1024 * 1024 * 1024 + 512 * 1024, // 1GB + 512KB
    ];

    for size in test_sizes {
        println!("Testing large file upload: {} MB", size / (1024 * 1024));

        // Use fallocate to create a sparse file (much faster than dd)
        println!("Creating test file with fallocate...");
        let size_str = size.to_string();
        match run_command(
            "fallocate",
            &["-l", &size_str, test_file_path.to_str().unwrap()],
        ) {
            Ok(_) => println!("✅ Test file created successfully"),
            Err(e) => {
                // Fallback to dd if fallocate is not available
                println!("fallocate failed ({}), falling back to dd...", e);
                let bs = "1048576"; // 1MB blocks
                let count = (size / 1048576).to_string();
                match run_command(
                    "dd",
                    &[
                        "if=/dev/zero",
                        &format!("of={}", test_file_path.to_str().unwrap()),
                        &format!("bs={}", bs),
                        &format!("count={}", count),
                        "status=none",
                    ],
                ) {
                    Ok(_) => println!("✅ Test file created with dd"),
                    Err(e) => panic!("Failed to create test file: {}", e),
                }
            }
        }

        // Verify the file was created with correct size
        let actual_size = fs::metadata(&test_file_path)
            .expect("Should be able to read test file metadata")
            .len() as usize;
        assert_eq!(actual_size, size, "Created file should have correct size");

        // Calculate SHA1 checksum of original file
        println!("Calculating SHA1 checksum of original file...");
        let original_checksum = run_command("sha1sum", &[test_file_path.to_str().unwrap()])
            .expect("Should be able to calculate SHA1")
            .split_whitespace()
            .next()
            .unwrap()
            .to_string();
        println!("Original file SHA1: {}", original_checksum);

        // Read the file into memory for upload
        println!("Reading test file into memory for upload...");
        let file_data = fs::read(&test_file_path).expect("Should be able to read test file");
        assert_eq!(
            file_data.len(),
            size,
            "File data should match expected size"
        );

        // Create multipart request
        println!("Creating multipart request...");
        let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
        let body = create_multipart_request_body(&file_data, filename, boundary);

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
        println!("Processing upload...");
        let response = upload_handler.handle_upload(&request, None);
        match response {
            Ok(http_response) => {
                assert_eq!(http_response.status_code, 200, "Upload should succeed");

                // Verify the uploaded file exists
                let uploaded_file_path = temp_dir.path().join(filename);
                assert!(
                    uploaded_file_path.exists(),
                    "Uploaded file should exist on disk"
                );

                // Check uploaded file size with stat
                println!("Verifying uploaded file size...");
                let uploaded_size = fs::metadata(&uploaded_file_path)
                    .expect("Should be able to read uploaded file metadata")
                    .len() as usize;
                assert_eq!(
                    uploaded_size, size,
                    "Uploaded file size should match original"
                );

                // Calculate SHA1 checksum of uploaded file
                println!("Calculating SHA1 checksum of uploaded file...");
                let uploaded_checksum =
                    run_command("sha1sum", &[uploaded_file_path.to_str().unwrap()])
                        .expect("Should be able to calculate SHA1 of uploaded file")
                        .split_whitespace()
                        .next()
                        .unwrap()
                        .to_string();
                println!("Uploaded file SHA1: {}", uploaded_checksum);

                // Verify checksums match
                assert_eq!(
                    original_checksum, uploaded_checksum,
                    "SHA1 checksums should match - upload should be bit-perfect"
                );

                // Additional verification: byte-by-byte comparison using cmp
                println!("Performing byte-by-byte comparison...");
                match run_command(
                    "cmp",
                    &[
                        test_file_path.to_str().unwrap(),
                        uploaded_file_path.to_str().unwrap(),
                    ],
                ) {
                    Ok(_) => println!("✅ Files are identical (cmp verification passed)"),
                    Err(e) => panic!("Files differ: {}", e),
                }

                println!(
                    "✅ Large file upload test PASSED for {} MB file",
                    size / (1024 * 1024)
                );
                println!("   - Size verification: ✅");
                println!("   - SHA1 verification: ✅");
                println!("   - Byte comparison: ✅");
            }
            Err(e) => panic!(
                "Upload failed for {} MB file: {:?}",
                size / (1024 * 1024),
                e
            ),
        }

        // Clean up the test file
        fs::remove_file(&test_file_path).ok();
    }
}

#[test]
fn test_very_large_file_upload_1gb_plus() {
    // This test is designed for local testing with large files
    // It's marked with #[ignore] by default to prevent CI timeouts
    // Run with: cargo test test_very_large_file_upload_1gb_plus -- --ignored --nocapture

    if std::env::var("ENABLE_1GB_TEST").is_err() {
        println!("Skipping 1GB+ test (set ENABLE_1GB_TEST=1 to enable)");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    let test_file_path = temp_dir.path().join("very_large_test.bin");
    let filename = "very_large_test.bin";
    let size = 1024 * 1024 * 1024 + 512 * 1024; // 1GB + 512KB

    println!(
        "Testing VERY large file upload: {} MB",
        size / (1024 * 1024)
    );
    println!("This may take several minutes...");

    // Create large file with fallocate
    println!("Creating 1GB+ test file...");
    let size_str = size.to_string();
    run_command(
        "fallocate",
        &["-l", &size_str, test_file_path.to_str().unwrap()],
    )
    .expect("Should create large test file");

    // Fill with some pattern to make it non-sparse
    println!("Writing pattern to file (this may take a while)...");
    run_command(
        "dd",
        &[
            "if=/dev/urandom",
            &format!("of={}", test_file_path.to_str().unwrap()),
            "bs=1048576", // 1MB blocks
            &format!("count={}", size / 1048576),
            "status=progress",
        ],
    )
    .expect("Should write pattern to file");

    // Calculate original checksum
    println!("Calculating SHA1 checksum (this will take a while)...");
    let original_checksum = run_command("sha1sum", &[test_file_path.to_str().unwrap()])
        .expect("Should calculate SHA1")
        .split_whitespace()
        .next()
        .unwrap()
        .to_string();
    println!("Original SHA1: {}", original_checksum);

    // Read file for upload (this will consume a lot of RAM)
    println!("Reading file into memory for upload...");
    let file_data = fs::read(&test_file_path).expect("Should read large file");

    // Create and process upload
    println!("Creating multipart request...");
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = create_multipart_request_body(&file_data, filename, boundary);

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

    println!("Processing 1GB+ upload...");
    let response = upload_handler.handle_upload(&request, None);
    match response {
        Ok(http_response) => {
            assert_eq!(
                http_response.status_code, 200,
                "Large upload should succeed"
            );

            let uploaded_file_path = temp_dir.path().join(filename);

            // Verify size
            let uploaded_size = fs::metadata(&uploaded_file_path)
                .expect("Should read uploaded file metadata")
                .len() as usize;
            assert_eq!(uploaded_size, size, "Uploaded size should match");

            // Verify checksum
            println!("Verifying uploaded file checksum...");
            let uploaded_checksum = run_command("sha1sum", &[uploaded_file_path.to_str().unwrap()])
                .expect("Should calculate uploaded file SHA1")
                .split_whitespace()
                .next()
                .unwrap()
                .to_string();

            assert_eq!(
                original_checksum, uploaded_checksum,
                "1GB+ file upload should be bit-perfect"
            );

            println!("🎉 1GB+ file upload test PASSED!");
            println!("   - File size: {} MB", size / (1024 * 1024));
            println!("   - SHA1 match: ✅");
        }
        Err(e) => panic!("1GB+ upload failed: {:?}", e),
    }
}

#[test]
fn test_multiple_large_files_bash_verification() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Create multiple test files with different sizes
    let test_files = vec![
        (10 * 1024 * 1024, "file1.bin"), // 10MB
        (25 * 1024 * 1024, "file2.bin"), // 25MB
        (50 * 1024 * 1024, "file3.bin"), // 50MB
    ];

    let mut original_checksums = HashMap::new();
    let mut file_data_map = HashMap::new();

    // Create all test files and calculate their checksums
    for (size, filename) in &test_files {
        let test_file_path = temp_dir.path().join(format!("orig_{}", filename));

        println!("Creating {} ({} MB)...", filename, size / (1024 * 1024));

        // Create file with dd (more reliable than fallocate for multiple files)
        run_command(
            "dd",
            &[
                "if=/dev/urandom",
                &format!("of={}", test_file_path.to_str().unwrap()),
                "bs=1048576",
                &format!("count={}", size / 1048576),
                "status=none",
            ],
        )
        .expect("Should create test file");

        // Calculate checksum
        let checksum = run_command("sha1sum", &[test_file_path.to_str().unwrap()])
            .expect("Should calculate SHA1")
            .split_whitespace()
            .next()
            .unwrap()
            .to_string();

        original_checksums.insert(*filename, checksum);

        // Read file data for upload
        let file_data = fs::read(&test_file_path).expect("Should read test file");
        file_data_map.insert(*filename, file_data);

        // Clean up original file
        fs::remove_file(&test_file_path).ok();
    }

    // Create multipart request with all files
    let boundary = "multifile_bash_test_boundary";
    let mut body = Vec::new();

    for (_, filename) in &test_files {
        let file_data = &file_data_map[filename];

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
        body.extend_from_slice(file_data);
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

    // Process the multi-file upload
    println!("Processing multi-file upload...");
    let response = upload_handler.handle_upload(&request, None);
    match response {
        Ok(http_response) => {
            assert_eq!(
                http_response.status_code, 200,
                "Multi-file upload should succeed"
            );

            // Verify each uploaded file
            for (size, filename) in &test_files {
                let uploaded_file_path = temp_dir.path().join(filename);
                assert!(
                    uploaded_file_path.exists(),
                    "File {} should exist",
                    filename
                );

                // Verify size
                let uploaded_size = fs::metadata(&uploaded_file_path)
                    .expect("Should read uploaded file metadata")
                    .len() as usize;
                assert_eq!(uploaded_size, *size, "File {} size should match", filename);

                // Verify checksum
                let uploaded_checksum =
                    run_command("sha1sum", &[uploaded_file_path.to_str().unwrap()])
                        .expect("Should calculate uploaded file SHA1")
                        .split_whitespace()
                        .next()
                        .unwrap()
                        .to_string();

                let original_checksum = &original_checksums[filename];
                assert_eq!(
                    uploaded_checksum, *original_checksum,
                    "File {} checksum should match",
                    filename
                );

                println!("✅ {} verified ({} MB)", filename, size / (1024 * 1024));
            }

            println!("🎉 Multi-file upload with bash verification PASSED!");
        }
        Err(e) => panic!("Multi-file upload failed: {:?}", e),
    }
}
