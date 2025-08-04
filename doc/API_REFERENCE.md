# IronDrop API Reference v2.5

## Overview

IronDrop provides a RESTful HTTP API for file operations, directory browsing, and system monitoring. This document describes all available endpoints, their parameters, and response formats.

## Base Configuration

### Default Settings
- **Host**: `127.0.0.1` (configurable via `--listen`)
- **Port**: `8080` (configurable via `--port`)
- **Protocol**: HTTP/1.1
- **Authentication**: Optional Basic Auth (configurable via `--username`/`--password`)

### Common Headers

#### Request Headers
```http
# Authentication (if enabled)
Authorization: Basic <base64-encoded-credentials>

# For file uploads
Content-Type: multipart/form-data; boundary=<boundary>
Content-Length: <content-length>

# For API requests
Accept: application/json, text/html
User-Agent: <client-identifier>
```

#### Response Headers
```http
# Standard headers
Server: IronDrop/2.5.0
Content-Type: <mime-type>
Content-Length: <content-length>
Connection: keep-alive

# Security headers
X-Content-Type-Options: nosniff
X-Frame-Options: DENY

# Caching headers (for static assets)
Cache-Control: public, max-age=3600
ETag: "<etag-value>"
```

## Endpoints

### 1. Directory Listing

#### `GET /` and `GET /<path>/`
Retrieves directory contents or serves files.

**Parameters:**
- `path`: Directory path relative to served directory (optional, defaults to root)

**Query Parameters:**
- `format`: Response format (`html` or `json`) - default: `html`
- `sort`: Sort order (`name`, `size`, `modified`) - default: `name`
- `order`: Sort direction (`asc`, `desc`) - default: `asc`

**Response Formats:**

**HTML Response (Default):**
```http
HTTP/1.1 200 OK
Content-Type: text/html; charset=utf-8

<!DOCTYPE html>
<html>
<!-- Professional directory listing with file table -->
</html>
```

**JSON Response:**
```json
{
  "path": "/",
  "entries": [
    {
      "name": "document.pdf",
      "type": "file",
      "size": 1048576,
      "modified": "2024-01-01T12:00:00Z",
      "mime_type": "application/pdf"
    },
    {
      "name": "subdirectory",
      "type": "directory",
      "size": null,
      "modified": "2024-01-01T12:00:00Z",
      "mime_type": null
    }
  ],
  "stats": {
    "total_files": 1,
    "total_directories": 1,
    "total_size": 1048576
  }
}
```

**Error Responses:**
```http
# Directory not found
HTTP/1.1 404 Not Found
Content-Type: text/html

# Access denied
HTTP/1.1 403 Forbidden
Content-Type: text/html

# Authentication required
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Basic realm="IronDrop"
```

### 2. File Downloads

#### `GET /<file-path>`
Downloads individual files with support for range requests.

**Parameters:**
- `file-path`: Path to file relative to served directory

**Request Headers:**
```http
# Range request (optional)
Range: bytes=0-1023

# Conditional requests (optional)
If-None-Match: "<etag>"
If-Modified-Since: <date>
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: application/octet-stream
Content-Length: 1048576
Content-Disposition: attachment; filename="document.pdf"
Accept-Ranges: bytes
ETag: "abc123"
Last-Modified: Mon, 01 Jan 2024 12:00:00 GMT

<file-content>
```

**Range Response:**
```http
HTTP/1.1 206 Partial Content
Content-Type: application/octet-stream
Content-Length: 1024
Content-Range: bytes 0-1023/1048576
Accept-Ranges: bytes

<partial-file-content>
```

**Error Responses:**
```http
# File not found
HTTP/1.1 404 Not Found

# File extension not allowed
HTTP/1.1 403 Forbidden

# Range not satisfiable
HTTP/1.1 416 Range Not Satisfiable
Content-Range: bytes */1048576
```

### 3. File Upload System

#### `GET /upload`
Displays the upload interface page.

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: text/html; charset=utf-8

<!DOCTYPE html>
<html>
<!-- Modern upload interface with drag-and-drop -->
</html>
```

#### `POST /upload` or `POST /?upload=true`
Uploads one or more files to the server.

**Request:**
```http
POST /upload HTTP/1.1
Content-Type: multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW
Content-Length: <content-length>

------WebKitFormBoundary7MA4YWxkTrZu0gW
Content-Disposition: form-data; name="file"; filename="document.pdf"
Content-Type: application/pdf

<file-data>
------WebKitFormBoundary7MA4YWxkTrZu0gW
Content-Disposition: form-data; name="file"; filename="image.jpg"
Content-Type: image/jpeg

<file-data>
------WebKitFormBoundary7MA4YWxkTrZu0gW--
```

**Success Response (JSON):**
```json
{
  "status": "success",
  "message": "Files uploaded successfully",
  "files": [
    {
      "name": "document.pdf",
      "size": 1048576,
      "saved_as": "document.pdf",
      "location": "/path/to/uploads/document.pdf"
    },
    {
      "name": "image.jpg",
      "size": 524288,
      "saved_as": "image_1.jpg",
      "location": "/path/to/uploads/image_1.jpg"
    }
  ],
  "upload_stats": {
    "total_files": 2,
    "total_size": 1572864,
    "upload_time_ms": 150
  }
}
```

**Success Response (HTML):**
```http
HTTP/1.1 200 OK
Content-Type: text/html

<!DOCTYPE html>
<html>
<body>
  <h1>Upload Successful</h1>
  <p>2 files uploaded successfully</p>
  <!-- Upload results page -->
</body>
</html>
```

**Error Responses:**
```json
# File too large
{
  "status": "error",
  "error": "PayloadTooLarge",
  "message": "File size exceeds maximum allowed size",
  "details": {
    "max_size": 10737418240,
    "file_size": 21474836480,
    "filename": "large_file.zip"
  }
}

# Invalid file type
{
  "status": "error",
  "error": "UnsupportedMediaType",
  "message": "File type not allowed",
  "details": {
    "filename": "script.exe",
    "extension": ".exe",
    "allowed_extensions": ["*.txt", "*.pdf", "*.jpg"]
  }
}

# Upload disabled
{
  "status": "error",
  "error": "MethodNotAllowed",
  "message": "File uploads are disabled on this server"
}
```

### 4. Static Assets

#### `GET /_static/<asset-path>`
Serves template assets (CSS, JavaScript, images).

**Examples:**
- `GET /_static/directory/styles.css`
- `GET /_static/upload/script.js`
- `GET /_static/error/styles.css`

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: text/css
Cache-Control: public, max-age=3600
ETag: "asset-hash"

/* CSS content */
```

**Error Response:**
```http
HTTP/1.1 404 Not Found
Content-Type: text/plain

Static asset not found
```

### 5. Health and Monitoring

#### `GET /_health`
Basic health check endpoint.

**Response:**
```json
{
  "status": "healthy",
  "version": "2.5.0",
  "uptime_seconds": 3600,
  "timestamp": "2024-01-01T12:00:00Z"
}
```

#### `GET /_status`
Detailed server status and statistics.

**Response:**
```json
{
  "status": "healthy",
  "version": "2.5.0",
  "uptime_seconds": 3600,
  "timestamp": "2024-01-01T12:00:00Z",
  "statistics": {
    "requests_served": 15420,
    "bytes_served": 1073741824,
    "errors_encountered": 12,
    "active_connections": 3,
    "rate_limit_hits": 5
  },
  "configuration": {
    "threads": 8,
    "chunk_size": 1024,
    "upload_enabled": true,
    "max_upload_size": 10737418240,
    "rate_limit": 120
  },
  "system": {
    "memory_usage_mb": 3.2,
    "cpu_usage_percent": 2.1,
    "disk_space_available": true
  }
}
```

### 6. API Information

#### `GET /_api`
API information and capabilities.

**Response:**
```json
{
  "name": "IronDrop",
  "version": "2.5.0",
  "description": "Lightweight file server with upload capabilities",
  "endpoints": {
    "directory_listing": {
      "method": "GET",
      "path": "/[path]",
      "description": "List directory contents or download files",
      "parameters": ["format", "sort", "order"],
      "formats": ["html", "json"]
    },
    "file_upload": {
      "method": "POST",
      "path": "/upload",
      "description": "Upload files to server",
      "content_type": "multipart/form-data",
      "max_size": 10737418240,
      "enabled": true
    },
    "health_check": {
      "method": "GET",
      "path": "/_health",
      "description": "Basic health check"
    },
    "status": {
      "method": "GET",
      "path": "/_status",
      "description": "Detailed server status"
    }
  },
  "features": {
    "uploads": true,
    "authentication": false,
    "rate_limiting": true,
    "range_requests": true,
    "static_assets": true
  }
}
```

## Authentication

### Basic Authentication

When authentication is enabled via `--username` and `--password`, all endpoints require Basic Authentication.

**Request:**
```http
GET / HTTP/1.1
Authorization: Basic <base64-encoded-username:password>
```

**Unauthorized Response:**
```http
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Basic realm="IronDrop"
Content-Type: text/html

<!DOCTYPE html>
<html>
<body>
  <h1>401 Unauthorized</h1>
  <p>This server requires authentication.</p>
</body>
</html>
```

## Rate Limiting

IronDrop implements rate limiting to prevent abuse:

### Default Limits
- **Requests per minute**: 120 (configurable)
- **Concurrent connections per IP**: 10 (configurable)

### Rate Limit Headers
```http
X-RateLimit-Limit: 120
X-RateLimit-Remaining: 115
X-RateLimit-Reset: 1704110400
```

### Rate Limit Exceeded
```http
HTTP/1.1 429 Too Many Requests
Retry-After: 60
X-RateLimit-Limit: 120
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1704110400

{
  "status": "error",
  "error": "TooManyRequests",
  "message": "Rate limit exceeded. Please try again later.",
  "retry_after": 60
}
```

## Error Handling

### HTTP Status Codes

| Code | Meaning | Description |
|------|---------|-------------|
| 200 | OK | Request successful |
| 206 | Partial Content | Range request successful |
| 400 | Bad Request | Malformed request |
| 401 | Unauthorized | Authentication required |
| 403 | Forbidden | Access denied or file type not allowed |
| 404 | Not Found | File or directory not found |
| 405 | Method Not Allowed | HTTP method not supported |
| 413 | Payload Too Large | Upload size exceeds limit |
| 415 | Unsupported Media Type | File type not allowed |
| 416 | Range Not Satisfiable | Invalid range request |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Server error |

### Error Response Format

**JSON Error Response:**
```json
{
  "status": "error",
  "error": "ErrorType",
  "message": "Human-readable error description",
  "details": {
    "additional": "error-specific information"
  },
  "request_id": "req_abc123",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

**HTML Error Response:**
```html
<!DOCTYPE html>
<html>
<head>
    <title>Error 404 - Not Found</title>
    <link rel="stylesheet" href="/_static/error/styles.css">
</head>
<body>
    <div class="error-container">
        <h1>404 - Not Found</h1>
        <p>The requested resource could not be found.</p>
        <a href="/">← Back to Home</a>
    </div>
</body>
</html>
```

## Client Integration Examples

### JavaScript/Fetch API

**Directory Listing:**
```javascript
// Get directory listing as JSON
const response = await fetch('/path/to/directory?format=json');
const data = await response.json();

console.log(`Found ${data.entries.length} items`);
data.entries.forEach(entry => {
    console.log(`${entry.name} (${entry.type})`);
});
```

**File Upload:**
```javascript
// Upload multiple files
const formData = new FormData();
formData.append('file', file1);
formData.append('file', file2);

const response = await fetch('/upload', {
    method: 'POST',
    body: formData,
    headers: {
        'Accept': 'application/json'
    }
});

const result = await response.json();
if (result.status === 'success') {
    console.log(`Uploaded ${result.files.length} files`);
}
```

**Health Check:**
```javascript
// Monitor server health
const health = await fetch('/_health').then(r => r.json());
console.log(`Server uptime: ${health.uptime_seconds}s`);
```

### cURL Examples

**Download file:**
```bash
curl -O http://localhost:8080/path/to/file.pdf
```

**Upload file:**
```bash
curl -X POST -F "file=@document.pdf" http://localhost:8080/upload
```

**Get directory listing as JSON:**
```bash
curl "http://localhost:8080/directory?format=json" | jq .
```

**Health check:**
```bash
curl http://localhost:8080/_health
```

**With authentication:**
```bash
curl -u username:password http://localhost:8080/
```

### Python/Requests

**Directory listing:**
```python
import requests

response = requests.get('http://localhost:8080/path?format=json')
data = response.json()

for entry in data['entries']:
    print(f"{entry['name']} - {entry['type']}")
```

**File upload:**
```python
import requests

files = {'file': open('document.pdf', 'rb')}
response = requests.post('http://localhost:8080/upload', files=files)

if response.status_code == 200:
    result = response.json()
    print(f"Upload successful: {result['message']}")
```

## Security Considerations

### Best Practices
1. **Always use HTTPS in production** (place behind reverse proxy)
2. **Enable authentication** for sensitive directories
3. **Configure appropriate file extension filters**
4. **Monitor rate limiting logs** for abuse detection
5. **Regularly review upload directories** for malicious content
6. **Set appropriate upload size limits** based on available storage
7. **Use strong passwords** for Basic Authentication

### Security Headers
IronDrop automatically includes security headers:
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- Proper `Content-Type` headers for all responses

### Input Validation
All inputs are validated:
- File extensions against allowed patterns
- Upload sizes against configured limits
- File names for path traversal attempts
- HTTP headers for malformed content

This API reference covers all functionality available in IronDrop v2.5 and provides comprehensive examples for client integration.