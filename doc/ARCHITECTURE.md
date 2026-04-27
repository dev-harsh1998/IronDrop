# IronDrop Architecture Documentation v2.7.2

## Overview

IronDrop is a file server written in Rust. Its HTTP stack (request parsing, routing, streaming) is implemented in-house without an external HTTP framework, and it uses Tokio for the async runtime and networking. This document provides an overview of the system architecture, component interactions, and implementation details.

## System Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   CLI Parser    │───▶│   Server Init   │───▶│  Tokio Runtime   │
│   (cli.rs)      │    │   (main.rs)     │    │   (server.rs)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                        │
                                                        ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Template Engine │◀───│   HTTP Handler  │◀───│  Request Router │
│ (templates.rs)  │    │  (response.rs)  │    │  (router.rs)    │
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
│ Range Requests  │    │ Direct Streaming │    │Ultra-compact    │
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
- **`config/mod.rs`**: Configuration system with hierarchical precedence (CLI > INI > defaults)
- **`config/ini_parser.rs`**: Zero-dependency INI parser for configuration files

### 2. **HTTP Processing Layer**
- **`server.rs`**: Tokio runtime ownership, async accept loop, TLS via `tokio-rustls`, rate limiting, and statistics
- **`http.rs`**: HTTP request parsing and response streaming
- **`response.rs`**: HTTP response building, MIME type detection, and error page generation
- **`router.rs`**: Simple HTTP router with exact and prefix path matching
- **`handlers.rs`**: Internal route handlers for health checks, status, uploads, and monitoring
- **`middleware.rs`**: Authentication middleware with Basic Auth support

### 3. **File Operations**
- **`fs.rs`**: Directory listing generation and file system interactions
- **`upload.rs`**: Direct upload handler with memory/disk streaming and atomic operations

### 4. **Search System**
- **`search.rs`**: Search system; ultra-compact mode with hierarchical path storage and string pooling

### 5. **Template System**
- **`templates.rs`**: Native template engine with embedded assets and variable interpolation
- **`templates/directory/`**: Directory listing templates (HTML, CSS, JS)
- **`templates/upload/`**: File upload templates (HTML, CSS, JS)  
- **`templates/error/`**: Error page templates (HTML, CSS, JS)
- **`templates/monitor/`**: Monitoring dashboard templates

### 6. **Support Systems**
- **`error.rs`**: Comprehensive error types including upload-specific errors
- **`utils.rs`**: Utility functions for path handling, URL parsing, and encoding

## Request Processing Flow

```
                           HTTP Request
                                │
                                ▼
                     ┌─────────────────┐
                     │  Rate Limiting  │───[Fail]───▶ Connection rejected/closed
                     │     Check       │
                     └─────────────────┘
                                │ [Pass]
                                ▼
                     ┌─────────────────┐
                     │ Authentication  │───[Fail]───▶ 401 Unauthorized
                     │  Middleware     │
                     └─────────────────┘
                                │ [Pass]
                                ▼
                     ┌─────────────────┐
                     │   HTTP Router   │
                     │  (router.rs)    │
                     └─────────────────┘
                                │
        ┌───────────┬───────────┼───────────┬───────────┬───────────┐
        │           │           │           │           │           │
        ▼           ▼           ▼           ▼           ▼           ▼
  [Static Assets] [Health]  [Upload API]  [File Sys]  [Search API] [Monitor]
   /_irondrop/    /_irondrop/health   /_irondrop/   Directory   /_irondrop/  /monitor
    /static/*               /upload       Listing     /search
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
├── main.rs
├── lib.rs
├── cli.rs
├── config/
│   ├── mod.rs
│   └── ini_parser.rs
├── server.rs            # Tokio runtime, async accept, TLS, rate limiting, stats
├── http.rs              # HTTP parsing + async response streaming
├── router.rs            # Routing and middleware pipeline
├── handlers.rs          # Internal route handlers
├── middleware.rs        # Authentication middleware
├── templates.rs         # Template engine with embedded assets
├── fs.rs                # File system operations, lazy metadata, and UI directory pagination
├── response.rs          # Response types and error response helpers
├── upload.rs            # Upload handling + validation
├── multipart.rs         # Multipart form parsing
├── search.rs            # Search subsystem (index + fallback search)
├── ultra_compact_search.rs
├── webdav.rs
├── ultra_memory_test.rs
├── error.rs
└── utils.rs

templates/
├── common/
├── directory/
├── upload/
├── error/
└── monitor/

tests/
└── Integration, upload, monitoring, and WebDAV RFC suites (see `tests/` directory)
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

- **`GET /_irondrop/search?q=query`**: Primary search interface
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

## HTTP Layer Streaming Architecture

### Overview
IronDrop v2.7.1 provides advanced HTTP layer streaming for efficient handling of large file uploads. The system automatically switches between memory-based and disk-based processing based on content size, providing optimal performance and resource utilization.

### RequestBody Architecture

```rust
pub enum RequestBody {
    Memory(Vec<u8>),           // Small uploads (≤64MB)
    File(PathBuf),             // Large uploads (>64MB)
}
```

The `RequestBody` enum provides a unified interface for handling HTTP request bodies of varying sizes:

- **Memory Variant**: Stores small uploads directly in memory for fast processing
- **File Variant**: Streams large uploads to temporary files to prevent memory exhaustion
- **Automatic Selection**: Transparent switching based on configurable size threshold
- **Resource Management**: Automatic cleanup of temporary files with error recovery

### Streaming Decision Flow

```
HTTP Request → Content-Length Check → Size Threshold Comparison
                                              │
                    ┌─────────────────────────┼─────────────────────────┐
                    │ ≤64MB                    │                    >64MB │
                    ▼                         ▼                         ▼
            Memory Processing           Disk Streaming              Disk Streaming
            ┌─────────────────┐        ┌─────────────────┐        ┌─────────────────┐
            │ Read to Vec<u8> │        │ Create Temp File│        │ Stream to Disk  │
            │ Fast Processing │        │ Stream Chunks   │        │ Memory Efficient│
            │ Low Latency     │        │ Auto Cleanup    │        │ Large File Safe │
            └─────────────────┘        └─────────────────┘        └─────────────────┘
                    │                         │                         │
                    └─────────────────────────┼─────────────────────────┘
                                              ▼
                                    Unified Processing
                                    (Multipart Parser)
```

### Performance Characteristics

| Upload Size | Processing Mode | Memory Usage | Disk I/O | Latency |
|-------------|----------------|--------------|----------|---------|
| <1KB        | Memory         | ~1KB         | None     | <1ms    |
| 1KB-64MB     | Memory         | ~Size        | None     | <10ms   |
| 64MB-100MB   | Disk Streaming | ~64KB        | Sequential| <100ms  |
| 100MB-1GB   | Disk Streaming | ~64KB        | Sequential| <1s     |
| 1GB-10GB    | Disk Streaming | ~64KB        | Sequential| <10s    |

### Security and Resource Protection

#### Temporary File Management
- **Creation**: Large request bodies may be streamed to a temporary file in the system temp directory
- **Unique Naming**: Filenames include process ID, timestamp, and a monotonic counter to avoid collisions
- **Cleanup**: Temporary request-body files are removed after request processing completes (best-effort cleanup on errors)

#### Resource Limits
- **Memory Protection**: Prevents memory exhaustion from large uploads
- **Disk Space Monitoring**: Checks available disk space before streaming
- **Concurrent Upload Limits**: Configurable limits on simultaneous uploads
- **Size Validation**: Enforces maximum upload size limits

### Integration with Upload System

The HTTP streaming layer integrates with uploads by representing request bodies as either in-memory buffers or a temporary file on disk (`RequestBody::Memory` / `RequestBody::File { path, size }`). This keeps large uploads bounded in RAM while preserving the same handler behavior.

### Monitoring and Observability

The streaming system provides comprehensive monitoring capabilities:

- **Upload Metrics**: Track memory vs. disk processing ratios
- **Performance Monitoring**: Measure processing times by upload size
- **Resource Usage**: Monitor temporary file creation and cleanup
- **Error Tracking**: Log streaming failures and recovery actions

### Configuration Options

See HTTP_STREAMING.md and CONFIGURATION_SYSTEM.md for the current configuration surface (upload size limits, chunk size, and upload directory).

### Testing Infrastructure

The test suite verifies correct mode selection (memory vs disk), cleanup behavior, and concurrency regressions under streaming load.

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
   - Tokio runtime worker threads and blocking-task isolation with configurable limits

4. **Audit and Monitoring Layer**
   - Comprehensive request logging with unique IDs
   - Performance metrics collection and statistics
   - Health check endpoints (`/_irondrop/health`, `/_irondrop/status`)
   - Unified monitoring dashboard (`/monitor`, `/_irondrop/monitor?json=1`)
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
- **Baseline**: Baseline + Tokio runtime worker threads + template cache
- **Template Cache**: In-memory storage for frequently accessed templates
- **Upload Buffer**: HTTP streaming with automatic memory/disk switching
- **Small Uploads (≤64MB)**: Direct memory processing for optimal performance
- **Large Uploads (>64MB)**: Disk streaming with ~64KB memory footprint
- **File Operations**: Configurable chunk sizes (default: 1KB)

### Concurrent Processing
- **Async Runtime**: Tokio runtime with configurable worker threads (`--threads`)
- **Blocking Isolation**: Filesystem-heavy request handling runs on a blocking pool so network I/O stays responsive
- **Upload Handling**: Supports multiple concurrent uploads
- **Rate Limiting**: Per-IP tracking with automatic cleanup
- **Connection Management**: Efficient file descriptor usage

### Request Latency
| Operation | Typical Latency | Notes |
|-----------|----------------|-------|
| Static Assets | <0.5ms | CSS/JS with caching headers |
| Directory Listing | <2ms | Template rendering with paginated lazy-loaded file metadata |
| Health Checks | <0.1ms | JSON status endpoints |
| File Downloads | Variable | Depends on file size and network |
| File Uploads | Variable | Includes validation and atomic writing |
| Error Pages | <1ms | Template-based professional pages |

### Scalability Limits
- **File Size**: Up to 10GB uploads supported
- **Concurrent Users**: Bounded primarily by OS file descriptors, disk bandwidth, and configured rate limits
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

See TESTING_DOCUMENTATION.md and the `tests/` directory for the current test inventory and categories.

### Custom Test Infrastructure
- **HTTP Clients**: Tests use a mix of raw TCP streams and lightweight HTTP clients
- **Mock File Systems**: Temporary directories and file operations
- **Concurrent Testing**: Multi-threaded test scenarios
- **Security Validation**: Path traversal and injection testing

## Configuration System

### CLI Configuration

The CLI surface (flags, defaults, and config precedence) is documented in CONFIGURATION_SYSTEM.md and the top-level README.md.

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
- **Single Binary**: Self-contained executable with embedded templates and assets
- **Embedded Assets**: Templates compiled into binary
- **Cross-Platform**: Supports Linux, macOS, and Windows
- **Portable**: No runtime services required; uses standard Rust crates (Tokio, rustls) for runtime functionality

### Production Hardening
- **Security Defaults**: Safe defaults with explicit feature activation
- **Resource Limits**: Configurable bounds to prevent abuse
- **Monitoring Integration**: Health endpoints for infrastructure monitoring
- **Graceful Degradation**: Error recovery and fallback mechanisms

### Scalability Options
- **Reverse Proxy**: Can be deployed behind nginx/Apache for additional features
- **Load Balancing**: Multiple instances can serve the same content
- **Resource Scaling**: Configurable Tokio runtime worker threads and rate limits
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
2. **Async Expansion**: Further async conversion of filesystem-heavy operations where beneficial
3. **Compression**: Built-in gzip/brotli compression
4. **CDN Integration**: Edge caching and global distribution
5. **Database Caching**: Redis integration for session management

This architecture documentation reflects the current state of IronDrop v2.7.2 and serves as a foundation for understanding the system's design principles, implementation details, and operational characteristics.
