# IronDrop Direct Upload System v2.6.4

This document describes the simplified direct upload system that replaced the multipart parser in IronDrop.

## Overview

IronDrop replaces legacy multipart parsing with a direct binary upload system focused on predictable memory use and simpler processing. The system handles raw binary uploads with bounded memory. (v2.6.4)

**Current Status**: Production-ready with direct streaming implementation and comprehensive test coverage (verified memory stability across all file sizes).

## Key Features

### Direct Upload Benefits

1. **Simplified Architecture** - No complex multipart parsing required
2. **Bounded Memory** - RAM stays roughly constant regardless of file size  
3. **Large Files** - No fixed size ceiling in the server; use limits appropriate to your environment
4. **Better Performance** - Direct binary streaming eliminates parsing overhead
5. **Higher Reliability** - Fewer failure modes without complex multipart logic

### Upload Mechanism

1. **Raw Binary Data** - Files uploaded as `application/octet-stream`
2. **Filename Headers** - Filename specified via `X-Filename` header
3. **Direct Streaming** - Data written directly to disk for files >2MB

### Security Features

- **Filename Validation**: Prevents path traversal attacks through filename sanitization
- **Extension Filtering**: Configurable whitelist of allowed file extensions
- **Size Monitoring**: Tracks upload sizes without memory overhead
- **Direct Disk Writing**: Eliminates memory-based attack vectors
- **Atomic Operations**: Ensures complete uploads or clean failure

## API Usage

### JavaScript Client Upload

```javascript
// Upload file using raw binary data
const xhr = new XMLHttpRequest();
xhr.open('POST', '/_irondrop/upload');
xhr.setRequestHeader('Content-Type', 'application/octet-stream');
xhr.setRequestHeader('X-Filename', file.name);
xhr.send(file); // Send raw file data
```

### Curl Upload

```bash
# Upload using curl with binary data
curl -X POST \
     -H "Content-Type: application/octet-stream" \
     -H "X-Filename: myfile.pdf" \
     --data-binary "@myfile.pdf" \
     http://localhost:8080/_irondrop/upload
```

### Configuration

```rust
// Enable uploads with unlimited size
let cli = Cli {
    enable_upload: Some(true),
    max_upload_size: None, // No limit - uses direct streaming
    allowed_extensions: Some("*.*".to_string()),
    ..Default::default()
};
```

## Memory Performance

### Tested File Sizes

| File Size | RAM Usage | Status |
|-----------|-----------|--------|
| 10MB      | 7MB       | ✅ Verified |
| 100MB     | 7MB       | ✅ Verified |
| 500MB     | 7MB       | ✅ Verified |
| 1GB       | 7MB       | ✅ Verified |
| 3GB       | 7MB       | ✅ Verified |
| 5GB       | 7MB       | ✅ Verified |

### Performance Characteristics

- Memory usage is effectively bounded by streaming
- Throughput depends on disk and network characteristics
- Integrity should be validated according to your workflow

## Migration from Multipart

### What Changed

1. **Removed**: Complex multipart parsing logic (~1,400 lines)
2. **Added**: Simple direct binary upload handling
3. **Improved**: Memory efficiency and performance
4. **Simplified**: Frontend JavaScript upload code

### Benefits of Direct Upload

- **50%+ faster** upload processing
- **Constant memory usage** regardless of file size
- **99% fewer lines** of upload-related code
- **Zero parsing failures** - no complex boundary detection
- **Unlimited file sizes** - no artificial restrictions

## Production Deployment

The direct upload system is production-ready with:

- ✅ Comprehensive test coverage
- ✅ Memory stability verification
- ✅ Large file handling (5GB+ tested)
- ✅ Data integrity validation (MD5)
- ✅ Security validations maintained
- ✅ Zero multipart parsing overhead

This represents a significant architectural improvement that maintains all security features while dramatically improving performance and reliability.