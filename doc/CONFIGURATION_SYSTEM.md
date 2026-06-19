# IronDrop Configuration System

This document describes the configuration behavior implemented today in `src/config/mod.rs` and related CLI code.

## Precedence

Current precedence is:

1. CLI flags
2. INI file values
3. built-in defaults

The current config loader does not apply `IRONDROP_*` environment-variable overrides.

The only environment-sensitive behavior in the current startup path is logging: if `RUST_LOG` is already set, IronDrop keeps that value instead of setting its own default log level.

## Config File Discovery

If `--config-file <path>` is provided, that exact file is loaded and startup fails if it does not exist.

Otherwise, IronDrop searches in this order:

1. `./irondrop.ini`
2. `./irondrop.conf`
3. `$HOME/.config/irondrop/config.ini`
4. `/etc/irondrop/config.ini` on Unix

## Required CLI Input

`--directory` is required on the CLI.

Although the sample config file contains a `directory = ...` comment in the `[server]` section, the current implementation always takes the served directory from the CLI argument.

## Supported Sections And Keys

### `[server]`

- `listen`
- `port`
- `threads`
- `chunk_size`
- `base_path`
- `enable_webdav` is also accepted here as a compatibility fallback, though `[webdav]` is the preferred section

### `[upload]`

- `enable_upload`
- `max_upload_size`
- `max_size` as a backward-compatible alias

Notes:

- upload size values are parsed as bytes from human-readable strings such as `100MB`, `2GB`, or `1.5GB`
- there is no config key for a separate upload directory because uploads are written inside the served directory tree

### `[webdav]`

- `enable_webdav`
- `disable_rate_limit`

`disable_rate_limit` only takes effect when WebDAV is enabled.

### `[auth]`

- `username`
- `password`

### `[security]`

- `allowed_extensions`

This is parsed as a comma-separated list of glob patterns.

### `[logging]`

- `verbose`
- `detailed`
- `log_dir`

### `[ssl]`

- `cert`
- `key`

Both must be present together to enable HTTPS.

## Current Defaults

Defaults applied by `Config::load()`:

- `listen = 127.0.0.1`
- `port = 8080`
- `threads = 8`
- `chunk_size = 1024`
- `enable_upload = false`
- `enable_webdav = false`
- `disable_rate_limit = false`
- `allowed_extensions = *.zip,*.txt`
- `verbose = false`
- `detailed = false`
- `base_path = ""`
- `max_upload_size = unlimited` at the config layer, subject to HTTP request parsing limits

## CLI Flags In The Current Codebase

The `Cli` struct currently exposes these user-facing options:

- `-d`, `--directory`
- `-l`, `--listen`
- `-p`, `--port`
- `-a`, `--allowed-extensions`
- `-t`, `--threads`
- `-c`, `--chunk-size`
- `-v`, `--verbose`
- `--detailed-logging`
- `--username`
- `--password`
- `--enable-upload`
- `--max-upload-size`
- `--enable-webdav`
- `--disable-rate-limit`
- `--config-file`
- `--log-dir`
- `--ssl-cert`
- `--ssl-key`
- `--base-path`

The current codebase does not expose:

- `--upload-dir`
- `--password-file`
- `IRONDROP_*` config overrides

## Example INI File

```ini
[server]
listen = 0.0.0.0
port = 8080
threads = 16
chunk_size = 8192
base_path = /files

[upload]
enable_upload = true
max_upload_size = 5GB

[webdav]
enable_webdav = true
disable_rate_limit = false

[auth]
username = admin
password = change-me

[security]
allowed_extensions = *.pdf,*.txt,*.jpg,*.png,*.zip

[logging]
detailed = true

[ssl]
cert = /etc/irondrop/cert.pem
key = /etc/irondrop/key.pem
```

Start the server with:

```bash
irondrop -d /srv/files --config-file /etc/irondrop/config.ini
```

## Logging Behavior

At startup, IronDrop maps config values to log levels like this when `RUST_LOG` is not already set:

- `verbose = true` -> `debug`
- `detailed = true` -> `info`
- otherwise -> `warn`

If `log_dir` is set, IronDrop writes to a timestamped log file in that directory. The directory must already exist and be writable.

## Validation Notes

Current validation includes:

- the served directory must exist and be a directory
- `--ssl-cert` and `--ssl-key` must be provided together
- `--config-file` must point to an existing readable file
- `--log-dir` must already exist and be writable
- `--base-path` is normalized to start with `/` and not end with `/`
- `--max-upload-size` must be greater than zero
