# IronDrop

<div align="center">
  <img src="irondrop-logo.png" alt="IronDrop Logo" width="150"/>
  
  <h1>IronDrop: The Zero-Dependency, High-Performance File Server</h1>
  
  <p>
    <strong>Drop files, not dependencies.</strong> IronDrop is a blazing-fast, secure, and feature-rich file server written in pure Rust, delivered as a single, portable binary.
  </p>
  
  [![Rust CI](https://github.com/dev-harsh1998/IronDrop/actions/workflows/rust.yml/badge.svg)](https://github.com/dev-harsh1998/IronDrop/actions/workflows/rust.yml)
</div>

**🎉 NEW in v2.5**: Complete file upload functionality with **10GB support**, enhanced multipart parsing, robust security validation, and comprehensive test coverage. **Plus ultra-compact search system supporting 10M+ files with <100MB memory usage**.

IronDrop is not just another file server. It's a production-ready toolkit designed for performance, security, and ease of use. Whether you're sharing files on your local network, setting up a lightweight digital archive, or need a robust upload endpoint, IronDrop provides a complete solution with zero external dependencies.

## ⭐ Why Choose IronDrop?

IronDrop was built to address the limitations of other open-source file servers. Here’s how it stands out:

| Feature | IronDrop | `python -m http.server` | `npx http-server` | Other Rust Servers (`miniserve`) |
| :--- | :---: | :---: | :---: | :---: |
| **Zero Dependencies** | ✅ | ❌ (Python) | ❌ (Node.js) | ✅ |
| **File Uploads** | ✅ | ❌ | ❌ | ✅ |
| **Advanced Search** | ✅ | ❌ | ❌ | ❌ |
| **Real-time Monitoring** | ✅ | ❌ | ❌ | ❌ |
| **Enterprise-Grade Security**| ✅ | ❌ | ❌ | ❌ |
| **Low Memory Usage** | ✅ | ❌ | ❌ | ✅ |
| **Single Binary** | ✅ | ❌ | ❌ | ✅ |

## 🚀 Key Features

*   **🚀 High-Performance File Serving:** Serve files with support for range requests, MIME type detection, and conditional caching headers.
*   **⬆️ Modern File Uploads:** A beautiful drag-and-drop interface for uploading files and entire folders. Supports files up to 10GB with RFC 7578 compliant multipart parsing.
*   **🧠 Advanced Dual-Mode Search:** A powerful search engine that automatically switches between a standard, full-featured engine and an "ultra-compact" mode for directories with millions of files.
*   **📊 Real-time Monitoring:** A built-in monitoring dashboard at `/monitor` provides live statistics on requests, uploads, and server health, with a JSON API for integration.
*   **🔒 Enterprise-Grade Security:** IronDrop is built with a security-first mindset, featuring:
    *   Rate limiting and connection management to prevent DoS attacks.
    *   Optional Basic Authentication with secure credential handling.
    *   Path traversal protection and filename sanitization.
    *   Comprehensive OWASP compliance and security validation.
*   **🖥️ Professional UI:** A modern, responsive, dark-themed interface that's a pleasure to use.
*   **📦 Zero Dependencies, Single Binary:** The entire application, including all assets, is compiled into a single, portable executable. No runtimes, no interpreters, no hassle.
*   **🧪 Battle-Tested:** 59 tests across 13 test files covering edge cases, security scenarios, and performance stress testing.

## ⚡ Performance

IronDrop is engineered for extreme performance and memory efficiency.

### Ultra-Compact Search Engine

The standout feature is the **ultra-compact search engine**, which can index over **10 million files using less than 100MB of RAM**.

| Directory Size | Search Time | Memory Usage | 
| :--- | :--- | :--- |
| 100K files | 5-15ms | ~1.1MB |
| 1M files | 20-80ms | ~11MB |
| **10M files** | **100-500ms** | **~110MB** |

This makes IronDrop the ideal choice for serving large archives, datasets, and media collections without sacrificing performance.

## 🛡️ Security

Security is a core design principle of IronDrop.

*   **OWASP Top 10 Compliant:** The server is designed to mitigate the most critical web application security risks.
*   **Comprehensive Input Validation:** All inputs, from CLI arguments to HTTP headers and filenames, are rigorously validated.
*   **Secure by Default:** Features like uploads and authentication are opt-in, ensuring a secure default configuration.
*   **Extensive Security Documentation:** For a detailed breakdown of security features, see the [RFC & OWASP Compliance](./doc/RFC_OWASP_COMPLIANCE.md) and [Security Fixes](./doc/SECURITY_FIXES.md) documents.

## 📦 Installation

Getting started with IronDrop is simple.

### From Source

```bash
# Clone the repository
git clone https://github.com/dev-harsh1998/IronDrop.git
cd IronDrop

# Build the release binary
cargo build --release

# The executable will be in ./target/release/irondrop
```

### System-Wide Installation

```bash
# For Linux/macOS
sudo cp ./target/release/irondrop /usr/local/bin/

# For Windows (in PowerShell)
mkdir "C:\ Program Files\IronDrop"
copy ".\target\release\irondrop.exe" "C:\ Program Files\IronDrop\"
# Then add C:\ Program Files\IronDrop to your system's PATH
```

## ⚙️ Usage

### Basic Usage
```bash
# Serve the current directory
irondrop -d .

# Enable uploads with authentication
irondrop -d . --enable-upload --username admin --password your-secret-password

# Serve on a different port and listen on all interfaces
irondrop -d /path/to/your/files --port 3000 --listen 0.0.0.0
```

### Advanced Configuration (v2.5+)
```bash
# Use configuration file for reproducible deployments
irondrop -d . --config-file production.ini

# Override specific settings from config file
irondrop -d . --config-file prod.ini --threads 32 --verbose
```

### Key Endpoints
- **`/`** - Directory listing and file browsing
- **`/monitor`** - Real-time monitoring dashboard
- **`/search?q=term`** - File search API

For a full list of options, run `irondrop --help` or see the [API Reference](./doc/API_REFERENCE.md).

## 🆕 What's New in v2.5

### 🎯 **Major Features**
- **Complete Upload System**: Professional drag-and-drop interface with 10GB file support
- **Ultra-Compact Search**: Handle 10M+ files with <100MB memory usage
- **Configuration System**: INI-based configuration with hierarchical precedence
- **Enhanced Security**: Comprehensive OWASP compliance and security validation
- **RFC 7578 Compliance**: Production-ready multipart parsing

### 🔧 **Technical Improvements**
- **Memory Optimization**: Radix-based indexing for massive directory support
- **Template Engine**: Embedded templates with zero filesystem dependencies
- **Monitoring Dashboard**: Real-time metrics and JSON API
- **Comprehensive Testing**: 59 tests across 13 files ensuring reliability

### 📊 **Performance Enhancements**
- **Dual-Mode Search**: Automatic switching between standard and ultra-compact engines
- **Streaming Uploads**: Efficient handling of large file uploads
- **Connection Management**: Advanced rate limiting and DoS protection
- **Memory Efficiency**: Optimized for both small and massive deployments

## 📚 Documentation

IronDrop has extensive documentation covering its architecture, API, and features.

### 📖 **Core Documentation**
*   [**Complete Documentation Index**](./doc/README.md) - Central hub for all documentation
*   [**Architecture Guide**](./doc/ARCHITECTURE.md) - System design and component overview
*   [**API Reference**](./doc/API_REFERENCE.md) - Complete HTTP API documentation
*   [**Deployment Guide**](./doc/DEPLOYMENT.md) - Production deployment strategies

### 🔧 **Feature Documentation**
*   [**Search Feature Deep Dive**](./doc/SEARCH_FEATURE.md) - Ultra-compact search system details
*   [**Upload Integration Guide**](./doc/UPLOAD_INTEGRATION.md) - File upload system and UI
*   [**Multipart Parser**](./doc/MULTIPART_README.md) - RFC 7578 compliant parser details
*   [**Configuration System**](./doc/CONFIGURATION_SYSTEM.md) - INI-based configuration guide
*   [**Template System**](./doc/TEMPLATE_SYSTEM.md) - Embedded template engine

### 🛡️ **Security & Quality**
*   [**Security Fixes**](./doc/SECURITY_FIXES.md) - Security enhancements and mitigations
*   [**RFC & OWASP Compliance**](./doc/RFC_OWASP_COMPLIANCE.md) - Standards compliance details
*   [**Testing Documentation**](./doc/TESTING_DOCUMENTATION.md) - Comprehensive test suite overview
*   [**Monitoring Guide**](./doc/MONITORING.md) - Real-time monitoring and metrics

## 🧪 Testing

IronDrop is rigorously tested with **59 comprehensive tests across 13 test files** covering all aspects of functionality.

### Test Categories
- **Core Server Tests** (19 tests): HTTP handling, directory listing, authentication
- **Upload System Tests** (29 tests): File uploads, validation, concurrent handling  
- **Edge Case Tests** (10 tests): Boundary conditions and error scenarios
- **Multipart Parser Tests** (7 tests): RFC 7578 compliance and edge cases
- **Performance & Memory Tests** (15 tests): Stress testing and optimization
- **Search Engine Tests** (7 tests): Ultra-compact search and template integration

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test comprehensive_test    # Core server functionality
cargo test upload_integration    # Upload system tests
cargo test edge_case_test        # Edge cases and error handling
cargo test multipart_test        # Multipart parser validation

# Run tests with output
cargo test -- --nocapture
```

For detailed testing information, see [Testing Documentation](./doc/TESTING_DOCUMENTATION.md).

## 📜 License

IronDrop is licensed under the [GPL-3.0 License](./LICENSE).

---

<div align="center">
  <p>
    <strong>Made with ❤️ and 🦀 in Rust</strong><br>
    <em>Zero dependencies • Production ready • Battle tested with 59 comprehensive tests</em>
  </p>
  <p>
    <a href="https://github.com/dev-harsh1998/IronDrop">⭐ Star us on GitHub</a>
    &nbsp;&nbsp;&bull;&nbsp;&nbsp;
    <a href="https://github.com/dev-harsh1998/IronDrop/issues">Report an Issue</a>
    &nbsp;&nbsp;&bull;&nbsp;&nbsp;
    <a href="./doc/README.md">📚 Read the Docs</a>
    &nbsp;&nbsp;&bull;&nbsp;&nbsp;
    <a href="./doc/TESTING_DOCUMENTATION.md">🧪 View Tests</a>
  </p>
</div>
