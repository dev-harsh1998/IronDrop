use irondrop::cli::Cli;
use irondrop::http::Request;
use irondrop::upload::UploadHandler;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
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
fn test_upload_performance_small_files() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test performance with many small files
    let file_count = 100;
    let file_size = 1024; // 1KB each
    let mut total_duration = Duration::new(0, 0);

    for i in 0..file_count {
        let filename = format!("perf_small_{}.bin", i);
        let test_data: Vec<u8> = (0..file_size).map(|j| ((i + j) % 256) as u8).collect();
        let boundary = "perf_test_boundary";
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

        let start = Instant::now();
        let response = upload_handler.handle_upload(&request, None);
        let duration = start.elapsed();
        total_duration += duration;

        assert!(response.is_ok(), "Small file {} upload should succeed", i);

        // Verify file exists and has correct size
        let uploaded_file_path = temp_dir.path().join(&filename);
        assert!(uploaded_file_path.exists());
        let uploaded_data = fs::read(&uploaded_file_path).unwrap();
        assert_eq!(uploaded_data.len(), file_size);
    }

    let avg_duration = total_duration / file_count as u32;
    println!(
        "Small files performance: {} files, avg {} ms per file",
        file_count,
        avg_duration.as_millis()
    );

    // Performance assertion: should be fast for small files
    assert!(
        avg_duration.as_millis() < 100,
        "Small file uploads should be fast"
    );
}

#[test]
fn test_upload_performance_medium_files() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test performance with medium files
    let file_sizes = vec![
        (64 * 1024, "64KB"),
        (256 * 1024, "256KB"),
        (1024 * 1024, "1MB"),
        (4 * 1024 * 1024, "4MB"),
    ];

    for (size, size_name) in file_sizes {
        let filename = format!("perf_medium_{}.bin", size_name);
        let test_data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let boundary = "perf_medium_boundary";
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

        let start = Instant::now();
        let response = upload_handler.handle_upload(&request, None);
        let duration = start.elapsed();

        assert!(
            response.is_ok(),
            "Medium file {} upload should succeed",
            size_name
        );

        // Verify file integrity
        let uploaded_file_path = temp_dir.path().join(&filename);
        let uploaded_data = fs::read(&uploaded_file_path).unwrap();
        assert_eq!(uploaded_data.len(), size);
        assert_eq!(uploaded_data, test_data);

        let throughput_mbps = (size as f64 / (1024.0 * 1024.0)) / duration.as_secs_f64();
        println!(
            "Medium file {}: {} ms, {:.2} MB/s throughput",
            size_name,
            duration.as_millis(),
            throughput_mbps
        );
    }
}

#[test]
fn test_concurrent_upload_simulation() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());

    // Simulate concurrent uploads by processing multiple files in sequence
    // (actual concurrency would require threading, but this tests buffer reuse)
    let upload_count = 10;
    let file_size = 128 * 1024; // 128KB each

    let mut total_duration = Duration::new(0, 0);
    let start_time = Instant::now();

    for i in 0..upload_count {
        let mut upload_handler = UploadHandler::new(&cli).unwrap();
        let filename = format!("concurrent_{}.bin", i);
        let test_data: Vec<u8> = (0..file_size).map(|j| ((i + j) % 256) as u8).collect();
        let boundary = "concurrent_test_boundary";
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

        let upload_start = Instant::now();
        let response = upload_handler.handle_upload(&request, None);
        let upload_duration = upload_start.elapsed();
        total_duration += upload_duration;

        assert!(response.is_ok(), "Concurrent upload {} should succeed", i);

        // Verify file
        let uploaded_file_path = temp_dir.path().join(&filename);
        let uploaded_data = fs::read(&uploaded_file_path).unwrap();
        assert_eq!(uploaded_data.len(), file_size);
    }

    let total_time = start_time.elapsed();
    let avg_upload_time = total_duration / upload_count as u32;
    let total_data_mb = (upload_count * file_size) as f64 / (1024.0 * 1024.0);
    let overall_throughput = total_data_mb / total_time.as_secs_f64();

    println!(
        "Concurrent simulation: {} uploads, {:.2} MB total",
        upload_count, total_data_mb
    );
    println!(
        "   Total time: {} ms, Avg per upload: {} ms",
        total_time.as_millis(),
        avg_upload_time.as_millis()
    );
    println!("   Overall throughput: {:.2} MB/s", overall_throughput);

    // Performance assertions
    assert!(
        avg_upload_time.as_millis() < 1000,
        "Average upload time should be reasonable"
    );
    assert!(
        overall_throughput > 1.0,
        "Overall throughput should be at least 1 MB/s"
    );
}

#[test]
fn test_memory_stability_repeated_uploads() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test memory stability by doing many uploads with the same handler
    let iterations = 50;
    let file_size = 64 * 1024; // 64KB

    let mut durations = Vec::new();

    for i in 0..iterations {
        let filename = format!("stability_{}.bin", i);
        let test_data: Vec<u8> = (0..file_size).map(|j| ((i + j) % 256) as u8).collect();
        let boundary = "stability_test_boundary";
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

        let start = Instant::now();
        let response = upload_handler.handle_upload(&request, None);
        let duration = start.elapsed();
        durations.push(duration);

        assert!(
            response.is_ok(),
            "Stability test upload {} should succeed",
            i
        );

        // Verify file
        let uploaded_file_path = temp_dir.path().join(&filename);
        let uploaded_data = fs::read(&uploaded_file_path).unwrap();
        assert_eq!(uploaded_data.len(), file_size);
    }

    // Analyze performance stability
    let first_half_avg: Duration =
        durations[0..iterations / 2].iter().sum::<Duration>() / (iterations / 2) as u32;
    let second_half_avg: Duration =
        durations[iterations / 2..].iter().sum::<Duration>() / (iterations / 2) as u32;

    println!("Memory stability test: {} iterations", iterations);
    println!(
        "   First half avg: {} ms, Second half avg: {} ms",
        first_half_avg.as_millis(),
        second_half_avg.as_millis()
    );

    // Performance should remain stable (no significant degradation)
    let performance_ratio = second_half_avg.as_millis() as f64 / first_half_avg.as_millis() as f64;
    assert!(
        performance_ratio < 2.0,
        "Performance should not degrade significantly over time"
    );

    println!(
        "   Performance ratio (later/earlier): {:.2}",
        performance_ratio
    );
}

#[test]
fn test_large_file_streaming_performance() {
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    let mut upload_handler = UploadHandler::new(&cli).unwrap();

    // Test streaming performance with a larger file
    let file_size = 10 * 1024 * 1024; // 10MB
    let filename = "large_streaming_test.bin";

    println!("Creating {} MB test data...", file_size / (1024 * 1024));
    let test_data: Vec<u8> = (0..file_size).map(|i| (i % 256) as u8).collect();

    let boundary = "large_streaming_boundary";
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

    println!("Starting large file upload...");
    let start = Instant::now();
    let response = upload_handler.handle_upload(&request, None);
    let duration = start.elapsed();

    assert!(response.is_ok(), "Large file upload should succeed");

    // Verify file integrity
    let uploaded_file_path = temp_dir.path().join(filename);
    let uploaded_data = fs::read(&uploaded_file_path).unwrap();
    assert_eq!(uploaded_data.len(), file_size);

    // Verify first and last chunks to ensure streaming worked correctly
    assert_eq!(&uploaded_data[0..1000], &test_data[0..1000]);
    assert_eq!(
        &uploaded_data[file_size - 1000..],
        &test_data[file_size - 1000..]
    );

    let throughput_mbps = (file_size as f64 / (1024.0 * 1024.0)) / duration.as_secs_f64();
    println!(
        "Large file streaming: {} MB in {} ms",
        file_size / (1024 * 1024),
        duration.as_millis()
    );
    println!("   Throughput: {:.2} MB/s", throughput_mbps);

    // Performance assertion: should handle large files efficiently
    assert!(
        throughput_mbps > 5.0,
        "Large file throughput should be at least 5 MB/s"
    );
}
