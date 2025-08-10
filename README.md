# IronDrop

<div align="center">
  <img src="irondrop-logo.png" alt="IronDrop Logo" width="150"/>
  
  <h1>IronDrop: The Zero-Dependency, High-Performance File Server</h1>
  
  <p>
    <strong>Drop files, not dependencies.</strong> IronDrop is a blazing-fast, secure, and feature-rich file server written in pure Rust, delivered as a single, portable binary.
  </p>
  
  [![Rust CI](https://github.com/dev-harsh1998/IronDrop/actions/workflows/rust.yml/badge.svg)](https://github.com/dev-harsh1998/IronDrop/actions/workflows/rust.yml)
</div>

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
*   **⬆️ Modern File Uploads:** A beautiful drag-and-drop interface for uploading files and entire folders. Supports files up to 10GB.
*   **🧠 Advanced Dual-Mode Search:** A powerful search engine that automatically switches between a standard, full-featured engine and an "ultra-compact" mode for directories with millions of files.
*   **📊 Real-time Monitoring:** A built-in monitoring dashboard at `/monitor` provides live statistics on requests, uploads, and server health, with a JSON API for integration.
*   **🔒 Enterprise-Grade Security:** IronDrop is built with a security-first mindset, featuring:
    *   Rate limiting and connection management to prevent DoS attacks.
    *   Optional Basic Authentication.
    *   Path traversal protection and filename sanitization.
    *   Comprehensive OWASP compliance.
*   **🖥️ Professional UI:** A modern, responsive, dark-themed interface that's a pleasure to use.
*   **📦 Zero Dependencies, Single Binary:** The entire application, including all assets, is compiled into a single, portable executable. No runtimes, no interpreters, no hassle.

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

```bash
# Serve the current directory
irondrop -d .

# Enable uploads with authentication
irondrop -d . --enable-upload --username admin --password your-secret-password

# Serve on a different port and listen on all interfaces
irondrop -d /path/to/your/files --port 3000 --listen 0.0.0.0
```

For a full list of options, run `irondrop --help`.

## 📚 Documentation

IronDrop has extensive documentation covering its architecture, API, and features.

*   [**Complete Documentation Index**](./doc/README.md)
*   [**Architecture Guide**](./doc/ARCHITECTURE.md)
*   [**API Reference**](./doc/API_REFERENCE.md)
*   [**Deployment Guide**](./doc/DEPLOYMENT.md)
*   [**Search Feature Deep Dive**](./doc/SEARCH_FEATURE.md)

## 🧪 Testing

IronDrop is rigorously tested with over 100 tests.

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

## 📜 License

IronDrop is licensed under the [GPL-3.0 License](./LICENSE).

---

<div align="center">
  <p>
    <strong>Made with ❤️ and 🦀 in Rust</strong>
  </p>
  <p>
    <a href="https://github.com/dev-harsh1998/IronDrop">⭐ Star us on GitHub</a>
    &nbsp;&nbsp;&bull;&nbsp;&nbsp;
    <a href="https://github.com/dev-harsh1998/IronDrop/issues">Report an Issue</a>
    &nbsp;&nbsp;&bull;&nbsp;&nbsp;
    <a href="./doc/README.md">Read the Docs</a>
  </p>
</div>
