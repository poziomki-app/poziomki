# Deployment Guide

**Document Owner:** Operations Team
**Last Updated:** 2026-02-02

---

## 1. Prerequisites

### Server Requirements

| Component | Minimum | Recommended (10K DAU) |
|-----------|---------|----------------------|
| CPU | 2 cores | 4 cores |
| RAM | 4 GB | 8 GB |
| Storage | 50 GB SSD | 100 GB SSD |
| OS | Linux (Debian/Ubuntu/Fedora) | - |

### Required Software

- Docker 24.0+
- Docker Compose 2.20+
- Git

### Recommended Hosting

- **OVHcloud VPS** (France) — EU data residency, GDPR compliant
- Alternative: Hetzner (Germany), Scaleway (France)

---

## 2. Initial Server Setup

### SSH Access

```bash
# Generate SSH key (if needed)
ssh-keygen -t ed25519 -C "your-email@example.com"

# Copy to server
ssh-copy-id root@your-server-ip

# Disable password authentication
sudo sed -i 's/PasswordAuthentication yes/PasswordAuthentication no/' /etc/ssh/sshd_config
sudo systemctl restart sshd
```

### Firewall Configuration

```bash
# UFW (Ubuntu/Debian)
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow ssh
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw enable

# Firewalld (Fedora/RHEL)
sudo firewall-cmd --permanent --add-service=ssh
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --reload
```

### Install Docker

```bash
# Install Docker
curl -fsSL https://get.docker.com | sh

# Add user to docker group
sudo usermod -aG docker $USER

# Enable on boot
sudo systemctl enable docker

# Verify
docker --version
docker compose version
```

---

## 3. Application Deployment

### Clone Repository

```bash
# Create app directory
sudo mkdir -p /opt/poziomki
sudo chown $USER:$USER /opt/poziomki

# Clone
cd /opt/poziomki
git clone https://github.com/poziomki/poziomki.git .
```

### Environment Configuration

```bash
# Copy production environment template
cp .env.example .env.production

# Edit with production values
nano .env.production
```

**Required environment variables:**

```bash
# Application
NODE_ENV=production
API_URL=https://mobile.poziomki.app

# Database
DB_HOST=postgres
DB_PORT=5432
DB_NAME=poziomki
DB_USER=poziomki
DB_PASSWORD=<strong-password>
DB_USE_SSL=true

# Authentication
BETTER_AUTH_SECRET=<32-char-random-string>

# Object Storage (MinIO now, SeaweedFS planned)
MINIO_ROOT_USER=<minio-user>
MINIO_ROOT_PASSWORD=<strong-password>
MINIO_ENDPOINT=http://minio:9000
MINIO_BUCKET=poziomki-uploads
MINIO_USE_SSL=false
MINIO_URL_EXPIRY=3600

# Email (Resend or alternative)
RESEND_API_KEY=<your-api-key>
EMAIL_FROM=noreply@poziomki.app

# CDN
CDN_URL=https://cdn.poziomki.app
```

**Generate secure secrets:**

```bash
# Generate random secrets
openssl rand -hex 32  # For BETTER_AUTH_SECRET
openssl rand -hex 16  # For database passwords
```

### Directory Structure

```bash
# Create data directories
mkdir -p /opt/poziomki/data/postgres
mkdir -p /opt/poziomki/data/minio
mkdir -p /opt/poziomki/data/caddy
mkdir -p /opt/poziomki/backups
```

---

## 4. Docker Compose Production

### docker-compose.prod.yml

The production compose file includes:
- Health checks for all services
- Resource limits
- Persistent volumes
- Internal networking

### Deploy Services

```bash
# Pull latest images
docker compose -f docker-compose.prod.yml pull

# Start services
docker compose -f docker-compose.prod.yml up -d

# Check status
docker compose -f docker-compose.prod.yml ps

# View logs
docker compose -f docker-compose.prod.yml logs -f
```

### Run Migrations

```bash
# Run database migrations
docker compose -f docker-compose.prod.yml exec api bun run db:migrate

# Or push schema (development only)
docker compose -f docker-compose.prod.yml exec api bun run db:push
```

---

## 5. Caddy Configuration

### Caddyfile

Create `/opt/poziomki/Caddyfile`:

```caddyfile
{
    email admin@poziomki.app
}

# API endpoint
mobile.poziomki.app {
    # Security headers
    header {
        Strict-Transport-Security "max-age=31536000; includeSubDomains; preload"
        X-Content-Type-Options "nosniff"
        X-Frame-Options "DENY"
        Referrer-Policy "strict-origin-when-cross-origin"
        -Server
    }

    # Reverse proxy to API
    reverse_proxy api:3000 {
        health_uri /health
        health_interval 30s
    }

    # Access logging
    log {
        output file /var/log/caddy/access.log
        format json
    }
}

# CDN endpoint for files
cdn.poziomki.app {
    # Cache headers for immutable files
    header {
        Cache-Control "public, max-age=31536000, immutable"
        X-Content-Type-Options "nosniff"
        X-Robots-Tag "noindex, nofollow"
        -Server
    }

    # Auth check before serving files
    forward_auth api:3000 {
        uri /api/v1/uploads/auth-check
        copy_headers Authorization
    }

    # Proxy to MinIO
    reverse_proxy minio:9000

    log {
        output file /var/log/caddy/cdn.log
        format json
    }
}
```

### Enable Caddy

```bash
# Restart Caddy to pick up config
docker compose -f docker-compose.prod.yml restart caddy

# Check certificates
docker compose -f docker-compose.prod.yml exec caddy caddy list-certificates
```

---

## 6. Object Storage Setup (MinIO → SeaweedFS)

> **Migration Planned:** MinIO entered maintenance mode (Dec 2025). Migrating to SeaweedFS with Chainguard hardened images (zero-CVE, FIPS option). SeaweedFS is S3-compatible - no code changes needed.

### Current: MinIO (to be replaced)

### Create Bucket

```bash
# Access MinIO container
docker compose -f docker-compose.prod.yml exec minio sh

# Create bucket
mc alias set local http://localhost:9000 $MINIO_ROOT_USER $MINIO_ROOT_PASSWORD
mc mb local/poziomki-uploads
mc anonymous set download local/poziomki-uploads  # Public read for CDN
```

### Configure CORS (if needed)

```bash
# Create cors.json
cat > /tmp/cors.json << 'EOF'
{
  "CORSRules": [{
    "AllowedOrigins": ["https://mobile.poziomki.app"],
    "AllowedMethods": ["GET", "PUT", "POST", "DELETE"],
    "AllowedHeaders": ["*"],
    "MaxAgeSeconds": 3000
  }]
}
EOF

mc cors set local/poziomki-uploads /tmp/cors.json
```

---

## 7. Database Setup

### Initial Setup

PostgreSQL is automatically initialized by Docker Compose. For manual operations:

```bash
# Connect to database
docker compose -f docker-compose.prod.yml exec postgres psql -U poziomki

# Check tables
\dt

# Check connections
SELECT * FROM pg_stat_activity;
```

### Enable SSL (Recommended)

```bash
# Generate certificates
openssl req -new -x509 -days 365 -nodes -text \
  -out /opt/poziomki/data/postgres/server.crt \
  -keyout /opt/poziomki/data/postgres/server.key \
  -subj "/CN=postgres"

# Set permissions
chmod 600 /opt/poziomki/data/postgres/server.key
chown 999:999 /opt/poziomki/data/postgres/server.*
```

Update `docker-compose.prod.yml` PostgreSQL service:

```yaml
command: >
  postgres
  -c ssl=on
  -c ssl_cert_file=/var/lib/postgresql/data/server.crt
  -c ssl_key_file=/var/lib/postgresql/data/server.key
```

---

## 8. Health Checks

### API Health Endpoint

The API exposes `/health` endpoint:

```bash
# Check API health
curl https://mobile.poziomki.app/health

# Response
{
  "status": "ok",
  "timestamp": "2026-02-02T12:00:00Z"
}
```

### Service Health Commands

```bash
# All services status
docker compose -f docker-compose.prod.yml ps

# Individual health checks
docker compose -f docker-compose.prod.yml exec api curl -s localhost:3000/health
docker compose -f docker-compose.prod.yml exec postgres pg_isready
docker compose -f docker-compose.prod.yml exec minio curl -s localhost:9000/minio/health/live
```

---

## 9. Backup Strategy

### Automated Backups

Create `/opt/poziomki/scripts/backup.sh`:

```bash
#!/bin/bash
set -e

BACKUP_DIR="/opt/poziomki/backups"
DATE=$(date +%Y%m%d_%H%M%S)

# PostgreSQL backup
echo "Backing up PostgreSQL..."
docker compose -f /opt/poziomki/docker-compose.prod.yml exec -T postgres \
  pg_dump -U poziomki poziomki | gzip > "$BACKUP_DIR/db_$DATE.sql.gz"

# MinIO backup (if using local storage)
echo "Backing up MinIO..."
docker compose -f /opt/poziomki/docker-compose.prod.yml exec -T minio \
  mc mirror local/poziomki-uploads /tmp/backup/
tar -czf "$BACKUP_DIR/minio_$DATE.tar.gz" /opt/poziomki/data/minio

# Cleanup old backups (keep 7 days)
find "$BACKUP_DIR" -name "*.gz" -mtime +7 -delete

echo "Backup complete: $DATE"
```

### Cron Schedule

```bash
# Edit crontab
crontab -e

# Add daily backup at 2 AM
0 2 * * * /opt/poziomki/scripts/backup.sh >> /var/log/poziomki-backup.log 2>&1
```

### Restore from Backup

```bash
# Restore PostgreSQL
gunzip -c /opt/poziomki/backups/db_YYYYMMDD_HHMMSS.sql.gz | \
  docker compose -f docker-compose.prod.yml exec -T postgres \
  psql -U poziomki poziomki
```

---

## 10. Monitoring

### Basic Monitoring (without external tools)

```bash
# Resource usage
docker stats

# Logs
docker compose -f docker-compose.prod.yml logs -f --tail=100

# Specific service
docker compose -f docker-compose.prod.yml logs -f api
```

### Prometheus + Grafana (Optional)

See [OBSERVABILITY.md](./OBSERVABILITY.md) for full monitoring setup.

---

## 11. Updates & Maintenance

### Deploy Updates

```bash
cd /opt/poziomki

# Pull latest code
git pull

# Pull latest images
docker compose -f docker-compose.prod.yml pull

# Restart with zero downtime
docker compose -f docker-compose.prod.yml up -d --no-deps api

# Run migrations (if any)
docker compose -f docker-compose.prod.yml exec api bun run db:migrate
```

### Rollback

```bash
# Revert to previous commit
git checkout HEAD~1

# Restart services
docker compose -f docker-compose.prod.yml up -d --no-deps api
```

### Container Maintenance

```bash
# Remove unused images
docker image prune -a

# Remove unused volumes (CAREFUL!)
docker volume prune

# View disk usage
docker system df
```

---

## 12. Troubleshooting

### Common Issues

| Issue | Diagnosis | Solution |
|-------|-----------|----------|
| API won't start | `docker logs api` | Check environment variables |
| Database connection failed | Check postgres logs | Verify DB_* env vars |
| HTTPS not working | Caddy logs | Check domain DNS |
| File uploads fail | MinIO logs | Check bucket permissions |

### Useful Commands

```bash
# Check all logs
docker compose -f docker-compose.prod.yml logs --tail=100

# Restart specific service
docker compose -f docker-compose.prod.yml restart api

# Shell into container
docker compose -f docker-compose.prod.yml exec api sh

# Check network
docker network inspect poziomki_default
```

### Log Locations

| Service | Log Location |
|---------|--------------|
| API | `docker logs api` or stdout |
| PostgreSQL | `docker logs postgres` |
| Caddy | `/var/log/caddy/` |
| MinIO | `docker logs minio` |

---

## 13. Security Checklist

Before going live:

- [ ] Strong passwords for all services
- [ ] Firewall configured (only 80, 443, SSH)
- [ ] SSH key-only authentication
- [ ] Database SSL enabled
- [ ] MinIO SSL enabled (internal)
- [ ] Backups configured and tested
- [ ] Monitoring in place
- [ ] Error alerting configured
- [ ] HTTPS working with valid certificates
- [ ] Security headers configured

---

## 14. DNS Configuration

### Required DNS Records

| Record | Type | Value |
|--------|------|-------|
| `mobile.poziomki.app` | A | `<server-ip>` |
| `cdn.poziomki.app` | A | `<server-ip>` |

### Verification

```bash
# Check DNS propagation
dig mobile.poziomki.app
dig cdn.poziomki.app

# Test HTTPS
curl -I https://mobile.poziomki.app
curl -I https://cdn.poziomki.app
```

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-02-02 | Operations Team | Initial version |
