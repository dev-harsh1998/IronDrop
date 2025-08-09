use irondrop::multipart::{MultipartConfig, MultipartParser};
use std::io::Cursor;

#[test]
fn test_debug_file_size_limits() {
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";

    // Test different file sizes to identify the exact truncation point
    let test_sizes = vec![
        200 * 1024,  // 200KB - should work based on 50 attempts * 4KB
        300 * 1024,  // 300KB
        400 * 1024,  // 400KB
        403 * 1024,  // 403KB - reported corruption point
        404 * 1024,  // 404KB
        500 * 1024,  // 500KB
        1024 * 1024, // 1MB
    ];

    for size in test_sizes {
        println!("Testing file size: {} KB ({} bytes)", size / 1024, size);

        // Create test data with a pattern so we can detect truncation
        let mut test_data = Vec::with_capacity(size);
        for i in 0..size {
            test_data.push((i % 256) as u8);
        }

        // Build multipart body
        let header = format!(
            "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
            Content-Disposition: form-data; name=\"file\"; filename=\"test_{}.bin\"\r\n\
            Content-Type: application/octet-stream\r\n\
            \r\n",
            size / 1024
        );
        let footer = b"\r\n------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n";

        let mut multipart_body = Vec::new();
        multipart_body.extend_from_slice(header.as_bytes());
        multipart_body.extend_from_slice(&test_data);
        multipart_body.extend_from_slice(footer);

        println!("Multipart body total size: {} bytes", multipart_body.len());

        let config = MultipartConfig::default();
        let parser = MultipartParser::new(Cursor::new(multipart_body), boundary, config);

        match parser {
            Ok(parser) => {
                let mut parts: Vec<_> = parser.into_iter().collect();

                if parts.len() == 1 {
                    match &mut parts[0] {
                        Ok(part) => {
                            match part.read_to_bytes() {
                                Ok(data) => {
                                    println!(
                                        "Successfully read {} bytes (expected {})",
                                        data.len(),
                                        size
                                    );

                                    if data.len() != size {
                                        println!(
                                            "❌ TRUNCATION DETECTED: Expected {}, got {}",
                                            size,
                                            data.len()
                                        );

                                        // Check if the data pattern is correct up to the truncation point
                                        let mut pattern_correct = true;
                                        for (i, &byte) in data.iter().enumerate() {
                                            if byte != (i % 256) as u8 {
                                                pattern_correct = false;
                                                println!("Data corruption at byte {}: expected {}, got {}", 
                                                    i, i % 256, byte);
                                                break;
                                            }
                                        }

                                        if pattern_correct {
                                            println!(
                                                "Data pattern is correct up to truncation point"
                                            );
                                        }
                                    } else {
                                        println!("✅ Size correct");
                                    }
                                }
                                Err(e) => println!("❌ Error reading data: {e:?}"),
                            }
                        }
                        Err(e) => println!("❌ Error with part: {e:?}"),
                    }
                } else {
                    println!("❌ Expected 1 part, got {}", parts.len());
                }
            }
            Err(e) => println!("❌ Parser creation failed: {e:?}"),
        }

        println!("---");
    }
}
