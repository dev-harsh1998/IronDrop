# IronDrop Testing Documentation v2.5

## Overview

IronDrop features a comprehensive testing suite with **61 tests** across multiple categories, ensuring reliability, performance, and security. All tests use English-only characters and messages for consistency and maintainability.

## Test Suite Structure

### 📊 Test Statistics
- **Total Tests**: 62 tests across 15 test files
- **Coverage Areas**: Core functionality, security, performance, memory optimization, stress testing, HTTP streaming, large file handling
- **Test Types**: Unit tests, integration tests, performance benchmarks, stress tests, streaming tests, bash integration tests
- **Languages**: Rust (`.rs`) and Shell (`.sh`) test files

## Core Test Categories

### 1. **Comprehensive Server Tests** (`comprehensive_test.rs`)
**Purpose**: Core server functionality and HTTP handling  
**Test Count**: 19 tests  
**Key Areas**:
- Server initialization and configuration
- Native HTTP client implementation
- Basic GET/POST request handling
- Static asset serving (`/_irondrop/static/`)
- Template rendering and variable interpolation
- Error handling and status codes
- Response header validation

**Notable Tests**:
- `test_server_setup()` - Server initialization
- `test_native_http_client()` - Custom HTTP client implementation
- `test_static_asset_serving()` - CSS/JS asset delivery
- `test_template_rendering()` - Template engine functionality
- `test_error_handling()` - Error response generation

### 2. **Integration Tests** (`integration_test.rs`)
**Purpose**: Authentication, security, and external client integration  
**Test Count**: 6 tests  
**Key Areas**:
- Basic authentication with username/password
- Security header validation
- Path traversal attack prevention
- Error response formatting
- External HTTP client integration (`reqwest`)

**Notable Tests**:
- `test_basic_auth()` - Authentication mechanism
- `test_security_headers()` - Security header presence
- `test_path_traversal_prevention()` - Security validation
- `test_error_responses()` - Error handling consistency

### 3. **Edge Case Tests** (`edge_case_test.rs`)
**Purpose**: Upload handler edge cases and boundary conditions  
**Test Count**: 10 tests  
**Key Areas**:
- Empty file upload handling
- Single-byte file processing
- Malformed multipart boundary detection
- Missing Content-Type header handling
- Invalid filename character sanitization
- Special character filename processing

**Notable Tests**:
- `test_empty_file_upload()` - Zero-byte file handling
- `test_single_byte_file()` - Minimal file processing
- `test_malformed_multipart_boundary()` - Parser robustness
- `test_missing_content_type()` - Header validation
- `test_special_character_filename()` - Filename sanitization

### 4. **Memory Optimization Tests** (`memory_optimization_test.rs`)
**Purpose**: Memory-efficient file handling and resource management  
**Test Count**: 6 tests  
**Key Areas**:
- Small file memory efficiency (1KB-10KB)
- Medium file handling (100KB-1MB)
- Boundary size file processing (exactly 1MB)
- Multiple file upload memory management
- CLI configuration for memory settings
- Multipart request body creation

**Notable Tests**:
- `test_small_file_memory_efficiency()` - Small file optimization
- `test_medium_file_handling()` - Medium file processing
- `test_boundary_size_file()` - Exact boundary handling
- `test_multiple_files_memory()` - Multi-file memory management

### 5. **Performance Tests** (`performance_test.rs`)
**Purpose**: Upload performance measurement and optimization  
**Test Count**: 5 tests  
**Key Areas**:
- Many small files upload performance
- Medium-sized file upload timing
- Concurrent upload simulation
- Average upload time measurement
- Throughput calculation and validation

**Notable Tests**:
- `test_many_small_files_performance()` - Small file throughput
- `test_medium_file_upload_performance()` - Medium file timing
- `test_concurrent_upload_simulation()` - Concurrency handling
- Performance assertions with timing thresholds

### 6. **Stress Tests** (`stress_test.rs`)
**Purpose**: System behavior under high load and stress conditions  
**Test Count**: 4 tests  
**Key Areas**:
- High-volume small file processing
- Mixed file size stress testing
- Total data processing measurement
- Duration and throughput metrics
- Performance threshold validation

**Notable Tests**:
- `test_many_small_files_stress()` - High-volume processing
- `test_mixed_file_sizes_stress()` - Varied load testing
- Comprehensive metrics collection (data processed, duration, throughput)

### 7. **Multipart Parser Tests** (`multipart_test.rs`)
**Purpose**: RFC 7578 compliant multipart/form-data parsing  
**Test Count**: 7 tests  
**Key Areas**:
- Multipart parser creation and initialization
- Multipart structure validation
- Boundary extraction from Content-Type headers
- Security limits enforcement (max parts)
- Filename security and path traversal prevention
- Empty multipart data handling

**Notable Tests**:
- `test_multipart_parser_creation()` - Parser initialization
- `test_multipart_validation()` - Structure validation
- `test_boundary_extraction()` - Header parsing
- `test_security_limits()` - Security enforcement
- `test_filename_security()` - Path traversal prevention

### 8. **Ultra-Compact Search Tests** (`ultra_compact_test.rs`)
**Purpose**: Memory-optimized search engine for large directories  
**Test Count**: 4 tests  
**Key Areas**:
- RadixIndex memory efficiency with 10M entries
- Search performance optimization
- Path reconstruction accuracy
- CompactCache memory efficiency
- Microsecond-level performance measurement

**Notable Tests**:
- `test_radix_index_memory_efficiency()` - Large-scale memory optimization
- `test_search_performance()` - Search speed validation
- `test_path_reconstruction()` - Data integrity verification
- `test_compact_cache_memory_efficiency()` - Cache optimization

### 9. **Template Embedding Tests** (`template_embedding_test.rs`)
**Purpose**: Template engine and embedded asset functionality  
**Test Count**: 3 tests  
**Key Areas**:
- Embedded template rendering without filesystem access
- Static asset retrieval (CSS, JavaScript)
- Directory listing template rendering
- Template variable interpolation

**Notable Tests**:
- `test_embedded_templates()` - Template engine functionality
- `test_embedded_static_assets()` - Asset serving
- `test_directory_listing_template()` - Directory rendering

### 10. **HTTP Streaming Tests** (`http_streaming_test.rs`) ⭐
**Purpose**: HTTP layer streaming functionality for efficient large file handling  
**Test Count**: 2 tests  
**Key Areas**:
- Automatic mode selection based on content size
- Memory vs. disk processing verification
- Small upload memory efficiency (≤1MB)
- Large upload disk streaming (>1MB)
- Resource cleanup and temporary file management
- Performance characteristics across size ranges

**Notable Tests**:
- `test_small_http_body_in_memory()` - Verifies small uploads (512KB) are processed in memory
- `test_large_http_body_streamed_to_disk()` - Verifies large uploads (2MB) are streamed to disk
- Automatic `RequestBody` mode selection testing
- Temporary file creation and cleanup validation
- Memory footprint verification for large uploads

## Shell Script Tests

### 1. **Upload Functionality Test** (`test_upload.sh`)
**Purpose**: End-to-end upload functionality validation  
**Key Areas**:
- GET request to `/upload` endpoint
- POST request file upload
- Single and multiple file uploads
- Invalid request method handling
- Missing Content-Type header detection

### 2. **Large File Upload Test** (`test_1gb_upload.sh`)
**Purpose**: Large file upload capability (1GB+)  
**Key Areas**:
- 1GB+ file upload testing
- Upload verification and integrity checking
- Resource-intensive testing (requires user confirmation)

### 3. **Large File Bash Verification Test** (`large_file_bash_test.rs`) ⭐
**Purpose**: Bash script integration testing for large file uploads  
**Test Count**: 1 test  
**Key Areas**:
- Multi-gigabyte file upload testing (85MB test files)
- Bash script execution and verification
- Memory efficiency validation during large uploads
- Streaming implementation verification
- Real-world upload scenario simulation

**Notable Tests**:
- `test_multiple_large_files_bash_verification()` - Executes bash script with multiple 85MB files
- Verifies streaming implementation prevents memory exhaustion
- Confirms large file uploads complete successfully without hanging
- Tests real-world upload scenarios with actual file I/O
- Large file handling validation

### 3. **Executable Portability Test** (`test_executable_portability.sh`)
**Purpose**: IronDrop executable portability verification  
**Key Areas**:
- Execution from different directories (`/tmp`, home, arbitrary)
- Server response validation
- CSS asset accessibility
- Directory listing functionality
- 404 page handling
- Embedded template resolution

## Test Infrastructure

### Helper Functions and Utilities

#### CLI Configuration Helper
```rust
fn create_cli_config() -> CliConfig {
    CliConfig {
        port: 0,
        directory: temp_dir(),
        auth: None,
        rate_limit: Some(100),
        max_connections: Some(50),
        max_upload_size: Some(10 * 1024 * 1024), // 10MB
    }
}
```

#### Multipart Request Body Creation
```rust
fn create_multipart_body(files: Vec<(&str, &[u8])>) -> (String, String) {
    // Creates RFC 7578 compliant multipart/form-data
    // Returns (boundary, body) tuple
}
```

#### Performance Measurement
```rust
fn measure_upload_time<F>(operation: F) -> Duration 
where F: FnOnce() -> Result<(), Box<dyn std::error::Error>>
{
    // Measures operation execution time
    // Used in performance and stress tests
}
```

### Test Data Management

#### File Size Categories
- **Small Files**: 1KB - 10KB (memory efficiency testing)
- **Medium Files**: 100KB - 1MB (standard processing)
- **Large Files**: 1MB+ (boundary and stress testing)
- **Boundary Files**: Exactly 1MB (edge case testing)

#### Test File Generation
```rust
fn generate_test_file(size: usize) -> Vec<u8> {
    // Generates deterministic test data
    // Ensures consistent test results
}
```

## Security Testing

### Path Traversal Prevention
- Tests for `../` and `..\\` sequences in filenames
- Validates filename sanitization
- Ensures directory escape prevention
- Verifies security header presence

### Input Validation
- Malformed multipart boundary handling
- Missing required headers detection
- Invalid Content-Type processing
- Filename character validation

### Authentication Testing
- Basic authentication mechanism
- Unauthorized access prevention
- Authentication header validation
- Error response consistency

## Performance Benchmarks

### Memory Efficiency Targets
- **Small Files**: <1MB memory overhead for 100 files
- **Medium Files**: <10MB memory overhead for 50 files
- **Large Directories**: <100MB for 10M+ files (ultra-compact mode)

### Performance Thresholds
- **Upload Speed**: >1MB/s for medium files
- **Concurrent Handling**: 10+ simultaneous uploads
- **Search Performance**: <100ms for 10K files
- **Memory Usage**: <11 bytes per file entry (ultra-compact)

### Stress Test Metrics
- **Total Data Processed**: Measured in MB/GB
- **Duration**: Total test execution time
- **Average Time per File**: Per-file processing time
- **Throughput**: Files processed per second

## Test Execution

### Running All Tests
```bash
# Run all Rust tests
cargo test

# Run specific test suites
cargo test --test comprehensive_test
cargo test --test integration_test
cargo test --test edge_case_test
cargo test --test memory_optimization_test
cargo test --test performance_test
cargo test --test stress_test
cargo test --test multipart_test
cargo test --test ultra_compact_test
cargo test --test template_embedding_test
cargo test --test http_streaming_test
cargo test --test large_file_bash_test

# Run shell script tests
./tests/test_upload.sh
./tests/test_1gb_upload.sh
./tests/test_executable_portability.sh
```

### Test Configuration
- **Temporary Directories**: All tests use isolated temp directories
- **Port Allocation**: Dynamic port allocation (port 0) for parallel testing
- **Resource Cleanup**: Automatic cleanup of test files and directories
- **Parallel Execution**: Tests designed for concurrent execution

## Test Maintenance

### Code Quality Standards
- **English-Only**: All test messages and comments use English
- **No Emojis**: Removed all Unicode emojis for consistency
- **Clear Assertions**: Descriptive assertion messages
- **Comprehensive Coverage**: Edge cases and error conditions tested

### Recent Improvements (v2.5)
- Removed all non-ASCII characters and emojis
- Standardized test output messages
- Enhanced filename testing with English special characters
- Improved error message clarity
- Added comprehensive upload system testing

### Future Enhancements
- **Benchmark Integration**: Performance regression detection
- **Coverage Reporting**: Code coverage measurement
- **Load Testing**: Extended stress testing capabilities
- **Security Scanning**: Automated vulnerability testing
- **Integration Testing**: External service integration tests

## Troubleshooting

### Common Test Issues
1. **Port Conflicts**: Tests use dynamic port allocation
2. **File Permissions**: Ensure write access to temp directories
3. **Resource Limits**: Large file tests require sufficient disk space
4. **Timing Issues**: Performance tests may vary based on system load

### Test Environment Requirements
- **Rust**: 1.70+ with cargo
- **Disk Space**: 2GB+ for large file tests
- **Memory**: 4GB+ for stress tests
- **Network**: Localhost access for HTTP tests

This comprehensive testing suite ensures IronDrop's reliability, security, and performance across all supported use cases and deployment scenarios.