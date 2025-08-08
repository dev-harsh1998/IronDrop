use irondrop::multipart::{MultipartConfig, MultipartParser};
use std::io::Cursor;

#[test]
fn test_large_file_uploads() {
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";

    // Test file sizes that previously failed
    let test_sizes = vec![
        404 * 1024,      // 404KB - Previously failed
        500 * 1024,      // 500KB
        1024 * 1024,     // 1MB
        2 * 1024 * 1024, // 2MB
        5 * 1024 * 1024, // 5MB
    ];

    for size in test_sizes {
        println!("Testing large file upload: {} MB", size / (1024 * 1024));

        // Create test data with a repeating pattern to verify integrity
        let mut test_data = Vec::with_capacity(size);
        for i in 0..size {
            test_data.push((i % 256) as u8);
        }

        // Build multipart body
        let header = format!(
            "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
            Content-Disposition: form-data; name=\"file\"; filename=\"large_test_{}.bin\"\r\n\
            Content-Type: application/octet-stream\r\n\
            \r\n",
            size / (1024 * 1024)
        );
        let footer = b"\r\n------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n";

        let mut multipart_body = Vec::new();
        multipart_body.extend_from_slice(header.as_bytes());
        multipart_body.extend_from_slice(&test_data);
        multipart_body.extend_from_slice(footer);

        let config = MultipartConfig::default();
        let parser = MultipartParser::new(Cursor::new(multipart_body), boundary, config);

        match parser {
            Ok(parser) => {
                let parts: Vec<_> = parser.into_iter().collect();
                assert_eq!(parts.len(), 1, "Should have exactly one part");

                let mut parts_iter = parts.into_iter();
                let mut part = parts_iter.next().unwrap().expect("Part should be valid");

                let data = part.read_to_bytes().expect("Should read data successfully");

                assert_eq!(data.len(), size, "Data length should match expected size");

                // Verify data integrity by checking the pattern
                for (i, &byte) in data.iter().enumerate() {
                    assert_eq!(
                        byte,
                        (i % 256) as u8,
                        "Data corruption at byte {} in {}MB file",
                        i,
                        size / (1024 * 1024)
                    );
                }

                println!(
                    "✅ Large file {}MB upload test passed",
                    size / (1024 * 1024)
                );
            }
            Err(e) => panic!(
                "Parser creation failed for {}MB file: {:?}",
                size / (1024 * 1024),
                e
            ),
        }
    }
}

#[test]
fn test_boundary_at_exact_403kb() {
    // Specific test for the exact boundary where the bug occurred
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let size = 403 * 1024 + 1024; // 403KB + 1KB to cross the boundary

    let mut test_data = Vec::with_capacity(size);
    for i in 0..size {
        test_data.push((i % 256) as u8);
    }

    let header = b"------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"file\"; filename=\"boundary_test.bin\"\r\n\
        Content-Type: application/octet-stream\r\n\
        \r\n";
    let footer = b"\r\n------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n";

    let mut multipart_body = Vec::new();
    multipart_body.extend_from_slice(header);
    multipart_body.extend_from_slice(&test_data);
    multipart_body.extend_from_slice(footer);

    let config = MultipartConfig::default();
    let parser = MultipartParser::new(Cursor::new(multipart_body), boundary, config).unwrap();

    let parts: Vec<_> = parser.into_iter().collect();
    assert_eq!(parts.len(), 1);

    let mut part = parts.into_iter().next().unwrap().unwrap();
    let data = part.read_to_bytes().unwrap();

    assert_eq!(
        data.len(),
        size,
        "Data at 403KB boundary should not be truncated"
    );

    // Verify data integrity
    for (i, &byte) in data.iter().enumerate() {
        assert_eq!(byte, (i % 256) as u8, "Data corruption at byte {}", i);
    }
}

#[test]
fn test_multiple_large_files() {
    // Test multiple large files in a single multipart request
    let boundary = "multifile_boundary";
    let file_size = 500 * 1024; // 500KB per file

    let mut multipart_body = Vec::new();
    let mut expected_files = Vec::new();

    for file_num in 1..=3 {
        // Create test data for this file
        let mut test_data = Vec::with_capacity(file_size);
        for i in 0..file_size {
            test_data.push(((i + file_num * 1000) % 256) as u8);
        }
        expected_files.push(test_data.clone());

        // Add multipart boundary and headers
        multipart_body.extend_from_slice(format!("--multifile_boundary\r\n").as_bytes());
        multipart_body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"file{}\"; filename=\"file{}.bin\"\r\n\
                Content-Type: application/octet-stream\r\n\
                \r\n",
                file_num, file_num
            )
            .as_bytes(),
        );
        multipart_body.extend_from_slice(&test_data);
        multipart_body.extend_from_slice(b"\r\n");
    }

    // Add final boundary
    multipart_body.extend_from_slice(b"--multifile_boundary--\r\n");

    let config = MultipartConfig::default();
    let parser = MultipartParser::new(Cursor::new(multipart_body), boundary, config).unwrap();

    let parts: Vec<_> = parser.into_iter().collect();
    assert_eq!(parts.len(), 3, "Should have exactly 3 parts");

    for (i, part_result) in parts.into_iter().enumerate() {
        let mut part = part_result.expect(&format!("Part {} should be valid", i + 1));
        let data = part
            .read_to_bytes()
            .expect(&format!("Should read data from part {}", i + 1));

        assert_eq!(
            data.len(),
            file_size,
            "File {} should have correct size",
            i + 1
        );
        assert_eq!(
            data,
            expected_files[i],
            "File {} data should match expected",
            i + 1
        );
    }

    println!("✅ Multiple large files test passed");
}

#[test]
fn test_binary_data_preservation() {
    // Test that binary data (including null bytes and high-value bytes) is preserved
    let boundary = "binary_test_boundary";

    // Create binary data with all possible byte values
    let mut test_data = Vec::new();
    for _ in 0..1000 {
        // Repeat the pattern 1000 times
        for byte_val in 0..=255u8 {
            test_data.push(byte_val);
        }
    }

    let header = b"--binary_test_boundary\r\n\
        Content-Disposition: form-data; name=\"binary_file\"; filename=\"binary.dat\"\r\n\
        Content-Type: application/octet-stream\r\n\
        \r\n";
    let footer = b"\r\n--binary_test_boundary--\r\n";

    let mut multipart_body = Vec::new();
    multipart_body.extend_from_slice(header);
    multipart_body.extend_from_slice(&test_data);
    multipart_body.extend_from_slice(footer);

    let config = MultipartConfig::default();
    let parser = MultipartParser::new(Cursor::new(multipart_body), boundary, config).unwrap();

    let parts: Vec<_> = parser.into_iter().collect();
    assert_eq!(parts.len(), 1);

    let mut part = parts.into_iter().next().unwrap().unwrap();
    let data = part.read_to_bytes().unwrap();

    assert_eq!(
        data.len(),
        test_data.len(),
        "Binary data length should be preserved"
    );
    assert_eq!(data, test_data, "Binary data should be exactly preserved");

    println!("✅ Binary data preservation test passed");
}
