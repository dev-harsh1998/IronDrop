# IronDrop Multipart Parser v2.5

This document describes the comprehensive multipart/form-data parser module created for IronDrop.

## Overview

The `src/multipart.rs` module provides a memory-efficient, security-focused multipart/form-data parser that uses only the standard Rust library. It's designed to handle file uploads and form fields with comprehensive security validations.

**Current Status**: Production-ready with RFC 7578 compliance and comprehensive test coverage (7 dedicated multipart tests as part of 59 total tests across the project).

## Key Features

### Core Structures

1. **`MultipartParser`** - Main parser struct that creates an iterator over multipart parts
2. **`MultipartPart`** - Represents a single part (file or field) with streaming access to data
3. **`PartHeaders`** - Parses and validates multipart part headers
4. **`MultipartConfig`** - Configuration for security limits and validation rules

### Security Features

- **Boundary Validation**: Prevents injection attacks through malicious boundary strings
- **Size Limits**: Configurable limits for part count, part size, and header size
- **Filename Sanitization**: Automatic sanitization to prevent path traversal attacks
- **Content-Type Validation**: Optional whitelist of allowed MIME types and file extensions
- **Memory Safety**: Streaming parsing prevents loading entire multipart content into memory

### RFC 7578 Compliance

The parser implements key aspects of RFC 7578 (multipart/form-data):
- Boundary detection and validation
- Content-Disposition header parsing
- Content-Type header handling
- Binary and text data support
- Proper filename extraction and sanitization

## API Usage

### Basic Usage

```rust
use irondrop::multipart::{MultipartParser, MultipartConfig};
use std::io::Cursor;

// Extract boundary from Content-Type header
let content_type = "multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW";
let boundary = MultipartParser::<Cursor<Vec<u8>>>::extract_boundary_from_content_type(content_type)?;

// Create parser with default security config
let config = MultipartConfig::default();
let parser = MultipartParser::new(Cursor::new(body_data), &boundary, config)?;

// Process each part
for part_result in parser {
    let mut part = part_result?;
    
    if part.is_file() {
        // Handle file upload
        println!("File: {}", part.filename.unwrap_or_default());
        let file_data = part.read_to_bytes()?;
        // Save to disk...
    } else {
        // Handle form field
        let field_name = part.field_name().unwrap_or("unknown");
        let value = part.read_to_string()?;
        println!("Field {}: {}", field_name, value);
    }
}
```

### Security Configuration

```rust
let config = MultipartConfig {
    max_parts: 50,                    // Maximum number of parts
    max_part_size: 5 * 1024 * 1024,  // 5MB per part
    max_filename_length: 255,         // Maximum filename length
    max_field_name_length: 100,       // Maximum field name length
    max_headers_size: 8 * 1024,       // Maximum headers size
    allowed_extensions: vec![          // Whitelist file extensions
        "jpg".to_string(),
        "png".to_string(),
        "pdf".to_string()
    ],
    allowed_mime_types: vec![          // Whitelist MIME types
        "image/jpeg".to_string(),
        "image/png".to_string(),
        "application/pdf".to_string()
    ],
};
```

### Integration with HTTP Requests

```rust
use irondrop::{multipart::MultipartParser, http::Request, error::AppError};

fn handle_upload(request: &Request) -> Result<(), AppError> {
    let content_type = request.headers.get("content-type")
        .ok_or_else(|| AppError::invalid_multipart("Missing Content-Type"))?;
    
    let boundary = MultipartParser::<Cursor<Vec<u8>>>::extract_boundary_from_content_type(content_type)?;
    
    let body = request.body.as_ref()
        .ok_or_else(|| AppError::invalid_multipart("No body"))?;
    
    let config = MultipartConfig::default();
    let parser = MultipartParser::new(Cursor::new(body.clone()), &boundary, config)?;
    
    for part_result in parser {
        let mut part = part_result?;
        // Process part...
    }
    
    Ok(())
}
```

## Error Handling

The parser integrates with IronDrop's `AppError` system:

- `AppError::InvalidMultipart(details)` - Malformed multipart data
- `AppError::InvalidFilename(filename)` - Dangerous filename detected
- `AppError::PayloadTooLarge(size)` - Size limits exceeded
- `AppError::UnsupportedMediaType(type)` - File type not allowed

## Testing

The module includes comprehensive tests covering:

- Boundary validation and extraction
- Header parsing and validation
- Filename sanitization security
- Configuration limits enforcement
- Content-Disposition parsing
- Integration with HTTP requests

Run tests with:
```bash
cargo test multipart
```

## Production Considerations

This implementation provides a solid foundation with proper security validations and error handling. For production use with complex multipart data, consider these enhancements:

1. **Enhanced Boundary Detection**: More sophisticated state machine for complex boundary scenarios
2. **Streaming Optimizations**: Better memory management for very large files
3. **Error Recovery**: More robust handling of malformed data
4. **Performance**: Optimized parsing for high-throughput scenarios

## Files in IronDrop v2.5

- `src/multipart.rs` - Main multipart parser module (661 lines, production-ready)
- `tests/multipart_test.rs` - Integration tests (7 comprehensive test cases)
- `tests/debug_upload_test.rs` - Debug and edge case testing
- `doc/MULTIPART_README.md` - This documentation

## Integration Status

The multipart parser is fully integrated into IronDrop's upload system:
- **Upload Handler**: `src/upload.rs` uses the parser for file processing
- **HTTP Processing**: Integrated with `src/http.rs` for request handling  
- **Error System**: Uses `AppError` for consistent error handling
- **CLI Configuration**: Respects size limits and validation rules from CLI
- **Template System**: Works with upload UI templates for web interface

## Version History

- **v2.5**: Production release with comprehensive security validations
- Full RFC 7578 compliance with boundary detection
- Memory-efficient streaming parser
- Extensive test coverage (59 total project tests across 13 test files)
- Integrated with upload UI system and template engine

The module provides enterprise-grade multipart parsing capabilities and is battle-tested with comprehensive security validations.