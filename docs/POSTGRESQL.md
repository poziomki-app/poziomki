# PostgreSQL Reliability & Performance Guide

Self-hosted PostgreSQL infrastructure for Poziomki.

## Current State

**Postgres 17-alpine** with basic health checks, no backups, no connection pooling, no monitoring.

| Component | Status | Risk |
|-----------|--------|------|
| Version | ✅ Postgres 17 (latest) | Low |
| Health checks | ✅ pg_isready | Low |
| Connection pooling | ❌ None (direct connections) | Medium |
| Backups | ❌ None | **CRITICAL** |
| Monitoring | ❌ None | High |
| Performance tuning | ❌ Default settings | Medium |
| High availability | ❌ Single node | Medium |

---

## 1. Connection Pooling

### Why You Need It

Direct connections to PostgreSQL are expensive (~2MB RAM each). Current setup allows 10 connections from the API, but:
- Each connection holds resources even when idle
- Connection storms during traffic spikes exhaust limits
- No query queueing when pool is full

### Options Compared

| Feature | PgBouncer | Supavisor | PgCat |
|---------|-----------|-----------|-------|
| Complexity | Low | Medium | Medium |
| Latency | Lowest (<50 conn) | +2ms overhead | Comparable to PgBouncer |
| Scalability | Single-threaded | Millions of connections | Multi-threaded |
| Memory | ~2-5MB | Higher (Elixir VM) | ~10-20MB |
| Best for | Most apps | Serverless/massive scale | Read replicas |

**Sources:**
- [Tembo Benchmark: PgBouncer vs PgCat vs Supavisor](https://legacy.tembo.io/blog/postgres-connection-poolers/)
- [Supavisor GitHub](https://github.com/supabase/supavisor)
- [PgBouncer Pitfalls](https://jpcamara.com/2023/04/12/pgbouncer-is-useful.html)

### Recommendation: PgBouncer

For Poziomki's scale (small VPS, <100 concurrent users), PgBouncer is the right choice:
- Battle-tested, simple configuration
- Minimal overhead
- Transaction pooling mode works with Drizzle

### Implementation

```yaml
# docker-compose.prod.yml
services:
  pgbouncer:
    image: edoburu/pgbouncer:1.23.1-p2
    restart: unless-stopped
    environment:
      DATABASE_URL: postgres://${DB_USER:-poziomki}:${DB_PASSWORD}@postgres:5432/${DB_NAME:-poziomki}
      POOL_MODE: transaction
      MAX_CLIENT_CONN: 200      # Max connections from apps
      DEFAULT_POOL_SIZE: 20     # Connections to Postgres per pool
      MIN_POOL_SIZE: 5          # Keep-alive connections
      RESERVE_POOL_SIZE: 5      # Emergency overflow
      RESERVE_POOL_TIMEOUT: 3   # Wait before using reserve
      SERVER_IDLE_TIMEOUT: 600  # Close idle server connections
      SERVER_LIFETIME: 3600     # Max connection age
      LOG_CONNECTIONS: 0        # Reduce log noise
      LOG_DISCONNECTIONS: 0
    depends_on:
      postgres:
        condition: service_healthy
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -h localhost -p 6432"]
      interval: 10s
      timeout: 5s
      retries: 3

  api:
    environment:
      # Connect through PgBouncer instead of direct
      DB_HOST: pgbouncer
      DB_PORT: 6432
      DB_POOL_SIZE: 50  # Can be higher now - PgBouncer manages actual connections
```

### Transaction vs Session Mode

| Mode | Behavior | Use Case |
|------|----------|----------|
| **Transaction** | Connection released after each transaction | Most apps (recommended) |
| Session | Connection held for entire session | Apps using prepared statements, LISTEN/NOTIFY |
| Statement | Connection released after each statement | Simple read queries |

**Caveat:** Transaction mode breaks:
- `SET` commands (use `SET LOCAL` instead)
- Prepared statements (Drizzle doesn't use them by default)
- `LISTEN/NOTIFY` (need separate direct connection)

---

## 2. Backup Strategy

### Why It's Critical

Currently: **Zero backups**. A disk failure, accidental deletion, or corruption means total data loss.

### Options Compared

| Tool | Type | Incremental | PITR | Complexity | Best For |
|------|------|-------------|------|------------|----------|
| **pg_dump** | Logical | ❌ | ❌ | Low | Small DBs, dev, migrations |
| **pg_basebackup** | Physical | ❌ | ✅ (with WAL) | Medium | Manual base backups |
| **pgBackRest** | Physical | ✅ | ✅ | High | Large DBs, enterprise |
| **Barman** | Physical | ✅ | ✅ | High | Multi-server environments |

**Sources:**
- [Top 5 PostgreSQL Backup Tools 2025](https://dev.to/rostislav_dugin/top-5-postgresql-backup-tools-in-2025-5801)
- [pgBackRest Official](https://pgbackrest.org/)
- [Crunchy Data Backup Guide](https://www.crunchydata.com/blog/introduction-to-postgres-backups)

### Recommendation: pg_dump + WAL Archiving

For Poziomki's scale, start simple and evolve:

**Phase 1 (Now):** Daily pg_dump backups
**Phase 2 (Growth):** Add WAL archiving for PITR
**Phase 3 (Scale):** Migrate to pgBackRest

### Implementation: Phase 1 - Automated Backups

```bash
#!/bin/bash
# /opt/poziomki/scripts/backup-postgres.sh
set -euo pipefail

BACKUP_DIR="/opt/poziomki/backups/postgres"
RETENTION_DAYS=14
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
CONTAINER="poziomki-postgres-1"

# Ensure backup directory exists
mkdir -p "${BACKUP_DIR}"

# Create compressed backup
echo "[$(date)] Starting backup..."
docker exec "${CONTAINER}" pg_dump \
  -U "${DB_USER:-poziomki}" \
  -d "${DB_NAME:-poziomki}" \
  --format=custom \
  --compress=9 \
  > "${BACKUP_DIR}/poziomki_${TIMESTAMP}.dump"

# Verify backup is valid
docker exec -i "${CONTAINER}" pg_restore \
  --list "${BACKUP_DIR}/poziomki_${TIMESTAMP}.dump" > /dev/null

echo "[$(date)] Backup created: poziomki_${TIMESTAMP}.dump ($(du -h ${BACKUP_DIR}/poziomki_${TIMESTAMP}.dump | cut -f1))"

# Clean old backups
find "${BACKUP_DIR}" -name "*.dump" -mtime +${RETENTION_DAYS} -delete
echo "[$(date)] Cleaned backups older than ${RETENTION_DAYS} days"

# Optional: Sync to remote storage (uncomment and configure)
# rclone copy "${BACKUP_DIR}" remote:poziomki-backups/postgres --max-age 1d
```

```bash
# Crontab entry (run daily at 3 AM)
0 3 * * * /opt/poziomki/scripts/backup-postgres.sh >> /var/log/poziomki-backup.log 2>&1
```

### Implementation: Phase 2 - WAL Archiving (PITR)

Point-in-Time Recovery allows restoring to any moment, not just backup snapshots.

```yaml
# docker-compose.prod.yml
services:
  postgres:
    image: postgres:17-alpine
    command:
      - "postgres"
      - "-c"
      - "archive_mode=on"
      - "-c"
      - "archive_command=gzip < %p > /backups/wal/%f.gz"
      - "-c"
      - "wal_level=replica"
    volumes:
      - ./data/postgres:/var/lib/postgresql/data
      - ./backups/wal:/backups/wal
```

### Restore Procedures

```bash
# Restore from pg_dump backup
docker exec -i poziomki-postgres-1 pg_restore \
  -U poziomki \
  -d poziomki \
  --clean \
  --if-exists \
  < /path/to/backup.dump

# For PITR (Phase 2), see PostgreSQL recovery documentation
```

---

## 3. Monitoring

### Why It Matters

Without monitoring you're blind to:
- Slow queries degrading UX
- Connection pool exhaustion
- Disk space running out
- Replication lag (if using replicas)

### Options Compared

| Tool | Metrics | Dashboards | Complexity | Cost |
|------|---------|------------|------------|------|
| **postgres_exporter + Prometheus** | 200+ | Grafana | Medium | Free |
| **pg_exporter** | 600+ | Grafana | Medium | Free |
| **pgwatch2** | Comprehensive | Built-in | Low | Free |
| **Datadog/New Relic** | Comprehensive | Built-in | Low | $$$ |

**Sources:**
- [postgres_exporter GitHub](https://github.com/prometheus-community/postgres_exporter)
- [pg_exporter v1.0](https://www.postgresql.org/about/news/pg_exporter-v100-released-next-level-pg-observability-3073/)
- [Postgres Monitoring Guide](https://tiagomelo.info/postgres/prometheus/grafana/2025/11/10/postgres-exporter.html)

### Recommendation: postgres_exporter + Prometheus + Grafana

Industry standard, well-documented, and free.

### Implementation

```yaml
# docker-compose.prod.yml
services:
  postgres_exporter:
    image: prometheuscommunity/postgres-exporter:v0.16.0
    restart: unless-stopped
    environment:
      DATA_SOURCE_NAME: "postgresql://${DB_USER:-poziomki}:${DB_PASSWORD}@postgres:5432/${DB_NAME:-poziomki}?sslmode=disable"
    depends_on:
      postgres:
        condition: service_healthy
    ports:
      - "127.0.0.1:9187:9187"

  prometheus:
    image: prom/prometheus:v2.54.1
    restart: unless-stopped
    volumes:
      - ./config/prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus
    ports:
      - "127.0.0.1:9090:9090"
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.retention.time=30d'

  grafana:
    image: grafana/grafana:11.2.2
    restart: unless-stopped
    volumes:
      - grafana_data:/var/lib/grafana
    ports:
      - "127.0.0.1:3001:3000"
    environment:
      GF_SECURITY_ADMIN_PASSWORD: ${GRAFANA_PASSWORD:-admin}
      GF_INSTALL_PLUGINS: grafana-clock-panel

volumes:
  prometheus_data:
  grafana_data:
```

```yaml
# config/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'postgres'
    static_configs:
      - targets: ['postgres_exporter:9187']

  - job_name: 'api'
    static_configs:
      - targets: ['api:3000']
    metrics_path: /metrics
```

### Enable pg_stat_statements

```sql
-- Run once to enable query performance tracking
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

-- View slow queries
SELECT
  calls,
  round(total_exec_time::numeric, 2) as total_ms,
  round(mean_exec_time::numeric, 2) as avg_ms,
  left(query, 100) as query
FROM pg_stat_statements
ORDER BY total_exec_time DESC
LIMIT 20;
```

### Key Metrics to Alert On

| Metric | Warning | Critical |
|--------|---------|----------|
| Connection usage | > 70% | > 90% |
| Replication lag | > 10s | > 60s |
| Transaction rate drop | > 50% | > 80% |
| Disk usage | > 80% | > 90% |
| Long-running queries | > 30s | > 120s |
| Dead tuples ratio | > 10% | > 20% |

---

## 4. Performance Tuning

### Current: Default Settings

PostgreSQL defaults are conservative for compatibility, not performance.

### Recommended Settings for 4GB VPS

```yaml
# docker-compose.prod.yml
services:
  postgres:
    image: postgres:17-alpine
    command:
      - "postgres"
      # Memory (adjust for your VPS RAM)
      - "-c"
      - "shared_buffers=1GB"              # 25% of RAM
      - "-c"
      - "effective_cache_size=2GB"        # 50% of RAM
      - "-c"
      - "work_mem=16MB"                   # Per-operation sort memory
      - "-c"
      - "maintenance_work_mem=256MB"      # Vacuum/index operations
      # WAL
      - "-c"
      - "wal_buffers=64MB"
      - "-c"
      - "checkpoint_completion_target=0.9"
      - "-c"
      - "max_wal_size=2GB"
      # Connections
      - "-c"
      - "max_connections=100"             # With PgBouncer, can be lower
      # Parallelism
      - "-c"
      - "max_parallel_workers_per_gather=2"
      - "-c"
      - "max_parallel_workers=4"
      # Logging
      - "-c"
      - "log_min_duration_statement=1000" # Log queries > 1s
      - "-c"
      - "log_checkpoints=on"
      - "-c"
      - "log_lock_waits=on"
      # Autovacuum (tuned for small VPS)
      - "-c"
      - "autovacuum_max_workers=2"
      - "-c"
      - "autovacuum_vacuum_cost_limit=1000"
    shm_size: 1gb  # Required for shared_buffers > 64MB
```

**Sources:**
- [PostgreSQL Wiki: Tuning](https://wiki.postgresql.org/wiki/Tuning_Your_PostgreSQL_Server)
- [EDB Memory Tuning](https://www.enterprisedb.com/postgres-tutorials/how-tune-postgresql-memory)
- [Crunchy Data Optimization](https://www.crunchydata.com/blog/optimize-postgresql-server-performance)

### Scaling Guidelines

| VPS RAM | shared_buffers | effective_cache_size | work_mem |
|---------|----------------|----------------------|----------|
| 2GB | 512MB | 1GB | 8MB |
| 4GB | 1GB | 2GB | 16MB |
| 8GB | 2GB | 4GB | 32MB |
| 16GB | 4GB | 10GB | 64MB |

### Maintenance Script

```bash
#!/bin/bash
# /opt/poziomki/scripts/maintain-postgres.sh
# Run weekly: 0 4 * * 0

CONTAINER="poziomki-postgres-1"

docker exec "${CONTAINER}" psql -U poziomki -d poziomki -c "
  -- Update query planner statistics
  ANALYZE VERBOSE;
"

docker exec "${CONTAINER}" psql -U poziomki -d poziomki -c "
  -- Reclaim space (non-blocking)
  VACUUM (VERBOSE, ANALYZE);
"

docker exec "${CONTAINER}" psql -U poziomki -d poziomki -c "
  -- Show table bloat
  SELECT
    schemaname || '.' || relname as table,
    pg_size_pretty(pg_total_relation_size(relid)) as size,
    n_dead_tup as dead_rows,
    n_live_tup as live_rows,
    round(n_dead_tup::numeric / nullif(n_live_tup + n_dead_tup, 0) * 100, 2) as dead_pct
  FROM pg_stat_user_tables
  WHERE n_dead_tup > 1000
  ORDER BY n_dead_tup DESC;
"
```

---

## 5. High Availability

### Current: Single Point of Failure

If Postgres goes down, the entire app is down.

### Options Compared

| Solution | Complexity | Failover | Minimum Nodes | Best For |
|----------|------------|----------|---------------|----------|
| **pg_auto_failover** | Low | Automatic | 3 (2 data + 1 monitor) | Small deployments |
| **repmgr** | Medium | Manual/scripted | 2+ | Medium scale |
| **Patroni** | High | Automatic | 3+ (needs etcd/consul) | Enterprise |
| **Stolon** | High | Automatic | 3+ (needs etcd/consul) | Kubernetes |

**Sources:**
- [pg_auto_failover GitHub](https://github.com/hapostgres/pg_auto_failover)
- [HA Comparison: PAF vs repmgr vs Patroni](https://medium.com/@kristi.anderson/whats-the-best-postgresql-high-availability-framework-paf-vs-repmgr-vs-patroni-infographic-8f11f3972ef3)
- [Ashnik HA Comparison](https://www.ashnik.com/architecting-postgresql-ha-patroni-vs-repmgr-vs-native-streaming/)

### Recommendation: pg_auto_failover (Future)

For Poziomki's scale, HA is a "nice to have" not a "must have" initially. When ready:

1. **Start with:** Good backups + fast restore procedure (RTO < 1 hour)
2. **Graduate to:** pg_auto_failover when downtime becomes costly

### pg_auto_failover Overview

```
┌─────────────────────────────────────────────────────┐
│                    Monitor Node                      │
│              (Orchestrates failover)                │
└─────────────────────────────────────────────────────┘
                         │
         ┌───────────────┴───────────────┐
         ▼                               ▼
┌─────────────────────┐       ┌─────────────────────┐
│   Primary Node      │◄─────►│   Secondary Node    │
│   (Read/Write)      │ sync  │   (Hot Standby)     │
└─────────────────────┘       └─────────────────────┘
```

Benefits:
- Zero data loss (synchronous replication)
- Automatic failover (~10-30 seconds)
- No external dependencies (no etcd/consul)

---

## 6. Implementation Priority

### Phase 1: Critical (Week 1)
- [ ] Implement automated daily backups (pg_dump)
- [ ] Add backup verification and alerting
- [ ] Document restore procedure
- [ ] Add PgBouncer connection pooling

### Phase 2: Stability (Week 2-3)
- [ ] Apply performance tuning settings
- [ ] Enable pg_stat_statements
- [ ] Add postgres_exporter + Prometheus
- [ ] Create Grafana dashboards
- [ ] Set up alerting rules

### Phase 3: Resilience (Month 2)
- [ ] Add WAL archiving for PITR
- [ ] Implement off-site backup sync (rclone)
- [ ] Weekly maintenance automation
- [ ] Load testing and capacity planning

### Phase 4: High Availability (Future)
- [ ] Evaluate pg_auto_failover
- [ ] Set up staging HA cluster
- [ ] Define RTO/RPO targets
- [ ] Production HA deployment

---

## 7. Quick Reference

### Health Check Commands

```bash
# Check if Postgres is accepting connections
docker exec poziomki-postgres-1 pg_isready -U poziomki

# Check connection count
docker exec poziomki-postgres-1 psql -U poziomki -d poziomki -c \
  "SELECT count(*) FROM pg_stat_activity WHERE state = 'active';"

# Check database size
docker exec poziomki-postgres-1 psql -U poziomki -d poziomki -c \
  "SELECT pg_size_pretty(pg_database_size('poziomki'));"

# Check table sizes
docker exec poziomki-postgres-1 psql -U poziomki -d poziomki -c \
  "SELECT relname, pg_size_pretty(pg_total_relation_size(relid))
   FROM pg_stat_user_tables ORDER BY pg_total_relation_size(relid) DESC LIMIT 10;"

# Check slow queries (if pg_stat_statements enabled)
docker exec poziomki-postgres-1 psql -U poziomki -d poziomki -c \
  "SELECT calls, mean_exec_time::int as avg_ms, left(query, 80)
   FROM pg_stat_statements ORDER BY mean_exec_time DESC LIMIT 5;"
```

### Backup Commands

```bash
# Manual backup
docker exec poziomki-postgres-1 pg_dump -U poziomki -d poziomki --format=custom > backup.dump

# Restore
docker exec -i poziomki-postgres-1 pg_restore -U poziomki -d poziomki --clean < backup.dump

# List backup contents
pg_restore --list backup.dump
```

### PgBouncer Commands

```bash
# Check pool status
docker exec poziomki-pgbouncer-1 psql -p 6432 -U poziomki pgbouncer -c "SHOW POOLS;"

# Check active connections
docker exec poziomki-pgbouncer-1 psql -p 6432 -U poziomki pgbouncer -c "SHOW CLIENTS;"

# Check server connections
docker exec poziomki-pgbouncer-1 psql -p 6432 -U poziomki pgbouncer -c "SHOW SERVERS;"
```

---

## Summary

| Improvement | Effort | Impact | Priority |
|-------------|--------|--------|----------|
| Automated backups | 1 hour | Critical | **P0** |
| PgBouncer | 30 min | High | P1 |
| Performance tuning | 30 min | Medium | P1 |
| Monitoring (Prometheus) | 2 hours | High | P2 |
| WAL archiving (PITR) | 1 hour | High | P2 |
| pg_auto_failover | 1 day | Medium | P3 |

**Start today:** Backups. Everything else can wait, but one disk failure without backups means losing all user data.
