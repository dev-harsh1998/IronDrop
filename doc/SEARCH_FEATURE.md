# IronDrop Search Feature

This guide documents the current search API and the user-visible behavior of the search subsystem.

## Public Endpoint

Search is exposed through:

- `GET /_irondrop/search`

When `--base-path` is configured, prefix the route with that base path.

Example:

```bash
curl 'http://127.0.0.1:8080/_irondrop/search?q=report&path=/&limit=10&offset=0'
```

## Query Parameters

- `q`: required, between 2 and 100 characters
- `path`: optional, default `/`
- `limit`: optional, default `50`, capped at `200`
- `offset`: optional, default `0`

The current HTTP handler always uses case-insensitive search for public requests.

## Response Shape

The current API returns a plain JSON array.

Example:

```json
[
  {
    "name": "document.txt",
    "path": "/docs/document.txt",
    "size": "8 B",
    "type": "file"
  },
  {
    "name": "reports",
    "path": "/docs/reports/",
    "size": "-",
    "type": "directory"
  }
]
```

Notes:

- directory paths in results end with `/`
- there is no wrapped `status`, `pagination`, or `search_stats` object in the current response
- there is no `/api/search` endpoint in the current codebase

## Implementation Notes

- search is initialized at server startup for the served directory
- results are sorted by internal score before pagination is applied
- the codebase contains both regular search logic and an ultra-compact memory-focused path for large trees
- when the in-memory index returns no results, the implementation can fall back to filesystem search

## Limits And Errors

Common failure cases:

- missing `q` -> `400 Bad Request`
- query length below 2 or above 100 -> `400 Bad Request`
- invalid route base path when `--base-path` is enabled -> `404 Not Found`

## Current Documentation Corrections

This document reflects the implementation as it exists today:

- no `IRONDROP_SEARCH_*` configuration variables are implemented
- no `/api/search` route exists
- the public search response is an array, not a wrapped object
- the public HTTP handler does not expose a `case_sensitive` query parameter
