# IronDrop Deployment Guide v2.6.4

## Overview

This guide covers deployment strategies, operational considerations, and best practices for running IronDrop in various environments.

## Quick Start Deployment

### Single Binary Installation

```bash
# Build from source
git clone https://github.com/dev-harsh1998/IronDrop.git
cd IronDrop
cargo build --release

# Install globally
sudo mv target/release/irondrop /usr/local/bin/

# Basic deployment
irondrop -d /srv/files --listen 0.0.0.0 --port 8080
```

### Verification

```bash
# Test server health
curl http://localhost:8080/_health

# Test file listing
curl http://localhost:8080/

# Test upload (if enabled)
curl -X POST -F "file=@test.txt" http://localhost:8080/upload
```

## Production Deployment

### System Requirements

#### Minimum System Requirements
| Resource | Minimum | Recommended | Notes |
|----------|---------|-------------|-------|
| CPU | 1 core | 2+ cores | More cores improve concurrent handling |
| RAM | 512MB | 2GB+ | Depends on concurrent uploads and file sizes |
| Disk | 1GB | 10GB+ | Based on served content and upload storage |
| Network | 100Mbps | 1Gbps+ | For high-throughput file serving |

#### Operating System Support
- **Linux**: Ubuntu 20.04+, CentOS 8+, Debian 11+, Alpine 3.14+
- **macOS**: 10.15+ (Catalina and newer)
- **Windows**: Windows 10, Windows Server 2019+

### Production Configuration

#### Basic Production Setup
```bash
# Create dedicated user
sudo useradd -r -s /bin/false irondrop

# Create directories
sudo mkdir -p /srv/irondrop/{files,uploads,logs}
sudo chown -R irondrop:irondrop /srv/irondrop

# Production command
sudo -u irondrop irondrop \
  --directory /srv/irondrop/files \
  --listen 0.0.0.0 \
  --port 8080 \
  --threads 16 \
  --chunk-size 8192 \
  --enable-upload \
  --upload-dir /srv/irondrop/uploads \
  --max-upload-size 5120 \
  --allowed-extensions "*.pdf,*.txt,*.jpg,*.png,*.zip" \
  --username admin \
  --password "$(openssl rand -base64 32)" \
  --detailed-logging
```

#### Environment Variables
```bash
# Logging configuration
export RUST_LOG=info
export RUST_BACKTRACE=1

# Production optimizations
export RUST_MIN_STACK=2097152  # 2MB stack size
```

## Service Management

### systemd Service (Linux)

Create `/etc/systemd/system/irondrop.service`:

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
ExecStart=/usr/local/bin/irondrop \
  --directory /srv/irondrop/files \
  --listen 0.0.0.0 \
  --port 8080 \
  --threads 16 \
  --enable-upload \
  --upload-dir /srv/irondrop/uploads \
  --max-upload-size 5120 \
  --allowed-extensions "*.pdf,*.txt,*.jpg,*.png,*.zip" \
  --username admin \
  --password-file /srv/irondrop/password \
  --detailed-logging

# Security settings
PrivateTmp=yes
PrivateDevices=yes
ProtectHome=yes
ProtectSystem=strict
ReadWritePaths=/srv/irondrop
NoNewPrivileges=true
MemoryDenyWriteExecute=true
RestrictRealtime=true
RestrictSUIDSGID=true
LockPersonality=true
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Restart policy
Restart=always
RestartSec=5s
StartLimitInterval=0

# Environment
Environment=RUST_LOG=info
Environment=RUST_BACKTRACE=1

[Install]
WantedBy=multi-user.target
```

**Service Management:**
```bash
# Enable and start service
sudo systemctl enable irondrop
sudo systemctl start irondrop

# Check status
sudo systemctl status irondrop

# View logs
sudo journalctl -u irondrop -f

# Reload configuration
sudo systemctl reload irondrop
```

### Docker Deployment

#### Dockerfile
```dockerfile
FROM rust:1.88-alpine AS builder

WORKDIR /app
COPY . .
RUN apk add --no-cache musl-dev && \
    cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:3.18
RUN apk add --no-cache ca-certificates

# Create non-root user
RUN adduser -D -s /bin/sh irondrop

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/irondrop /usr/local/bin/

# Create directories
RUN mkdir -p /srv/files /srv/uploads && \
    chown -R irondrop:irondrop /srv

USER irondrop
WORKDIR /srv

EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:8080/_health || exit 1

CMD ["irondrop", "--directory", "/srv/files", "--listen", "0.0.0.0", "--port", "8080"]
```

#### Docker Compose
```yaml
version: '3.8'

services:
  irondrop:
    build: .
    ports:
      - "8080:8080"
    volumes:
      - ./files:/srv/files:ro
      - ./uploads:/srv/uploads:rw
    environment:
      - RUST_LOG=info
    command: >
      irondrop
      --directory /srv/files
      --listen 0.0.0.0
      --port 8080
      --threads 16
      --enable-upload
      --upload-dir /srv/uploads
      --max-upload-size 5120
      --allowed-extensions "*.pdf,*.txt,*.jpg,*.png,*.zip"
      --detailed-logging
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "--spider", "-q", "http://localhost:8080/_health"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 10s
```

**Docker Commands:**
```bash
# Build and run
docker-compose up -d

# Scale service
docker-compose up -d --scale irondrop=3

# View logs
docker-compose logs -f irondrop

# Update
docker-compose pull && docker-compose up -d
```

## Reverse Proxy Configuration

### nginx Configuration

```nginx
# /etc/nginx/sites-available/irondrop
upstream irondrop_backend {
    server 127.0.0.1:8080;
    # For multiple instances:
    # server 127.0.0.1:8081;
    # server 127.0.0.1:8082;
}

server {
    listen 80;
    server_name files.example.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name files.example.com;

    # SSL configuration
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;

    # Security headers
    add_header X-Frame-Options DENY always;
    add_header X-Content-Type-Options nosniff always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Strict-Transport-Security "max-age=63072000" always;

    # Upload size limit
    client_max_body_size 10G;
    client_body_timeout 300s;

    # Compression
    gzip on;
    gzip_vary on;
    gzip_min_length 1024;
    gzip_types text/plain text/css application/javascript application/json;

    location / {
        proxy_pass http://irondrop_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeouts
        proxy_connect_timeout 30s;
        proxy_send_timeout 300s;
        proxy_read_timeout 300s;

        # Buffer settings for large uploads
        proxy_buffering off;
        proxy_request_buffering off;
    }

    # Health check endpoint
    location /_health {
        proxy_pass http://irondrop_backend;
        access_log off;
    }

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=uploads:10m rate=10r/m;
    location /upload {
        limit_req zone=uploads burst=5 nodelay;
        proxy_pass http://irondrop_backend;
        
        # Extended timeouts for uploads
        proxy_send_timeout 600s;
        proxy_read_timeout 600s;
    }
}
```

### Apache Configuration

```apache
# /etc/apache2/sites-available/irondrop.conf
<VirtualHost *:80>
    ServerName files.example.com
    Redirect permanent / https://files.example.com/
</VirtualHost>

<VirtualHost *:443>
    ServerName files.example.com

    # SSL configuration
    SSLEngine on
    SSLCertificateFile /path/to/cert.pem
    SSLCertificateKeyFile /path/to/key.pem
    SSLProtocol all -SSLv3 -TLSv1 -TLSv1.1
    SSLCipherSuite ECDHE+AESGCM:ECDHE+CHACHA20:DHE+AESGCM:DHE+CHACHA20:!aNULL:!MD5:!DSS

    # Security headers
    Header always set X-Frame-Options DENY
    Header always set X-Content-Type-Options nosniff
    Header always set X-XSS-Protection "1; mode=block"
    Header always set Strict-Transport-Security "max-age=63072000"

    # Upload limits
    LimitRequestBody 10737418240  # 10GB

    # Compression
    LoadModule deflate_module modules/mod_deflate.so
    <Location />
        SetOutputFilter DEFLATE
        SetEnvIfNoCase Request_URI \
            \.(?:gif|jpe?g|png|zip|tar|gz)$ no-gzip dont-vary
    </Location>

    # Proxy configuration
    ProxyPreserveHost On
    ProxyRequests Off

    <Proxy *>
        Order deny,allow
        Allow from all
    </Proxy>

    ProxyPass /_health http://127.0.0.1:8080/_health
    ProxyPassReverse /_health http://127.0.0.1:8080/_health

    ProxyPass / http://127.0.0.1:8080/
    ProxyPassReverse / http://127.0.0.1:8080/
</VirtualHost>
```

## Monitoring and Observability

### Health Monitoring

**Basic Health Check Script:**
```bash
#!/bin/bash
# /usr/local/bin/check-irondrop.sh

HEALTH_URL="http://localhost:8080/_health"
EXPECTED_STATUS="healthy"

response=$(curl -s -f "$HEALTH_URL" | jq -r '.status' 2>/dev/null)

if [ "$response" = "$EXPECTED_STATUS" ]; then
    echo "IronDrop is healthy"
    exit 0
else
    echo "IronDrop health check failed: $response"
    exit 1
fi
```

**Cron Job for Monitoring:**
```bash
# Check every 5 minutes
*/5 * * * * /usr/local/bin/check-irondrop.sh || logger "IronDrop health check failed"
```

### Log Management

**Log Rotation Configuration:**
```bash
# /etc/logrotate.d/irondrop
/var/log/irondrop/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 0644 irondrop irondrop
    postrotate
        systemctl reload irondrop
    endscript
}
```

**Structured Logging with journald:**
```bash
# Query logs
journalctl -u irondrop --since "1 hour ago" --follow

# Filter by log level
journalctl -u irondrop -p err

# Export logs for analysis
journalctl -u irondrop --since "1 day ago" -o json > irondrop.log
```

### Prometheus Metrics (Future Enhancement)

While not currently implemented, here's a suggested metrics endpoint:

```yaml
# Potential metrics
irondrop_requests_total{method="GET", status="200"}
irondrop_request_duration_seconds{method="GET", path="/"}
irondrop_upload_size_bytes{status="success"}
irondrop_concurrent_connections
irondrop_rate_limit_hits_total
irondrop_file_operations_total{operation="download"}
```

## Security Hardening

### File System Security

```bash
# Set restrictive permissions
chmod 750 /srv/irondrop
chmod 640 /srv/irondrop/files/*
chmod 755 /srv/irondrop/uploads

# Use extended attributes (Linux)
setfacl -d -m u::rwx,g::rx,o::- /srv/irondrop/uploads

# SELinux context (RHEL/CentOS)
setsebool -P httpd_can_network_connect 1
semanage fcontext -a -t httpd_exec_t /usr/local/bin/irondrop
restorecon -v /usr/local/bin/irondrop
```

### Network Security

**Firewall Configuration (iptables):**
```bash
# Allow HTTP/HTTPS traffic
iptables -A INPUT -p tcp --dport 80 -j ACCEPT
iptables -A INPUT -p tcp --dport 443 -j ACCEPT

# Rate limiting at network level
iptables -A INPUT -p tcp --dport 8080 -m state --state NEW -m recent --set
iptables -A INPUT -p tcp --dport 8080 -m state --state NEW -m recent --update --seconds 60 --hitcount 20 -j DROP
```

**Firewall Configuration (ufw):**
```bash
ufw allow 80/tcp
ufw allow 443/tcp
ufw limit 8080/tcp
ufw enable
```

### Application Security

**Security Configuration:**
```bash
# Generate strong password
openssl rand -base64 32 > /srv/irondrop/password
chmod 600 /srv/irondrop/password
chown irondrop:irondrop /srv/irondrop/password

# Run with additional security
irondrop \
  --directory /srv/irondrop/files \
  --upload-dir /srv/irondrop/uploads \
  --allowed-extensions "*.txt,*.pdf" \
  --max-upload-size 100 \
  --username admin \
  --password-file /srv/irondrop/password
```

## Performance Optimization

### System Tuning

**File Descriptor Limits:**
```bash
# /etc/security/limits.conf
irondrop soft nofile 65536
irondrop hard nofile 65536
```

**Network Tuning:**
```bash
# /etc/sysctl.conf
net.core.somaxconn = 65535
net.core.netdev_max_backlog = 5000
net.ipv4.tcp_max_syn_backlog = 65536
net.ipv4.tcp_fin_timeout = 15
```

### Application Tuning

**High-Performance Configuration:**
```bash
irondrop \
  --directory /srv/files \
  --threads 32 \
  --chunk-size 65536 \
  --listen 0.0.0.0 \
  --port 8080
```

**Memory Optimization:**
```bash
# Set optimal stack size
export RUST_MIN_STACK=1048576  # 1MB

# Reduce memory fragmentation
export MALLOC_ARENA_MAX=2
```

## Backup and Recovery

### Backup Strategy

**Configuration Backup:**
```bash
#!/bin/bash
# backup-irondrop-config.sh

BACKUP_DIR="/backup/irondrop/$(date +%Y%m%d)"
mkdir -p "$BACKUP_DIR"

# Backup configuration
cp /etc/systemd/system/irondrop.service "$BACKUP_DIR/"
cp /srv/irondrop/password "$BACKUP_DIR/"

# Backup uploaded files
tar -czf "$BACKUP_DIR/uploads.tar.gz" /srv/irondrop/uploads/

# Backup served files (if managed by IronDrop)
tar -czf "$BACKUP_DIR/files.tar.gz" /srv/irondrop/files/

echo "Backup completed: $BACKUP_DIR"
```

**Recovery Procedure:**
```bash
#!/bin/bash
# restore-irondrop.sh

BACKUP_DIR="$1"
if [ -z "$BACKUP_DIR" ]; then
    echo "Usage: $0 <backup-directory>"
    exit 1
fi

systemctl stop irondrop

# Restore files
tar -xzf "$BACKUP_DIR/uploads.tar.gz" -C /
tar -xzf "$BACKUP_DIR/files.tar.gz" -C /

# Restore configuration
cp "$BACKUP_DIR/irondrop.service" /etc/systemd/system/
cp "$BACKUP_DIR/password" /srv/irondrop/

# Fix permissions
chown -R irondrop:irondrop /srv/irondrop

systemctl daemon-reload
systemctl start irondrop

echo "Recovery completed"
```

## Troubleshooting

### Common Issues

**Port Already in Use:**
```bash
# Find process using port
sudo lsof -i :8080
sudo netstat -tulpn | grep :8080

# Kill process if necessary
sudo kill -9 <PID>
```

**Permission Denied:**
```bash
# Check file permissions
ls -la /srv/irondrop/
sudo chown -R irondrop:irondrop /srv/irondrop/

# Check SELinux (RHEL/CentOS)
sudo sealert -a /var/log/audit/audit.log
```

**High Memory Usage:**
```bash
# Monitor memory usage
ps aux | grep irondrop
systemctl status irondrop

# Check for memory leaks
valgrind --leak-check=full irondrop -d /tmp
```

**Upload Failures:**
```bash
# Check disk space
df -h /srv/irondrop/uploads/

# Check upload directory permissions
ls -ld /srv/irondrop/uploads/
sudo -u irondrop touch /srv/irondrop/uploads/test
```

### Debugging Commands

```bash
# Enable debug logging
RUST_LOG=debug irondrop -d /srv/files -v

# Enable backtrace
RUST_BACKTRACE=1 irondrop -d /srv/files

# Network debugging
tcpdump -i any port 8080
ss -tulpn | grep :8080

# Performance profiling
perf record -g irondrop -d /srv/files
strace -p $(pgrep irondrop)
```

This deployment guide provides comprehensive coverage of production deployment scenarios and operational best practices for IronDrop v2.6.4.