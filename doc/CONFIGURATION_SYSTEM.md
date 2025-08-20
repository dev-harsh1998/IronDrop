## IronDrop Configuration System (v2.6)

### Overview
IronDrop 2.5 introduces a first‑class configuration system with hierarchical precedence and zero external dependencies. It complements (not replaces) the existing CLI flags, enabling reproducible deployments, easier automation, and environment portability. The system is intentionally simple: an internal INI parser (`src/config/ini_parser.rs`) plus a composition layer (`src/config/mod.rs`) that merges values from multiple sources.

### Goals
* Deterministic startup configuration (documented precedence)
* Human‑readable, comment‑friendly format (INI)
* Zero dependencies (fully in‑tree parser)
* Security by validation (size bounds, directory integrity, auth opt‑in)
* Backwards compatibility (all existing CLI flags still function)

### Precedence Model
Highest → Lowest (first match wins):
1. CLI Flag (explicit non‑default value)  
2. INI File Value  
3. Built‑in Default

The INI file itself is optional. If absent, behavior is identical to pre‑2.5 versions except for the new `--config-file` flag.

### Configuration File Discovery Order
If `--config-file <path>` is NOT provided, IronDrop searches:
1. `./irondrop.ini`
2. `./irondrop.conf`
3. `$HOME/.config/irondrop/config.ini`
4. `/etc/irondrop/config.ini` (Unix only)

If none exist, startup proceeds with defaults + CLI overrides.

### New CLI Flag
| Flag | Description |
|------|-------------|
| `--config-file <path>` | Explicit path to an INI configuration file. Errors if not found. |

### INI Format Features
* Sections (`[server]`, `[upload]`, `[auth]`, `[logging]`, `[security]`)
* Comments starting with `#` or `;`
* Key = value pairs (whitespace tolerant)
* Inline comments after values (`key = value  # note`)
* Empty lines ignored
* Graceful handling of malformed section headers (skipped, not fatal)

### Supported Keys (by Section)
```
[server]
listen = 0.0.0.0            # String (IP or hostname)
port = 8080                 # Integer (u16)
threads = 16                # Integer (usize)
chunk_size = 2048           # Integer (usize, bytes per read)
directory = /data/files     # (Not used for precedence; directory always comes from CLI)

[upload]
enabled = true              # bool (true/false/yes/no/on/off/1/0)
max_size = 5GB              # File size parser (B, KB, MB, GB, TB; decimals allowed: 1.5GB)
directory = /data/uploads   # Optional override for upload target

[auth]
username = alice
password = secret123

[security]
allowed_extensions = *.zip,*.txt,*.pdf

[logging]
verbose = true              # Enables debug logging
detailed = false            # Enables info‑level below verbose
```

### Data Type Parsing
| Type | Behavior |
|------|----------|
| Boolean | Case‑insensitive: true/false, yes/no, on/off, 1/0 |
| Integer | Parsed via `str::parse()`; invalid -> ignored (falls back) |
| File Size | Supports suffixes B / KB / MB / GB / TB; decimal numeric part accepted |
| List | Comma‑separated, trimmed entries; empty entries removed |

### Internal Architecture
Component | Responsibility | File
----------|----------------|------
`IniConfig` | Parse & store raw key/value data | `src/config/ini_parser.rs`
`Config` | Merge CLI + INI + defaults; expose strongly typed fields | `src/config/mod.rs`
`Config::load()` | Orchestrates discovery, parsing, precedence, assembly | `src/config/mod.rs`
`run_server_with_config()` | Transitional adapter (Config → Cli) | `src/server.rs`

### Safety & Validation
* Explicit error if user supplies `--config-file` and file is missing.
* Upload size normalized to bytes internally (CLI still in MB for backwards compatibility).
* Directory for serving content always comes from required CLI `--directory` (prevents surprising relocation by config files outside working context).
* Parser avoids panics: malformed lines are either validated or produce targeted errors (empty key, empty section) while benign malformed section headers are ignored.

### Example Minimal INI
```
[server]
listen = 0.0.0.0
port = 9090

[upload]
enabled = true
max_size = 2.5GB

[auth]
username = demo
password = changeMe!
```

### Example Combined Usage
```
irondrop -d ./public --config-file prod.ini --threads 32 --verbose
```
Explanation:
* `threads` + `verbose` come from CLI (override INI).
* Remaining unset CLI values (e.g., port/listen) come from `prod.ini`.
* Any unspecified keys fall back to defaults.

### Migration Guidance (Pre‑2.5 → 2.5)
Scenario | Action
---------|-------
Existing shell scripts | Keep working; optionally add a pinned INI for reproducibility.
Multiple environments | Create `irondrop.{dev,staging,prod}.ini`; select via `--config-file`.
Secrets management | Keep credentials out of scripts; place in controlled permission INI.

### Test Coverage (Highlights)
Test Focus | File | Purpose
-----------|------|--------
INI parsing primitives | `tests/config_test.rs` | Booleans, lists, file sizes, comments
Precedence correctness | `tests/config_test.rs` | CLI overrides vs INI
Upload configuration | `tests/config_test.rs` | Directory + size conversions
Authentication fields | `tests/config_test.rs` | Username/password propagation
Edge cases | `src/config/ini_parser.rs` (unit tests) | Malformed sections, decimal sizes

### Future Enhancements
Planned ideas (not yet implemented):
* Environment variable interpolation (`${VAR}`) with allow‑list
* Hot reload signal (SIGHUP) for config values safe to update (logging, limits)
* Export effective configuration as JSON via an admin endpoint
* Validate `allowed_extensions` globs at load time with detailed diagnostics

### Quick Troubleshooting
Symptom | Likely Cause | Fix
--------|--------------|----
"Config file specified but not found" | Wrong path to `--config-file` | Use absolute path or place file in working dir
Upload larger than expected limit rejected | `max_upload_size` parsed lower than intended | Ensure suffix (e.g., `5GB` not `5G`)
Verbose logging not active | Only `detailed` set in INI | Use `verbose = true` OR `--verbose`
Auth not enforced | Missing `[auth]` values | Supply both `username` and `password`

### Summary
The configuration system provides deterministic, transparent startup behavior while retaining the original simplicity of the CLI interface. It is intentionally minimal, auditable, and fully covered by tests to ensure reliability in production deployments.

---
Return to documentation index: [./README.md](./README.md)
