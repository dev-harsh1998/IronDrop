# IronDrop Documentation Index

This index tracks the current documentation that matches the code in this repository.

## Core Guides

- `API_REFERENCE.md`: public HTTP routes, request formats, responses, auth, and monitoring endpoints
- `CONFIGURATION_SYSTEM.md`: CLI and INI configuration behavior, discovery order, and real precedence rules
- `DEPLOYMENT.md`: local, systemd, Docker, HTTPS, and reverse proxy deployment notes
- `ARCHITECTURE.md`: internal module layout and high-level design notes

## Feature Guides

- `HTTP_STREAMING.md`: request body buffering and disk-spooling behavior for uploads
- `MONITORING.md`: `/monitor`, `/_irondrop/monitor`, health routes, and JSON metrics
- `WEBDAV_IMPLEMENTATION.md`: implemented WebDAV methods and RFC-focused behavior
- `SEARCH_FEATURE.md`: current search endpoint and result model
- `TEMPLATE_SYSTEM.md`: embedded templates and static asset layout

## Supporting References

- `RFC_OWASP_COMPLIANCE.md`: security posture and standards-oriented notes
- `TESTING_DOCUMENTATION.md`: test inventory and execution notes
- `MULTIPART_README.md`: legacy upload/body-processing background material; treat it as historical context unless it matches the newer guides above

## Current Documentation Notes

The current codebase behavior differs from some older design notes that may still appear elsewhere in the repository:

- the config loader currently uses `CLI > INI > defaults`
- `IRONDROP_*` environment-variable config overrides are not implemented
- uploads do not use a separate `--upload-dir` flag
- the public search endpoint is `/_irondrop/search`, not `/api/search`
- directory listings are HTML pages, not a separate JSON listing API

Start with `../README.md` for a quick overview, then use the focused guides above for implementation details.
