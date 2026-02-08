# Performance & Benchmarks

Performance baselines, capacity planning, and benchmarking procedures for Poziomki.

**Related docs:**
- [OBSERVABILITY.md](../OBSERVABILITY.md) — Metrics & monitoring strategy
- [CACHING.md](../CACHING.md) — Caching architecture & capacity planning

---

## Current State

### Implemented

| Component | Status | Notes |
|-----------|--------|-------|
| Request timing | Partial | `Date.now()` based, logged to Pino |
| Request correlation | Yes | `requestId` UUID per request |
| Rate limiting | Yes | 5/min auth, 100/min general |
| Health endpoint | Yes | `/health` returns `{ status: 'ok' }` |

### Not Yet Implemented

| Component | Priority | Documented In |
|-----------|----------|---------------|
| Prometheus metrics endpoint | High | OBSERVABILITY.md Phase 1 |
| Database query monitoring | High | OBSERVABILITY.md Phase 1 |
| WebSocket connection metrics | Medium | OBSERVABILITY.md Phase 2 |
| Cache hit/miss tracking | Medium | CACHING.md |
| Distributed tracing | Low | OBSERVABILITY.md Phase 4 |

---

## Capacity Assessment

Based on CACHING.md analysis for a 3-person team with limited resources:

### Current Infrastructure Capacity

| Component | Theoretical Max | 10K DAU Need | Headroom |
|-----------|-----------------|--------------|----------|
| Elysia API (Bun) | 50,000+ req/s | ~500 req/s | 100x |
| PostgreSQL 17 | 10,000 queries/s | ~1,000 queries/s | 10x |
| MinIO | 10,000 req/s | ~100 req/s | 100x |
| WebSocket | 100,000 connections | ~3,000 connections | 33x |

**Verdict:** Infrastructure can handle 10K DAU without horizontal scaling. Focus on caching and query optimization.

### Scaling Triggers

| Metric | Current Capacity | Scale When |
|--------|------------------|------------|
| API response time (p95) | - | > 500ms |
| Database connections | 10 (prod) | > 80% utilization |
| Memory usage | - | > 80% |
| WebSocket connections | - | > 50,000 |

---

## Performance Targets

### API Response Times

| Endpoint Category | Target p50 | Target p95 | Target p99 |
|-------------------|------------|------------|------------|
| Auth endpoints | < 100ms | < 300ms | < 500ms |
| Profile read | < 50ms | < 150ms | < 300ms |
| Profile write | < 100ms | < 300ms | < 500ms |
| Event list | < 100ms | < 300ms | < 500ms |
| Chat messages | < 50ms | < 150ms | < 300ms |
| File upload | < 1s | < 3s | < 5s |
| Recommendations | < 200ms | < 500ms | < 1s |

### Database Query Times

| Query Type | Target | Alert Threshold |
|------------|--------|-----------------|
| Simple SELECT | < 10ms | > 50ms |
| Indexed lookup | < 5ms | > 20ms |
| Complex JOIN | < 50ms | > 200ms |
| Aggregation | < 100ms | > 500ms |

### WebSocket

| Metric | Target |
|--------|--------|
| Message delivery | < 100ms |
| Connection establishment | < 500ms |
| Reconnection | < 2s |

---

## Benchmarking Procedures

### Load Testing with k6

Install k6:
```bash
# Fedora/RHEL
sudo dnf install k6

# macOS
brew install k6
```

Basic load test script (`benchmark/load-test.js`):
```javascript
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '30s', target: 10 },  // Ramp up
    { duration: '1m', target: 10 },   // Steady state
    { duration: '30s', target: 0 },   // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<500'],  // 95% under 500ms
    http_req_failed: ['rate<0.01'],    // <1% errors
  },
};

const BASE_URL = __ENV.API_URL || 'http://localhost:3000';

export default function () {
  // Health check
  const health = http.get(`${BASE_URL}/health`);
  check(health, {
    'health status 200': (r) => r.status === 200,
  });

  sleep(1);
}
```

Run:
```bash
k6 run benchmark/load-test.js
```

### Database Query Benchmarking

Enable slow query logging in PostgreSQL:
```sql
-- In postgresql.conf or via ALTER SYSTEM
ALTER SYSTEM SET log_min_duration_statement = 100;  -- Log queries > 100ms
SELECT pg_reload_conf();
```

Check slow queries:
```sql
-- View pg_stat_statements (if extension enabled)
SELECT query, calls, mean_exec_time, total_exec_time
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 20;
```

### API Endpoint Benchmarking

Using `hyperfine` for quick benchmarks:
```bash
# Install
cargo install hyperfine

# Benchmark health endpoint
hyperfine --warmup 3 'curl -s http://localhost:3000/health'

# Benchmark with auth (replace TOKEN)
hyperfine --warmup 3 \
  'curl -s -H "Authorization: Bearer $TOKEN" http://localhost:3000/api/profiles/me'
```

Using `autocannon` for HTTP benchmarks:
```bash
bunx autocannon -c 10 -d 30 http://localhost:3000/health
```

---

## Baseline Measurements

Record baseline measurements before major changes. Template:

### Baseline: [Date]

**Environment:**
- Hardware: [CPU, RAM]
- Database: PostgreSQL 17, [connection count]
- Data volume: [users, messages, events]

**Results:**

| Endpoint | Method | p50 | p95 | p99 | RPS |
|----------|--------|-----|-----|-----|-----|
| `/health` | GET | - | - | - | - |
| `/api/profiles/:id` | GET | - | - | - | - |
| `/api/events` | GET | - | - | - | - |
| `/api/chats` | GET | - | - | - | - |

**Database:**

| Query | Avg Time | Max Time |
|-------|----------|----------|
| Profile lookup | - | - |
| Event list | - | - |
| Message history | - | - |

---

## Performance Checklist

### Before Launch

- [ ] Record baseline measurements for all critical endpoints
- [ ] Enable slow query logging (> 100ms threshold)
- [ ] Verify rate limiting works under load
- [ ] Test WebSocket under concurrent connections
- [ ] Verify file upload performance with 10MB files

### Monthly

- [ ] Run load tests, compare to baseline
- [ ] Review slow query logs
- [ ] Check database index usage (`pg_stat_user_indexes`)
- [ ] Review connection pool utilization

### After Major Changes

- [ ] Run benchmark suite
- [ ] Compare p95 latencies to baseline
- [ ] Check for new slow queries
- [ ] Verify no memory leaks (monitor over time)

---

## Optimization Priorities

Based on typical social app patterns:

### High Impact

1. **Database indexes** — Ensure all WHERE/JOIN columns are indexed
2. **N+1 queries** — Use eager loading for related data
3. **Connection pooling** — Configured (3 dev / 10 prod)
4. **Response caching** — Hot paths (recommendations, event lists)

### Medium Impact

5. **Query optimization** — Simplify complex JOINs
6. **Pagination** — Cursor-based for large lists
7. **CDN caching** — Static assets, user uploads

### Lower Priority (Scale When Needed)

8. **Read replicas** — When write load conflicts with reads
9. **Horizontal scaling** — When single instance insufficient
10. **Sharding** — Unlikely to be needed at student app scale

---

## Monitoring Setup (TODO)

Per OBSERVABILITY.md Phase 1:

```typescript
// apps/api/src/plugins/metrics.ts (to implement)
import { Elysia } from 'elysia';
import { Counter, Histogram, Registry } from 'prom-client';

const register = new Registry();

const httpRequestDuration = new Histogram({
  name: 'http_request_duration_seconds',
  help: 'HTTP request duration in seconds',
  labelNames: ['method', 'route', 'status'],
  buckets: [0.01, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5],
  registers: [register],
});

const httpRequestTotal = new Counter({
  name: 'http_requests_total',
  help: 'Total HTTP requests',
  labelNames: ['method', 'route', 'status'],
  registers: [register],
});

export const metrics = new Elysia({ name: 'metrics' })
  .get('/metrics', async () => {
    return new Response(await register.metrics(), {
      headers: { 'Content-Type': register.contentType },
    });
  })
  .onAfterHandle(({ request, set }) => {
    const duration = /* calculate */;
    httpRequestDuration
      .labels(request.method, /* route */, String(set.status))
      .observe(duration);
    httpRequestTotal
      .labels(request.method, /* route */, String(set.status))
      .inc();
  });
```

---

*Last updated: 2026-02-03*
