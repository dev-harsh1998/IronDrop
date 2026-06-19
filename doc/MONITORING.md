# IronDrop Monitoring Guide

This guide documents the monitoring endpoints that exist in the current server implementation.

## Endpoints

- `/monitor`: HTML dashboard
- `/monitor?json=1`: JSON metrics through the legacy-friendly route
- `/_irondrop/monitor`: HTML dashboard through the internal namespace
- `/_irondrop/monitor?json=1`: JSON metrics through the internal namespace
- `/_irondrop/health`: health payload
- `/_irondrop/status`: status payload, currently the same as health
- `/_health`: compatibility health route

If Basic Auth is enabled, these endpoints require credentials like every other route.

## JSON Payload

`/_irondrop/monitor?json=1` returns this shape:

```json
{
  "requests": { "total": 42, "successful": 40, "errors": 2 },
  "downloads": { "bytes_served": 1048576 },
  "uptime_secs": 360,
  "memory": {
    "available": true,
    "current_bytes": 33554432,
    "peak_bytes": 67108864,
    "current_mb": 32.0,
    "peak_mb": 64.0
  },
  "uploads": {
    "total_uploads": 5,
    "successful_uploads": 5,
    "failed_uploads": 0,
    "files_uploaded": 7,
    "upload_bytes": 5242880,
    "average_upload_size": 748982,
    "largest_upload": 2097152,
    "concurrent_uploads": 0,
    "average_processing_ms": 152.4,
    "success_rate": 100.0
  }
}
```

## Field Notes

- `requests.total` counts handled requests since startup
- `downloads.bytes_served` counts response-body bytes, not headers
- `uploads.average_processing_ms` is a rolling average across the last 100 upload samples
- `memory.available` can be `false` on platforms or environments where process memory cannot be read

## Dashboard Behavior

The embedded dashboard JavaScript currently refreshes every 4 seconds.

It renders:

- request totals and success ratio
- upload counters and throughput summaries
- download byte counts
- memory data when available
- charts backed by in-page history buffers

## Example Commands

Health check:

```bash
curl -f http://127.0.0.1:8080/_irondrop/health
```

Download bytes served:

```bash
curl -s 'http://127.0.0.1:8080/_irondrop/monitor?json=1' | jq '.downloads.bytes_served'
```

Upload failure count:

```bash
curl -s 'http://127.0.0.1:8080/_irondrop/monitor?json=1' | jq '.uploads.failed_uploads'
```

## Maintenance Endpoint

`POST /_irondrop/cleanup-memory` triggers explicit search-memory cleanup and returns a small JSON status document.
