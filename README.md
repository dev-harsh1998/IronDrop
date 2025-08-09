# IronDrop

<div align="center">
  <img src="irondrop-logo.png" alt="IronDrop Logo" width="120"/>
  
  [![Rust CI](https://github.com/dev-harsh1998/IronDrop/actions/workflows/rust.yml/badge.svg)](https://github.com/dev-harsh1998/IronDrop/actions/workflows/rust.yml)
</div>

A lightweight, high-performance file server written in Rust with **zero external dependencies**. Production-ready with comprehensive upload functionality, dual-mode search engine, and enterprise-grade security.

## 🚀 Features

• **File Downloads** - Secure file serving with range requests and MIME detection  
• **File Uploads** - Drag-and-drop interface supporting up to 10GB files  
• **Advanced Search** - Dual-mode search engine optimized for directories of any size  
• **Professional UI** - Modern blackish-grey interface with responsive design  
• **Security Built-in** - Rate limiting, authentication, path traversal protection  
• **Real-time Monitoring** - Live dashboard at `/monitor` with JSON API  
• **Zero Dependencies** - Pure Rust implementation, single binary deployment  

## 📦 Installation

### Quick Start
```bash
# Clone and build
git clone https://github.com/dev-harsh1998/IronDrop.git
cd IronDrop
cargo build --release

# Run server
./target/release/irondrop -d /path/to/files
```

### System Installation (Optional)

Make `irondrop` available system-wide:

**Linux/macOS:**
```bash
# Copy to system PATH
sudo cp ./target/release/irondrop /usr/local/bin/

# Or user-local installation
mkdir -p ~/.local/bin
cp ./target/release/irondrop ~/.local/bin/
# Add ~/.local/bin to PATH in ~/.bashrc or ~/.zshrc
export PATH="$HOME/.local/bin:$PATH"
```

**Windows:**
```powershell
# Copy to a directory in PATH, or create one
mkdir "C:\Program Files\IronDrop"
copy ".\target\release\irondrop.exe" "C:\Program Files\IronDrop\"
# Add C:\Program Files\IronDrop to system PATH via Environment Variables
```

### Basic Usage
```bash
# Serve current directory
irondrop -d .

# Enable uploads with authentication
irondrop -d . --enable-upload --username admin --password secret

# Custom port and network interface
irondrop -d ./files --listen 0.0.0.0 --port 3000
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Format and lint
cargo fmt && cargo clippy
```

## 📋 Current Version

**v2.5.0** - Latest stable release with advanced search system, comprehensive file upload functionality, and monitoring dashboard

## 📖 Documentation

For comprehensive documentation, deployment guides, and API reference:

**[📚 Complete Documentation](./doc/README.md)**

### Quick Links
• [🏗️ Architecture Guide](./doc/ARCHITECTURE.md) - System design and components  
• [🔌 API Reference](./doc/API_REFERENCE.md) - REST endpoints and examples  
• [🔍 Search System](./doc/SEARCH_FEATURE.md) - Dual-mode search implementation  
• [🚀 Deployment Guide](./doc/DEPLOYMENT.md) - Production setup and Docker  
• [🔒 Security Guide](./doc/SECURITY_FIXES.md) - Security features and best practices  

## 🌟 Why IronDrop?

• **Zero Config** - Works out of the box with sensible defaults  
• **Production Ready** - 101+ tests, comprehensive security, monitoring built-in  
• **Memory Efficient** - <100MB for 10M+ files with ultra-compact search  
• **Developer Friendly** - Clear architecture, extensive documentation  

## 📜 License

GPL-3.0 License - see [LICENSE](LICENSE) for details.

---

<div align="center">

*Made with 🦀 in Rust*

**[⭐ Star us on GitHub](https://github.com/dev-harsh1998/IronDrop) • [📖 Documentation](./doc/) • [🐛 Issues](https://github.com/dev-harsh1998/IronDrop/issues)**

</div>