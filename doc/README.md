# IronDrop Documentation Index v2.5

Welcome to the comprehensive documentation for IronDrop, a lightweight, high-performance file server written in Rust with bidirectional file sharing capabilities.

## 📚 Documentation Overview

This documentation suite provides complete coverage of IronDrop's architecture, API, deployment, and specialized features. Each document is designed to serve specific audiences and use cases.

## 📖 Core Documentation

### 🏗️ [Architecture Documentation](./ARCHITECTURE.md)
**Audience**: Developers, System Architects, DevOps Engineers  
**Purpose**: Complete system design and component interaction overview

**Contents:**
- System architecture diagrams and component relationships
- Request processing flow and data paths
- Module-by-module code organization (19 Rust source files)
- Security architecture and defense-in-depth implementation
- Performance characteristics and scalability considerations
- Template system design and asset pipeline
- Testing architecture with 101+ comprehensive tests

**Key Sections:**
- Core module breakdown with line counts and responsibilities
- HTTP request processing pipeline with security checkpoints
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
- Upload API with multipart form-data handling
- Health monitoring and status endpoints
- Error response formats and HTTP status codes

**Key Features:**
- Directory listing API (HTML and JSON responses)
- File download with range request support
- File upload system with progress tracking
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
- Supports up to 10GB file uploads with progress indicators

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
- Size limits with overflow protection (1MB-10GB range)
- Integration with core systems for consistent security
- Extensive test coverage for security scenarios

### 🔄 [Multipart Parser Documentation](./MULTIPART_README.md)
### 📊 [Monitoring Guide](./MONITORING.md)
**Audience**: Operators, Observability Engineers, SREs  
**Purpose**: Details on `/monitor`, health endpoints, data model, integration patterns

**Contents:**
- `/monitor` HTML dashboard behavior and refresh model
- `/monitor?json=1` schema and field semantics
- Health vs status endpoint differences
- Example automation + jq scraping patterns
- Extensibility roadmap (Prometheus, per-endpoint stats)
**Audience**: Backend Developers, Protocol Implementers  
**Purpose**: RFC 7578 compliant multipart/form-data parser details

**Contents:**
- Memory-efficient streaming parser implementation
- Security validations and boundary detection
- Integration with upload system and error handling
- Configuration options and customization
- Comprehensive API usage examples

**Implementation Status**: ✅ **Production Ready** (v2.5)
- RFC 7578 compliance with robust boundary detection
- 661 lines of production-quality code
- 7+ dedicated test cases covering edge cases
- Integrated with upload handler and HTTP processing
- Zero external dependencies with pure Rust implementation

## 📊 Documentation Statistics

| Document | Pages | Focus Area | Last Updated |
|----------|--------|------------|--------------|
| **Architecture** | ~15 | System Design & Implementation | v2.5 |
| **API Reference** | ~20 | REST API & Integration | v2.5 |
| **Deployment** | ~18 | Operations & Production | v2.5 |
| **Upload Integration** | ~8 | UI System & Templates | v2.5 |
| **Security Fixes** | ~6 | Security Implementation | v2.5 |
| **Multipart Parser** | ~5 | Protocol Implementation | v2.5 |

## 🎯 Documentation by Audience

### For **Developers**
1. Start with [Architecture Documentation](./ARCHITECTURE.md) for system overview
2. Review [API Reference](./API_REFERENCE.md) for integration details
3. Check [Upload Integration](./UPLOAD_INTEGRATION.md) for UI implementation
4. Examine [Multipart Parser](./MULTIPART_README.md) for protocol details

### For **DevOps/SysAdmins**
1. Begin with [Deployment Guide](./DEPLOYMENT.md) for production setup
2. Review [Security Fixes](./SECURITY_FIXES.md) for security implementation
3. Check [Architecture Documentation](./ARCHITECTURE.md) for performance tuning
4. Reference [API Reference](./API_REFERENCE.md) for monitoring endpoints

### For **Security Teams**
1. Start with [Security Fixes](./SECURITY_FIXES.md) for vulnerability remediation
2. Review [Architecture Documentation](./ARCHITECTURE.md) for security architecture
3. Check [Deployment Guide](./DEPLOYMENT.md) for hardening procedures
4. Examine [Multipart Parser](./MULTIPART_README.md) for input validation

### For **Integration Teams**
1. Begin with [API Reference](./API_REFERENCE.md) for endpoint specifications
2. Review [Upload Integration](./UPLOAD_INTEGRATION.md) for UI components
3. Check [Architecture Documentation](./ARCHITECTURE.md) for system boundaries
4. Reference [Deployment Guide](./DEPLOYMENT.md) for environment setup

## 🔍 Quick Reference

### Essential Commands
```bash
# Basic server start
irondrop -d /path/to/files

# Production server with uploads
irondrop -d /srv/files --enable-upload --listen 0.0.0.0 --port 8080

# Health check
curl http://localhost:8080/_health

# Upload file
curl -X POST -F "file=@document.pdf" http://localhost:8080/upload
```

### Key Endpoints
- **Directory Listing**: `GET /` or `GET /path/`
- **File Upload**: `POST /upload`
- **Health Check**: `GET /_health`
- **Server Status**: `GET /_status`
- **Static Assets**: `GET /_static/path/file.css`

### Configuration Files
- **systemd Service**: `/etc/systemd/system/irondrop.service`
- **nginx Config**: `/etc/nginx/sites-available/irondrop`
- **Docker Compose**: `docker-compose.yml`

## 🚦 Current Implementation Status

### ✅ **Production Ready Features**
- **Core Server**: Robust HTTP server with thread pool (19 tests)
- **File Downloads**: Range requests and MIME detection
- **Upload System**: Complete with drag-drop UI (29 tests)
- **Multipart Parser**: RFC 7578 compliant (7 tests)
- **Security**: Comprehensive input validation and protection
- **Template System**: Professional UI with modular architecture
- **Authentication**: Basic Auth with secure credential handling
- **Monitoring**: Health endpoints and comprehensive logging

### 📈 **Performance Metrics**
- **Memory Usage**: ~3MB baseline + configurable thread stack
- **Concurrent Connections**: Custom thread pool with rate limiting
- **File Size Support**: Up to 10GB uploads with streaming
- **Request Latency**: Sub-millisecond for static assets
- **Test Coverage**: 101+ comprehensive tests across all components

### 🔒 **Security Implementation**
- **Input Validation**: Multi-layer validation with bounds checking
- **Path Traversal Protection**: Comprehensive directory validation
- **Rate Limiting**: Configurable per-IP limits (120 req/min default)
- **File Extension Filtering**: Glob pattern support for allowed types
- **Resource Protection**: Size limits, timeouts, and memory management
- **Audit Logging**: Request tracking with unique IDs

## 📝 Documentation Maintenance

This documentation is actively maintained and updated with each release. Each document includes:

- **Version tracking** for feature alignment
- **Implementation status** indicators
- **Code references** with file paths and line numbers
- **Practical examples** and usage scenarios
- **Troubleshooting sections** for common issues

### Contributing to Documentation

To contribute to the documentation:

1. **Technical corrections**: Submit issues with specific document references
2. **Usage examples**: Provide real-world scenarios and configurations
3. **Missing topics**: Suggest additional documentation areas
4. **Clarity improvements**: Report unclear sections or missing context

### Documentation Standards

All IronDrop documentation follows these standards:

- **Comprehensive coverage** of features and functionality
- **Practical examples** with working code snippets
- **Security considerations** for production environments
- **Version-specific information** tied to release cycles
- **Cross-references** between related documents
- **Audience-specific organization** for different use cases

## 🎉 Getting Started

1. **New Users**: Start with the main [README.md](../Readme.md) in the project root
2. **Developers**: Begin with [Architecture Documentation](./ARCHITECTURE.md)
3. **Operators**: Jump to [Deployment Guide](./DEPLOYMENT.md)
4. **API Users**: Reference [API Documentation](./API_REFERENCE.md)

Each document is designed to be self-contained while providing clear paths to related information. The documentation evolves with the codebase to ensure accuracy and completeness.

---

*This documentation index covers IronDrop v2.5 and is maintained alongside the codebase for accuracy and completeness.*