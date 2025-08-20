# IronDrop

<div align="center">
  <img src="irondrop-logo.png" alt="IronDrop Logo" width="150"/>
  
  <h1>IronDrop: The Zero-Dependency, High-Performance File Server</h1>
  
  <p>
    <strong>Drop files, not dependencies.</strong> IronDrop is a blazing-fast, secure, and feature-rich file server written in pure Rust, delivered as a single, portable binary.
  </p>
  
  [![Rust CI](https://github.com/dev-harsh1998/IronDrop/actions/workflows/rust.yml/badge.svg)](https://github.com/dev-harsh1998/IronDrop/actions/workflows/rust.yml)
</div>

**🎉 NEW in v2.5.1**: Revolutionary direct streaming upload system with **unlimited file size support**, constant memory usage (~7MB), and simplified binary upload architecture. **Plus ultra-compact search system supporting 10M+ files with <100MB memory usage**.

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
*   **⬆️ Modern File Uploads:** A beautiful drag-and-drop interface for uploading files and entire folders. Supports unlimited file sizes with efficient direct streaming architecture.
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

## ⚙️ Getting Started

### 🚀 Quick Start (30 seconds to file sharing!)

**Step 1:** Download or build IronDrop
```bash
# Build from source (requires Rust)
git clone https://github.com/dev-harsh1998/IronDrop.git
cd IronDrop
cargo build --release
```

**Step 2:** Start sharing files immediately
```bash
# Share your current directory (safest - local access only)
./target/release/irondrop -d .

# Share with your network (accessible to other devices)
./target/release/irondrop -d . --listen 0.0.0.0
```

**Step 3:** Open your browser and visit `http://localhost:8080` 🎉

### 📖 Common Use Cases

#### 🏠 **Home File Sharing**
```bash
# Share your Downloads folder with family devices
irondrop -d ~/Downloads --listen 0.0.0.0 --port 8080
```

#### 💼 **Work File Server**
```bash
# Secure file server with uploads and authentication
irondrop -d ./shared-files \
  --enable-upload \
  --username admin \
  --password your-secure-password \
  --listen 0.0.0.0
```

#### 🎬 **Media Server**
```bash
# Serve your media collection (videos, music, photos)
irondrop -d /path/to/media \
  --allowed-extensions "*.mp4,*.mp3,*.jpg,*.png" \
  --threads 16 \
  --listen 0.0.0.0
```

#### ☁️ **Cloud Storage Alternative**
```bash
# Use a configuration file for consistent setup
irondrop --config-file ./config/production.ini
```

### 🛠️ Configuration Options

#### **Command Line Options**
IronDrop offers extensive customization through command-line arguments:

| Option | Description | Example |
|--------|-------------|----------|
| `-d, --directory` | **Required** - Directory to serve | `-d /home/user/files` |
| `-l, --listen` | Listen address (default: 127.0.0.1) | `-l 0.0.0.0` |
| `-p, --port` | Port number (default: 8080) | `-p 3000` |
| `--enable-upload` | Enable file uploads | `--enable-upload true` |
| `--username/--password` | Basic authentication | `--username admin --password secret` |
| `-a, --allowed-extensions` | Restrict file types | `-a "*.pdf,*.doc,*.zip"` |
| `-t, --threads` | Worker threads (default: 8) | `-t 16` |
| `--config-file` | Use INI configuration file | `--config-file prod.ini` |
| `-v, --verbose` | Debug logging | `-v true` |

#### **📄 Configuration File (Recommended for Production)**

For consistent deployments, use an INI configuration file:

```bash
# Create your config file
cp config/irondrop.ini my-server.ini
# Edit it with your settings
# Then run:
irondrop --config-file my-server.ini
```

The configuration file supports all command-line options and more! See the [detailed example](./config/irondrop.ini) with comments explaining every option.

**Configuration Priority (highest to lowest):**
1. Command line arguments
2. Environment variables (`IRONDROP_*`)
3. Configuration file
4. Built-in defaults

### 🌐 Key Endpoints

Once IronDrop is running, these endpoints are available:

| Endpoint | Purpose | Example |
|----------|---------|----------|
| **`/`** | 📁 Directory listing and file browsing | `http://localhost:8080/` |
| **`/monitor`** | 📊 Real-time server monitoring dashboard | `http://localhost:8080/monitor` |
| **`/search?q=term`** | 🔍 File search API | `http://localhost:8080/search?q=document` |
| **`/_irondrop/upload`** | ⬆️ File upload endpoint (if enabled) | Used by the web interface |

### 💡 Pro Tips

- **🔒 Security First**: Always use authentication (`--username`/`--password`) when exposing to networks
- **🚀 Performance**: Increase `--threads` for high-traffic scenarios (try 16-32 threads)
- **💾 Large Files**: IronDrop handles unlimited file sizes with constant ~7MB memory usage
- **🔍 Search**: The ultra-compact search engine can handle 10M+ files efficiently
- **📱 Mobile Friendly**: The web interface works great on phones and tablets

### ❓ Need Help?

```bash
# Get detailed help for all options
irondrop --help

# Check your version
irondrop --version

# Test with verbose logging
irondrop -d . --verbose true
```

For comprehensive documentation, see our [Complete Documentation Index](./doc/README.md).

## 🆕 What's New in v2.5.1

### 🎯 **Major Features**
- **Direct Streaming Upload System**: Revolutionary architecture with unlimited file size support
- **Ultra-Compact Search**: Handle 10M+ files with <100MB memory usage
- **Configuration System**: INI-based configuration with hierarchical precedence
- **Enhanced Security**: Comprehensive OWASP compliance and security validation
- **Memory Efficiency**: Constant ~7MB RAM usage regardless of file size

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
*   [**Direct Upload System**](./doc/MULTIPART_README.md) - Memory-efficient direct streaming architecture
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
- **Direct Upload Tests** (7 tests): Memory efficiency and streaming validation
- **Performance & Memory Tests** (15 tests): Stress testing and optimization
- **Search Engine Tests** (7 tests): Ultra-compact search and template integration

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test comprehensive_test    # Core server functionality
cargo test upload_integration    # Upload system tests
cargo test edge_case_test        # Edge cases and error handling
cargo test direct_upload_test    # Direct streaming validation

# Run tests with output
cargo test -- --nocapture
```

For detailed testing information, see [Testing Documentation](./doc/TESTING_DOCUMENTATION.md).

## 📜 License

IronDrop is licensed under the [MIT License](./LICENSE).

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
