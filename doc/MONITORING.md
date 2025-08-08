# IronDrop Monitoring Guide (v2.5)

This guide documents the built-in monitoring capabilities introduced with the `/monitor` endpoint and supporting health APIs.

## Overview

IronDrop exposes lightweight operational telemetry without external dependencies:

| Endpoint | Format | Purpose |
|----------|--------|---------|
| `/monitor` | HTML | Human dashboard for live stats |
| `/monitor?json=1` | JSON | Machine-readable metrics for scripting / scraping |
| `/_health` | JSON | Minimal liveness probe (OK / version / uptime) |
| `/_status` | JSON | Extended status (configuration + cumulative counters) |

## Data Model

`/monitor?json=1` returns three top-level sections:

```json
{
  "requests": {
    "total": 42,
    "successful": 40,
    "errors": 2,
    "bytes_served": 1048576,
    "uptime_secs": 360
  },
  "downloads": {
    "bytes_served": 1048576
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
    "average_processing_time": 152.4,
    "success_rate": 100.0
  }
}
```

### Field Semantics
- `requests.total` – All handled requests (success + error) since start.
- `requests.successful` / `errors` – Outcome classification.
- `requests.bytes_served` / `downloads.bytes_served` – Cumulative body bytes sent (excludes HTTP headers) across all responses.
- `requests.uptime_secs` – Elapsed seconds since the first server start instant.
- `uploads.*` – Aggregated upload subsystem metrics (only updated when uploads enabled).
- `average_processing_time` – Rolling average (last 100 uploads) in milliseconds.
- `success_rate` – Percentage of successful uploads over total uploads (0 if none yet).
- `concurrent_uploads` – Point-in-time counter incremented on start and decremented on completion.

## HTML Dashboard
The `/monitor` HTML view is an embedded template with:
- No external network calls (all assets embedded) 
- Auto-refresh polling every 30 seconds (JavaScript fetch to `?json=1`)
- Separate sections for Requests, Downloads, Uploads

## Usage Examples

### Quick CLI Scrape
```bash
curl -s http://localhost:8080/monitor?json=1 | jq '.requests.bytes_served'
```

### Basic Health Probe (Kubernetes / Docker)
```bash
curl -f http://localhost:8080/_health > /dev/null || echo "Unhealthy"
```

### Shell Alert When Upload Failures Detected
```bash
if [ "$(curl -s http://localhost:8080/monitor?json=1 | jq '.uploads.failed_uploads')" -gt 0 ]; then
  echo "Upload failures detected" >&2
fi
```

### Log Requests per Minute (Approximate)
```bash
prev=0
while sleep 60; do
  cur=$(curl -s http://localhost:8080/monitor?json=1 | jq '.requests.total')
  echo "RPM=$((cur-prev))"
  prev=$cur
done
```

## Integration Guidelines
- Poll interval recommendation: 15–60s (dashboard uses 30s).
- For higher resolution, build a sidecar scraper and aggregate externally.
- Avoid sub-second polling—counters are cumulative and inexpensive but not real-time high-frequency telemetry.

## Design Notes
- Byte counting performed at response write boundary (body only) to avoid skew from variable header length.
- Upload averages retained in a fixed-size ring (truncate beyond 100 samples) to bound memory.
- Stats guarded by `Arc<Mutex<T>>`; contention is minimal under typical loads. Future optimization: swap to atomics + snapshot struct.

## Extensibility Roadmap (Open for Contribution)
| Feature | Description | Status |
|---------|-------------|--------|
| Active Connections | Live in-flight connection count | Planned |
| Per-Endpoint Stats | Breakdown by route (/, /upload, /monitor, static) | Planned |
| Prometheus Export | `/metrics` in OpenMetrics format | Planned |
| Rolling Rates | Requests/sec and upload throughput windows | Planned |
| Error Categorization | Distribution by HTTP status class | Planned |

## Security Considerations
- Endpoint intentionally unauthenticated to support lightweight self-host setups; wrap behind reverse proxy if sensitive.
- For private deployments: require Basic Auth (same as other routes) or network ACLs.
- No user-identifying data exposed—only aggregate counters.

## Troubleshooting
| Symptom | Possible Cause | Action |
|---------|----------------|--------|
| `bytes_served` stays 0 | Only HEAD/empty responses so far or early fetch before first response recorded | Perform a file download then re-check |
| Upload counters not changing | Uploads disabled (`--enable-upload` missing) | Start server with `--enable-upload` |
| High error count | Client probing invalid paths | Inspect logs (`RUST_LOG=info` or `-v`) |
| Negative/Unexpected averages | Mutex poisoning from panic (rare) | Check logs for prior panic, restart server |

## Versioning
Monitoring schema may evolve with additive fields. Consumers should ignore unknown keys. Breaking changes (renames/removals) will bump minor version >= 2.x.

---
*Monitoring Guide for IronDrop v2.5*
