# IronDrop API Reference

This document describes the current HTTP surface implemented by the code in this repository.

## Base Behavior

- default listen address: `127.0.0.1`
- default port: `8080`
- optional Basic Auth applies to the whole server when configured
- when `--base-path /prefix` is set, every route in this document must be prefixed with `/prefix`

## Directory And File Routes

### `GET /` and `GET /<path>`

Serves either:

- an HTML directory listing for directories
- a file stream for files

Notes:

- directories without a trailing slash are redirected to their canonical slash form with `301 Moved Permanently`
- directory listings are HTML only
- directory pagination uses `?p=<page>`
- file responses include `Accept-Ranges: bytes`
- valid range requests may return `206 Partial Content`

Common error codes:

- `401 Unauthorized` when Basic Auth is enabled and credentials are missing or invalid
- `403 Forbidden` for path traversal attempts or blocked extensions
- `404 Not Found` for missing paths
- `405 Method Not Allowed` for unsupported methods

## Upload Routes

Uploads are disabled unless `--enable-upload true` or `enable_upload = true` is set.

### `GET /_irondrop/upload`

Returns the embedded HTML upload page.

Optional query parameters:

- `upload_to`: subdirectory inside the served tree where uploaded files should be written

### `POST /_irondrop/upload`

Accepts an uploaded file body and writes it into the served directory tree.

Filename resolution order:

1. `Content-Disposition: ...; filename=...`
2. `X-Filename`
3. final URL segment when it looks like a filename
4. generated fallback name such as `upload_<timestamp>.bin`

Target directory:

- default: the served directory
- override: `upload_to=/subdir`

Response format:

- JSON when `Accept: application/json` is sent or the request looks like an XHR request
- HTML otherwise

Example raw upload:

```bash
curl -X POST \
  -H 'Content-Type: application/octet-stream' \
  -H 'X-Filename: document.txt' \
  --data-binary @document.txt \
  'http://127.0.0.1:8080/_irondrop/upload?upload_to=/incoming'
```

Current behavior to note:

- public upload routing is `POST` only
- request bodies up to 2 MiB stay in memory; larger ones are spooled to a temporary file
- uploads are still bounded by the HTTP parser request-body limit of 10 GiB
- there is no public `--upload-dir` flag

Common upload errors:

- `400 Bad Request` when the body is missing or malformed
- `401 Unauthorized` when auth is enabled
- `405 Method Not Allowed` when uploads are disabled
- `413 Payload Too Large` when the configured upload limit is exceeded
- `415 Unsupported Media Type` when the filename extension is rejected

## Search Route

### `GET /_irondrop/search`

Query parameters:

- `q`: required, 2 to 100 characters
- `path`: optional search root inside the served tree, default `/`
- `limit`: optional, default `50`, max `200`
- `offset`: optional, default `0`

Example:

```bash
curl 'http://127.0.0.1:8080/_irondrop/search?q=document&path=/&limit=10&offset=0'
```

Response shape:

```json
[
  {
    "name": "document.txt",
    "path": "/docs/document.txt",
    "size": "8 B",
    "type": "file"
  }
]
```

Notes:

- the public API returns a JSON array, not a wrapped object
- results are sorted by internal score before pagination
- there is no `/api/search` route in the current codebase

## Monitoring And Health Routes

### `GET /monitor`
### `GET /_irondrop/monitor`

Returns the HTML monitoring dashboard.

### `GET /monitor?json=1`
### `GET /_irondrop/monitor?json=1`

Returns machine-readable monitoring data.

### `GET /_irondrop/health`
### `GET /_irondrop/status`
### `GET /_health`

Returns a JSON health payload. `/_irondrop/status` currently matches the health payload and `/_health` is kept for compatibility.

## Static And Internal Utility Routes

- `GET /_irondrop/static/<asset>`: embedded CSS and JavaScript assets
- `GET /_irondrop/logo`: embedded project logo
- `GET /favicon.ico`, `GET /favicon-16x16.png`, `GET /favicon-32x32.png`: embedded browser icons
- `GET /_irondrop/logout`: logout page that returns `401` and `WWW-Authenticate`
- `POST /_irondrop/cleanup-memory`: triggers search-memory cleanup and returns a small JSON status payload

## WebDAV Methods

WebDAV methods are handled on normal file paths, not on `/_irondrop/*` routes.

When WebDAV is enabled, these methods are accepted:

- `OPTIONS`
- `PROPFIND`
- `PROPPATCH`
- `MKCOL`
- `PUT`
- `DELETE`
- `COPY`
- `MOVE`
- `LOCK`
- `UNLOCK`

When WebDAV is disabled, those methods return `405 Method Not Allowed`.

## Authentication

When `--username` and `--password` are configured, auth middleware runs before route handling. That includes:

- file and directory routes
- upload routes
- monitoring and health routes
- internal static assets

Example:

```bash
curl -u admin:secret http://127.0.0.1:8080/_irondrop/health
```

## Error Summary

Common status codes used by the current implementation:

- `200 OK`
- `206 Partial Content`
- `301 Moved Permanently`
- `400 Bad Request`
- `401 Unauthorized`
- `403 Forbidden`
- `404 Not Found`
- `405 Method Not Allowed`
- `413 Payload Too Large`
- `415 Unsupported Media Type`
- `500 Internal Server Error`
