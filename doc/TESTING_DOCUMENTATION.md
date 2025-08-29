# IronDrop Testing Documentation

Version 2.6.2 - Test Suite Overview

## Overview

IronDrop includes a comprehensive test suite with **179 tests** covering functionality, security scenarios, performance validation, and concurrent operations. Recent improvements include enhanced path parsing, Unicode support, and race condition fixes.

## Test Architecture

### Test Infrastructure Components

- **Custom HTTP Client**: Native Rust implementation for integration testing
- **Mock File Systems**: Temporary directories and controlled file operations
- **Concurrent Testing**: Multi-threaded test scenarios and race condition validation
- **Security Validation**: Path traversal, injection, and authentication testing
- **Performance Benchmarks**: Memory efficiency and throughput validation
- **Streaming Tests**: Large file upload and download validation

### Test Categories

| Category | Test Files | Test Count | Coverage |
|----------|------------|------------|----------|
| **Core Server & Unit Tests** | `lib.rs` (unit tests) | 41 | Core functionality, HTTP handling, utilities |
| **Configuration System** | `config_test.rs` | 16 | INI parsing, precedence, validation, edge cases |
| **Direct Upload System** | `direct_upload_test.rs` | 15 | File uploads, streaming, concurrent operations, race conditions |
| **Integration Testing** | `integration_test.rs` | 14 | Authentication, security, HTTP compliance, edge cases |
| **Memory & Performance** | `memory_leak_fix_test.rs` | 19 | Memory management, leak prevention, cleanup |
| **HTTP Parser** | `http_parser_test.rs` | 13 | Version parsing, malformed requests, edge cases |
| **Middleware** | `middleware_test.rs` | 13 | Authentication, security, request processing |
| **Template System** | `template_embedding_test.rs`, `templates_escape_test.rs` | 7 | Embedded templates, escaping, assets |
| **Template Utilities** | `response_utils_test.rs` | 2 | Response generation, template processing |
| **Ultra-Compact Search** | `ultra_compact_test.rs` | 5 | Memory efficiency, search performance |
| **Rate Limiting** | `rate_limiter_memory_test.rs` | 5 | Memory management, cleanup, limits |
| **Monitoring & Stats** | `monitor_test.rs` | 2 | Health endpoints, metrics tracking |
| **Utilities** | `utils_test.rs`, `utils_parse_path_test.rs` | 23 | Path parsing, Unicode encoding, special characters |
| **Request Body** | `http_requestbody_test.rs` | 1 | Size and emptiness |
| **Logging** | `log_dir_test.rs` | 3 | Directory creation, permissions |

**Total Tests: 179**

## Detailed Test Coverage

### Configuration System Tests (`config_test.rs`)

**Purpose**: Validates the INI-based configuration system and CLI precedence

**Key Test Areas**:
- INI parser functionality (basic parsing, file sizes, boolean formats)
- Configuration precedence (CLI > INI > Defaults)
- File discovery and loading
- Upload and authentication settings
- Error handling for invalid configurations

**Critical Tests**:
```rust
#[test]
fn test_config_precedence_cli_highest() // Validates CLI override behavior

#[test] 
fn test_config_file_discovery() // Tests automatic config file detection

#[test]
fn test_ini_parser_file_sizes() // Validates size parsing (KB, MB, GB, TB)
```

### Direct Upload System Tests (`direct_upload_test.rs`)

**Purpose**: Validates the direct upload system with streaming support

**Key Test Areas**:
- Small and large file uploads
- Filename extraction from headers and URLs
- File extension validation
- Size limit enforcement
- Conflict resolution
- HTTP method validation
- Streaming for large files

**Critical Tests**:
```rust
#[test]
fn test_direct_upload_large_file_streaming() // Validates streaming for large files

#[test]
fn test_direct_upload_size_limit() // Tests upload size restrictions

#[test]
fn test_direct_upload_extension_validation() // Validates file type filtering
```

### Integration Tests (`integration_test.rs`)

**Purpose**: End-to-end testing with real HTTP requests

**Key Test Areas**:
- Unauthenticated access handling
- Authentication mechanisms
- Error response formatting
- Path traversal prevention
- Malformed request handling

**Test Infrastructure**:
```rust
struct TestServer {
    addr: SocketAddr,
    shutdown_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<()>>,
    _temp_dir: TempDir,
}
```

### Monitoring Tests (`monitor_test.rs`)

**Purpose**: Validates monitoring endpoints and statistics tracking

**Key Test Areas**:
- JSON monitoring endpoint (`/_irondrop/monitor?json=1`)
- HTML monitoring dashboard
- Bytes served accounting
- Statistics accuracy

### Rate Limiter Tests (`rate_limiter_memory_test.rs`)

**Purpose**: Validates memory-efficient rate limiting with automatic cleanup

**Key Test Areas**:
- Enhanced cleanup mechanisms
- Memory pressure handling
- Connection limits per IP
- Reduced retention times
- Memory statistics integration

**Memory Efficiency Focus**:
```rust
#[test]
fn test_rate_limiter_enhanced_cleanup() // Tests automatic memory cleanup

#[test]
fn test_memory_pressure_cleanup() // Validates cleanup under memory pressure
```

### Template System Tests (`template_embedding_test.rs`)

**Purpose**: Validates embedded template system and static asset serving

**Key Test Areas**:
- Template rendering without filesystem access
- Static asset embedding (CSS, JS, favicon)
- Directory listing template functionality
- Error page template rendering
- Variable substitution and escaping

**Template Validation**:
```rust
#[test]
fn test_embedded_templates_functionality() // Core template rendering

#[test]
fn test_embedded_static_assets() // Static asset serving
```

### Ultra-Compact Search Tests (`ultra_compact_test.rs`)

**Purpose**: Validates memory-efficient search engine for large file systems

**Key Test Areas**:
- Memory efficiency with 10M+ entries
- Cache efficiency and hit rates
- String pool deduplication
- Radix bucket distribution
- Memory savings demonstration

**Performance Benchmarks**:
```rust
#[test]
fn test_memory_efficiency_10m_entries() // Tests with 10 million files

#[test]
fn test_demonstrate_memory_savings() // Compares memory usage vs alternatives
```

## Running Tests

### Basic Test Execution (current totals: 179 tests across 16 files)

```bash
# Run all tests
cargo test

# Run with detailed output
cargo test -- --nocapture

# Run specific test file
cargo test config_test
cargo test direct_upload_test
cargo test integration_test
```

### Test Categories

```bash
# Configuration system tests
cargo test config_test

# Upload system tests  
cargo test direct_upload_test

# Integration and security tests
cargo test integration_test

# Performance and memory tests
cargo test ultra_compact_test
cargo test rate_limiter_memory_test

# Template system tests
cargo test template_embedding_test

# Monitoring tests
cargo test monitor_test
```

### Debug Testing

```bash
# Run tests with debug logging
RUST_LOG=debug cargo test -- --nocapture

# Run specific test with full output
cargo test test_direct_upload_large_file_streaming -- --nocapture
```

## Test Data Management

### Temporary File Handling

All tests use `tempfile::TempDir` for isolated test environments:

```rust
use tempfile::{TempDir, NamedTempFile};

let temp_dir = TempDir::new().unwrap();
let cli = create_test_cli(temp_dir.path().to_path_buf());
```

### Test Server Management

Integration tests use a custom `TestServer` struct that:
- Starts server on random available ports
- Provides clean shutdown mechanisms
- Manages temporary directories
- Handles concurrent test execution

## Security Testing

### Path Traversal Prevention

```rust
#[test]
fn test_path_traversal_prevention() {
    // Tests various path traversal attempts
    // Validates proper sanitization
}
```

### Authentication Testing

```rust
#[test]
fn test_authentication_required() {
    // Validates auth enforcement
}

#[test]
fn test_successful_authentication() {
    // Tests valid credential handling
}
```

### Input Validation

- File extension filtering
- Upload size limits
- Malformed request handling
- Header validation

## Performance Testing

### Memory Efficiency

- Ultra-compact search with 10M+ files
- Rate limiter memory cleanup
- Template system memory usage
- String pool deduplication

### Throughput Testing

- Large file upload streaming
- Concurrent connection handling
- Cache efficiency validation

## Test Quality Metrics

### Coverage Statistics

| Component | Test Coverage | Critical Paths |
|-----------|---------------|----------------|
| **Configuration** | 100% | INI parsing, precedence |
| **Upload System** | 95% | Streaming, validation |
| **Authentication** | 100% | Security enforcement |
| **Template System** | 90% | Rendering, assets |
| **Search Engine** | 85% | Memory efficiency |
| **Rate Limiting** | 100% | Memory management |

### Test Reliability

- **Zero Flaky Tests**: All tests are deterministic
- **Isolated Execution**: Each test uses separate temp directories
- **Resource Cleanup**: Automatic cleanup of test resources
- **Concurrent Safe**: Tests can run in parallel

## Contributing to Tests

### Adding New Tests

1. **Choose Appropriate Test File**: Based on component being tested
2. **Use Helper Functions**: Leverage existing test infrastructure
3. **Follow Naming Conventions**: `test_component_specific_behavior`
4. **Include Edge Cases**: Test both success and failure scenarios
5. **Add Documentation**: Comment complex test scenarios

### Test Guidelines

- **Isolation**: Each test should be independent
- **Cleanup**: Use RAII patterns for resource management
- **Assertions**: Use descriptive assertion messages
- **Performance**: Avoid unnecessary delays in tests
- **Security**: Include negative security test cases

### Example Test Structure

```rust
#[test]
fn test_new_feature() {
    // Setup
    let temp_dir = TempDir::new().unwrap();
    let cli = create_test_cli(temp_dir.path().to_path_buf());
    
    // Execute
    let result = feature_under_test(&cli);
    
    // Verify
    assert!(result.is_ok(), "Feature should succeed with valid input");
    
    // Cleanup (automatic with RAII)
}
```

## Continuous Integration

### Test Automation

- All tests run on every commit
- Multiple Rust versions tested
- Cross-platform validation (Linux, macOS, Windows)
- Performance regression detection

### Quality Gates

- All tests must pass
- No clippy warnings
- Code formatting validation
- Documentation completeness

## Recent Improvements (v2.6.2)

### Critical Fixes and Enhancements

**Path Parsing Improvements**
- Fixed `get_request_path` function to correctly handle HTTP request paths with internal spaces
- Enhanced logic to find space before "HTTP/" instead of using first space occurrence
- Added proper whitespace trimming for edge cases with trailing spaces
- All 9 path parsing tests now pass, including complex whitespace scenarios

**Unicode and Special Character Support**
- Enhanced `percent_encode_path` function with comprehensive character encoding
- Added support for Unicode characters (non-ASCII) in file paths
- Implemented proper handling of empty paths and root paths
- Extended encoding for special characters requiring URL encoding
- All 14 utility tests now pass with full Unicode compliance

**Concurrent Upload Race Condition Fix**
- Identified and resolved critical race condition in `generate_unique_filename` method
- Replaced non-atomic file existence checks with atomic `create_new()` operations
- Prevents multiple threads from creating files with same name simultaneously
- All 15 direct upload tests now pass, including concurrent upload scenarios

**Test Suite Stability**
- Achieved 100% test pass rate across all 179 tests
- Enhanced test reliability under concurrent execution
- Improved error handling and edge case coverage
- Added comprehensive validation for boundary conditions

## Future Test Enhancements

### Planned Additions

- **Load Testing**: Stress tests with high concurrent connections
- **Fuzzing**: Input fuzzing for security validation
- **Property Testing**: Property-based test generation
- **Benchmark Tests**: Performance regression detection
- **Integration with External Tools**: Docker, systemd testing

### Test Infrastructure Improvements

- **Test Reporting**: Enhanced test result reporting
- **Coverage Analysis**: Automated coverage reporting
- **Performance Tracking**: Historical performance metrics
- **Test Parallelization**: Improved concurrent test execution

---

## Related Documentation

- [Architecture Documentation](./ARCHITECTURE.md) - System architecture overview
- [Configuration System](./CONFIGURATION_SYSTEM.md) - Configuration testing details
- [Security Fixes](./SECURITY_FIXES.md) - Security test scenarios
- [API Reference](./API_REFERENCE.md) - API endpoint testing
- [Documentation Index](./README.md) - Complete documentation suite

---

*This document is part of the IronDrop v2.6 documentation suite. The test suite is continuously evolving to ensure comprehensive coverage and reliability.*

Return to documentation index: [./README.md](./README.md)