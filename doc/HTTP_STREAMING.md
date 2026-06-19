# IronDrop HTTP Body Streaming

This guide documents the current request-body handling used by `src/http.rs` and `src/upload.rs`.

## Overview

IronDrop does not keep every upload body in memory.

The HTTP layer uses a two-stage model:

- request bodies up to 2 MiB are stored in memory
- larger request bodies are spooled to a temporary file and passed to handlers as a file-backed body

This is represented by `RequestBody`:

```rust
pub enum RequestBody {
    Memory(Vec<u8>),
    File { path: PathBuf, size: u64 },
}
```

## Thresholds

Current thresholds in the codebase:

- HTTP parser spool-to-disk threshold: `2 * 1024 * 1024` bytes
- upload handler in-memory threshold: `2 * 1024 * 1024` bytes
- HTTP parser maximum request-body size: `10 * 1024 * 1024 * 1024` bytes

That means uploads are not unbounded today: they are still capped by the request parser at 10 GiB unless the code changes.

## Request Flow

1. IronDrop reads the request line and headers.
2. The body reader decides whether to keep the body in memory or spill it to a temp file.
3. Route handling receives a `Request` whose body is either `RequestBody::Memory` or `RequestBody::File`.
4. After request processing completes, file-backed temp bodies are removed.

The cleanup happens in `handle_client_async()` after the response has been sent.

## Upload Integration

The upload handler at `/_irondrop/upload` works on top of the same `RequestBody` abstraction.

Behavior today:

- small bodies are written from memory to the destination file
- larger bodies are copied from the temp file to the final destination
- final writes are atomic through a temp-file-and-rename pattern
- filenames are validated and checked against allowed extensions

Public upload routing is `POST /_irondrop/upload`. The internal upload handler also accepts `PUT` if called directly in code, but that is not currently exposed as a public route.

## Temp Files

Temp files are created with unique names using:

- process id
- current time
- a monotonic counter

They are used in two places:

- HTTP request-body spooling in `src/http.rs`
- final atomic upload writes in `src/upload.rs`

## Operational Notes

- very large uploads still need enough disk space for temporary storage
- if uploads are enabled, the served directory tree must be writable where uploads should land
- the request parser supports both `Content-Length` bodies and chunked transfer encoding
- JSON upload responses are selected through `Accept: application/json` or XHR-style requests

## What This Guide Corrects

The current implementation does not match some older documentation claims:

- the spool threshold is 2 MiB, not 64 MiB
- there is no `IRONDROP_STREAMING_THRESHOLD` runtime override
- uploads are not described by a multipart-only model
- the parser still enforces a 10 GiB maximum request-body size

## Example Raw Upload

```bash
curl -X POST \
  -H 'Content-Type: application/octet-stream' \
  -H 'X-Filename: big.bin' \
  --data-binary @big.bin \
  http://127.0.0.1:8080/_irondrop/upload
```
