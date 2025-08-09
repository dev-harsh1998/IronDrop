# IronDrop Architecture Documentation v2.5 (Updated)

## Overview

IronDrop is a lightweight, high-performance file server written in Rust featuring bidirectional file sharing, a hierarchical configuration system, modular template & UI architecture, and professional dark theme design. This document provides a comprehensive overview of the system architecture, component interactions, configuration precedence, and implementation details.

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
│   Downloads     │    │     Uploads     │    │Security & Monitor│
│ Range Requests  │    │ 10GB + Concurrent│    │ Rate Limit+Stats │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Core Modules

### 1. **Entry Point & Configuration**
- **`main.rs`**: Entry point calling `irondrop::run()`
- **`lib.rs`**: Library initialization, logging setup, configuration load, server bootstrap
- **`cli.rs`**: Command-line interface with validation (adds `--config-file` flag)
- **`config/ini_parser.rs`**: Zero‑dependency INI parser (sections, booleans, lists, file sizes)
- **`config/mod.rs`**: Precedence resolver (CLI > INI > defaults) producing strongly typed `Config`

### 2. **HTTP Processing Layer**
- **`server.rs`**: Custom thread pool implementation with rate limiting
- **`http.rs`**: HTTP request parsing, routing, and static asset serving
- **`response.rs`**: HTTP response building, file streaming, and error handling

### 3. **File Operations**
- **`fs.rs`**: Directory operations and file system interactions
- **`upload.rs`**: Secure file upload handling with atomic operations
- **`multipart.rs`**: RFC 7578 compliant multipart/form-data parser

### 4. **Template & UI System**
- **`templates.rs`**: Native template engine with variable interpolation & static asset registry
- **`templates/common/base.css`**: Unified design system (tokens, components, utilities)
- **`templates/directory/`**: Directory listing templates (HTML, CSS, JS)
- **`templates/upload/`**: Upload templates (HTML, CSS, JS, form component)
- **`templates/error/`**: Error templates using new variables (`ERROR_CODE`, `ERROR_MESSAGE`, `ERROR_DESCRIPTION`, `REQUEST_ID`, `TIMESTAMP`)

### 5. **Support Systems**
- **`error.rs`**: Custom error types and error handling
- **`utils.rs`**: Utility functions and helper methods
 - **Monitoring (integrated)**: `/monitor` endpoint (HTML + JSON) implemented inside `http.rs` using `ServerStats` from `server.rs`.

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
        ┌───────────┬───────────┼───────────┬───────────┐
        │           │           │           │           │
        ▼           ▼           ▼           ▼           ▼
  [Static Assets] [Health] [Upload Routes] [File Sys] [API]
        │           │           │           │           │
        ▼           ▼           ▼           ▼           ▼
   Serve CSS/JS  JSON Status Process Upload Path Check Template
                                  │           │      Render
                         [Pass]   │   [Fail]  │
                                  ▼           ▼
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
├── upload_integration_test.rs # Upload system tests (29 tests)
├── multipart_test.rs         # Multipart parser tests (7 tests)
├── debug_upload_test.rs      # Debug tests
├── post_body_test.rs         # POST body handling
└── template_embedding_test.rs # Template system tests
```

## Configuration Architecture

### Precedence Model
Order of resolution (highest first):
1. Explicit CLI flags (non-default values)
2. INI file values (if discovered / specified)
3. Built‑in defaults

### Discovery Order (when `--config-file` not provided)
1. `./irondrop.ini`
2. `./irondrop.conf`
3. `$HOME/.config/irondrop/config.ini`
4. `/etc/irondrop/config.ini` (Unix)

### Normalization Highlights
| Field | CLI Unit | Internal Storage | INI Formats |
|-------|----------|------------------|------------|
| max_upload_size | MB | Bytes (u64) | `500MB`, `1.5GB`, `2048` (bytes) |
| allowed_extensions | Comma string | Vec<String> | Comma list |
| verbose/detailed | Flags | bool | true/false/yes/no/on/off/1/0 |

### Safety
* Upload size bounded (1MB – 10GB default) with overflow avoidance
* Serve directory always sourced from CLI (prevents relocation via config)
* Graceful parse of malformed section headers; strict on empty keys/sections

### Transitional Adapter
`run_server_with_config` converts `Config` → legacy `Cli` struct to minimize internal churn.

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

## Template & UI System Architecture

### Template Engine Design

The native template engine provides:
- **Variable Interpolation**: `{{VARIABLE}}` syntax with HTML escaping (error variables renamed to `ERROR_CODE`, `ERROR_MESSAGE`, `ERROR_DESCRIPTION` + metadata `REQUEST_ID`, `TIMESTAMP`)
- **Static Asset Serving**: Organized CSS/JS delivery via `/_static/` routes
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

## CLI Configuration (Snapshot)
```rust
pub struct Cli {
   pub directory: PathBuf,          // Required: serve root
   pub listen: String,              // Default: 127.0.0.1
   pub port: u16,                   // Default: 8080
   pub allowed_extensions: String,  // Default: "*.zip,*.txt"
   pub threads: usize,              // Default: 8
   pub chunk_size: usize,           // Default: 1024 (bytes)
   pub verbose: bool,               // Debug logging
   pub detailed_logging: bool,      // Info logging
   pub username: Option<String>,    // Basic auth (optional)
   pub password: Option<String>,    // Basic auth (optional)
   pub enable_upload: bool,         // Upload toggle
   pub max_upload_size: u32,        // MB (converted to bytes in Config)
   pub upload_dir: Option<PathBuf>, // Upload target dir (optional)
   pub config_file: Option<String>, // INI path override
}
```

### Validation Layers
1. Parse-time (clap parsers: numeric bounds, path existence for config file)
2. Config assembly (unit conversions, precedence application, list parsing, file size parsing)
3. Request-time (path traversal prevention, extension filtering, auth, rate limits, range validation)

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