use irondrop::cli::Cli;
use irondrop::http::Request;
use irondrop::upload::UploadHandler;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
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
fn test_stress_many_small_files() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Stress test with many small files to verify buffer pooling efficiency
    let file_count = 500;
    let file_size = 2048; // 2KB each
    let total_data_mb = (file_count * file_size) as f64 / (1024.0 * 1024.0);

    println!(
        "Starting stress test: {} files, {:.2} MB total",
        file_count, total_data_mb
    );
    let start_time = Instant::now();

    for i in 0..file_count {
        let filename = format!("stress_small_{:04}.bin", i);
        let test_data: Vec<u8> = (0..file_size)
            .map(|j| ((i * 17 + j * 23) % 256) as u8)
            .collect();

        let boundary = "stress_test_boundary";
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
        assert!(response.is_ok(), "Stress test file {} should succeed", i);

        // Periodic verification (every 100 files)
        if i % 100 == 0 {
            let uploaded_file_path = temp_dir.path().join(&filename);
            assert!(
                uploaded_file_path.exists(),
                "File {} should exist",
                filename
            );
            let uploaded_data = fs::read(&uploaded_file_path).unwrap();
            assert_eq!(
                uploaded_data.len(),
                file_size,
                "File {} size should match",
                filename
            );
            println!("  Processed {} files...", i + 1);
        }
    }

    let total_duration = start_time.elapsed();
    let throughput = total_data_mb / total_duration.as_secs_f64();
    let avg_time_per_file = total_duration.as_millis() / file_count as u128;

    println!("Stress test completed:");
    println!(
        "   {} files in {} ms",
        file_count,
        total_duration.as_millis()
    );
    println!("   Average: {} ms per file", avg_time_per_file);
    println!("   Throughput: {:.2} MB/s", throughput);

    // Performance assertions - adjusted for small file overhead
    assert!(
        avg_time_per_file < 50,
        "Average time per small file should be under 50ms"
    );
    assert!(throughput > 0.1, "Throughput should be at least 0.1 MB/s");
}

#[test]
fn test_stress_mixed_file_sizes() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Stress test with mixed file sizes to test buffer adaptation
    let file_configs = vec![
        (1024, 50, "1KB"),    // 50 x 1KB files
        (8192, 30, "8KB"),    // 30 x 8KB files (buffer size)
        (16384, 20, "16KB"),  // 20 x 16KB files
        (65536, 10, "64KB"),  // 10 x 64KB files
        (262144, 5, "256KB"), // 5 x 256KB files
    ];

    let mut total_files = 0;
    let mut total_data_mb = 0.0;
    let start_time = Instant::now();

    for (file_size, count, size_name) in file_configs {
        println!("Processing {} files of size {}...", count, size_name);

        for i in 0..count {
            let filename = format!("stress_mixed_{}_{:03}.bin", size_name, i);
            let test_data: Vec<u8> = (0..file_size)
                .map(|j| ((i * file_size + j) % 256) as u8)
                .collect();

            let boundary = "stress_mixed_boundary";
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
                "Mixed stress test file {} should succeed",
                filename
            );

            total_files += 1;
            total_data_mb += file_size as f64 / (1024.0 * 1024.0);
        }
    }

    let total_duration = start_time.elapsed();
    let throughput = total_data_mb / total_duration.as_secs_f64();

    println!("Mixed size stress test completed:");
    println!(
        "   {} files, {:.2} MB total in {} ms",
        total_files,
        total_data_mb,
        total_duration.as_millis()
    );
    println!("   Throughput: {:.2} MB/s", throughput);

    // Verify a sample of files
    let sample_files = vec![
        "stress_mixed_1KB_000.bin",
        "stress_mixed_8KB_015.bin",
        "stress_mixed_64KB_005.bin",
        "stress_mixed_256KB_002.bin",
    ];

    for filename in sample_files {
        let uploaded_file_path = temp_dir.path().join(filename);
        assert!(
            uploaded_file_path.exists(),
            "Sample file {} should exist",
            filename
        );
        let uploaded_data = fs::read(&uploaded_file_path).unwrap();
        assert!(
            uploaded_data.len() > 0,
            "Sample file {} should have content",
            filename
        );
    }

    assert!(
        throughput > 1.5,
        "Mixed size throughput should be at least 1.5 MB/s"
    );
}

#[test]
fn test_stress_large_files_sequential() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Stress test with several large files to test memory efficiency
    let large_file_sizes = vec![
        (2 * 1024 * 1024, "2MB"), // 2MB
        (5 * 1024 * 1024, "5MB"), // 5MB
        (8 * 1024 * 1024, "8MB"), // 8MB
        (3 * 1024 * 1024, "3MB"), // 3MB (different order)
        (1 * 1024 * 1024, "1MB"), // 1MB
    ];

    let mut total_data_mb = 0.0;
    let start_time = Instant::now();

    for (file_size, size_name) in large_file_sizes {
        println!("Processing large file: {}...", size_name);
        let filename = format!("stress_large_{}.bin", size_name);

        // Create test data with a pattern that's easy to verify
        let test_data: Vec<u8> = (0..file_size)
            .map(|i| ((i / 1024) % 256) as u8) // Changes every KB
            .collect();

        let boundary = "stress_large_boundary";
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

        let file_start = Instant::now();
        let response = upload_handler.handle_upload(&request, None);
        let file_duration = file_start.elapsed();

        assert!(response.is_ok(), "Large file {} should succeed", size_name);

        // Verify file integrity
        let uploaded_file_path = temp_dir.path().join(&filename);
        let uploaded_data = fs::read(&uploaded_file_path).unwrap();
        assert_eq!(
            uploaded_data.len(),
            file_size,
            "Large file {} size should match",
            size_name
        );

        // Verify pattern at beginning, middle, and end
        assert_eq!(
            uploaded_data[0], test_data[0],
            "Large file {} start should match",
            size_name
        );
        assert_eq!(
            uploaded_data[file_size / 2],
            test_data[file_size / 2],
            "Large file {} middle should match",
            size_name
        );
        assert_eq!(
            uploaded_data[file_size - 1],
            test_data[file_size - 1],
            "Large file {} end should match",
            size_name
        );

        let file_throughput = (file_size as f64 / (1024.0 * 1024.0)) / file_duration.as_secs_f64();
        println!(
            "  {} completed in {} ms ({:.2} MB/s)",
            size_name,
            file_duration.as_millis(),
            file_throughput
        );

        total_data_mb += file_size as f64 / (1024.0 * 1024.0);
    }

    let total_duration = start_time.elapsed();
    let overall_throughput = total_data_mb / total_duration.as_secs_f64();

    println!("Large files stress test completed:");
    println!(
        "   {:.2} MB total in {} ms",
        total_data_mb,
        total_duration.as_millis()
    );
    println!("   Overall throughput: {:.2} MB/s", overall_throughput);

    assert!(
        overall_throughput > 3.0,
        "Large files throughput should be at least 3 MB/s"
    );
}

#[test]
fn test_stress_multipart_multiple_files() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Stress test with multiple files in single requests
    let requests_count = 20;
    let files_per_request = 5;
    let file_size = 32 * 1024; // 32KB per file

    let mut total_files = 0;
    let start_time = Instant::now();

    for req_i in 0..requests_count {
        println!(
            "Processing multipart request {} of {}...",
            req_i + 1,
            requests_count
        );

        let boundary = format!("stress_multipart_boundary_{}", req_i);
        let mut body = Vec::new();
        let mut expected_files = Vec::new();

        // Build multipart request with multiple files
        for file_i in 0..files_per_request {
            let filename = format!("stress_multi_req{}_file{}.bin", req_i, file_i);
            let file_data: Vec<u8> = (0..file_size)
                .map(|j| ((req_i * files_per_request + file_i + j) % 256) as u8)
                .collect();
            expected_files.push((filename.clone(), file_data.clone()));

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

        let response = upload_handler.handle_upload(&request, None);
        assert!(
            response.is_ok(),
            "Multipart request {} should succeed",
            req_i
        );

        // Verify files from this request
        for (filename, expected_data) in expected_files {
            let uploaded_file_path = temp_dir.path().join(&filename);
            assert!(
                uploaded_file_path.exists(),
                "File {} should exist",
                filename
            );

            let uploaded_data = fs::read(&uploaded_file_path).unwrap();
            assert_eq!(
                uploaded_data.len(),
                file_size,
                "File {} size should match",
                filename
            );
            assert_eq!(
                uploaded_data, expected_data,
                "File {} content should match",
                filename
            );
        }

        total_files += files_per_request;
    }

    let total_duration = start_time.elapsed();
    let total_data_mb = (total_files * file_size) as f64 / (1024.0 * 1024.0);
    let throughput = total_data_mb / total_duration.as_secs_f64();

    println!("Multipart stress test completed:");
    println!(
        "   {} requests, {} files total, {:.2} MB",
        requests_count, total_files, total_data_mb
    );
    println!("   Total time: {} ms", total_duration.as_millis());
    println!("   Throughput: {:.2} MB/s", throughput);

    assert!(
        throughput > 2.0,
        "Multipart throughput should be at least 2 MB/s"
    );
    assert_eq!(
        total_files,
        requests_count * files_per_request,
        "All files should be processed"
    );
}
