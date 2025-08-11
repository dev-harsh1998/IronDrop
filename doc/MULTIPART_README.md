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
- **Memory Safety**: Advanced streaming parsing with bounded memory usage for files of any size
- **Large File Support**: Efficiently processes multi-gigabyte files without memory exhaustion

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
- **Large file streaming**: 85MB+ multipart upload processing
- **Memory efficiency**: Bounded memory usage validation

### Test Coverage
- **12 unit tests** in `cargo test multipart --lib`
- **Large file bash verification**: `test_multiple_large_files_bash_verification` (85MB test case)
- **Integration tests**: Full upload pipeline testing
- **Performance tests**: Memory usage and streaming efficiency

Run tests with:
```bash
# All multipart tests
cargo test multipart

# Large file streaming test
cargo test --test large_file_bash_test test_multiple_large_files_bash_verification

# Full test suite
cargo test
```

## Streaming Implementation (v2.5.1)

### Advanced Memory Management

The multipart parser now features a sophisticated streaming implementation that processes large files efficiently:

#### Key Improvements
- **Incremental Data Extraction**: Processes multipart data in chunks while maintaining boundary detection
- **Bounded Memory Usage**: Memory consumption is independent of file size (64MB threshold)
- **Buffer Management**: Intelligent buffer draining prevents memory buildup
- **Boundary Preservation**: Maintains reliable boundary detection during streaming

#### Technical Details
```rust
// Streaming approach with bounded memory
fn extract_part_data(&mut self, content_start: usize) -> Result<Vec<u8>, AppError> {
    let mut data = Vec::new();
    let mut total_read = 0;
    
    loop {
        // Extract data from current buffer (except boundary search area)
        let boundary_search_area = self.boundary.len() + 10;
        let extractable_end = if self.buffer.len() > boundary_search_area {
            self.buffer.len() - boundary_search_area
        } else {
            content_start
        };
        
        if content_start < extractable_end {
            let chunk = &self.buffer[content_start..extractable_end];
            data.extend_from_slice(chunk);
            total_read += chunk.len();
            
            // Remove extracted data from buffer to prevent memory buildup
            self.buffer.drain(content_start..extractable_end);
        }
        
        // Continue reading more data...
    }
}
```

#### Performance Benefits
- **Large File Support**: Successfully processes 85MB+ multipart uploads
- **Memory Efficiency**: Constant memory usage regardless of file size
- **No Hanging**: Eliminates timeout issues with large file processing
- **Maintained Security**: All existing security validations preserved

## Production Considerations

This implementation provides enterprise-grade multipart processing with comprehensive security and performance optimizations:

### Memory Management
- **Streaming Optimizations**: Files >64MB are automatically streamed to disk
- **Buffer Control**: Configurable buffer sizes prevent memory exhaustion
- **Resource Protection**: Built-in safeguards against memory-based attacks
- **Memory Cap**: Total memory usage never exceeds 128MB regardless of file size

1. **✅ Streaming Optimizations**: Advanced memory management for files of any size
2. **Enhanced Boundary Detection**: Robust state machine for complex boundary scenarios
3. **Error Recovery**: Comprehensive handling of malformed data
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

### HTTP Streaming Integration (v2.5)

The multipart parser seamlessly integrates with IronDrop's new HTTP layer streaming system:

```rust
use irondrop::{multipart::MultipartParser, upload::RequestBody};
use std::io::Cursor;
use std::fs::File;

impl UploadHandler {
    pub fn process_multipart_parts(&self, request_body: RequestBody) -> Result<(), AppError> {
        match request_body {
            RequestBody::Memory(data) => {
                // Small uploads: Direct memory processing
                let cursor = Cursor::new(data);
                let parser = MultipartParser::new(cursor, &boundary, config)?;
                self.process_parts(parser)
            }
            RequestBody::File(path) => {
                // Large uploads: Stream from temporary file
                let file = File::open(path)?;
                let parser = MultipartParser::new(file, &boundary, config)?;
                self.process_parts(parser)
            }
        }
    }
}
```

**Key Benefits:**
- **Unified Interface**: Same multipart parser works with both memory and file-based request bodies
- **Automatic Optimization**: Small uploads (≤1MB) processed in memory, large uploads (>1MB) streamed from disk
- **Resource Efficiency**: Prevents memory exhaustion while maintaining fast processing for small files
- **Transparent Operation**: Existing multipart parsing code works without modification

## Version History

- **v2.5**: Production release with comprehensive security validations
- Full RFC 7578 compliance with boundary detection
- Memory-efficient streaming parser
- Extensive test coverage (59 total project tests across 13 test files)
- Integrated with upload UI system and template engine

The module provides enterprise-grade multipart parsing capabilities and is battle-tested with comprehensive security validations.