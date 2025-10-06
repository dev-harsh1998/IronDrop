# IronDrop

<div align="center">
  <img src="irondrop-logo.png" alt="IronDrop Logo" width="150"/>
  
  <h1>IronDrop file server</h1>
  
  <p>
    IronDrop is a file server written in Rust. It serves directories, supports optional uploads, provides search, and includes a monitoring page. It ships as a single binary with embedded templates.
  </p>
  
  [![Rust CI](https://github.com/dev-harsh1998/IronDrop/actions/workflows/rust.yml/badge.svg)](https://github.com/dev-harsh1998/IronDrop/actions/workflows/rust.yml)
</div>

IronDrop focuses on predictable behavior, simplicity, and low overhead. Use it to serve or share files locally or on your network.

## Overview

## Features

- File browsing and downloads with range requests and MIME detection
- Optional uploads with a drag-and-drop web UI (direct-to-disk streaming)
- Search (standard and ultra-compact modes for large directories)
- Monitoring dashboard at `/monitor` and a JSON endpoint (`/monitor?json=1`)
- Basic security features: rate limiting, optional Basic Auth, path safety checks
- Single binary; templates and assets are embedded
 - Pure standard library networking and file I/O (no external HTTP stack or async runtime)
 - Ultra-compact search index option for very large directory trees (tested up to ~10M entries)

## Performance

Designed to keep memory usage steady and to stream large files without buffering them in memory. The ultra-compact search mode reduces memory for very large directory trees.

- Ultra-compact search: approximately ~110 MB of RAM for around 10 million paths; search latency depends on CPU, disk, and query specifics.
- No-dependency footprint: networking and file streaming are implemented with Rust's `std::net` and `std::fs`, producing a single self-contained binary.

## Security

Includes rate limiting, optional Basic Auth, basic input validation, and path traversal protection. See [RFC & OWASP Compliance](./doc/RFC_OWASP_COMPLIANCE.md) and [Security Fixes](./doc/SECURITY_FIXES.md) for details.

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

### System-Wide Installation (Recommended)

To use IronDrop from anywhere on your system, install it to a directory in your PATH:

```bash
# Linux/macOS - Install to /usr/local/bin (requires sudo)
sudo cp ./target/release/irondrop /usr/local/bin/

# Alternative: Install to ~/.local/bin (no sudo required)
mkdir -p ~/.local/bin
cp ./target/release/irondrop ~/.local/bin/
# Add ~/.local/bin to PATH if not already:
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc  # or ~/.zshrc
source ~/.bashrc  # or restart terminal

# Windows (PowerShell as Administrator)
# Create program directory
New-Item -ItemType Directory -Force -Path "C:\Program Files\IronDrop"
# Copy executable
Copy-Item ".\target\release\irondrop.exe" "C:\Program Files\IronDrop\"
# Add to system PATH (requires restart or new terminal)
$env:PATH += ";C:\Program Files\IronDrop"
[Environment]::SetEnvironmentVariable("PATH", $env:PATH, [EnvironmentVariableTarget]::Machine)
```

**Verify Installation:**
```bash
# Test that irondrop is available globally
irondrop --version

# Now you can run from any directory:
irondrop -d ~/Documents --listen 0.0.0.0
```

## Getting started

### Quick start

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

**Step 3:** Open your browser and visit `http://localhost:8080`

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

### Key endpoints

Once IronDrop is running, these endpoints are available:

| Endpoint | Purpose | Example |
|----------|---------|----------|
| **`/`** | 📁 Directory listing and file browsing | `http://localhost:8080/` |
| **`/monitor`** | 📊 Real-time server monitoring dashboard | `http://localhost:8080/monitor` |
| **`/search?q=term`** | 🔍 File search API | `http://localhost:8080/search?q=document` |
| **`/_irondrop/upload`** | ⬆️ File upload endpoint (if enabled) | Used by the web interface |

### Notes

- Use authentication (`--username`/`--password`) when exposing to untrusted networks
- Adjust `--threads` based on workload

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

## Version notes

Recent releases include direct-to-disk uploads, an ultra-compact search mode, and a `/monitor` page with a JSON endpoint.

## Documentation

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

## Testing

IronDrop is rigorously tested with **199 comprehensive tests across 16 test files** covering all aspects of functionality.

### Test Categories
- **Integration Tests** (16 tests): End-to-end functionality and HTTP handling
- **Monitor Tests** (2 tests): Real-time monitoring dashboard and metrics
- **Rate Limiter Tests** (7 tests): Memory-based rate limiting and DoS protection
- **Template Tests** (8 tests): Embedded template system and rendering
- **Ultra-Compact Search Tests** (10 tests): Advanced search engine functionality
- **Configuration Tests** (12 tests): INI parsing and configuration validation
- **Core Server & Unit Tests** (40 tests): Library functions, utilities, and core logic

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

## License

IronDrop is licensed under the [MIT License](./LICENSE).

---

<div align="center">
  <p>
    <strong>Made with ❤️ and 🦀 in Rust</strong><br>
    <em>Zero dependencies • Production ready • Battle tested with 199 comprehensive tests</em>
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
