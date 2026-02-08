# Architecture Overview

**Document Owner:** Engineering Team
**Last Updated:** 2026-02-02

---

## 1. System Overview

Poziomki is a monorepo-based social application for university students with:
- **Mobile clients** (iOS, Android) built with Expo/React Native
- **Backend API** built with Elysia on Bun runtime
- **PostgreSQL** for persistent data storage
- **MinIO** for S3-compatible object storage (planned migration to SeaweedFS)
- **Caddy** as reverse proxy with automatic HTTPS

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              CLIENTS                                     │
│                                                                          │
│    ┌────────────────┐          ┌────────────────┐                       │
│    │   iOS App      │          │  Android App   │                       │
│    │   (Expo)       │          │    (Expo)      │                       │
│    └───────┬────────┘          └───────┬────────┘                       │
│            │                           │                                 │
│            └───────────┬───────────────┘                                │
│                        │ HTTPS / WSS                                     │
└────────────────────────┼────────────────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────────────────┐
│                         EDGE LAYER                                      │
│                                                                         │
│    ┌──────────────────────────────────────────────────────────────┐    │
│    │                          Caddy                                │    │
│    │  • Auto HTTPS (Let's Encrypt)                                │    │
│    │  • Reverse proxy                                              │    │
│    │  • Rate limiting (optional)                                   │    │
│    │  • CDN auth forwarding                                        │    │
│    └──────────────────────────┬───────────────────────────────────┘    │
│                               │                                         │
└───────────────────────────────┼─────────────────────────────────────────┘
                                │
                                ▼
┌────────────────────────────────────────────────────────────────────────┐
│                       APPLICATION LAYER                                 │
│                                                                         │
│    ┌──────────────────────────────────────────────────────────────┐    │
│    │                      Elysia API                               │    │
│    │  • HTTP REST endpoints                                        │    │
│    │  • WebSocket for real-time chat                              │    │
│    │  • Authentication & session management                        │    │
│    │  • Business logic in services                                 │    │
│    └────────────────────┬────────────────┬────────────────────────┘    │
│                         │                │                              │
└─────────────────────────┼────────────────┼──────────────────────────────┘
                          │                │
              ┌───────────┘                └───────────┐
              ▼                                        ▼
┌─────────────────────────────┐    ┌─────────────────────────────┐
│        DATA LAYER           │    │       STORAGE LAYER         │
│                             │    │                             │
│  ┌───────────────────────┐  │    │  ┌───────────────────────┐  │
│  │     PostgreSQL 17     │  │    │  │ MinIO → SeaweedFS     │  │
│  │  • Users & profiles   │  │    │  │  • Profile photos     │  │
│  │  • Events             │  │    │  │  • Event covers       │  │
│  │  • Messages           │  │    │  │  • Chat attachments   │  │
│  │  • Sessions           │  │    │  └───────────────────────┘  │
│  └───────────────────────┘  │    │                             │
└─────────────────────────────┘    └─────────────────────────────┘
```

---

## 2. Project Structure

```
poziomki/
├── apps/
│   ├── api/                    # Backend API
│   │   └── src/
│   │       ├── app.ts          # Main application
│   │       ├── features/       # Feature modules
│   │       │   ├── auth/       # Authentication
│   │       │   ├── profiles/   # User profiles
│   │       │   ├── events/     # Events management
│   │       │   ├── chats/      # Messaging
│   │       │   ├── matching/   # Profile matching
│   │       │   └── uploads/    # File uploads
│   │       ├── plugins/        # Elysia plugins
│   │       └── lib/            # Shared utilities
│   │
│   └── mobile/                 # Mobile application
│       └── src/
│           ├── app/            # Screen routes (Expo Router)
│           ├── components/     # UI components
│           ├── hooks/          # Custom hooks
│           └── lib/            # Utilities
│
├── packages/
│   ├── db/                     # Database package
│   │   └── src/
│   │       ├── schema/         # Drizzle schema definitions
│   │       ├── migrations/     # SQL migrations
│   │       └── client.ts       # Database client
│   │
│   └── core/                   # Shared utilities
│       └── src/
│           └── logger.ts       # Structured logging
│
├── docs/                       # Documentation
├── docker-compose.yml          # Development environment
└── docker-compose.prod.yml     # Production environment
```

---

## 3. API Architecture

### Feature Module Structure

Each feature follows a consistent pattern:

```
features/{feature}/
├── index.ts        # Routes (Elysia controller)
├── service.ts      # Business logic
└── model.ts        # Type schemas (Typebox)
```

### Request Flow

```
Request
    │
    ▼
┌──────────────────────────────────────────────────────┐
│                    Plugins                            │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  │
│  │ CORS    │─▶│ Logger  │─▶│ Auth    │─▶│ Rate    │  │
│  │         │  │         │  │ Guard   │  │ Limit   │  │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘  │
└──────────────────────────┬───────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────┐
│                  Route Handler                        │
│  • Validate request (schema)                          │
│  • Extract authenticated user                         │
│  • Call service method                                │
│  • Return response                                    │
└──────────────────────────┬───────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────┐
│                    Service                            │
│  • Business logic                                     │
│  • Database operations (Drizzle)                      │
│  • External service calls                             │
└──────────────────────────────────────────────────────┘
```

### API Design Principles

1. **Type-safe contracts** — Eden infers types from server, no codegen needed
2. **Validation at boundaries** — Typebox schemas for request/response
3. **Errors as exceptions** — `throw new HttpError(status, code, message)`
4. **Success wrapper** — `return { data }` for consistent responses
5. **Services own logic** — Routes are thin, services contain business rules

### Authentication

```typescript
// Session-based authentication with secure tokens
// Token stored in: Cookie (web) or SecureStore (mobile)

// Protected endpoint pattern:
.get('/protected', async ({ user }) => {
    requireAuth(user);  // Throws 401 if not authenticated
    // user.id, user.profileId available
})
```

---

## 4. Database Architecture

### Schema Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                           USERS & AUTH                               │
│                                                                      │
│  ┌────────────┐     ┌────────────┐     ┌────────────┐               │
│  │   users    │────▶│  sessions  │     │    otp     │               │
│  │            │     │            │     │            │               │
│  │ id (PK)    │     │ id (PK)    │     │ id (PK)    │               │
│  │ email      │     │ user_id    │     │ email      │               │
│  │ password   │     │ token      │     │ code       │               │
│  │ created_at │     │ expires_at │     │ expires_at │               │
│  └─────┬──────┘     └────────────┘     └────────────┘               │
│        │                                                             │
│        │ 1:1                                                         │
│        ▼                                                             │
│  ┌────────────┐                                                      │
│  │  profiles  │                                                      │
│  │            │                                                      │
│  │ id (PK)    │                                                      │
│  │ user_id    │                                                      │
│  │ name       │                                                      │
│  │ bio        │                                                      │
│  │ photos[]   │                                                      │
│  │ interests[]│                                                      │
│  └─────┬──────┘                                                      │
│        │                                                             │
└────────┼─────────────────────────────────────────────────────────────┘
         │
         │ 1:N                                   1:N
         ├──────────────────────────────────────┐
         ▼                                      ▼
┌─────────────────────────┐      ┌─────────────────────────┐
│        EVENTS           │      │         CHATS           │
│                         │      │                         │
│  ┌────────────┐         │      │  ┌────────────┐         │
│  │   events   │         │      │  │conversations│        │
│  │            │         │      │  │            │         │
│  │ id (PK)    │         │      │  │ id (PK)    │         │
│  │ organizer  │◀────────│──────│──│ event_id   │         │
│  │ title      │         │      │  │ type       │         │
│  │ starts_at  │         │      │  └──────┬─────┘         │
│  └──────┬─────┘         │      │         │               │
│         │               │      │         │ 1:N           │
│         │ 1:N           │      │         ▼               │
│         ▼               │      │  ┌────────────┐         │
│  ┌────────────┐         │      │  │ participants│        │
│  │ attendees  │         │      │  │            │         │
│  │            │         │      │  │ conv_id    │         │
│  │ event_id   │         │      │  │ profile_id │         │
│  │ profile_id │         │      │  └────────────┘         │
│  │ status     │         │      │                         │
│  └────────────┘         │      │  ┌────────────┐         │
│                         │      │  │  messages  │         │
└─────────────────────────┘      │  │            │         │
                                 │  │ id (PK)    │         │
                                 │  │ conv_id    │         │
                                 │  │ sender_id  │         │
                                 │  │ content    │         │
                                 │  │ created_at │         │
                                 │  └────────────┘         │
                                 │                         │
                                 └─────────────────────────┘
```

### Key Relationships

| Relationship | Type | Description |
|--------------|------|-------------|
| `users` → `profiles` | 1:1 | Each user has one profile |
| `profiles` → `events` | 1:N | User organizes events |
| `profiles` → `attendees` | M:N | Users attend events |
| `conversations` → `messages` | 1:N | Conversation contains messages |
| `conversations` → `participants` | 1:N | Conversation has participants |

### Database Access Patterns

```typescript
// Drizzle ORM - Type-safe SQL queries
const profiles = await db.query.profiles.findMany({
    where: eq(profiles.userId, userId),
    with: {
        photos: true,
        interests: true,
    },
});
```

---

## 5. Real-Time Architecture

### WebSocket Chat System

```
┌──────────────┐                     ┌──────────────┐
│   Client 1   │                     │   Client 2   │
│              │                     │              │
└──────┬───────┘                     └───────┬──────┘
       │                                     │
       │ WSS                           WSS   │
       ▼                                     ▼
┌─────────────────────────────────────────────────────┐
│                   Elysia WebSocket                   │
│                                                      │
│  ┌─────────────────────────────────────────────┐    │
│  │           Connection Manager                 │    │
│  │  • Map<userId, Set<WebSocket>>              │    │
│  │  • Map<conversationId, Set<userId>>         │    │
│  └─────────────────────────────────────────────┘    │
│                        │                             │
│                        ▼                             │
│  ┌─────────────────────────────────────────────┐    │
│  │              Message Handler                 │    │
│  │  • Receive → Validate → Store → Broadcast   │    │
│  └─────────────────────────────────────────────┘    │
│                                                      │
└─────────────────────────────────────────────────────┘
```

### Message Flow

1. Client sends message via WebSocket
2. Server validates sender is participant
3. Message stored in PostgreSQL
4. Server broadcasts to all participants
5. Clients receive real-time update

---

## 6. Storage Architecture

### MinIO Object Storage

```
poziomki-uploads/
├── profile-pics/       # Profile photos
│   └── {uuid}.{ext}
├── event-covers/       # Event cover images
│   └── {uuid}.{ext}
└── chat-attachments/   # Chat media (future)
    └── {uuid}.{ext}
```

### File Access Flow

```
1. Mobile requests file: cdn.poziomki.app/{filename}
2. Caddy forwards auth check to API: /api/v1/uploads/auth-check
3. API validates: token + file ownership
4. If authorized: Caddy proxies to MinIO
5. File served with cache headers
```

### Upload Flow

```
1. Mobile selects file
2. Mobile sends multipart POST to /api/v1/uploads
3. API validates: file type (magic bytes), size, user quota
4. API stores in MinIO with UUID filename
5. API records in uploads table (ownership tracking)
6. API returns CDN URL
```

---

## 7. Mobile Architecture

### Navigation Structure (Expo Router)

```
app/
├── _layout.tsx           # Root layout with providers
├── (auth)/               # Authentication screens
│   ├── login.tsx
│   └── verify.tsx
├── (tabs)/               # Main tab navigation
│   ├── _layout.tsx       # Tab bar configuration
│   ├── index.tsx         # Home/feed
│   ├── events.tsx        # Events list
│   ├── matches.tsx       # Matching
│   └── profile.tsx       # User profile
├── event/
│   ├── [id].tsx          # Event detail
│   └── create.tsx        # Create event
├── chat/
│   └── [id].tsx          # Chat screen
└── profile/
    ├── [id].tsx          # View profile
    └── edit.tsx          # Edit profile
```

### State Management

```
┌─────────────────────────────────────────────────────┐
│                  React Context                       │
│                                                      │
│  ┌─────────────────┐  ┌─────────────────┐           │
│  │ AuthContext     │  │ ThemeContext    │           │
│  │ • user          │  │ • theme         │           │
│  │ • login()       │  │ • toggle()      │           │
│  │ • logout()      │  │                 │           │
│  └─────────────────┘  └─────────────────┘           │
│                                                      │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│                  TanStack Query                      │
│                                                      │
│  • Server state caching                              │
│  • Automatic refetching                              │
│  • Optimistic updates                                │
│  • Offline support                                   │
│                                                      │
└─────────────────────────────────────────────────────┘
```

### API Client (Eden Treaty)

```typescript
// Type-safe API client generated from server types
import { api } from '@/lib/api';

// Automatic type inference
const { data } = await api.events.get({ query: { limit: 20 } });
// data is typed as EventResponse[]
```

---

## 8. Deployment Architecture

### Production Setup

```
┌─────────────────────────────────────────────────────────────────────┐
│                         OVHcloud VPS                                 │
│                        (France - EU)                                 │
│                                                                      │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                      Docker Compose                           │  │
│  │                                                                │  │
│  │   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐         │  │
│  │   │  Caddy  │  │   API   │  │ Postgres│  │  MinIO  │         │  │
│  │   │ :443    │  │ :3000   │  │ :5432   │  │ :9000   │         │  │
│  │   └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘         │  │
│  │        │            │            │            │                │  │
│  │        └────────────┴────────────┴────────────┘                │  │
│  │                         Docker Network                         │  │
│  │                                                                │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  Storage:                                                            │
│  • /data/postgres - Database files                                   │
│  • /data/minio - Object storage                                      │
│  • /data/caddy - TLS certificates                                    │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Environment Separation

| Environment | Purpose | Infrastructure |
|-------------|---------|----------------|
| Development | Local dev | Docker Compose (local) |
| Staging | Testing | Not implemented |
| Production | Live users | OVHcloud VPS (France) |

---

## 9. Technology Decisions

### Why Bun?

- Native TypeScript support (no transpilation)
- Fast startup time
- Built-in bundler
- Native test runner
- Active development

### Why Elysia?

- Type-safe by design
- Eden treaty for client types
- High performance
- Excellent DX
- Plugin ecosystem

### Why Drizzle?

- Type-safe SQL (not just types)
- Zero-cost abstractions
- Schema as code
- Excellent migrations
- No runtime overhead

### Why Expo?

- Cross-platform from single codebase
- Over-the-air updates
- Excellent developer experience
- Strong community
- Easy native module integration

### Why PostgreSQL?

- ACID compliance
- Excellent reliability
- Rich feature set
- Strong community
- Self-hosted capability

### Why SeaweedFS? (Planned Migration)

> **Note:** Currently using MinIO, planned migration to SeaweedFS.

- S3-compatible API (drop-in replacement)
- **Actively maintained open source** (MinIO entered maintenance mode Dec 2025)
- **Official hardened Docker images** (Chainguard zero-CVE, FIPS option)
- O(1) disk seek optimized for small files (profile photos, event images)
- Can use PostgreSQL for filer metadata (already have PostgreSQL)
- Self-hosted with EU data residency
- Apache 2.0 license

---

## 10. Future Considerations

### Scaling Path

| Phase | Users | Changes Needed |
|-------|-------|----------------|
| Current | <5K | Single instance |
| Phase 2 | 5K-20K | Add Valkey caching, optimize queries |
| Phase 3 | 20K-100K | Horizontal API scaling, read replicas |
| Phase 4 | 100K+ | Full distributed architecture |

### Potential Additions

- **Dragonfly** — In-memory cache (25x faster than Redis, Chainguard hardened image)
- **Message queue** — Background jobs, notifications
- **CDN** — Bunny.net for static assets (EU-based)
- **Monitoring** — Prometheus + Grafana stack (self-hosted)
- **E2E encryption** — Signal protocol for messages

### Why Dragonfly over Redis/Valkey?

| Criteria | Dragonfly | Valkey |
|----------|-----------|--------|
| Speed | 25x faster (multi-threaded) | Single-threaded |
| Memory | 38% more efficient | Baseline |
| Config | Simpler (fewer options) | More knobs |
| Hardened image | Chainguard | Chainguard, Docker, Bitnami, Canonical |

For our use cases (session cache, rate limiting, pub/sub), Dragonfly provides all needed features with better performance and simpler configuration. See [CACHING.md](../CACHING.md) for detailed comparison.

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-02-02 | Engineering Team | Initial version |
