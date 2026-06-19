# IronDrop Deployment Guide

This guide focuses on deployment patterns that match the current codebase and CLI.

## Quick Local Run

```bash
irondrop -d /srv/files --listen 0.0.0.0 --port 8080
```

Verify the server:

```bash
curl http://127.0.0.1:8080/_irondrop/health
curl http://127.0.0.1:8080/
curl 'http://127.0.0.1:8080/_irondrop/search?q=test&path=/'
```

## Production Notes

- use a dedicated service account
- make the served directory readable by that account
- if uploads are enabled, the served directory tree also needs write permission where uploads should land
- if `log_dir` is configured, create it before startup
- if auth is enabled, monitoring and health endpoints also require credentials

## Config-File Driven Deployment

The easiest way to keep production settings consistent is to place most settings in an INI file and keep `--directory` on the command line.

Example `/etc/irondrop/config.ini`:

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
log_dir = /var/log/irondrop
```

Start it with:

```bash
irondrop -d /srv/irondrop/files --config-file /etc/irondrop/config.ini
```

## systemd

Example `/etc/systemd/system/irondrop.service`:

```ini
[Unit]
Description=IronDrop File Server
After=network.target
Wants=network.target

[Service]
Type=simple
User=irondrop
Group=irondrop
WorkingDirectory=/srv/irondrop
ExecStart=/usr/local/bin/irondrop --directory /srv/irondrop/files --config-file /etc/irondrop/config.ini
Restart=always
RestartSec=5
NoNewPrivileges=true
PrivateTmp=true
ProtectHome=true
ProtectSystem=strict
ReadWritePaths=/srv/irondrop /var/log/irondrop
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

Common commands:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now irondrop
sudo systemctl status irondrop
sudo journalctl -u irondrop -f
```

## Native HTTPS

IronDrop can serve HTTPS directly through `rustls`.

Create a self-signed cert for testing:

```bash
openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 365 -nodes -subj '/CN=localhost'
```

Start HTTPS:

```bash
irondrop -d /srv/files --ssl-cert cert.pem --ssl-key key.pem --listen 0.0.0.0 --port 8443
```

Notes:

- both `--ssl-cert` and `--ssl-key` are required
- the current TLS stack supports TLS 1.2 and 1.3 through `rustls`
- native HTTPS is often enough for simple deployments

## Reverse Proxy

A reverse proxy is optional, but useful for HTTP/2, central TLS management, or broader ingress policy.

### Root deployment

```nginx
location / {
    proxy_pass http://127.0.0.1:8080;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    client_max_body_size 0;
    proxy_buffering off;
    proxy_request_buffering off;
}
```

### Subpath deployment

Start IronDrop with a base path:

```bash
irondrop -d /srv/files --base-path /webstorage --listen 0.0.0.0
```

Proxy the full path through nginx without stripping it:

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

No `sub_filter` rules are required for the current base-path implementation.

## Docker

A minimal container flow:

```dockerfile
FROM rust:1.88-alpine AS builder
WORKDIR /app
COPY . .
RUN apk add --no-cache musl-dev && cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:3.20
RUN adduser -D -s /bin/sh irondrop
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/irondrop /usr/local/bin/irondrop
USER irondrop
WORKDIR /srv
EXPOSE 8080
CMD ["irondrop", "-d", "/srv/files", "--listen", "0.0.0.0"]
```

Runtime notes:

- mount `/srv/files` read-only if uploads are disabled
- mount it read-write if uploads are enabled because uploads land inside the served tree

Example compose service:

```yaml
services:
  irondrop:
    build: .
    ports:
      - "8080:8080"
    volumes:
      - ./files:/srv/files
    command:
      - irondrop
      - -d
      - /srv/files
      - --listen
      - 0.0.0.0
      - --enable-upload
      - "true"
    restart: unless-stopped
```

## Monitoring And Health Checks

Useful probes:

```bash
curl -f http://127.0.0.1:8080/_irondrop/health
curl -f 'http://127.0.0.1:8080/_irondrop/monitor?json=1'
```

If Basic Auth is enabled, include credentials in those probes.

## Current CLI Reality Check

The following options are not present in the current codebase and should not be used in deployment examples:

- `--upload-dir`
- `--password-file`

The current config loader also does not consume `IRONDROP_*` application settings.
