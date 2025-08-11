use irondrop::error::AppError;
use irondrop::multipart::{MultipartConfig, MultipartParser};
use std::io::{Cursor, Write};

/// A simple writer implementation for testing streaming
struct TestWriter {
    data: Vec<u8>,
}

impl TestWriter {
    fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn get_data(&self) -> &[u8] {
        &self.data
    }
}

impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.data.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_stream_to_method() -> Result<(), AppError> {
    // Create test data
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let test_content = "This is test content for streaming";

    let multipart_data = format!(
        "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
        Content-Type: text/plain\r\n\
        \r\n\
        {}\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n",
        test_content
    );

    // Create parser
    let config = MultipartConfig::default();
    let parser = MultipartParser::new(
        Cursor::new(multipart_data.as_bytes()),
        boundary,
        config.clone(),
    )?;

    // Process the part
    let mut parts = parser.into_iter().collect::<Result<Vec<_>, _>>()?;
    assert_eq!(parts.len(), 1, "Should have exactly one part");

    let part = parts.remove(0);
    assert_eq!(
        part.filename,
        Some("test.txt".to_string()),
        "Filename should match"
    );

    // Test streaming with different buffer sizes
    let buffer_sizes = [1, 2, 5, 10, 100];

    for buffer_size in buffer_sizes {
        // Reset the part reader position
        let mut test_writer = TestWriter::new();

        // Create a new parser for each test since we consumed the previous one
        let parser = MultipartParser::new(
            Cursor::new(multipart_data.as_bytes()),
            boundary,
            config.clone(),
        )?;
        let mut parts = parser.into_iter().collect::<Result<Vec<_>, _>>()?;
        let mut part = parts.remove(0);

        // Stream to the test writer
        let bytes_written = part.stream_to(&mut test_writer, buffer_size)?;

        // Verify results
        assert_eq!(
            bytes_written as usize,
            test_content.len(),
            "Should write correct number of bytes with buffer size {}",
            buffer_size
        );
        assert_eq!(
            test_writer.get_data(),
            test_content.as_bytes(),
            "Written data should match original with buffer size {}",
            buffer_size
        );
    }

    Ok(())
}

#[test]
fn test_stream_to_with_large_content() -> Result<(), AppError> {
    // Create a larger test content (100KB)
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let test_content = "A".repeat(100 * 1024); // 100KB of data

    let multipart_data = format!(
        "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"file\"; filename=\"large.txt\"\r\n\
        Content-Type: text/plain\r\n\
        \r\n\
        {}\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n",
        test_content
    );

    // Create parser
    let config = MultipartConfig::default();
    let parser = MultipartParser::new(
        Cursor::new(multipart_data.as_bytes()),
        boundary,
        config.clone(),
    )?;

    // Process the part
    let mut parts = parser.into_iter().collect::<Result<Vec<_>, _>>()?;
    assert_eq!(parts.len(), 1, "Should have exactly one part");

    let _part = parts.remove(0); // We don't use this directly, just checking it exists

    // Test streaming with different buffer sizes
    let buffer_sizes = [1024, 4096, 8192, 16384];

    for buffer_size in buffer_sizes {
        // Create a new parser for each test
        let parser = MultipartParser::new(
            Cursor::new(multipart_data.as_bytes()),
            boundary,
            config.clone(),
        )?;
        let mut parts = parser.into_iter().collect::<Result<Vec<_>, _>>()?;
        let mut part = parts.remove(0);

        let mut test_writer = TestWriter::new();
        let bytes_written = part.stream_to(&mut test_writer, buffer_size)?;

        // Verify results
        assert_eq!(
            bytes_written as usize,
            test_content.len(),
            "Should write correct number of bytes with buffer size {}",
            buffer_size
        );
        assert_eq!(
            test_writer.get_data(),
            test_content.as_bytes(),
            "Written data should match original with buffer size {}",
            buffer_size
        );
    }

    Ok(())
}

#[test]
fn test_stream_to_vs_read_to_bytes() -> Result<(), AppError> {
    // Test that stream_to produces the same result as read_to_bytes
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let test_content = "Content to compare between streaming and full reading";

    let multipart_data = format!(
        "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"file\"; filename=\"compare.txt\"\r\n\
        Content-Type: text/plain\r\n\
        \r\n\
        {}\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n",
        test_content
    );

    // Create two parsers - one for each method
    let config = MultipartConfig::default();

    // Parser for read_to_bytes
    let parser1 = MultipartParser::new(
        Cursor::new(multipart_data.as_bytes()),
        boundary,
        config.clone(),
    )?;
    let mut parts1 = parser1.into_iter().collect::<Result<Vec<_>, _>>()?;
    let mut part1 = parts1.remove(0);

    // Parser for stream_to
    let parser2 = MultipartParser::new(Cursor::new(multipart_data.as_bytes()), boundary, config)?;
    let mut parts2 = parser2.into_iter().collect::<Result<Vec<_>, _>>()?;
    let mut part2 = parts2.remove(0);

    // Get data using read_to_bytes
    let bytes_data = part1.read_to_bytes()?;

    // Get data using stream_to
    let mut test_writer = TestWriter::new();
    part2.stream_to(&mut test_writer, 1024)?;

    // Compare results
    assert_eq!(
        bytes_data,
        test_writer.get_data(),
        "Data from read_to_bytes and stream_to should be identical"
    );

    Ok(())
}

#[test]
fn test_stream_to_error_handling() -> Result<(), AppError> {
    // Create test data with a boundary that will cause an error during streaming
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let test_content = "This is test content for streaming";

    let multipart_data = format!(
        "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
        Content-Type: text/plain\r\n\
        \r\n\
        {}\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n",
        test_content
    );

    // Create a test writer that will fail on write
    struct FailingWriter;
    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Simulated write error",
            ))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    // Create parser with normal config
    let config = MultipartConfig::default();
    let parser = MultipartParser::new(Cursor::new(multipart_data.as_bytes()), boundary, config)?;

    // Process the part
    let mut parts = parser.into_iter().collect::<Result<Vec<_>, _>>()?;
    assert_eq!(parts.len(), 1, "Should have exactly one part");

    let mut part = parts.remove(0);

    // Attempt to stream to the failing writer
    let mut failing_writer = FailingWriter {};
    let result = part.stream_to(&mut failing_writer, 1024);

    // Verify we get an IO error
    assert!(result.is_err(), "Should return an error");
    if let Err(err) = result {
        match err {
            AppError::Io(_) => {} // Expected error
            _ => panic!("Expected Io error, got: {:?}", err),
        }
    }

    Ok(())
}
