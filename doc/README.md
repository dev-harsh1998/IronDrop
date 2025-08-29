# IronDrop documentation index

IronDrop is a lightweight file server written in Rust. It focuses on:
- stdlib-only networking and file I/O (no external HTTP framework or async runtime)
- a single self-contained binary with embedded templates
- optional uploads with direct-to-disk streaming
- a search engine with an ultra-compact mode suitable for large trees (tested around 10M entries)

## 📚 Documentation Overview

This documentation suite provides complete coverage of IronDrop's architecture, API, deployment, and specialized features. Each document is designed to serve specific audiences and use cases.

Recent updates include direct streaming uploads and the ultra-compact search mode.

## 📖 Core Documentation

### 🏗️ [Architecture Documentation](./ARCHITECTURE.md)
**Audience**: Developers, System Architects, DevOps Engineers  
**Purpose**: Complete system design and component interaction overview

**Contents:**
- System architecture diagrams and component relationships
- Request processing flow and data paths
- Module-by-module code organization (19 Rust source files)
- Ultra-compact search system architecture and memory optimization
- Security architecture and defense-in-depth implementation
- Performance characteristics and scalability considerations
- Template system design and asset pipeline
- Testing architecture with 125 comprehensive tests across 16 test files

**Key Sections:**
- Core module breakdown with line counts and responsibilities
- HTTP request processing pipeline with security checkpoints
- Dual-mode search engine implementation and ultra-compact optimization
- Template engine implementation and static asset serving
- Error handling system and custom error types
- Future architecture considerations and enhancement opportunities

### 🔌 [API Reference](./API_REFERENCE.md)
**Audience**: Frontend Developers, API Consumers, Integration Teams  
**Purpose**: Complete REST API specification with examples

**Contents:**
- All HTTP endpoints with parameters and response formats
- Authentication and authorization mechanisms
- Rate limiting and security headers
- Upload API with direct binary streaming
- **Search API endpoints** with ultra-compact search integration
- Health monitoring and status endpoints
- Error response formats and HTTP status codes

**Key Features:**
- Directory listing API (HTML and JSON responses)
- File download with range request support
- File upload system with progress tracking
- **Advanced search API** supporting massive directories (10M+ files)
- Health check and monitoring endpoints
- Static asset serving for templates
- Comprehensive client integration examples (JavaScript, cURL, Python)

### 🚀 [Deployment Guide](./DEPLOYMENT.md)
**Audience**: DevOps Engineers, System Administrators, Production Teams  
**Purpose**: Production deployment strategies and operational best practices

**Contents:**
- Single binary and containerized deployment options
- systemd service configuration with security hardening
- Docker and Docker Compose deployment examples
- Reverse proxy configuration (nginx and Apache)
- Monitoring, logging, and observability setup
- Security hardening and backup/recovery procedures

**Key Sections:**
- Production-ready systemd service with resource limits
- Docker multi-stage build with Alpine Linux base
- nginx/Apache reverse proxy with SSL/TLS and security headers
- Prometheus metrics planning and log management
- Performance tuning and system optimization
- Comprehensive troubleshooting guide

## 🔧 Specialized Component Documentation

### 🧩 [Configuration System](./CONFIGURATION_SYSTEM.md)
Hierarchical configuration (CLI > INI > defaults) with zero‑dep INI parser, secure size parsing, auth provisioning, deterministic startup.

### 🎨 [Template & UI System](./TEMPLATE_SYSTEM.md)
Native zero-dependency template engine: variables, conditionals, embedded assets, security model, theming & roadmap.

### 🧪 [Testing Documentation](./TESTING_DOCUMENTATION.md) ⭐
**Audience**: Developers, QA Engineers, DevOps Teams  
**Purpose**: Comprehensive testing suite documentation and validation procedures

**Contents:**
- **Complete Test Coverage**: 179 tests across 16 test files covering all functionality
- **Test Categories**: Core server, integration, edge cases, memory optimization, performance, stress testing, streaming
- **Security Testing**: Path traversal prevention, input validation, authentication mechanisms
- **Performance Benchmarks**: Memory efficiency targets, upload speed thresholds, stress test metrics
- **Shell Script Tests**: End-to-end validation, large file uploads, executable portability
- **Streaming Tests**: HTTP layer streaming validation and large file bash integration tests
- **Test Infrastructure**: Helper functions, data management, execution procedures

**Implementation Status**: ✅ **Production Ready** (v2.6)
- **English-Only Testing**: All test messages and output standardized to English
- **Comprehensive Coverage**: Edge cases, security scenarios, performance validation, and streaming functionality
- **Memory Optimization Tests**: Ultra-compact search engine validation for 10M+ files
- **Upload System Tests**: Complete file upload functionality with streaming support
- **Multipart Parser Tests**: RFC 7578 compliance validation with streaming implementation
- **Large File Testing**: Bash integration tests for multi-gigabyte file uploads
- **Performance Validation**: Stress testing with throughput and memory efficiency metrics

### 📤 [Upload Integration Guide](./UPLOAD_INTEGRATION.md)
**Audience**: Frontend Developers, UI/UX Implementers  
**Purpose**: Modern upload UI system implementation details

**Contents:**
- Professional drag-and-drop upload interface
- Template integration with blackish-grey theme
- JavaScript upload manager with progress tracking
- Multi-file concurrent upload handling
- Client-side validation and error handling

**Implementation Status**: ✅ **Production Ready** (v2.5)
- Complete upload system with 29 comprehensive tests
- Professional UI matching IronDrop's design language
- Integrated with template engine and security systems
- Supports unlimited file uploads with direct streaming architecture

### 🛡️ [Security Fixes Documentation](./SECURITY_FIXES.md)
**Audience**: Security Engineers, DevOps Teams, Compliance Officers  
**Purpose**: Security vulnerability fixes and implementation details

**Contents:**
- OWASP vulnerability remediation (A01:2021, A05:2021)
- Path traversal protection and input validation
- Upload size validation and resource protection
- CLI configuration security enhancements
- Defense-in-depth implementation details

**Security Status**: ✅ **Fully Implemented** (v2.5)
- Comprehensive input validation at multiple layers
- System directory blacklisting and write permission checks
- Direct streaming with unlimited file size support
- Integration with core systems for consistent security
- Extensive test coverage for security scenarios

### 🔄 [Direct Upload System Documentation](./MULTIPART_README.md)
**Audience**: Backend Developers, System Integrators  
**Purpose**: Direct binary upload system architecture and benefits

**Contents:**
- Memory-efficient direct streaming implementation
- Constant memory usage regardless of file size
- Security validations and filename handling
- Configuration options and customization
- Comprehensive API usage examples

**Implementation Status**: ✅ **Production Ready** (v2.5.1)
- RFC 7578 compliance with robust boundary detection and streaming support
- Advanced streaming implementation for memory-efficient large file processing
- 7+ dedicated test cases covering edge cases and streaming scenarios
- Integrated with upload handler and HTTP processing with automatic mode selection
- Zero external dependencies with pure Rust implementation
- Prevents memory exhaustion for multi-gigabyte file uploads

### 🌊 [HTTP Layer Streaming Documentation](./HTTP_STREAMING.md) ⭐
**Audience**: Backend Developers, System Architects, Performance Engineers  
**Purpose**: Advanced HTTP layer streaming implementation for efficient large file handling

**Contents:**
- **Automatic Streaming Logic**: Smart switching between memory and disk storage based on content size
- **RequestBody Architecture**: Unified interface for memory and file-based request bodies
- **Performance Optimization**: Memory-efficient handling with constant ~7MB RAM usage
- **Resource Management**: Automatic temporary file cleanup and error recovery
- **Security Features**: Secure temporary file creation and resource protection
- **Integration Guide**: Seamless integration with existing upload handlers
- **Testing Framework**: Comprehensive test coverage with dedicated HTTP streaming tests

**Implementation Status**: ✅ **Production Ready** (v2.5)
- **Automatic Mode Selection**: ≤1MB in memory, >1MB streamed to disk
- **Zero Configuration**: Works transparently with existing upload handlers
- **Resource Protection**: Prevents memory exhaustion from large uploads
- **Backward Compatibility**: Existing code continues to work without modification
- **Comprehensive Testing**: Dedicated HTTP streaming tests verify correct behavior
- **Security Hardening**: Secure temporary file handling with automatic cleanup
- **Performance Optimized**: Consistent performance regardless of upload size

### 📊 [Monitoring Guide](./MONITORING.md)
**Audience**: Operators, Observability Engineers, SREs  
**Purpose**: Details on `/monitor`, health endpoints, data model, integration patterns

**Contents:**
- `/monitor` HTML dashboard behavior and refresh model
- `/monitor?json=1` schema and field semantics
- Health vs status endpoint differences
- Example automation + jq scraping patterns
- Extensibility roadmap (Prometheus, per-endpoint stats)

### 🔍 [Search Feature Documentation](./SEARCH_FEATURE.md) ⭐
**Audience**: Frontend Developers, Backend Engineers, System Architects  
**Purpose**: Comprehensive search functionality implementation details

**Contents:**
- **Dual-Mode Search Engine**: Standard mode for <100K files, ultra-compact mode for 10M+ files
- **Memory-Optimized Architecture**: <100MB memory usage for 10M+ files (11 bytes per entry)
- **Ultra-Compact Implementation**: Hierarchical path storage with parent references instead of full paths
- **String Pool Optimization**: Unified string storage with binary search for massive memory savings
- **Radix-Accelerated Indexing**: Cache-aligned structures for CPU optimization
- Real-time frontend search interface with debounced input
- RESTful search API with JSON responses
- Performance optimization and scalability considerations
- Security implementation and access control
- Configuration options and troubleshooting guide

**Implementation Status**: ✅ **Production Ready** (v2.5)
- **Standard Search Engine**: Thread-safe search with LRU caching (5-minute TTL)
- **Ultra-Compact Search Engine**: Memory-optimized for massive directories (10M+ files)
- **Automatic Mode Selection**: Transparent switching based on directory size
- **Memory Efficiency**: 58% memory reduction through bit-packing and hierarchical storage
- Real-time client-side search with 300ms debouncing
- Comprehensive test coverage including template integration
- Support for unlimited indexed files with 20-level directory depth
- Accessibility-compliant UI with keyboard navigation support
- Performance testing and benchmarking infrastructure

**🎉 NEW in v2.6**: Revolutionary direct streaming upload system with **unlimited file size support**, constant memory usage (~7MB), and simplified binary upload architecture.

---

## 🚀 Key Features

### 🎨 **Modern Web Interface**
- **Professional Blackish-Grey UI** – Clean, corporate-grade design with sophisticated glassmorphism effects
- **Modular Template System** – Organized HTML/CSS/JS architecture with variable interpolation
- **Static Asset Serving** – Efficient delivery of stylesheets and scripts via `/_irondrop/static/` routes
- **Responsive Design** – Mobile-friendly interface with adaptive layouts

### 🔐 **Advanced Security & Monitoring**
- **Rate Limiting** – DoS protection with configurable requests per minute and concurrent connections per IP
- **Server Statistics** – Real-time monitoring of requests, bytes served, uptime, and performance metrics
- **Health Check Endpoints** – Built-in `/_health` and `/_status` endpoints for monitoring
- **Unified Monitoring Dashboard** – NEW `/monitor` endpoint with live HTML dashboard and JSON API (`/monitor?json=1`) exposing request, download and upload metrics
- **Path-Traversal Protection** – Canonicalises every request path and rejects any attempt that escapes the served directory
- **Optional Basic Authentication** – Username and password can be supplied via CLI flags

### 📁 **Bidirectional File Management** ⭐
- **Enhanced Directory Listing** – Beautiful table-based layout with file type indicators and sorting
- **Secure File Downloads** – Streams large files efficiently, honours HTTP range requests, and limits downloads to allowed extensions with glob support
- **Production-Ready File Uploads** – Secure, configurable file uploads with **unlimited size support** using direct streaming architecture, extension filtering, and filename sanitization
- **Upload UI Integration** – Professional web interface for file uploads with drag-and-drop support and progress indicators
- **Concurrent Upload Handling** – Thread-safe processing of multiple simultaneous uploads with atomic file operations
- **MIME Type Detection** – Native file type detection for proper Content-Type headers
- **File Type Visualization** – Color-coded indicators for different file categories

### 🔍 **Advanced Search System** ⭐
- **Dual-Mode Search Engine** – Standard mode for <100K files, ultra-compact mode for 10M+ files
- **Memory-Optimized Architecture** – <100MB memory usage for 10M+ files (11 bytes per entry)
- **Real-Time Search** – Client-side search with 300ms debouncing and fuzzy matching
- **RESTful Search API** – `/api/search` endpoint with pagination and relevance scoring
- **LRU Caching** – 5-minute TTL cache with automatic cleanup for improved performance
- **Hierarchical Path Storage** – Parent references instead of full paths for massive memory savings
- **Thread-Safe Operations** – Concurrent search operations with background indexing

### ⚡ **Performance & Architecture**
- **Custom Thread Pool** – Native implementation without external dependencies for optimal performance
- **Comprehensive Error Handling** – Professional error pages with consistent theming and user-friendly messages
- **Request Timeout Protection** – Prevents resource exhaustion with configurable timeouts
- **Rich Logging** – Each request is tagged with unique IDs and logged at multiple verbosity levels

### 🛠️ **Zero External Dependencies**
- **Pure Rust Implementation** – Networking, HTTP parsing, and template rendering using only Rust's standard library
- **Custom HTTP Client** – Native testing infrastructure without external HTTP libraries
- **Native Template Engine** – Variable interpolation and rendering without template crates
- **Built-in MIME Detection** – File type recognition without external MIME libraries

---

## 📋 Requirements

| Tool                    | Minimum Version | Purpose                   |
|-------------------------|-----------------|---------------------------|
| Rust                    | 1.88            | Compile the project       |
| Cargo                   | Comes with Rust | Dependency management     |
| Linux / macOS / Windows | –               | Runtime platform support |

---

## 🛠️ Installation

### Build from Source

```bash
# Clone the repository
git clone https://github.com/dev-harsh1998/IronDrop.git
cd IronDrop

# Build in release mode
cargo build --release
```

The resulting binary is `target/release/irondrop`; move it into any directory on your `$PATH`.

```bash
sudo mv target/release/irondrop /usr/local/bin/
```

### Windows

```powershell
move target\release\irondrop.exe C:\Tools\
```

---

## 🚦 Quick Start

Serve the current directory on the default port:

```bash
# Basic download server with search
irondrop -d .

# Enable file uploads with default settings
irondrop -d . --enable-upload

# Customize upload configuration with 5GB limit
irondrop -d . --enable-upload --max-upload-size 5120 --upload-dir /path/to/uploads
```

Open a browser at [http://127.0.0.1:8080](http://127.0.0.1:8080) and you will see the auto-generated directory index with built-in search functionality.

---

## 🎉 What's New in v2.5

### 📤 **Complete File Upload System**
IronDrop v2.5 introduces a **production-ready file upload system** with enterprise-grade features:

- **🔒 Enhanced Security**: Comprehensive input validation, boundary verification, and filename sanitization
- **⚡ Performance**: Handles unlimited file sizes with constant memory usage and concurrent processing
- **🎨 Professional UI**: Integrated upload interface accessible at `/upload` with real-time feedback
- **🛡️ Robust Validation**: Multi-layer security including extension filtering, size limits, and malformed data rejection
- **🧪 Battle-Tested**: 179 tests across 16 test files covering edge cases, security scenarios, and performance stress testing

### 🔍 **Advanced Search System** (New in v2.5)
IronDrop v2.5 introduces a **dual-mode search engine** optimized for directories of any size:

- **🚀 Ultra-Low Memory**: <100MB for 10M+ files using 11-byte entries and hierarchical storage
- **⚡ Lightning Fast**: Real-time search with 300ms debouncing and LRU caching
- **🎯 Smart Search**: Fuzzy matching, relevance scoring, and automatic mode selection
- **🔧 RESTful API**: `/api/search` endpoint with pagination and JSON responses
- **🔄 Thread-Safe**: Concurrent operations with background indexing

### 📊 **Integrated Monitoring Dashboard** (Added in v2.5)
The new `/monitor` endpoint provides both an HTML dashboard and a JSON API for tooling integration. It auto-updates in the browser and can be scraped by observability agents.

Example JSON (`GET /monitor?json=1`):

```json
{
   "requests": {
      "total": 42,
      "successful": 40,
      "errors": 2,
      "bytes_served": 1048576,
      "uptime_secs": 360
   },
   "downloads": {
      "bytes_served": 1048576
   },
   "uploads": {
      "total_uploads": 5,
      "successful_uploads": 5,
      "failed_uploads": 0,
      "files_uploaded": 7,
      "upload_bytes": 5242880,
      "average_upload_size": 748982,
      "largest_upload": 2097152,
      "concurrent_uploads": 0,
      "average_processing_time": 152.4,
      "success_rate": 100.0
   }
}
```

HTML Dashboard (`GET /monitor`):
- Lightweight embedded template (no external assets) served with caching disabled for freshness
- Auto-refresh JavaScript polling (`?json=1`) to update counters
- Shows cumulative bytes served (downloads) and upload metrics side-by-side

Use cases:
- Local debugging of throughput
- Basic operational visibility without external APM
- Simple integration point for external monitoring (curl + jq / cron)

Planned extensions (open to contribution):
- Active connection count
- Per-endpoint breakdown & rolling window rates
- Exporter mode (Prometheus/OpenMetrics formatting)

### 🏗️ **Architecture Improvements**
- **Enhanced Multipart Parser**: Robust RFC-compliant parsing with streaming support
- **Improved Error Handling**: Graceful handling of malformed requests and resource exhaustion
- **Better Concurrency**: Thread-safe file operations with unique filename generation
- **Security Hardening**: Enhanced validation layers and attack prevention

---

## 🎛️ Friendly CLI Reference

| Flag                 | Alias | Description                        | Default         |
|----------------------|-------|------------------------------------|-----------------|
| `--directory`        | `-d`  | Directory to serve (required)      | –               |
| `--listen`           | `-l`  | Bind address                       | `127.0.0.1`     |
| `--port`             | `-p`  | TCP port                           | `8080`          |
| `--allowed-extensions` | `-a`| Comma-separated glob patterns      | `*.zip,*.txt`   |
| `--threads`          | `-t`  | Thread-pool size                   | `8`             |
| `--chunk-size`       | `-c`  | File read buffer in bytes          | `1024`          |
| `--username`         | –     | Basic-auth user                    | none            |
| `--password`         | –     | Basic-auth password                | none            |
| `--verbose`          | `-v`  | Debug-level logs                   | `false`         |
| `--detailed-logging` | –     | Info-level logs                    | `false`         |
| `--enable-upload`    | –     | Enable file upload functionality   | `false`         |
| `--max-upload-size`  | –     | Maximum upload file size in MB     | `unlimited`     |
| `--upload-dir`       | –     | Target directory for uploaded files| OS Download Dir |

### Practical Examples

| Scenario | Command | Features |
|----------|---------|----------|
| **Public File Share** | `irondrop -d /srv/files -p 3000 -l 0.0.0.0` | Professional UI, rate limiting, health monitoring |
| **Document Repository** | `irondrop -d ./docs -a "*.pdf,*.png,*.jpg"` | Filtered downloads, file type indicators |
| **High-Performance Server** | `irondrop -d ./big -t 16 -c 8192` | Custom thread pool, optimized streaming |
| **Secure Corporate Share** | `irondrop -d ./private --username alice --password s3cret` | Authentication, audit logging, professional design |
| **Development Server** | `irondrop -d . -v --detailed-logging` | Debug logging, template development, hot reload |
| **Production Monitoring** | `irondrop -d /data -l 0.0.0.0` + health checks at `/_health` | Statistics, uptime monitoring, rate limiting |
| **Monitoring Dashboard** | `irondrop -d .` then visit `/monitor` | Live HTML + JSON metrics |
| **Secure Upload Server** | `irondrop -d ./shared --enable-upload --max-upload-size 5120 -a "*.txt,*.pdf,*.jpg"` | Controlled file uploads up to 5GB, extension filtering |
| **Corporate File Share** | `irondrop -d /data --enable-upload --upload-dir /data/uploads --username admin` | Authenticated uploads, custom upload directory |

---

## 📤 File Upload Features

IronDrop provides secure, configurable file upload capabilities:

### Upload Configuration
- **Enable/Disable Uploads**: Control upload functionality via CLI
- **Maximum Upload Size**: Configurable size limit (default: unlimited)
- **Flexible Upload Directory**: 
  - Default: OS-specific download directory
  - Customizable via `--upload-dir`
- **Security Controls**:
  - File extension filtering
  - Size limit enforcement
  - Path traversal prevention
  - Filename sanitization

### Upload Endpoints
- **Web Upload**: Interactive `/upload` page with professional UI
- **API Upload**: RESTful upload with JSON/HTML responses
- **Multipart Form Support**: Standard file upload mechanisms

### Upload Workflow
1. Select files to upload
2. Files validated against:
   - Allowed extensions
   - File size limits
   - Safe filename rules
3. Unique filename generation
4. Atomic file writing
5. Detailed upload statistics

### Example Use Cases
- **Personal File Sharing**: Quick, secure file transfers
- **Temporary File Storage**: Controlled upload environments
- **Development Servers**: Flexible file management

---

## 🏗️ Architecture Overview

The codebase features a **modular template architecture** with clear separation of concerns. Core modules include `server.rs` for the custom thread-pool listener, `http.rs` for request parsing and static asset serving, `upload.rs` for secure file upload handling, `multipart.rs` for RFC-compliant multipart parsing, `search.rs` and `ultra_compact_search.rs` for the dual-mode search system, `templates.rs` for the native template engine, `fs.rs` for directory operations, and `response.rs` for file streaming and error handling. The `templates/` directory contains organized HTML/CSS/JS assets for directory listing, uploads, and search interfaces.

### System Architecture Flow

```
    +-------------------+       +------------------+       +-------------------+
    |   CLI Parser      | ----> |   Server Init    | ----> |Custom Thread Pool |
    |   (cli.rs)        |       |   (main.rs)      |       |   (server.rs)     |
    +-------------------+       +------------------+       +-------------------+
                                                                      |
                                                                      v
    +-------------------+       +------------------+       +-------------------+
    | Template Engine   | <---- |   HTTP Handler   | <---- |  Request Router   |
    | (templates.rs)    |       |  (response.rs)   |       |   (http.rs)       |
    +-------------------+       +------------------+       +-------------------+
             |                           |                           |
             v                           v                           v
    +-------------------+       +------------------+       +-------------------+
    |  Static Assets    |       |   File System    |       |Upload & Multipart |
    | (templates/*.css) |       |    (fs.rs)       |       | upload.rs+multipart|
    +-------------------+       +------------------+       +-------------------+
                                         |                           |
                                         v                           v
    +-------------------+       +------------------+       +-------------------+
    |   Downloads       |       |     Uploads      |       |   Search Engine   |
    | Range Requests    |       | Unlimited + Direct|       |Ultra-Low Memory|
    +-------------------+       +------------------+       +-------------------+
                                                                     |
                                                                     v
                                                           +-------------------+
                                                           |Security & Monitor |
                                                           | Rate Limit+Stats  |
                                                           +-------------------+
```

### Request Processing Flow

```
                                   HTTP Request
                                        |
                                        v
                             +---------------------+
                             |   Rate Limiting     |  --[Fail]--> 429 Too Many Requests
                             |      Check          |
                             +---------------------+
                                        | [Pass]
                                        v
                             +---------------------+
                             |   Authentication    |  --[Fail]--> 401 Unauthorized
                             |       Check         |
                             +---------------------+
                                        | [Pass]
                                        v
                             +---------------------+
                             |     Route Type      |
                             |     Detection       |
                             +---------------------+
                                        |
                    +-------------------+-------------------+-------------------+
                    |                   |                   |                   |
                    v                   v                   v                   v
            [Static Assets]      [Health Check]        [Upload Routes]      [File System]
                    |                   |                   |                   |
                    v                   v                   v                   v
            Serve CSS/JS         JSON Status         Process Uploads      Path Safety Check
                                                             |
                                                     [Pass]  |  [Fail]
                                                             v     |
                                                   Resource Type   |
                                                    Detection      |
                                                         |         |
                                              +----------+---------+----> 403 Forbidden
                                              |                    |
                                              v                    v
                                       [Directory]             [File]
                                              |                    |
                                              v                    v
                                  Template-based Listing   Stream File Content
                                              |                    |
                                              v         +----------+----------+
                                     Professional UI    |                     |
                                     (Blackish Grey)    v                     v
                                                   [Range Request]    [Full Request]
                                                        |                     |
                                                        v                     v
                                                 Partial Content       Complete File
```

---

## 📦 Project Layout

```
src/
├── main.rs          # Entry point
├── lib.rs           # Logger + CLI bootstrap
├── cli.rs           # Command-line definitions
├── server.rs        # Custom thread pool + rate limiting + statistics
├── http.rs          # HTTP parsing, routing & static asset serving
├── templates.rs     # Native template engine with variable interpolation
├── fs.rs            # Directory operations + template-based listing
├── response.rs      # File streaming + template-based error pages
├── upload.rs        # File upload handling + multipart processing
├── multipart.rs     # Multipart form data parsing
├── error.rs         # Custom error enum
└── utils.rs         # Helper utilities

templates/
├── directory/       # Directory listing templates
│   ├── index.html   # Clean HTML structure
│   ├── styles.css   # Professional blackish-grey design
│   └── script.js    # Enhanced interactions + file type detection
├── upload/          # File upload templates
│   ├── form.html    # Upload form structure
│   ├── page.html    # Upload page layout
│   ├── styles.css   # Upload UI styling
│   └── script.js    # Upload functionality
└── error/           # Error page templates
    ├── page.html    # Error page structure
    ├── styles.css   # Consistent error styling
    └── script.js    # Error page enhancements

tests/
├── comprehensive_test.rs  # 13 comprehensive tests with custom HTTP client
└── integration_test.rs    # 6 integration tests for core functionality

assets/
├── error_400.dat   # Legacy error assets (now template-based)
├── error_403.dat
└── error_404.dat
```

**Architecture Highlights:**
- **Modular Templates**: Organized separation of HTML/CSS/JS with native rendering
- **Zero Dependencies**: Pure Rust implementation without external HTTP or template libraries
- **Professional UI**: Corporate-grade blackish-grey design with glassmorphism effects
- **Comprehensive Testing**: 19 total tests including custom HTTP client for static assets

Every module is documented and formatted with `cargo fmt` and `clippy -- -D warnings` to keep technical debt at zero.

---

## 🧪 Testing

### Comprehensive Test Suite

The project includes **179 comprehensive tests across 16 test files** covering all aspects of functionality, with complete upload system validation:

```bash
# Run all tests (covers upload, download, security, concurrency)
cargo test

# Run with detailed output
cargo test -- --nocapture

# Run specific test suites
cargo test comprehensive_test    # Core server functionality (19 tests)
cargo test integration_test      # Authentication & security (6 tests)  
cargo test upload_integration_test # Upload functionality (29 tests)
cargo test debug_upload_test     # Multipart parser (7 tests)
```

### Test Architecture

**Custom HTTP Client**: Tests use a native HTTP client implementation (zero external dependencies) that directly connects via `TcpStream` to verify:

- **Bidirectional File Operations**: Upload and download functionality with unlimited size support
- **Multipart Processing**: RFC-compliant parsing with boundary detection and validation
- **Template System**: Modular HTML/CSS/JS serving for both download and upload interfaces
- **Security Validation**: Input sanitization, boundary verification, extension filtering
- **Concurrency Handling**: Multiple simultaneous uploads with thread safety
- **Error Scenarios**: Malformed data rejection, resource exhaustion protection
- **Authentication**: Secure upload/download with basic auth integration
- **HTTP Compliance**: Headers, status codes, and protocol adherence across all endpoints

### Test Coverage

| Test Category | Count | Description |
|---------------|-------|-------------|
| **Upload System** | 29 | Single/multi-file uploads, unlimited size support, concurrency, validation |
| **Core Server** | 19 | Directory listing, error pages, security, authentication |
| **Multipart Parser** | 7 | Boundary detection, content extraction, validation |
| **Security** | 12+ | Authentication, rate limiting, path traversal, input validation |
| **File Operations** | 15+ | Downloads, uploads, MIME detection, atomic operations |
| **Monitoring** | 8+ | Health checks, statistics, performance tracking |
| **UI & Templates** | 10+ | Upload/download interfaces, error pages, responsive design |

Tests start the server on random ports and issue real HTTP requests to verify both functionality and integration.

---

## 🛠️ Development

Developers can launch the server with live `debug` logs by exporting `RUST_LOG=debug` before running `cargo run`.

### Development Workflow

1. **Setup Development Environment**
   ```bash
   git clone https://github.com/dev-harsh1998/IronDrop.git
   cd IronDrop
   cargo build
   ```

2. **Run with Debug Logging**
   ```bash
   RUST_LOG=debug cargo run -- -d ./test-files -v
   ```

3. **Format and Lint**
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   ```

4. **Run Tests**
   ```bash
   cargo test
   ```

---

## 👥 Contributors & Test Coverage Initiative

### Current Contributors

We're proud to acknowledge our contributors who have helped make IronDrop a reliable and feature-rich project:

| Name              | GitHub Profile | Primary Contributions                            |
|-------------------|----------------|--------------------------------------------------|
| **Harshit Jain**  | [@dev-harsh1998](https://github.com/dev-harsh1998) | Project founder, core architecture, main development |
| **Sonu Kumar Saw** | [@dev-saw99](https://github.com/dev-saw99)         | Code improvements and enhancements              |

> **Want to see your name here?** We actively welcome new contributors! Your name will be added to this list after your first merged pull request.

### 🧪 **Test Coverage & Quality Initiative**

**We strongly believe that robust testing is the foundation of reliable software.** To maintain and improve the quality of IronDrop, we have a special focus on test coverage and encourage all contributors to prioritize testing.

#### 🎯 **What We're Looking For:**

1. **Test Cases for New Features** - Every new feature or bug fix should include corresponding test cases
2. **Test Cases for Existing Code** - We welcome PRs that only add tests for existing functionality
3. **Integration Tests** - Tests that verify end-to-end functionality
4. **Edge Case Testing** - Tests that cover error conditions, boundary conditions, and security scenarios

#### 💡 **Easy Ways to Contribute:**

**For Code Contributors:**
- Add at least one test case for every PR you submit
- Include both positive and negative test scenarios
- Test error handling and edge cases
- Document your test strategy in the PR description

**For Test-Only Contributors:**
- Submit PRs that **only add test cases** for existing features
- Look for untested code paths in our current codebase
- Add regression tests for previously reported issues
- Improve test coverage for security features (authentication, path traversal protection)

#### **Current Testing Areas That Need Help:**

- Range request handling edge cases
- Authentication bypass attempts
- File extension filtering with complex glob patterns
- Error page generation under various conditions
- Concurrent connection stress testing
- Memory usage under high load

---

## 🤝 Contribution Guide

We love new ideas! Follow these simple steps to join the party:

### **Step-by-Step Process:**

1. **Fork** the repository and create your feature branch:
   ```bash
   git checkout -b feature/your-improvement
   # or for test-only contributions:
   git checkout -b tests/add-authentication-tests
   ```

2. **Make your changes** and **add tests** (this is crucial!):
   - For new features: implement both the feature and its tests
   - For test-only contributions: focus on comprehensive test coverage
   - For bug fixes: add a test that reproduces the bug, then fix it

3. **Run the full test suite** and formatting tools:
   ```bash
   cargo test
   cargo fmt && cargo clippy -- -D warnings
   ```

4. **Commit with descriptive messages:**
   ```bash
   git commit -m "feat: add timeout handling for downloads"
   # or
   git commit -m "test: add comprehensive tests for basic auth"
   ```

5. **Push and create a Pull Request:**
   ```bash
   git push origin feature/your-improvement
   ```

6. **In your PR description, please include:**
   - What changes you made
   - **What tests you added and why**
   - How to verify your changes work
   - Any edge cases you considered

### **PR Review Criteria:**

✅ **We prioritize PRs that include:**
- Comprehensive test coverage
- Clear documentation of test strategy
- Tests for both success and failure scenarios
- Integration tests where applicable

✅ **Special fast-track for:**
- Test-only contributions
- PRs that significantly improve test coverage
- Bug fixes with accompanying regression tests

### Developer Etiquette

- Be kind in code reviews—every improvement helps the project grow

### 🎉 **Get Started Today!**

Don't know where to start? Here are some **beginner-friendly test contributions:**

1. Add tests for CLI parameter validation
2. Test error message formatting
3. Add tests for directory listing HTML generation
4. Test file streaming with various file sizes
5. Add security tests for path traversal attempts

**Every test case counts!** Even if you can only add one test, it makes the project better for everyone.

---

## 📈 Performance Characteristics

### Runtime Performance
- **Memory Usage**: ~3MB baseline + (thread_count × 8KB stack) + template cache + upload buffer memory
- **Concurrent Connections**: Custom thread pool (default: 8) + rate limiting protection
- **File Streaming**: Configurable chunk size (default: 1KB) with range request support
- **Template Rendering**: Sub-millisecond variable interpolation with built-in caching
- **Large Upload Handling**: Supports unlimited file sizes with constant memory usage (~7MB)

### Request Latency
| Operation | Typical Latency | Notes |
|-----------|----------------|-------|
| **Static Assets** | <0.5ms | CSS/JS served with caching headers |
| **Directory Listing** | <2ms | Template-based rendering with file sorting |
| **Health Checks** | <0.1ms | JSON status endpoints |
| **File Downloads** | Variable | Depends on file size and network |
| **File Uploads** | Variable | Depends on file size, includes validation |
| **Error Pages** | <1ms | Template-based professional error pages |

### Upload Performance
- **Upload Processing**: Sub-millisecond file validation and atomic writing
- **Concurrent Uploads**: Integrated with existing thread pool and rate limiting
- **Resource Management**: Dynamic upload directory detection and configurable size limits

### Security & Monitoring Overhead
- **Rate Limiting**: ~0.1ms per request for IP tracking and cleanup
- **Authentication**: ~0.2ms for Basic Auth header parsing
- **Path Validation**: <0.1ms for canonicalization and traversal checks
- **Statistics Collection**: <0.05ms per request for metrics tracking

### Scalability
- **Rate Limiting**: 120 requests/minute per IP (configurable)
- **Concurrent Connections**: 10 per IP address (configurable)  
- **Template Cache**: In-memory storage for frequently accessed templates
- **File Descriptor Usage**: Efficient cleanup prevents resource exhaustion

---

## 🔒 Security Features

### Core Security
- **Path Traversal Prevention**: All paths are canonicalized and validated against the served directory
- **Extension Filtering**: Configurable glob patterns restrict downloadable file types
- **Basic Authentication**: Optional username/password protection with proper challenge responses
- **Static Asset Protection**: Template files served only through controlled `/_irondrop/static/` routes

### Advanced Protection
- **Rate Limiting**: DoS protection with configurable requests per minute (default: 120)
- **Connection Limiting**: Maximum concurrent connections per IP address (default: 10)
- **Request Timeouts**: Prevents resource exhaustion from slow or malicious clients
- **Input Validation**: Robust HTTP header parsing with malformed request rejection
- **Upload Security Suite** ⭐:
  - **Multi-layer Validation**: Boundary verification, content-type checking, size enforcement
  - **Filename Sanitization**: Path traversal prevention with character filtering
  - **Extension Validation**: Configurable glob patterns with wildcard support
  - **Atomic Operations**: Safe file writing with temporary files and rename
  - **Resource Protection**: Disk space checking and concurrent upload limiting
  - **Malformed Data Rejection**: Robust parsing with comprehensive error handling

### Monitoring & Auditing
- **Request Logging**: Every request tagged with unique IDs for comprehensive auditing
- **Performance Tracking**: Slow request detection and logging for security analysis
- **Statistics Collection**: Real-time monitoring of request patterns and error rates
- **Health Endpoints**: Built-in `/_health` and `/_status` for infrastructure monitoring

### Zero-Trust Architecture
- **No External Dependencies**: Eliminates third-party security vulnerabilities
- **Native Implementation**: All security features implemented in pure Rust
- **Template Security**: Variable interpolation with HTML escaping and URL encoding
- **Memory Safety**: Rust's ownership model prevents buffer overflows and memory leaks

### Compliance Features
- **HTTP Security Headers**: Proper `Server`, `Content-Type`, and caching headers
- **Error Information Disclosure**: Professional error pages without sensitive details
- **Access Control**: Configurable authentication with secure credential handling
- **Audit Trail**: Comprehensive logging for security incident investigation

---

## 🎨 Modern Web Interface

### Professional Design
The server features a completely **modular template system** with a sophisticated **blackish-grey corporate design**:

- **Clean Architecture**: Separated HTML structure, CSS styling, and JavaScript functionality
- **Professional Color Scheme**: Elegant blackish-grey palette (#0a0a0a to #ffffff) suitable for corporate environments
- **Glassmorphism Effects**: Modern backdrop blur effects and transparent overlays
- **Responsive Layout**: Mobile-friendly design that adapts to all screen sizes

### User Experience Features
- **Enhanced File Browsing**: Clean table layout with improved column separation and striping
- **File Type Indicators**: Color-coded dots for different file categories (directories, documents, images, archives)
- **Interactive Elements**: Smooth hover effects with professional white highlights
- **Keyboard Navigation**: Arrow key support for efficient file browsing
- **Performance Optimizations**: Intersection Observer for large directories and fade-in animations

### Template Architecture
```
templates/directory/     # Directory listing templates
├── index.html          # Clean HTML structure with {{VARIABLE}} interpolation
├── styles.css          # Professional CSS with custom properties
└── script.js           # Enhanced interactions and file type detection

templates/error/         # Error page templates
├── page.html           # Consistent error page structure
├── styles.css          # Matching error page styling
└── script.js           # Error page enhancements and shortcuts
```

### Static Asset Delivery
- **Optimized Serving**: CSS/JS files delivered via `/_irondrop/static/` routes with proper caching headers
- **MIME Detection**: Accurate Content-Type headers for all static assets
- **Security**: Path traversal protection prevents access outside template directories
- **Performance**: Efficient file streaming with conditional request support

### Customization
The modular template system allows easy customization:
- **Colors**: Modify CSS custom properties in `styles.css` files
- **Layout**: Update HTML structure in template files
- **Interactions**: Enhance JavaScript functionality in `script.js` files
- **Branding**: Replace server info and styling to match corporate identity

---

## 📚 Documentation for Developers & Contributors

### 🔧 **For Developers**

If you're looking to understand the codebase, integrate IronDrop, or contribute to development:

- **📖 [Complete Documentation Suite](./doc/)** - Comprehensive technical documentation
- **🏗️ [Architecture Guide](./doc/ARCHITECTURE.md)** - System design, component breakdown, and code organization
- **🔌 [API Reference](./doc/API_REFERENCE.md)** - Complete REST API specification with examples
- **🔍 [Search Feature Guide](./doc/SEARCH_FEATURE.md)** - Dual-mode search engine implementation and usage
- **🚀 [Deployment Guide](./doc/DEPLOYMENT.md)** - Production deployment with Docker, systemd, and reverse proxy

### 🛡️ **For Security & DevOps Teams**

Production deployment and security implementation details:

- **🔒 [Security Implementation](./doc/SECURITY_FIXES.md)** - OWASP vulnerability fixes and security controls
- **🚀 [Production Deployment](./doc/DEPLOYMENT.md)** - systemd, Docker, monitoring, and security hardening
- **📊 [System Monitoring](./doc/API_REFERENCE.md#health-and-monitoring)** - Health endpoints and operational metrics

### 🎨 **For Frontend Developers**

UI system and template integration:

- **📤 [Upload UI System](./doc/UPLOAD_INTEGRATION.md)** - Modern drag-and-drop interface implementation
- **🎨 [Template System](./doc/ARCHITECTURE.md#template-system-architecture)** - Professional blackish-grey UI with modular architecture
- **🔧 [API Integration](./doc/API_REFERENCE.md#client-integration-examples)** - JavaScript, cURL, and Python examples

### 🧪 **Testing & Quality Assurance**

IronDrop includes **179 comprehensive tests across 16 test files** covering:

- **Core Server Tests** (19 tests): HTTP handling, directory listing, authentication
- **Upload System Tests** (29 tests): File uploads, validation, concurrent handling
- **Security Tests** (12+ tests): Path traversal protection, input validation
- **Multipart Parser Tests** (7 tests): RFC 7578 compliance and edge cases
- **Integration Tests** (30+ tests): End-to-end functionality and performance

```bash
# Run all tests
cargo test

# Run with detailed output
cargo test -- --nocapture

# Run specific test suites
cargo test comprehensive_test    # Core functionality
cargo test upload_integration    # Upload system
cargo test multipart_test       # Multipart parser
```

### 📈 **Project Statistics**

| Metric | Count | Description |
|--------|--------|-------------|
| **Source Files** | 15 | Rust modules with clear separation of concerns |
| **Lines of Code** | 3000+ | Production-ready implementation |
| **Template Files** | 10 | Professional UI with HTML/CSS/JS separation |
| **Test Cases** | 179 across 16 files | Comprehensive coverage including security tests |
| **Documentation Pages** | 10 | Complete technical documentation suite |

---

## 🤝 Contributing

We welcome contributions! Here's how to get started:

### 🎯 **Quick Contribution Guide**

1. **Fork** the repository and create your feature branch
2. **Add tests** for any new functionality (this is crucial!)
3. **Run the test suite** and ensure all tests pass
4. **Follow code style** with `cargo fmt && cargo clippy`
5. **Submit a pull request** with a clear description

### 📋 **Contribution Areas**

**For Code Contributors:**
- New features with comprehensive test coverage
- Performance optimizations and bug fixes
- Security enhancements and vulnerability fixes
- UI/UX improvements and accessibility features

**For Test Contributors:**
- Test cases for existing functionality (we love test-only PRs!)
- Edge case testing and security scenario coverage
- Performance and load testing
- Integration test improvements

**For Documentation Contributors:**
- Usage examples and tutorials
- Deployment guides for specific environments
- API documentation improvements
- Translation and localization

### 🏆 **Current Contributors**

| Name | GitHub | Contributions |
|------|--------|---------------|
| **Harshit Jain** | [@dev-harsh1998](https://github.com/dev-harsh1998) | Project founder, core architecture, main development |
| **Sonu Kumar Saw** | [@dev-saw99](https://github.com/dev-saw99) | Code improvements and UI enhancements |

> **Want to see your name here?** Your name will be added after your first merged pull request!

### 🐛 **Bug Reports & Feature Requests**

- **Bug Reports**: Use GitHub Issues with detailed reproduction steps
- **Feature Requests**: Describe the use case and proposed implementation
- **Security Issues**: Report privately via GitHub Security Advisory

---

## 🌟 **Why Choose IronDrop?**

### **For End Users**
- **Zero Configuration**: Works out of the box with sensible defaults
- **Professional Interface**: Clean, modern web UI suitable for any environment
- **Secure by Default**: Built-in security features without complex setup
- **Cross-Platform**: Runs on Linux, macOS, and Windows

### **For Developers**
- **Pure Rust**: No external dependencies, everything built from scratch
- **Comprehensive Tests**: 179 tests across 16 files ensure reliability and stability
- **Clean Architecture**: Well-documented, modular codebase
- **Performance Focus**: Custom thread pool and optimized file streaming

### **For DevOps Teams**
- **Single Binary**: Easy deployment with no runtime dependencies
- **Container Ready**: Docker support with optimized images
- **Monitoring Built-in**: Health endpoints and comprehensive logging
- **Security Hardened**: Multiple layers of protection and validation

---

## 📞 Support & Community

- **📖 Documentation**: Start with [./doc/README.md](./doc/README.md) for complete guides
- **🐛 Issues**: Report bugs and request features via GitHub Issues
- **💬 Discussions**: GitHub Discussions for questions and community support
- **🔒 Security**: Responsible disclosure via GitHub Security Advisory

---

## 📜 License

IronDrop is distributed under the **MIT** license; see `LICENSE` for details.

---

<div align="center">

*Made with 🦀 in Bengaluru*

**[⭐ Star us on GitHub](https://github.com/dev-harsh1998/IronDrop) • [📖 Read the Docs](./doc/) • [🚀 Get Started](#-quick-start)**

</div>