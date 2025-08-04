use irondrop::error::AppError;
use irondrop::multipart::{MultipartConfig, MultipartParser};
use std::io::Cursor;

#[test]
fn test_multipart_parser_creation() -> Result<(), AppError> {
    // Test that the parser can be created with valid boundary and config
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let config = MultipartConfig::default();
    let data = b"test data";

    let _parser = MultipartParser::new(Cursor::new(data), boundary, config)?;

    // Test that the parser was created successfully
    // Note: Full parsing implementation would require more sophisticated
    // boundary detection and state machine for production use
    // Parser created successfully

    Ok(())
}

// Note: This is a simplified test. A production multipart parser would need:
// 1. More sophisticated state machine for boundary detection
// 2. Proper handling of nested boundaries
// 3. Support for different line endings (\r\n vs \n)
// 4. Better streaming support for very large files
// 5. More robust error recovery
#[test]
fn test_multipart_structure_validation() {
    // Test the individual components that make up multipart parsing
    let config = MultipartConfig::default();

    // Test Content-Disposition parsing
    let cd = irondrop::multipart::PartHeaders::parse_content_disposition(
        "form-data; name=\"field1\"",
        &config,
    )
    .unwrap();

    assert_eq!(cd.disposition_type, "form-data");
    assert_eq!(cd.name, "field1");
    assert_eq!(cd.filename, None);

    // Test with filename
    let cd_file = irondrop::multipart::PartHeaders::parse_content_disposition(
        "form-data; name=\"file\"; filename=\"test.txt\"",
        &config,
    )
    .unwrap();

    assert_eq!(cd_file.disposition_type, "form-data");
    assert_eq!(cd_file.name, "file");
    assert_eq!(cd_file.filename, Some("test.txt".to_string()));
}

#[test]
fn test_boundary_extraction_from_content_type() {
    let content_type = "multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let boundary =
        MultipartParser::<Cursor<Vec<u8>>>::extract_boundary_from_content_type(content_type)
            .unwrap();
    assert_eq!(boundary, "----WebKitFormBoundary7MA4YWxkTrZu0gW");

    // Test with quoted boundary
    let content_type_quoted =
        r#"multipart/form-data; boundary="----WebKitFormBoundary7MA4YWxkTrZu0gW""#;
    let boundary_quoted =
        MultipartParser::<Cursor<Vec<u8>>>::extract_boundary_from_content_type(content_type_quoted)
            .unwrap();
    assert_eq!(boundary_quoted, "----WebKitFormBoundary7MA4YWxkTrZu0gW");

    // Test invalid content type
    let invalid_content_type = "application/json";
    assert!(
        MultipartParser::<Cursor<Vec<u8>>>::extract_boundary_from_content_type(
            invalid_content_type
        )
        .is_err()
    );
}

#[test]
fn test_security_limits() {
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";

    // Test max parts limit
    let config = MultipartConfig {
        max_parts: 1,
        ..Default::default()
    };

    let multipart_data = concat!(
        "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n",
        "Content-Disposition: form-data; name=\"field1\"\r\n",
        "\r\n",
        "value1\r\n",
        "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n",
        "Content-Disposition: form-data; name=\"field2\"\r\n",
        "\r\n",
        "value2\r\n",
        "------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n"
    );

    let parser =
        MultipartParser::new(Cursor::new(multipart_data.as_bytes()), boundary, config).unwrap();
    let mut parts_processed = 0;
    let mut error_encountered = false;

    for part_result in parser {
        match part_result {
            Err(error) => {
                error_encountered = true;
                assert!(matches!(error, AppError::InvalidMultipart(_)));
                break;
            }
            Ok(_) => {
                parts_processed += 1;
            }
        }
    }

    // Should process first part and then encounter error on second part
    assert!(parts_processed <= 1);
    assert!(error_encountered);
}

#[test]
fn test_filename_security() {
    // Test that dangerous filenames are rejected or sanitized
    use irondrop::multipart::PartHeaders;

    let config = MultipartConfig::default();

    // Test path traversal rejection
    assert!(PartHeaders::parse_content_disposition(
        "form-data; name=\"file\"; filename=\"../../../etc/passwd\"",
        &config
    )
    .is_err());

    // Test filename sanitization
    let cd = PartHeaders::parse_content_disposition(
        "form-data; name=\"file\"; filename=\"safe-file.txt\"",
        &config,
    )
    .unwrap();
    assert_eq!(cd.filename, Some("safe-file.txt".to_string()));
}

#[test]
fn test_empty_multipart() {
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let config = MultipartConfig::default();

    // Empty multipart data (just boundary end)
    let multipart_data = "------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n";

    let parser =
        MultipartParser::new(Cursor::new(multipart_data.as_bytes()), boundary, config).unwrap();
    let parts: Result<Vec<_>, _> = parser.into_iter().collect();
    let parts = parts.unwrap();

    assert_eq!(parts.len(), 0);
}
