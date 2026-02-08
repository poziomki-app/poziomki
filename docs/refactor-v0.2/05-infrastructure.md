# Infrastructure

## Hardened Containers

All containers use Chainguard images where available — near-zero CVEs, signed, with SBOMs.

| Service | Image | Notes |
|---------|-------|-------|
| Postgres | `cgr.dev/chainguard/postgres:latest` | Chainguard hardened |
| Dragonfly | `cgr.dev/chainguard/dragonfly:latest` | Chainguard hardened |
| SeaweedFS | `chrislusf/seaweedfs:latest` | Hardened via security_opt |
| Electric | `electricsql/electric:latest` | Official image |

## SeaweedFS (Object Storage)

Replaces MinIO (which entered maintenance mode December 2025).

| Aspect | MinIO | SeaweedFS |
|--------|-------|-----------|
| License | AGPL-3.0 | Apache 2.0 |
| Status | Maintenance mode | Active development |
| S3 API | Full | Core + versioning |
| Small files | Good | Optimized (O(1) seek) |

**Docker Compose (hardened):**

```yaml
seaweedfs:
  image: chrislusf/seaweedfs:latest
  command: server -s3 -s3.port=8333 -dir=/data -master.volumeSizeLimitMB=100
  user: "1000:1000"
  security_opt:
    - no-new-privileges:true
  cap_drop:
    - ALL
  read_only: true
  tmpfs:
    - /tmp
  volumes:
    - ./data/seaweedfs:/data
    - ./config/s3.json:/etc/seaweedfs/s3.json:ro
  ports:
    - "127.0.0.1:8333:8333"
  healthcheck:
    test: ["CMD", "wget", "-q", "--spider", "http://localhost:8333/status"]
    interval: 5s
    timeout: 5s
    retries: 5
```

**S3 credentials (config/s3.json):**

```json
{
  "identities": [{
    "name": "poziomki",
    "credentials": [{
      "accessKey": "${S3_ACCESS_KEY}",
      "secretKey": "${S3_SECRET_KEY}"
    }],
    "actions": ["Admin", "Read", "Write", "List", "Tagging"]
  }]
}
```

**Environment variables:**

```bash
S3_ENDPOINT=seaweedfs
S3_PORT=8333
S3_ACCESS_KEY=your_access_key
S3_SECRET_KEY=your_secret_key
S3_BUCKET=poziomki-uploads
S3_USE_SSL=false
```

**API code change:** None required — MinIO client is S3-compatible.

## Dragonfly (Cache)

Redis-compatible with 25x throughput, 30% less memory. Using Chainguard hardened image.

| Use Case | Implementation |
|----------|----------------|
| Rate limiting | Distributed counters |
| Session cache | Reduce DB queries |
| Account lockout | Failed attempt tracking |
| Typing indicators | Pub/sub with TTL |
| Participant cache | Avoid DB lookups |

**Docker Compose (Chainguard hardened):**

```yaml
dragonfly:
  image: cgr.dev/chainguard/dragonfly:latest
  command: >
    dragonfly
    --logtostderr
    --requirepass=${DRAGONFLY_PASSWORD}
    --maxmemory=256mb
    --maxmemory-policy=allkeys-lru
    --dir=/data
  security_opt:
    - no-new-privileges:true
  volumes:
    - ./data/dragonfly:/data
  ports:
    - "127.0.0.1:6379:6379"
  ulimits:
    memlock: -1
  healthcheck:
    test: ["CMD", "redis-cli", "-a", "${DRAGONFLY_PASSWORD}", "ping"]
    interval: 5s
    timeout: 5s
    retries: 5
```

**Cache client:**

```typescript
// apps/api/src/lib/cache.ts
import Redis from 'ioredis'

const redis = new Redis({
  host: process.env.DRAGONFLY_HOST ?? 'localhost',
  port: parseInt(process.env.DRAGONFLY_PORT ?? '6379'),
  password: process.env.DRAGONFLY_PASSWORD,
  maxRetriesPerRequest: 3,
  lazyConnect: true,
})

export const cache = {
  async get<T>(key: string): Promise<T | null> {
    const data = await redis.get(key)
    return data ? JSON.parse(data) : null
  },

  async set<T>(key: string, value: T, ttlSeconds = 300): Promise<void> {
    await redis.setex(key, ttlSeconds, JSON.stringify(value))
  },

  async del(key: string): Promise<void> {
    await redis.del(key)
  },
}
```

**Rate limiting:**

```typescript
// apps/api/src/lib/rate-limit.ts
export async function checkRateLimit(
  key: string,
  limit: number,
  windowMs: number
): Promise<{ allowed: boolean; remaining: number }> {
  const windowKey = `ratelimit:${key}:${Math.floor(Date.now() / windowMs)}`

  const count = await redis.incr(windowKey)
  if (count === 1) {
    await redis.pexpire(windowKey, windowMs)
  }

  return {
    allowed: count <= limit,
    remaining: Math.max(0, limit - count),
  }
}
```

**Chat caching (server-side):**

```typescript
// apps/api/src/lib/chat-cache.ts

// Cache conversation participants (avoid DB lookup on every message)
export async function getParticipants(conversationId: string): Promise<string[]> {
  const cacheKey = `participants:${conversationId}`
  const cached = await redis.get(cacheKey)
  if (cached) return JSON.parse(cached)

  const participants = await db.query.conversationParticipants.findMany({
    where: eq(conversationParticipants.conversationId, conversationId),
  })
  const profileIds = participants.map(p => p.profileId)

  await redis.setex(cacheKey, 300, JSON.stringify(profileIds)) // 5 min TTL
  return profileIds
}

// Invalidate on participant change
export async function invalidateParticipants(conversationId: string): Promise<void> {
  await redis.del(`participants:${conversationId}`)
}

// Cache unread counts
export async function getUnreadCount(profileId: string, conversationId: string): Promise<number> {
  const cacheKey = `unread:${profileId}:${conversationId}`
  const cached = await redis.get(cacheKey)
  if (cached) return parseInt(cached)

  const count = await db.query.messages.count({
    where: and(
      eq(messages.conversationId, conversationId),
      gt(messages.createdAt, getLastReadTime(profileId, conversationId))
    )
  })

  await redis.setex(cacheKey, 60, count.toString()) // 1 min TTL
  return count
}

// Invalidate on new message or read
export async function invalidateUnread(profileId: string, conversationId: string): Promise<void> {
  await redis.del(`unread:${profileId}:${conversationId}`)
}
```

**Typing indicator pub/sub (real-time via Dragonfly):**

```typescript
// apps/api/src/features/chats/typing.ts

// Publish typing status (TTL auto-expires)
export async function setTyping(conversationId: string, profileId: string, isTyping: boolean): Promise<void> {
  const key = `typing:${conversationId}:${profileId}`

  if (isTyping) {
    await redis.setex(key, 5, '1') // 5 second TTL
    await redis.publish(`typing:${conversationId}`, JSON.stringify({ profileId, isTyping: true }))
  } else {
    await redis.del(key)
    await redis.publish(`typing:${conversationId}`, JSON.stringify({ profileId, isTyping: false }))
  }
}

// Get who's currently typing
export async function getTyping(conversationId: string): Promise<string[]> {
  const keys = await redis.keys(`typing:${conversationId}:*`)
  return keys.map(k => k.split(':')[2])
}
```

## Complete Docker Compose (Hardened)

```yaml
# docker-compose.yml
services:
  postgres:
    image: cgr.dev/chainguard/postgres:latest
    environment:
      POSTGRES_DB: poziomki
      POSTGRES_USER: poziomki
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    cap_add:
      - CHOWN
      - SETGID
      - SETUID
    read_only: true
    tmpfs:
      - /tmp
      - /var/run/postgresql
    volumes:
      - ./data/postgres:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U poziomki"]
      interval: 5s
      timeout: 5s
      retries: 5

  electric:
    image: electricsql/electric:latest
    environment:
      DATABASE_URL: postgres://poziomki:${DB_PASSWORD}@postgres:5432/poziomki
      AUTH_MODE: secure
      AUTH_JWT_KEY: ${ELECTRIC_JWT_KEY}
    security_opt:
      - no-new-privileges:true
    ports:
      - "127.0.0.1:5133:5133"
    depends_on:
      postgres:
        condition: service_healthy

  seaweedfs:
    image: chrislusf/seaweedfs:latest
    command: server -s3 -s3.port=8333 -dir=/data
    user: "1000:1000"
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    read_only: true
    tmpfs:
      - /tmp
    volumes:
      - ./data/seaweedfs:/data
      - ./config/s3.json:/etc/seaweedfs/s3.json:ro
    ports:
      - "127.0.0.1:8333:8333"
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:8333/status"]
      interval: 5s
      timeout: 5s
      retries: 5

  dragonfly:
    image: cgr.dev/chainguard/dragonfly:latest
    command: dragonfly --logtostderr --requirepass=${DRAGONFLY_PASSWORD} --maxmemory=256mb --dir=/data
    security_opt:
      - no-new-privileges:true
    volumes:
      - ./data/dragonfly:/data
    ports:
      - "127.0.0.1:6379:6379"
    ulimits:
      memlock: -1
    healthcheck:
      test: ["CMD", "redis-cli", "-a", "${DRAGONFLY_PASSWORD}", "ping"]
      interval: 5s
      timeout: 5s
      retries: 5

  api:
    build: .
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    read_only: true
    tmpfs:
      - /tmp
    depends_on:
      postgres:
        condition: service_healthy
      electric:
        condition: service_started
      seaweedfs:
        condition: service_healthy
      dragonfly:
        condition: service_healthy
    environment:
      DATABASE_URL: postgres://poziomki:${DB_PASSWORD}@postgres:5432/poziomki
      S3_ENDPOINT: seaweedfs
      S3_PORT: 8333
      DRAGONFLY_HOST: dragonfly
      DRAGONFLY_PORT: 6379
      DRAGONFLY_PASSWORD: ${DRAGONFLY_PASSWORD}
      ELECTRIC_URL: http://electric:5133
    ports:
      - "127.0.0.1:3000:3000"

networks:
  default:
    driver: bridge
```

## Environment Variables

```bash
# .env
DB_PASSWORD=secure_password_here
DRAGONFLY_PASSWORD=secure_password_here
ELECTRIC_JWT_KEY=secure_jwt_key_here
S3_ACCESS_KEY=your_access_key
S3_SECRET_KEY=your_secret_key
```

## Data Migration (MinIO → SeaweedFS)

```bash
# 1. Install mc (MinIO client) - works with both MinIO and SeaweedFS
brew install minio-mc  # or download from minio.io

# 2. Configure both endpoints
mc alias set old-minio http://localhost:9000 OLD_ACCESS_KEY OLD_SECRET_KEY
mc alias set seaweedfs http://localhost:8333 NEW_ACCESS_KEY NEW_SECRET_KEY

# 3. Create bucket in SeaweedFS
mc mb seaweedfs/poziomki-uploads

# 4. Mirror data
mc mirror old-minio/poziomki-uploads seaweedfs/poziomki-uploads

# 5. Verify
mc ls seaweedfs/poziomki-uploads --recursive | wc -l
```
