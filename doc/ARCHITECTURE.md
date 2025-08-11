# IronDrop Architecture Documentation v2.5

## Overview

IronDrop is a lightweight, high-performance file server written in Rust featuring bidirectional file sharing, modular template architecture, and professional UI design. This document provides a comprehensive overview of the system architecture, component interactions, and implementation details.

## System Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   CLI Parser    │───▶│   Server Init   │───▶│Custom Thread Pool│
│   (cli.rs)      │    │   (main.rs)     │    │   (server.rs)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                        │
                                                        ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Template Engine │◀───│   HTTP Handler  │◀───│  Request Router │
│ (templates.rs)  │    │  (response.rs)  │    │   (http.rs)     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Static Assets  │    │   File System   │    │Upload & Multipart│
│ (templates/*)   │    │    (fs.rs)      │    │upload.rs+multipart│
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │                       │
                                ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Downloads     │    │     Uploads     │    │   Search Engine │
│ Range Requests  │    │ 10GB + Concurrent│    │Ultra-Low Memory │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                        │
                                                        ▼
                                             ┌─────────────────┐
                                             │Security & Monitor│
                                             │ Rate Limit+Stats │
                                             └─────────────────┘
```

## Core Modules

### 1. **Entry Point & Configuration**
- **`main.rs`** (6 lines): Simple entry point that calls `irondrop::run()`
- **`lib.rs`** (56 lines): Library initialization, logging setup, and server bootstrap
- **`cli.rs`** (200+ lines): Command-line interface with comprehensive validation

### 2. **HTTP Processing Layer**
- **`server.rs`**: Custom thread pool implementation with rate limiting
- **`http.rs`**: HTTP request parsing, routing, and static asset serving
- **`response.rs`**: HTTP response building, file streaming, and error handling

### 3. **File Operations**
- **`fs.rs`**: Directory operations and file system interactions
- **`upload.rs`**: Secure file upload handling with atomic operations
- **`multipart.rs`**: RFC 7578 compliant multipart/form-data parser

### 4. **Search System**
- **`search.rs`**: Ultra-low memory search engine with LRU caching and indexing
- **`ultra_compact_search.rs`**: Memory-optimized search implementation for 10M+ entries
- **`ultra_memory_test.rs`**: Search performance testing and benchmarking

### 5. **Template System**
- **`templates.rs`**: Native template engine with variable interpolation
- **`templates/directory/`**: Directory listing templates (HTML, CSS, JS)
- **`templates/upload/`**: File upload templates (HTML, CSS, JS)  
- **`templates/error/`**: Error page templates (HTML, CSS, JS)

### 6. **Support Systems**
- **`error.rs`**: Custom error types and error handling
- **`utils.rs`**: Utility functions and helper methods
- **Monitoring (integrated)**: `/monitor` endpoint (HTML + JSON) implemented inside `http.rs` using `ServerStats` from `server.rs`

## Request Processing Flow

```
                           HTTP Request
                                │
                                ▼
                     ┌─────────────────┐
                     │  Rate Limiting  │───[Fail]───▶ 429 Too Many Requests
                     │     Check       │
                     └─────────────────┘
                                │ [Pass]
                                ▼
                     ┌─────────────────┐
                     │ Authentication  │───[Fail]───▶ 401 Unauthorized
                     │     Check       │
                     └─────────────────┘
                                │ [Pass]
                                ▼
                     ┌─────────────────┐
                     │   Route Type    │
                     │   Detection     │
                     └─────────────────┘
                                │
        ┌───────────┬───────────┼───────────┬───────────┬───────────┐
        │           │           │           │           │           │
        ▼           ▼           ▼           ▼           ▼           ▼
  [Static Assets] [Health] [Upload Routes] [File Sys] [Search API] [Monitor]
        │           │           │           │           │           │
        ▼           ▼           ▼           ▼           ▼           ▼
   Serve CSS/JS  JSON Status Process Upload Path Check Search Engine Dashboard
                                  │           │           │
                         [Pass]   │   [Fail]  │           ▼
                                  ▼           ▼      JSON Results
                            Resource Type  403 Forbidden
                             Detection
                                  │
                    ┌─────────────┼─────────────┐
                    │             │             │
                    ▼             ▼             ▼
              [Directory]       [File]      [Upload UI]
                    │             │             │
                    ▼             ▼             ▼
           Template-based    Stream File   Upload Interface
             Listing          Content        Processing
```

## File Structure

```
src/
├── main.rs              # Entry point (6 lines)
├── lib.rs               # Library initialization (56 lines)
├── cli.rs               # CLI interface with validation (200+ lines)
├── server.rs            # Thread pool + rate limiting (400+ lines)
├── http.rs              # HTTP parsing + routing (600+ lines)
├── templates.rs         # Template engine (300+ lines)
├── fs.rs                # File system operations (200+ lines)
├── response.rs          # Response handling + streaming (400+ lines)
├── upload.rs            # File upload system (500+ lines)
├── multipart.rs         # Multipart parser (661 lines)
├── search.rs            # Ultra-low memory search engine (400+ lines)
├── ultra_compact_search.rs # Memory-optimized search (300+ lines)
├── ultra_memory_test.rs # Search performance testing (200+ lines)
├── error.rs             # Error types (100+ lines)
└── utils.rs             # Utility functions

templates/
├── directory/           # Directory listing UI
│   ├── index.html       # HTML structure
│   ├── styles.css       # Professional blackish-grey theme
│   └── script.js        # Interactive file browsing
├── upload/              # Upload interface
│   ├── page.html        # Standalone upload page
│   ├── form.html        # Reusable upload component
│   ├── styles.css       # Upload styling
│   └── script.js        # Drag-drop + progress tracking
└── error/               # Error pages
    ├── page.html        # Error page structure
    ├── styles.css       # Error styling
    └── script.js        # Error page enhancements

tests/
├── comprehensive_test.rs     # Core server tests (19 tests)
├── integration_test.rs       # Auth + security tests (6 tests)
├── edge_case_test.rs         # Upload edge cases (10 tests)
├── memory_optimization_test.rs # Memory efficiency (6 tests)
├── performance_test.rs       # Upload performance (5 tests)
├── stress_test.rs           # Stress testing (4 tests)
├── multipart_test.rs        # Multipart parser tests (7 tests)
├── ultra_compact_test.rs    # Search engine tests (4 tests)
├── template_embedding_test.rs # Template system tests (3 tests)
├── test_upload.sh           # End-to-end upload testing
├── test_1gb_upload.sh       # Large file upload testing
└── test_executable_portability.sh # Portability validation

Total: 59 tests across 13 test files
```

## Search System Architecture

### Overview
IronDrop features a sophisticated dual-mode search system designed for both efficiency and scalability, with support for directories containing millions of files while maintaining low memory usage.

### Search Implementation Modes

#### 1. **Standard Search Engine (`search.rs`)**
- **Target**: Directories with up to 100K files
- **Memory Usage**: ~10MB for 10K files
- **Features**:
  - LRU cache with 5-minute TTL
  - Thread-safe operations with `Arc<Mutex<>>`
  - Fuzzy search with relevance scoring
  - Real-time indexing with background updates
  - Full-text search with token matching

#### 2. **Ultra-Compact Search (`ultra_compact_search.rs`)**
- **Target**: Directories with 10M+ files
- **Memory Usage**: <100MB for 10M files (11 bytes per entry)
- **Features**:
  - Hierarchical path storage with parent references
  - Unified string pool with binary search
  - Bit-packed metadata (size, timestamps, flags)
  - Cache-aligned structures for CPU optimization
  - Radix-accelerated indexing

### Memory Optimization Techniques

```
Standard Entry (24 bytes):     Ultra-Compact Entry (11 bytes):
┌────────────────────┐        ┌─────────────────┐
│ Full Path (String) │        │ Name Offset (3) │
│ Name (String)      │        │ Parent ID (3)   │
│ Size (u64)         │        │ Size Log2 (1)   │
│ Modified (u64)     │        │ Packed Data (4) │
│ Flags (u32)        │        └─────────────────┘
└────────────────────┘        58% memory reduction
```

### Search Performance Characteristics

| Directory Size | Standard Mode | Ultra-Compact Mode |
|----------------|---------------|-------------------|
| 1K files       | <1ms         | <1ms              |
| 10K files      | 2-5ms        | 1-3ms             |
| 100K files     | 10-20ms      | 5-10ms            |
| 1M files       | N/A          | 20-50ms           |
| 10M files      | N/A          | 100-200ms         |

### Search API Integration

The search system integrates with the HTTP layer through dedicated endpoints:

- **`GET /api/search?q=query`**: Primary search interface
- **Frontend Integration**: Real-time search with 300ms debouncing
- **Result Pagination**: Configurable limits and offsets
- **JSON Response Format**: Structured results with metadata

### Caching Strategy

```
Request → Cache Check → Hit: Return Cached Results
             │
             └─ Miss → Index Search → Cache Store → Return Results
```

- **LRU Eviction**: Least recently used entries removed first
- **TTL Expiration**: 5-minute automatic cache invalidation
- **Memory Bounds**: Maximum 1000 cached queries
- **Thread Safety**: Concurrent read/write operations supported

## Security Architecture

### Defense in Depth

1. **Input Validation Layer**
   - CLI parameter validation with bounds checking
   - HTTP header parsing with malformed request rejection
   - Multipart boundary validation and size limits
   - Filename sanitization and path traversal prevention

2. **Access Control Layer**
   - Optional Basic Authentication with secure credential handling
   - Rate limiting (120 requests/minute per IP, configurable)
   - Connection limiting (10 concurrent per IP, configurable)
   - Extension filtering with glob pattern support

3. **Resource Protection Layer**
   - Request timeouts to prevent resource exhaustion
   - Memory-efficient streaming for large file operations
   - Disk space checking before upload operations
   - Thread pool management with configurable limits

4. **Audit and Monitoring Layer**
   - Comprehensive request logging with unique IDs
   - Performance metrics collection and statistics
   - Health check endpoints (`/_health`, `/_status`)
   - Unified monitoring dashboard (`/monitor`, `/monitor?json=1`)
   - Error tracking and security event logging

### Security Features by Component

| Component | Security Features |
|-----------|------------------|
| **CLI** | Input validation, path traversal prevention, size bounds |
| **HTTP Layer** | Header validation, method restrictions, rate limiting |
| **Upload System** | File validation, atomic operations, extension filtering |
| **Multipart Parser** | Boundary validation, size limits, malformed data rejection |
| **Template System** | Path restrictions, variable escaping, static asset control |
| **File System** | Canonicalization, directory traversal prevention |

## Performance Characteristics

### Memory Usage
- **Baseline**: ~3MB + (thread_count × 8KB stack)
- **Template Cache**: In-memory storage for frequently accessed templates
- **Upload Buffer**: Streaming processing minimizes memory usage
- **File Operations**: Configurable chunk sizes (default: 1KB)

### Concurrent Processing
- **Thread Pool**: Custom implementation (default: 8 threads)
- **Upload Handling**: Supports multiple concurrent uploads
- **Rate Limiting**: Per-IP tracking with automatic cleanup
- **Connection Management**: Efficient file descriptor usage

### Request Latency
| Operation | Typical Latency | Notes |
|-----------|----------------|-------|
| Static Assets | <0.5ms | CSS/JS with caching headers |
| Directory Listing | <2ms | Template rendering with file sorting |
| Health Checks | <0.1ms | JSON status endpoints |
| File Downloads | Variable | Depends on file size and network |
| File Uploads | Variable | Includes validation and atomic writing |
| Error Pages | <1ms | Template-based professional pages |

### Scalability Limits
- **File Size**: Up to 10GB uploads supported
- **Concurrent Users**: Limited by thread pool and system resources
- **Directory Size**: Efficient handling of large directories
- **Template Complexity**: Sub-millisecond variable interpolation

## Template System Architecture

### Template Engine Design

The native template engine provides:
- **Variable Interpolation**: `{{VARIABLE}}` syntax with HTML escaping
- **Static Asset Serving**: Organized CSS/JS delivery via `/_irondrop/static/` routes
- **Modular Templates**: Separated concerns (HTML structure, CSS styling, JS behavior)
- **Caching**: In-memory template storage for performance

### Template Organization

Each template module follows consistent patterns:
- **HTML**: Clean semantic structure with accessibility features
- **CSS**: Professional design with CSS custom properties
- **JavaScript**: Progressive enhancement with graceful degradation

### Asset Pipeline

```
Template Request → Template Engine → Variable Interpolation → HTML Response
                       ↓
Static Asset Request → Asset Router → Direct File Serving → CSS/JS Response
```

## Testing Architecture

### Test Organization
- **Unit Tests**: Individual component testing
- **Integration Tests**: End-to-end functionality verification
- **Security Tests**: Boundary and vulnerability testing
- **Performance Tests**: Load and stress testing scenarios

### Test Coverage by Component
| Test File | Component Coverage | Test Count |
|-----------|-------------------|------------|
| `comprehensive_test.rs` | Core server functionality | 19 |
| `integration_test.rs` | Authentication and security | 6 |
| `upload_integration_test.rs` | Upload system | 29 |
| `multipart_test.rs` | Multipart parser | 7 |
| `debug_upload_test.rs` | Edge cases and debugging | Variable |
| Others | Template system, HTTP handling | 40+ |

### Custom Test Infrastructure
- **Native HTTP Client**: Pure Rust implementation for testing
- **Mock File Systems**: Temporary directories and file operations
- **Concurrent Testing**: Multi-threaded test scenarios
- **Security Validation**: Path traversal and injection testing

## Configuration System

### CLI Configuration
```rust
pub struct Cli {
    directory: PathBuf,           // Required: directory to serve
    listen: String,               // Default: "127.0.0.1"
    port: u16,                    // Default: 8080
    allowed_extensions: String,    // Default: "*.zip,*.txt"
    threads: usize,               // Default: 8
    chunk_size: usize,            // Default: 1024
    verbose: bool,                // Default: false
    detailed_logging: bool,       // Default: false
    username: Option<String>,     // Optional: basic auth
    password: Option<String>,     // Optional: basic auth
    enable_upload: bool,          // Default: false
    max_upload_size: u32,         // Default: 10240 (10GB)
    upload_dir: Option<PathBuf>,  // Optional: custom upload dir
}
```

### Validation Pipeline
1. **Parse-time Validation**: Clap value parsers and constraints
2. **Runtime Validation**: Additional checks during server initialization
3. **Operation Validation**: Per-request validation and security checks

## Error Handling System

### Error Types
```rust
pub enum AppError {
    Io(std::io::Error),
    InvalidMultipart(String),
    InvalidFilename(String),
    PayloadTooLarge(u64),
    UnsupportedMediaType(String),
    InvalidConfiguration(String),
    // ... additional error variants
}
```

### Error Propagation
- **Result Types**: Consistent error handling throughout the application
- **Error Context**: Detailed error messages for debugging
- **User-Friendly Messages**: Professional error pages with guidance
- **Logging Integration**: Error events logged for monitoring

## Deployment Considerations

### Single Binary Deployment
- **Zero Dependencies**: Pure Rust implementation
- **Embedded Assets**: Templates compiled into binary
- **Cross-Platform**: Supports Linux, macOS, and Windows
- **Portable**: Self-contained executable with no external dependencies

### Production Hardening
- **Security Defaults**: Safe defaults with explicit feature activation
- **Resource Limits**: Configurable bounds to prevent abuse
- **Monitoring Integration**: Health endpoints for infrastructure monitoring
- **Graceful Degradation**: Error recovery and fallback mechanisms

### Scalability Options
- **Reverse Proxy**: Can be deployed behind nginx/Apache for additional features
- **Load Balancing**: Multiple instances can serve the same content
- **Resource Scaling**: Configurable thread pool and memory limits
- **Container Deployment**: Docker-friendly single binary

## Future Architecture Considerations

### Potential Enhancements
1. **Database Integration**: Optional metadata storage for advanced features
2. **Plugin System**: Extensible architecture for custom functionality
3. **WebSocket Support**: Real-time features like upload progress
4. **Distributed Storage**: Support for cloud storage backends
5. **Advanced Auth**: OAuth2/OIDC integration for enterprise deployments

### Performance Optimizations
1. **HTTP/2 Support**: Enhanced protocol capabilities
2. **Async I/O**: Tokio integration for improved concurrency
3. **Compression**: Built-in gzip/brotli compression
4. **CDN Integration**: Edge caching and global distribution
5. **Database Caching**: Redis integration for session management

This architecture documentation reflects the current state of IronDrop v2.5 and serves as a foundation for understanding the system's design principles, implementation details, and operational characteristics.