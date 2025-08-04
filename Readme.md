<div align="center">
  <img src="irondrop-logo.png" alt="IronDrop Logo" width="200"/>
  
  # IronDrop
</div>

A lightweight, high-performance file download server written in Rust featuring a **modular template architecture** and **professional UI design**. Offers secure, cross-platform file sharing with advanced monitoring, comprehensive security features, and a modern web interface. Every component has been designed for clarity, reliability, and developer friendliness with **zero external dependencies**.

---

## 🚀 Key Features

### 🎨 **Modern Web Interface**
- **Professional Blackish-Grey UI** – Clean, corporate-grade design with sophisticated glassmorphism effects
- **Modular Template System** – Organized HTML/CSS/JS architecture with variable interpolation
- **Static Asset Serving** – Efficient delivery of stylesheets and scripts via `/_static/` routes
- **Responsive Design** – Mobile-friendly interface with adaptive layouts

### 🔐 **Advanced Security & Monitoring**
- **Rate Limiting** – DoS protection with configurable requests per minute and concurrent connections per IP
- **Server Statistics** – Real-time monitoring of requests, bytes served, uptime, and performance metrics
- **Health Check Endpoints** – Built-in `/_health` and `/_status` endpoints for monitoring
- **Path-Traversal Protection** – Canonicalises every request path and rejects any attempt that escapes the served directory
- **Optional Basic Authentication** – Username and password can be supplied via CLI flags

### 📁 **File Management**
- **Enhanced Directory Listing** – Beautiful table-based layout with file type indicators and sorting
- **Secure File Downloads** – Streams large files efficiently, honours HTTP range requests, and limits downloads to allowed extensions with glob support
- **MIME Type Detection** – Native file type detection for proper Content-Type headers
- **File Type Visualization** – Color-coded indicators for different file categories

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
irondrop -d .
```

Open a browser at [http://127.0.0.1:8080](http://127.0.0.1:8080) and you will see the auto-generated directory index.

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

### Practical Examples

| Scenario | Command | Features |
|----------|---------|----------|
| **Public File Share** | `irondrop -d /srv/files -p 3000 -l 0.0.0.0` | Professional UI, rate limiting, health monitoring |
| **Document Repository** | `irondrop -d ./docs -a "*.pdf,*.png,*.jpg"` | Filtered downloads, file type indicators |
| **High-Performance Server** | `irondrop -d ./big -t 16 -c 8192` | Custom thread pool, optimized streaming |
| **Secure Corporate Share** | `irondrop -d ./private --username alice --password s3cret` | Authentication, audit logging, professional design |
| **Development Server** | `irondrop -d . -v --detailed-logging` | Debug logging, template development, hot reload |
| **Production Monitoring** | `irondrop -d /data -l 0.0.0.0` + health checks at `/_health` | Statistics, uptime monitoring, rate limiting |

---

## 🏗️ Architecture Overview

The codebase features a **modular template architecture** with clear separation of concerns. Core modules include `server.rs` for the custom thread-pool listener, `http.rs` for request parsing and static asset serving, `templates.rs` for the native template engine, `fs.rs` for directory operations, and `response.rs` for file streaming and error handling. The `templates/` directory contains organized HTML/CSS/JS assets for the professional web interface.

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
    |  Static Assets    |       |   File System    |       |Security & Monitor |
    | (templates/*.css) |       |    (fs.rs)       |       | Rate Limit+Stats  |
    +-------------------+       +------------------+       +-------------------+
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
                    +-------------------+-------------------+
                    |                   |                   |
                    v                   v                   v
            [Static Assets]      [Health Check]        [File System]
                    |                   |                   |
                    v                   v                   v
            Serve CSS/JS         JSON Status         Path Safety Check
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
├── error.rs         # Custom error enum
└── utils.rs         # Helper utilities

templates/
├── directory/       # Directory listing templates
│   ├── index.html   # Clean HTML structure
│   ├── styles.css   # Professional blackish-grey design
│   └── script.js    # Enhanced interactions + file type detection
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

The project includes **19 comprehensive tests** covering all aspects of functionality:

```bash
# Run all tests (13 comprehensive + 6 integration)
cargo test

# Run with detailed output
cargo test -- --nocapture

# Run specific test suites
cargo test comprehensive_test    # New modular template tests
cargo test integration_test      # Core functionality tests
```

### Test Architecture

**Custom HTTP Client**: Tests use a native HTTP client implementation (zero external dependencies) that directly connects via `TcpStream` to verify:

- **Template System**: Modular HTML/CSS/JS serving and rendering
- **Static Asset Delivery**: CSS/JS file serving with proper MIME types
- **Professional UI**: Blackish-grey design elements and glassmorphism effects
- **Security Features**: Rate limiting, authentication, path traversal protection
- **Health Monitoring**: Status endpoints and server statistics
- **Error Handling**: Template-based error pages with consistent theming
- **File Operations**: Range requests, MIME detection, large file handling
- **HTTP Compliance**: Headers, status codes, and protocol adherence

### Test Coverage

| Test Category | Count | Description |
|---------------|-------|-------------|
| **UI & Templates** | 4 | Directory listing, error pages, static assets, template rendering |
| **Security** | 4 | Authentication, rate limiting, path traversal, malformed requests |
| **File Operations** | 3 | MIME detection, large files, nested directories |
| **Monitoring** | 2 | Health checks, server statistics |
| **Core HTTP** | 6 | Range requests, headers, protocol compliance, error responses |

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
- **Memory Usage**: ~3MB baseline + (thread_count × 8KB stack) + template cache
- **Concurrent Connections**: Custom thread pool (default: 8) + rate limiting protection
- **File Streaming**: Configurable chunk size (default: 1KB) with range request support
- **Template Rendering**: Sub-millisecond variable interpolation with built-in caching

### Request Latency
| Operation | Typical Latency | Notes |
|-----------|----------------|-------|
| **Static Assets** | <0.5ms | CSS/JS served with caching headers |
| **Directory Listing** | <2ms | Template-based rendering with file sorting |
| **Health Checks** | <0.1ms | JSON status endpoints |
| **File Downloads** | Variable | Depends on file size and network |
| **Error Pages** | <1ms | Template-based professional error pages |

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
- **Static Asset Protection**: Template files served only through controlled `/_static/` routes

### Advanced Protection
- **Rate Limiting**: DoS protection with configurable requests per minute (default: 120)
- **Connection Limiting**: Maximum concurrent connections per IP address (default: 10)
- **Request Timeouts**: Prevents resource exhaustion from slow or malicious clients
- **Input Validation**: Robust HTTP header parsing with malformed request rejection

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
- **Optimized Serving**: CSS/JS files delivered via `/_static/` routes with proper caching headers
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

## 📜 License

IronDrop is distributed under the **GPL-3.0** license; see `LICENSE` for details.

---

*Made with 🦀 in Bengaluru*