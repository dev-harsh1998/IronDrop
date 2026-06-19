# IronDrop

IronDrop is a Rust file server for browsing directories, downloading files, optional uploads, built-in monitoring, search, and WebDAV. It ships as a single binary with embedded HTML, CSS, and JavaScript assets.

## Features

- Directory browsing with embedded UI templates
- File downloads with MIME detection and `Accept-Ranges: bytes`
- Optional uploads through `/_irondrop/upload`
- Built-in search at `/_irondrop/search`
- Monitoring pages at `/monitor` and `/_irondrop/monitor`
- Optional Basic Auth for the whole server
- Built-in HTTPS with `--ssl-cert` and `--ssl-key`
- Reverse proxy subpath support with `--base-path`
- Optional WebDAV support for `OPTIONS`, `PROPFIND`, `PROPPATCH`, `MKCOL`, `PUT`, `DELETE`, `COPY`, `MOVE`, `LOCK`, and `UNLOCK`

## Install

### crates.io

```bash
cargo install irondrop
```

### From source

```bash
git clone https://github.com/dev-harsh1998/IronDrop.git
cd IronDrop
cargo build --release
./target/release/irondrop --help
```

## Quick Start

Serve the current directory on `127.0.0.1:8080`:

```bash
irondrop -d .
```

Share on your LAN:

```bash
irondrop -d . --listen 0.0.0.0
```

Enable uploads and WebDAV explicitly:

```bash
irondrop -d ./shared       --enable-upload true       --enable-webdav true       --listen 0.0.0.0
```

Run with native HTTPS:

```bash
openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 365 -nodes -subj '/CN=localhost'

irondrop -d ./shared       --ssl-cert cert.pem       --ssl-key key.pem       --listen 0.0.0.0
```

## Common Endpoints

- `/` and `/<path>/`: directory listing and file download surface
- `/_irondrop/upload`: upload form on `GET`, upload handler on `POST`
- `/_irondrop/search?q=<term>&path=/`: JSON search API
- `/monitor` and `/_irondrop/monitor`: HTML monitoring page
- `/monitor?json=1` and `/_irondrop/monitor?json=1`: JSON monitoring payload
- `/_irondrop/health`: health payload
- `/_irondrop/status`: status payload, currently the same as health
- `/_health`: legacy compatibility health route

## Upload Behavior

Uploads are disabled by default.

When uploads are enabled, files are written into the served directory tree:

- `GET /_irondrop/upload` renders the embedded upload page
- `POST /_irondrop/upload` accepts the file body
- `upload_to=/subdir` targets a subdirectory inside the served tree
- filenames are taken from `Content-Disposition`, then `X-Filename`, then the URL path
- small request bodies stay in memory and larger ones are spooled to a temporary file before the final atomic write

There is no separate `--upload-dir` flag in the current implementation.

## Search Behavior

Search is initialized at startup for the served directory and exposed through `/_irondrop/search`.

Supported query parameters:

- `q`: required, 2 to 100 characters
- `path`: optional subdirectory filter, default `/`
- `limit`: optional, default `50`, capped at `200`
- `offset`: optional, default `0`

The response is a JSON array of objects like:

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

## Configuration

IronDrop currently resolves configuration in this order:

1. CLI flags
2. INI configuration file
3. Built-in defaults

The current config loader does not read `IRONDROP_*` environment variables.

Config file discovery order when `--config-file` is not provided:

1. `./irondrop.ini`
2. `./irondrop.conf`
3. `$HOME/.config/irondrop/config.ini`
4. `/etc/irondrop/config.ini` on Unix

Important current defaults:

- listen address: `127.0.0.1`
- port: `8080`
- worker threads: `8`
- chunk size: `1024`
- uploads: disabled
- WebDAV: disabled
- allowed extensions default from the config layer: `*.zip,*.txt`

## Reverse Proxy Subpath

To serve IronDrop behind a subpath, start it with `--base-path` and forward the full path without stripping:

```bash
irondrop -d ./shared --base-path /webstorage --listen 0.0.0.0
```

Example nginx location:

```nginx
location /webstorage/ {
    proxy_pass http://127.0.0.1:8080/webstorage/;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    client_max_body_size 0;
    proxy_buffering off;
    proxy_request_buffering off;
}
```

## WebDAV

WebDAV is off by default. Enable it with `--enable-webdav true` or in the `[webdav]` section of the config file.

Current scope:

- methods: `OPTIONS`, `PROPFIND`, `PROPPATCH`, `MKCOL`, `PUT`, `DELETE`, `COPY`, `MOVE`, `LOCK`, `UNLOCK`
- advertised capabilities: `DAV: 1,2`
- lock and dead-property state is in-memory only

See `doc/WEBDAV_IMPLEMENTATION.md` for the implementation notes.

## Documentation

The curated documentation entry point is `doc/README.md`.

Recommended docs:

- `doc/API_REFERENCE.md`
- `doc/CONFIGURATION_SYSTEM.md`
- `doc/DEPLOYMENT.md`
- `doc/MONITORING.md`
- `doc/HTTP_STREAMING.md`
- `doc/WEBDAV_IMPLEMENTATION.md`

## Testing

Run the test suite with:

```bash
cargo test
```

## License

IronDrop is licensed under the MIT License. See `LICENSE`.
