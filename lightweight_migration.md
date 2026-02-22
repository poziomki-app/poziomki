# Lightweight Migration Plan

Last updated: 2026-02-22

## Goal

Comprehensive stack optimization across backend, mobile, and infrastructure to achieve:

- Minimal dependency footprint (fewer crates, fewer lines of code)
- Maximum compile-time safety
- Lowest operational cost (memory, CPU, disk, dollars)
- Modern, secure defaults (as of 2026)
- Fastest runtime performance

Constraints:

- Keep ops cheap, self-hosted, sovereign
- Keep Postgres-first search (`tsvector`/`pg_trgm`) and support future `pgvector`
- Line count is not a concern; correctness, control, and performance are

## Current Repo Reality (important)

This is **not** a greenfield rewrite:

- Loco usage is spread across many files (`~41` files reference `loco_rs` in `backend/src`)
- Route handlers are already mostly Axum-style handlers, but routing registration uses Loco wrappers
- `OpenDAL` is actively used for Garage S3 + local FS fallback in uploads storage
- Search is already SQL-heavy and Postgres-optimized (`tsvector`, `pg_trgm`, `earthdistance`)

Key touchpoints:

- Loco bootstrap/hooks: `backend/src/app.rs`, `backend/src/bin/main.rs`
- Loco route wrapper: `backend/src/controllers/migration_api/mod.rs`
- Loco auth/hash/jwt coupling: `backend/src/models/users.rs`
- OpenDAL storage adapter: `backend/src/controllers/migration_api/uploads_storage.rs`
- Upload endpoints using storage adapter: `backend/src/controllers/migration_api/uploads_support.rs`, `backend/src/controllers/migration_api/uploads.rs`
- Search SQL (already custom/raw): `backend/src/search.rs`

## Target Architecture (recommended)

- Web/API: `axum` + `tower` + `tower-http`
- Runtime: `tokio`
- DB runtime: `diesel` + `diesel-async` + pool (`deadpool` feature)
- DB migrations: `diesel_migrations` via `diesel-async` `migrations` feature (or plain SQL + external runner)
- Storage: Garage-only S3 adapter (`s3` crate preferred initial candidate)
- Auth/JWT/passwords (replace Loco helpers): `jsonwebtoken`, `argon2`, `password-hash`
- Errors: `thiserror` + app-specific `AppError`

### Scalability-first design tweaks (cheap + sovereign)

These are additive and should be designed in during the rewrite:

- keep the API stateless (easy horizontal scaling on cheap VPS nodes)
- split request path from side effects:
  - request path = validate + minimal DB write + return
  - workers = Matrix side effects, mail sending, media variants, recomputes
- keep Postgres as the durable system of record, but serve precomputed/read-model tables for heavy endpoints
- keep media transfer off the API path (signed Garage URLs, direct upload/download)
- add graceful degradation switches for expensive features (candidate counts, recomputes)

---

# Part 1: Backend (Rust)

## Current Stack

| Component | Current | Crates | Lines (excl platform) |
|---|---|---:|---:|
| Web Framework | Loco-rs 0.16.4 (wraps Axum) | ~23 direct deps | Heavy |
| ORM | SeaORM 1.1.19 (wraps SQLx) | 206 unique | 1.9M |
| S3 Client | OpenDAL 0.55 | Multi-backend abstraction | Heavy |
| HTTP Client | reqwest 0.16.5 | — | — |
| DateTime | chrono 0.4.43 | — | — |
| Matrix | Ruma 0.14.1 (declared, likely unused in backend) | — | — |
| Serialization | serde | — | — |
| Async Runtime | Tokio 1.49 | — | — |
| Docker | rust:1.88-bookworm -> debian:bookworm-slim | — | ~80MB final image |

## Target Stack

| Component | Target | Crates | Lines (excl platform) |
|---|---|---:|---:|
| Web Framework | **Axum 0.8.8** (drop Loco) | Minimal | Thin layer on hyper |
| ORM | **Diesel 2.3 + diesel-async 0.7** | 96 unique | 917K |
| S3 Client | **s3 crate** (Garage-only) | Minimal | — |
| HTTP Client | reqwest (keep) | — | — |
| DateTime | chrono (keep for DB) + **jiff 0.2** (business logic) | — | — |
| Matrix | **Remove backend `ruma` completely** | — | Backend stays `reqwest + serde`; chat protocol lives in Tuwunel + Kotlin SDK |
| Serialization | serde (keep, irreplaceable for JSON) | — | — |
| Async Runtime | Tokio (keep, ecosystem lock-in) | — | — |
| Docker | **cargo-chef + musl + distroless** | — | ~20MB final image |

## Dependency Comparison (measured, vendored)

| Metric | tokio-postgres | Diesel | SQLx | SeaORM |
|---|---:|---:|---:|---:|
| Vendored crates | 125 | **120** | 196 | 275 |
| Unique crates | 77 | **96** | 171 | 206 |
| Total .rs lines (excl platform) | 1.0M | **917K** | 1.3M | 1.9M |
| Library's own code | 22K | 75K | 71K | 153K |

Diesel is the lightest option that includes a query builder + compile-time schema safety.

## Matrix / Ruma (Backend) Decision

### Current reality in this repo

- Backend Matrix orchestration is **real** (session bootstrap, room creation/join/leave, avatar sync, push gateway), but implemented mostly with **`reqwest + serde`**.
- `ruma` is declared in `backend/Cargo.toml`, but backend code paths currently do not import `ruma::*` directly (verify during migration with `rg` + `cargo tree -i ruma`).
- Client-side E2E chat already lives in **Tuwunel + Kotlin Matrix SDK bindings**, so backend is not the primary Matrix protocol engine.

### Recommendation (for lightweight rewrite)

- **Drop backend `ruma` now** to reduce dependency surface while removing Loco/SeaORM.
- Keep backend Matrix integration as thin `reqwest + serde` wrappers for:
  - session bootstrap
  - room/membership orchestration
  - media/profile sync glue
  - push gateway endpoint handling
- Do **not** plan to reintroduce Ruma in the backend.
- Only revisit this if the backend is intentionally redesigned into a real Matrix protocol actor (unlikely in target architecture).

### Why this fits the target architecture

- Stateless API does **not** require Ruma.
- Tuwunel and the Kotlin Matrix SDK already handle chat/E2E protocol behavior.
- Keeping Matrix side effects in workers/outbox preserves reliability without needing a broad Matrix type dependency in the request path.

## Why Diesel over SQLx

| Feature | Diesel | SQLx |
|---|---|---|
| Compile-time query checking | **No running DB needed** | Needs live DB or stale `sqlx-data.json` |
| Compile-time schema validation | **Yes** (codegen `schema.rs`) | No |
| Type-safe query builder | **Yes** (zero-cost, compile-time) | No (raw SQL strings) |
| Query pipelining | **Yes** (diesel-async exclusive) | No |
| Join/insert/column validation | **All at compile time** | None |
| Unique crate count | **96** | 171 |
| Total dependency code | **917K lines** | 1.3M lines |
| TLS stack | Uses system libpq (no Rust TLS) | Bundles rustls + ring |
| CI complexity | `cargo check` is sufficient | Needs `DATABASE_URL` or `cargo sqlx prepare --check` |

SQLx's only advantage (lighter than Diesel) turned out to be false — it's nearly 2x the deps due to the bundled Rust TLS stack.

## Why Diesel over SeaORM

| Metric | Diesel | SeaORM |
|---|---|---|
| Query building | Compile-time (monomorphized, zero-cost) | **Runtime** (SeaQuery AST -> SQL string) |
| Result mapping | Direct row -> struct | Row -> `sea_orm::Value` -> struct |
| Unique crates | 96 | 206 |
| Total code | 917K | 1.9M |
| Compile time | Fast (no heavy proc macros) | Slow (DeriveEntityModel expands heavily) |
| Architecture | Self-contained | Wraps SQLx (you pay for SQLx + SeaQuery + SeaSchema + proc macros) |

SeaORM is three abstraction layers deep: your code -> SeaORM -> SeaQuery -> SQLx -> Postgres.

## Diesel Feature Matrix

| Feature | Diesel | SQLx | tokio-postgres | SeaORM |
|---|---|---|---|---|
| Compile-time query checking | **No DB needed** | Needs live DB or stale JSON | No | No |
| Compile-time schema validation | **Yes** | No | No | No |
| Type-safe query builder | **Yes** | No (raw SQL) | No (raw SQL) | Yes (runtime) |
| Query pipelining | **Yes** (diesel-async) | No | No | No |
| Compile-time join validation | **Yes** | No | No | No |
| Compile-time insert field checking | **Yes** | No | No | No |
| Upsert DSL (`ON CONFLICT`) | **First-class** | Raw SQL | Raw SQL | Partial |
| Postgres native types (JSONB, arrays, enums) | **First-class** | Via raw SQL | Via raw SQL | Partial |
| `COPY` bulk insert | **Yes** | No | Yes | No |
| Migrations | Built-in | Built-in | No | Built-in |
| Raw SQL escape hatch | `sql_query()` | Native | Native | `Statement` |

## Research Summary (2026)

### Web stack

- `axum` current stable line: `0.8.8` (good target for migration)
- Axum 0.8 is mature and aligns with your existing handler style and path syntax already used (`/{id}`)
- Maintained by the Tokio team. In TechEmpower R23 (Jan 2026), Axum achieves ~400k req/s with Postgres
- Memory usage is the most efficient among Rust frameworks, ideal for container deployments

### Diesel stack

- `diesel` current stable: `2.3.6`
- `diesel-async` current stable: `0.7.4`
- `diesel-async` supports:
  - async PostgreSQL
  - pools (`deadpool`, `bb8`, etc.)
  - async migrations via `AsyncMigrationHarness` (`migrations` feature)

### `pgvector` / `tsvector` compatibility

- `pgvector-rust` supports Diesel
- For `tsvector` and advanced ranking/geo/vector queries, plan to use **raw SQL** (`sql_query`) on hot paths even with Diesel

### Storage client options (Garage-only S3)

Recommended initial replacement for OpenDAL:

- `s3` crate (`0.1.15`) because it supports:
  - async client
  - put/get/delete
  - presigned URLs
  - feature-gated small surface

Alternative:

- `rusty-s3` (`0.8.1`) if you only want presigning and are happy using `reqwest` for object ops manually

Not recommended as first replacement in this rewrite:

- `rust-s3` (works, but broader feature surface and maintenance profile is less ideal for a "light, focused rewrite")
- `aws-sdk-s3` — recently broke S3-compatible services (Minio, Garage, etc.) with mandatory integrity checksums

## Diesel-Specific Notes

### `tsvector` / `pgvector` / `earthdistance`

Diesel handles this via `sql_query()` — the raw SQL escape hatch. ~30% of queries (search/spatial/vector) use this; the other ~70% (CRUD) get full compile-time safety.

```rust
// CRUD — fully type-safe
profiles::table
    .filter(profiles::city.eq("Warsaw"))
    .select(Profile::as_select())
    .load(conn).await?;

// Complex search — raw SQL, typed results
diesel::sql_query(
    "SELECT *, ts_rank_cd(search_tsv, plainto_tsquery('simple', $1)) AS rank
     FROM profiles WHERE search_tsv @@ plainto_tsquery('simple', $1)
     ORDER BY rank DESC LIMIT $2"
)
.bind::<Text, _>(query)
.bind::<Integer, _>(limit)
.load::<SearchResult>(conn).await?;
```

### Why Diesel can still work for `tsvector` / `pgvector`

Diesel is best used here as:

- type-safe CRUD/query builder for common paths
- transaction manager
- row mapping

And **not** as the only way to express advanced search logic.

For your hot paths, use raw SQL. This is normal and compatible with Diesel.

### Async in Axum

Use `diesel-async` (Postgres + deadpool):

- avoid wrapping sync Diesel in ad-hoc `spawn_blocking` everywhere
- keep request handlers async
- run migrations via `diesel-async` migration harness at startup (or separate CLI)

### `pgvector`

Plan:

- `CREATE EXTENSION IF NOT EXISTS vector`
- use `pgvector` Rust crate with Diesel feature
- keep nearest-neighbor queries explicit SQL for ranking/index tuning

Cheap-scale retrieval strategy (recommended):

1. Use `tsvector` + filters + geo to get a bounded candidate set
2. Re-rank candidates with `pgvector` (avoid default full-table vector search)
3. Keep embeddings in a separate table to isolate index bloat from OLTP rows
4. Generate/update embeddings asynchronously (never on request path)

### `tsvector`

Keep current approach:

- generated/search columns in SQL migrations
- GIN indexes
- raw SQL for ranking and hybrid search

You already have this pattern working in `backend/src/search.rs`.

Scalability notes for search:

- keep candidate counts bounded in request path
- re-run `EXPLAIN ANALYZE` after every ranking/filter/index change
- prefer denormalized search/read-model rows over repeated heavy joins when endpoints get hot
- combine Postgres FTS + vector re-ranking rather than replacing FTS with vectors

## Axum Migration (Drop Loco)

### Why drop Loco

Loco-rs brings ~23 direct dependencies and a heavy transitive tree (SeaORM, lettre, jsonwebtoken, etc.). It's a Rails-style framework wrapper around Axum. Your handlers are already mostly Axum-style — Loco adds compile time and dependency weight without proportional value.

**Axum 0.8.8** is maintained by the Tokio team. In TechEmpower R23 (Jan 2026), Axum achieves ~400k req/s with Postgres. Memory usage is the most efficient among Rust frameworks.

### What Loco provides that must be replaced

| Loco Feature | Replacement |
|---|---|
| `Hooks`, `create_app`, `AppRoutes` | Plain Axum `Router` + `AppState` |
| `loco_rs::app::AppContext` | Custom `AppState` (DB pool, config, HTTP clients) |
| `loco_rs::Result` / `loco_rs::Error` | `thiserror` enum + `IntoResponse` |
| `loco_rs::hash` | `argon2` + `password-hash` |
| `loco_rs::auth::jwt` | `jsonwebtoken` |
| Background workers (no-op) | Drop |
| Tasks registration (no-op) | Drop |
| CLI scaffolding | Drop |

Scalability additions to include while replacing Loco:

- define `AppState` for separation of concerns (DB, config, side-effect clients, job/outbox publisher)
- create separate binaries/process roles early (`api-core`, `worker`) even if the worker starts minimal
- keep handlers thin and side-effect-free where possible (publish intent, don’t perform expensive work inline)

### Files to touch

- `backend/src/app.rs` — Loco Hooks/app boot
- `backend/src/bin/main.rs` — Loco CLI entrypoint
- `backend/src/controllers/migration_api/mod.rs` — Loco Routes wrapper
- `backend/src/models/users.rs` — Loco auth/hash/jwt
- `backend/src/controllers/migration_api/auth_helpers.rs` — JWT helpers
- `backend/src/controllers/migration_api/auth_account.rs` — Auth endpoints
- ~41 files reference `loco_rs` in `backend/src`

## Loco Removal Map (what must be replaced)

### Framework shell

- `Hooks`, `create_app`, `AppRoutes`, `cli::main`

Files:

- `backend/src/app.rs`
- `backend/src/bin/main.rs`
- `backend/src/controllers/migration_api/mod.rs`

### Context/state

- `loco_rs::app::AppContext` -> custom `AppState`

Used in many handlers (already Axum `State(...)` style, which helps).

Design `AppState` for scale from day one:

- DB pool (Diesel async)
- HTTP clients only where truly needed on request path
- storage signer/client
- config snapshot
- optional outbox publisher/job dispatcher handle
- clock/time provider (helps deterministic retries/tests)

### Error/result aliases

- `loco_rs::Result`
- `loco_rs::Error::{Message, Any}`

Replace with:

- `type AppResult<T> = Result<T, AppError>`
- `thiserror` enum + `IntoResponse`

### Password/JWT helpers

- `loco_rs::hash`
- `loco_rs::auth::jwt`

Replace in:

- `backend/src/models/users.rs`
- `backend/src/controllers/migration_api/auth_helpers.rs`
- `backend/src/controllers/migration_api/auth_account.rs`

### Optional Loco features likely safe to drop

- background workers / queues (currently no-op in `backend/src/app.rs`)
- tasks registration (currently no-op)
- Loco CLI scaffolding

## S3 Client (Replace OpenDAL)

### Why replace OpenDAL

OpenDAL is a multi-backend storage abstraction (S3, GCS, Azure, local FS, etc.). You use one backend: Garage S3. The abstraction isn't needed.

### Current OpenDAL operations used

- `read`, `write`, `stat`, `delete`, `presign` — narrow subset

### Target: `s3` crate (0.1.15)

- Async client, put/get/delete, presigned URLs, feature-gated small surface
- Alternative: `rusty-s3` if you only need presigning (sans-IO, BYO HTTP client)
- Avoid `aws-sdk-s3` — recently broke S3-compatible services with mandatory integrity checksums

Scalability additions for storage path:

- move to direct-to-Garage uploads (API signs, client uploads)
- avoid proxying media bytes through API except exceptional/private cases
- use immutable object keys/variant names for cacheability and safer retries
- keep originals private and serve cache-friendly variants directly when possible
- move resizing/variant generation to worker jobs

## OpenDAL Replacement Design (Garage-only)

Current OpenDAL capabilities used:

- `read`
- `write`
- `stat`
- `delete`
- `presign`

This is a narrow subset. Good rewrite candidate.

### Proposed adapter interface (keep stable)

Use a local trait or module contract (same as current function API) so endpoint code is unchanged.

Design the adapter boundary to support:

- signing-only flows (future optimization)
- direct upload/download object operations
- idempotent writes/deletes (worker retries)
- public URL rewrite / CDN fronting without changing endpoint code

Recommended implementation path:

1. Garage-only config loader
   - remove local FS fallback (`Fs`) and `NODE_ENV` storage branching to force Garage parity everywhere
   - require `GARAGE_S3_*` in local dev/staging/prod
2. `s3` crate client init
3. `put/get/delete/head` wrappers
4. presign GET URL generation
5. preserve public URL rewrite logic (already in current module)

### Why not `rusty-s3` first

`rusty-s3` is excellent for presigning, but for your module you also need direct object read/write/delete.
Using `rusty-s3` would require pairing with manual HTTP requests (`reqwest`) for each operation.

That can be a good *second* optimization if you later reduce server-side object operations.

## Other Backend Changes

| Area | Current | Target | Notes |
|---|---|---|---|
| DateTime | chrono 0.4.43 | Keep chrono (diesel compat) + **jiff 0.2.20** for business logic | jiff: correct-by-default TZ, RFC 9557, by BurntSushi. 1.0 expected Spring/Summer 2026 |
| Image processing | `image` 0.25.9 | Keep + add **zune-image** decoders for hot paths | SIMD-optimized JPEG/PNG. Rust PNG now 1.8x faster than C libpng |
| Binary serialization | N/A | Consider **bitcode** for internal IPC/caching | Tops every Rust serialization benchmark (Jan 2026) |
| HTTP client | reqwest 0.16.5 | Keep | No lighter async alternative in Tokio ecosystem |
| Matrix | Ruma 0.14.1 | **Remove from backend** | Keep `reqwest + serde` for thin Matrix HTTP glue only |
| Async runtime | Tokio 1.49 | Keep | io_uring integration coming natively; monoio/glommio break ecosystem compat |

### Frameworks not recommended

| Framework | Status | Why not |
|---|---|---|
| Pavex | Open beta v0.2.x | Not production-ready, breaking changes expected |
| Salvo 0.89 | Growing | Smaller ecosystem than Axum |
| Actix-web | Mature | Actor model overhead, Axum is lighter and Tokio-native |

## Docker Optimization

### Current

```
Builder:  rust:1.88-bookworm
Runtime:  debian:bookworm-slim (~80MB)
```

### Target

```
Builder:  rust:alpine + cargo-chef (5x faster rebuilds)
Runtime:  gcr.io/distroless/static (~2.5MB, CA certs, non-root, no shell)
```

**cargo-chef** pattern:
1. `prepare` — extract Cargo.lock + manifests
2. `cook` — build deps (cached Docker layer)
3. `build` — only recompiles your code

Result: **5x build speedup** on a ~500-dep codebase. Adding **sccache** reduces builds by 75%+ further.

**musl static linking** via `rust:alpine` produces a single static binary — no runtime dependencies, runs on scratch/distroless.

Total final image: **under 20MB** (vs current ~80MB+ bookworm-slim).

---

# Part 2: Mobile (Kotlin Multiplatform)

## Current vs Latest Versions

| Component | Current | Latest | Gap | Priority |
|---|---|---|---|---|
| Kotlin | 2.2.0 | **2.3.0** (Jan 2026) | 1 major | High |
| Compose Multiplatform | 1.8.2 | **1.10.1** (Feb 2026) | 2 major | High |
| Gradle | (check) | **9.3.1** (Jan 2026) | Major | High |
| AGP | 8.9.3 | **9.0.1** (Jan 2026) | 1 major | Medium |
| Navigation | 2.9.0-beta03 | **2.9.2** (stable) | Beta -> stable | Low effort |
| Ktor | 3.1.3 | **3.4.0** (Jan 2026) | 3 minor | Low effort |
| SQLDelight | 2.0.2 | **2.2.1** (Nov 2025) | 2 minor | Low effort |
| Matrix Rust SDK FFI | 26.2.6 | Latest | — | Medium |
| Coil | 3.2.0 | **3.3.0** | 1 minor | Low effort |
| Koin | 4.1.0 | **4.1.1** | Patch | Low effort |
| MapLibre Compose | 0.12.1 | 0.12.1 | Current | None |
| DataStore | 1.1.7 | Consider **multiplatform-settings 1.3.0** | — | Optional |

## High-Impact Upgrades

### Kotlin 2.3.0

- K2 compiler fully stable and default since 2.0.0, ~2x faster compilation
- New `smallBinary` option sets `-Oz` for LLVM — reduces Native binary sizes
- Unused return value checker
- Explicit backing fields (stable)
- Gradle 9.0+ compatibility
- Swift export improvements for iOS

### Compose Multiplatform 1.10.1

- **Compose Hot Reload** — stable and enabled by default. Zero-config, bundled in Gradle plugin. Massive DX win
- **`@Preview` annotation in `commonMain`** — no more platform-specific preview wrappers
- **Navigation 3** support on all non-Android targets (iOS, Desktop, Web)
- iOS rendering stability fixes (text style cache, font/icon corruption, first frame)
- Requires Kotlin 2.2.20+ for iOS and web targets

### Gradle 9.3.1

- **Configuration Cache is preferred mode** — skips configuration phase on cached builds, parallel task execution by default
- Java 17+ required to run Gradle 9.x
- `org.gradle.configuration-cache=true` in `gradle.properties`
- `org.gradle.parallel=true`
- `org.gradle.caching=true`

### AGP 9.0.1

- Built-in Kotlin support — AGP handles Kotlin compilation directly
- New `com.android.kotlin.multiplatform.library` plugin for shared modules
- Java compilation disabled by default (faster)
- Requires Gradle 9.1.0+
- **Breaking:** Requires structural project changes. Do with full toolchain upgrade

### Matrix Rust SDK FFI (latest)

- **50-70MB binary size reduction per architecture** via new `dist` profile
- MSRV bumped to Rust 1.88, Rust 2024 edition
- OAuth no longer gated behind experimental flag
- On a 4-ABI APK, this saves **200-280MB total**

### Navigation 2.9.2

- Stable release of what you're already using in beta
- Type-safe routes via Kotlin Serialization now stable
- Future: Navigation 3 (ground-up rewrite) available in CMP 1.10.0 for all targets

### Ktor 3.4.0

- Darwin engine dispatcher fix — uses `Dispatchers.IO` by default (was broken)
- OkHttp duplex streaming
- Zstd compression plugin
- For mobile: OkHttp engine on Android, Darwin engine on iOS

### SQLDelight 2.2.1

- Bug fixes. Stay with SQLDelight (not Room KMP) — you have existing `.sq` files
- Room KMP is stable since 2.7.0 but migration cost is high with no perf benefit

## Alternatives Evaluated (not recommended to switch)

| Current | Alternative | Verdict |
|---|---|---|
| Koin 4.1 | kotlin-inject 0.8 (compile-time DI) | Keep Koin — runtime DI is faster to iterate, less build overhead |
| Coil 3 | Landscapist Core (~312KB, 47% smaller) | Keep Coil — Landscapist Core just announced Jan 2026, not battle-tested |
| Coil 3 | Kamel 1.0.9 (lighter) | Keep Coil — most mature KMP image loader |
| Navigation Compose | Voyager 1.1.0-beta / Decompose 3.0.0-beta | Keep JetBrains navigation — first-party, best ongoing support |
| DataStore 1.1.7 | multiplatform-settings 1.3.0 | Consider — wraps platform-native storage (SharedPrefs/NSUserDefaults), zero overhead |

## Upgrade Order

Batch 1 (toolchain — do together):
1. Kotlin 2.2.0 -> 2.3.0
2. Compose Multiplatform 1.8.2 -> 1.10.1
3. Gradle -> 9.3.1 + enable configuration cache

Batch 2 (low-effort bumps — one PR):
4. navigation-compose 2.9.0-beta03 -> 2.9.2
5. Ktor 3.1.3 -> 3.4.0
6. SQLDelight 2.0.2 -> 2.2.1
7. Coil 3.2.0 -> 3.3.0
8. Koin 4.1.0 -> 4.1.1

Batch 3 (medium effort):
9. Matrix Rust SDK FFI -> latest (50-70MB/arch savings)
10. AGP 8.9.3 -> 9.0.1 (structural changes, do with Batch 1)

---

# Part 3: Infrastructure

## Production Reality (measured 2026-02-22)

**VPS:** OVH VPS-2, 7.6GB RAM, 72GB disk, kernel 6.14.0-37-generic, uptime 21 days, load 0.00

| Container | **Actual RAM** | Limit | Utilization | Notes |
|---|---:|---:|---:|---|
| **Postgres** | **44 MB** | 1 GB | 4% | Barely loaded |
| **API** | **90 MB** | 512 MB | 18% | Rust binary, efficient |
| **Stalwart** | **131 MB** | 256 MB | **51%** | Hungriest — for OTP-only email |
| **Tuwunel** | **68 MB** | 512 MB | 13% | Matrix homeserver |
| **Garage** | **6 MB** | 512 MB | 1% | 85x overprovisioned |
| **ntfy** | **14 MB** | unlimited | — | Push notifications |
| **Dozzle** | **9 MB** | 128 MB | 7% | Log viewer |
| **node-exporter** | **3 MB** | 128 MB | 2% | Exports to nothing |
| **Total containers** | **~365 MB** | ~3 GB | 12% | |
| **System total** | **1.3 GB** / 7.6 GB | | 17% | |

**No swap configured.** Disk 63% full (45G/72G).

Stale Docker images wasting disk:
- `matrixdotorg/synapse:latest` — 456MB (not running, replaced by Tuwunel)
- `getmeili/meilisearch:v1.12` — 238MB (not running, replaced by Postgres FTS)

Docker image sizes (running):
- API: 203MB (target: ~20MB with distroless)
- Stalwart: 271MB
- Tuwunel: 116MB
- ntfy: 114MB
- Dozzle: 75.7MB
- Postgres: 403MB (alpine, includes JIT)
- node-exporter: 40.8MB
- Garage: 32.8MB

## Current Memory Budget (configured limits)

| Service | Image | Memory Limit |
|---|---|---:|
| PostgreSQL | postgres:18-alpine | 1 GB |
| API | Custom Rust | 512 MB |
| Tuwunel | jevolk/tuwunel:latest | 512 MB |
| Garage | dxflrs/garage:v1.1.0 | 512 MB |
| Stalwart | stalwartlabs/stalwart:latest | 256 MB |
| Dozzle | amir20/dozzle:v8.14.8 | 128 MB |
| node-exporter | prom/node-exporter:v1.9.1 | 128 MB |
| **Total** | | **~3 GB** |

## Scaling Projection (100k registered users, 500-1000 concurrent)

| Component | At 5 users (actual) | At 500-1000 concurrent (projected) | Notes |
|---|---:|---:|---|
| **Rust API** | 90 MB | 50-150 MB | Lightest component. Tokio task = ~64 bytes. Use mimalloc to avoid glibc fragmentation |
| **PostgreSQL + PgBouncer** | 44 MB | 500-700 MB | **PgBouncer is mandatory.** Maps 500 clients to 20-30 backends. Without it, 500 direct conns = ~5 GB. PgBouncer itself uses ~1 MB |
| **Tuwunel** | 68 MB | 300-512 MB | **First to OOM** if chat is heavy. RocksDB block cache grows with room count. Non-federating helps. Plan 768MB-1GB when chat grows |
| **Garage** | 6 MB | 150-256 MB | CDN caching absorbs reads. Only upload writes hit Garage directly |
| **Stalwart** | 131 MB | 128-200 MB | OTP-only, short-lived SMTP connections. Bottleneck is receiving server rate limits, not Stalwart |
| **Caddy** (host) | ~40 MB | 150-300 MB | **Go GC can grow unbounded.** Set `GOMEMLIMIT=256MiB` in systemd unit |
| **Dozzle** | 9 MB | 50-128 MB | Monitoring |
| **node-exporter** | 3 MB | 20-50 MB | Lightweight |
| **OS + Docker daemon** | ~900 MB | 500-800 MB | Kernel, page cache, dockerd |
| **Total** | **~1.3 GB** | **~2.5-3.5 GB** | **Fits in 7.6 GB with ~4-5 GB left for OS page cache** |

### Critical scaling unlocks (must-do before 100+ concurrent)

1. **Add PgBouncer** — transaction pooling mode. `default_pool_size=30`, `max_client_conn=500`. Memory limit 64 MB. This is the single biggest scaling unlock. Without it, you cannot reach 500 concurrent users.
2. **Set `GOMEMLIMIT=256MiB`** for Caddy systemd service — prevents unbounded Go GC growth under load.
3. **Use mimalloc allocator** in Rust API — glibc can hold 3.5-4.5 GB after millions of requests due to fragmentation. mimalloc stays at 0.7-1.5 GB. Add: `mimalloc = { version = "0.1", default-features = false }` + `#[global_allocator]`.
4. **Lower Postgres `max_connections` to 30-50** — PgBouncer handles client connections. Saves ~5-8 MB per unused backend.

### Cheap-scale architecture patterns (recommended during redesign)

These are higher-leverage than most dependency swaps:

1. **Outbox pattern for side effects** — write domain row + outbox event in one DB transaction; workers handle Matrix/mail/media retries.
2. **Separate worker process** — same repo, separate binary for async jobs (image variants, Matrix actions, OTP delivery, recomputes).
3. **Read models / precomputed tables** — serve heavy endpoints from denormalized/pre-ranked tables instead of repeated expensive joins.
4. **Direct media path** — API signs Garage requests, Garage/CDN serves bytes.
5. **Feature shedding** — config toggles to reduce candidate counts / defer expensive recomputes under load.

Why this matters:

- Keeps user-facing request latency flat under load
- Lets you scale workers independently from API
- Preserves cheap single-node viability much longer

### Bottleneck order (what breaks first at scale)

1. **Postgres without PgBouncer** — 500 direct connections = OOM
2. **Tuwunel memory** — RocksDB block cache hits 512 MB limit under heavy chat
3. **Disk (72 GB, 63% used)** — Postgres + RocksDB + Garage objects + WAL compete. Upgrade to VPS-2 (160 GB) when disk hits 70%
4. **CPU (4 vCores)** — Postgres query processing + TLS termination + Tuwunel sync. Becomes bottleneck at 1000+ concurrent before RAM does
5. **Caddy GC** — Unbounded Go heap without `GOMEMLIMIT`

### Connection pooling comparison

| Pooler | Memory | TPS limit | Notes |
|---|---:|---:|---|
| **PgBouncer** | **~1 MB** (2 kB/conn) | ~44k (single-threaded) | Battle-tested 15+ years. Best for this scale |
| pgcat | ~10-50 MB | ~59k (multi-threaded) | Sharding/failover. Overkill for single-node |
| Supavisor | ~50-100+ MB (Elixir VM) | ~21k | Cloud-native. Worst latency, highest overhead |

**PgBouncer wins.** Two orders of magnitude lighter than alternatives. Single-threaded limit (44k TPS) is far beyond what 500-1000 concurrent users generate.

## Optimized Memory Budget

| Service | Image | Current | Target | Savings |
|---|---|---:|---:|---:|
| PostgreSQL | postgres:18-alpine + tuning | 1 GB | 1 GB (tuned) | ~200-400MB effective via `max_connections=20` |
| API | Rust distroless | 512 MB | 256 MB | 256 MB |
| Tuwunel | jevolk/tuwunel:**v1.5.0** | 512 MB | **256 MB** | 256 MB |
| Garage | dxflrs/garage:v1.1.0 | 512 MB | **256 MB** | 256 MB |
| Stalwart -> chasquid | chasquid or lettre direct | 256 MB | **32 MB** or **0** | 224-256 MB |
| Dozzle | amir20/dozzle:**v10.x** | 128 MB | **64 MB** | 64 MB |
| node-exporter -> Beszel | henrygd/beszel | 128 MB | **16 MB** | 112 MB |
| Docker daemon -> Podman | Daemonless | ~150 MB | **0** | 150 MB |
| **Total** | | **~3 GB** | **~1.9 GB** | **~1.1 GB saved** |

## PostgreSQL Tuning (1GB deployment)

Mount custom `postgresql.conf`:

```
shared_buffers = 128MB            # 10-15% of 1GB
work_mem = 4MB                    # conservative per-operation
maintenance_work_mem = 64MB
effective_cache_size = 512MB      # ~50% of total RAM
max_connections = 20              # each idle conn ~10MB; default 100 wastes RAM
wal_buffers = 4MB
```

Each idle Postgres connection consumes ~10MB. Dropping from default 100 to 20 saves ~800MB potential overhead. Use PgBouncer if connection counts grow.

## Tuwunel (Matrix Homeserver)

**Current:** `:latest` (unpinned)
**Target:** `jevolk/tuwunel:v1.5.0`

v1.5.0 improvements: jemalloc repackaged with platform-specific optimizations, reduced allocator load for database queries and JSON serialization, sync v3 hot-path optimized.

Context: Conduwuit was abandoned 2025-05-30. Tuwunel is the official successor, deployed at scale for Swiss government. Idles at ~50-100MB. Your 512MB limit is generous — 256MB is safe.

| Matrix Server | Language | Idle RAM | Status |
|---|---|---:|---|
| **Tuwunel** v1.5.0 | Rust | ~50-100MB | Active, recommended |
| Continuwuity v0.5.1 | Rust | ~50-100MB | Community fork |
| Dendrite v0.13.x | Go | ~100-200MB | Matrix.org project |
| Synapse v1.x | Python | ~500MB-1GB+ | Heaviest, avoid |

## Garage S3

**Current:** v1.1.0 (stable)
**Latest:** v2.0.0-beta1 (April 2025, breaking admin API changes)

Garage idles at ~5MB RSS. Your 512MB limit can safely drop to 256MB.

**Critical news:** MinIO was archived February 12, 2026. It is dead for open-source use. Garage is the clear winner for lightweight self-hosted S3.

| Solution | Idle RAM | Status |
|---|---:|---|
| **Garage v1.1.0** | ~5MB | Stable, recommended |
| Garage v2.0-beta | ~5MB | Beta, breaking changes |
| MinIO | 500MB-1GB+ | **Dead** (archived Feb 2026) |
| SeaweedFS | 2-4GB | Overkill |

## Stalwart (Mail Server)

**Current:** `:latest`, 256MB limit, ~100MB idle
**Problem:** Running a full IMAP/JMAP/CalDAV/CardDAV server just to send OTP emails

Options for OTP-only email:

| Option | RAM | Approach |
|---|---:|---|
| **chasquid** | ~10-20MB | SMTP-only MTA, Go, designed for simplicity |
| **lettre direct** | 0 (in-process) | Use lettre crate with external SMTP relay, eliminate mail container entirely |
| Keep Stalwart | ~100MB | If you plan mailbox features later |

Recommendation: **chasquid** (saves ~80-100MB) or **lettre direct** (saves the entire container).

## Dozzle (Log Viewer)

**Current:** v8.14.8, 128MB limit, ~20-30MB actual
**Target:** **v10.x** (Jan 2026)

v10.0 adds: webhooks, redesigned notifications, SQL log queries via DuckDB/WASM. Image is ~17MB. Lower limit from 128MB to 64MB.

## Monitoring

**Current:** node-exporter v1.9.1, 128MB limit
**Problem:** node-exporter exports metrics but you have no Prometheus or Grafana to view them. Collecting data nobody reads.

**Target:** **Beszel**

| | node-exporter | Beszel |
|---|---|---|
| RAM | ~15-30MB | <10MB (agent) |
| Web UI | **None** (needs Prometheus + Grafana) | **Built-in** with historical charts |
| Docker stats | No | **Yes** |
| Alerts | No (needs Alertmanager) | **Yes** (email/webhook) |
| Setup | Standalone metric exporter | Agent + Hub, single binary each |

Beszel replaces node-exporter + the missing Prometheus + the missing Grafana with a single <10MB agent.

## Reverse Proxy (Caddy)

**Current:** Caddy (host-level, outside Docker)
**Status:** Keep for now

| Proxy | Language | Idle RAM | Auto TLS | Notes |
|---|---|---:|---|---|
| **Caddy** | Go | ~40MB | Yes | Works, known memory leak under sustained load |
| **Pingap** (Pingora) | Rust | ~5-15MB | Yes (ACME) | Most promising future replacement |
| River (Pingora) | Rust | Very low | Not yet | Development paused |
| Nginx | C | ~2-5MB | No (certbot) | Manual TLS |

Watch **Pingap** as a future migration target. Caddy is fine for now — automatic TLS is invaluable.

## Docker Compose Alternatives

| Tool | Daemon Overhead | Idle RAM | Rootless |
|---|---:|---:|---|
| **Docker Compose** | ~140-180MB (dockerd) | ~180MB | Optional |
| **Podman + podman-compose** | 0 (daemonless) | ~0MB | Default |
| k3s | ~500MB+ | ~500MB+ | No (overkill) |

Podman reads the same `docker-compose.yml` files. Saves ~150MB daemon overhead. Rootless by default = better security. Migrate after MVP.

## Kernel Tuning (Linux 6.19 CachyOS)

### zram (strongly recommended)

Effectively doubles available memory by compressing swap in RAM with ~2-3x ratio. On 1GB VPS, gives ~1.5-2GB effective.

```ini
# /etc/systemd/zram-generator.conf
[zram0]
zram-size = ram / 2
compression-algorithm = zstd
swap-priority = 100
```

Check if CachyOS already has it: `swapon --show`

### sysctl tuning

```ini
# /etc/sysctl.d/99-lowmem.conf
vm.swappiness = 10
vm.overcommit_memory = 0
vm.dirty_ratio = 10
vm.dirty_background_ratio = 5
vm.vfs_cache_pressure = 150

net.core.somaxconn = 1024
net.ipv4.tcp_fastopen = 3
net.ipv4.tcp_max_syn_backlog = 1024
```

### io_uring

Not recommended yet. `tokio-uring` exists but ecosystem is not ready. Standard Tokio with epoll is fine for this scale. Tokio team is working on native io_uring integration — benefits will come without migration cost.

## VPS Provider

**Current:** OVH (~$5.47/mo)

| Provider | vCPU | RAM | Storage | Price/mo | Notes |
|---|---:|---:|---|---:|---|
| **Hetzner CX22** | 2 | **4GB** | 40GB NVMe | **~3.49 EUR** | Best price/perf, GDPR (DE/FI) |
| OVH Starter | 1 | 2GB | 20GB SSD | ~5.50 EUR | Unlimited traffic |
| Contabo S | 4 | 8GB | 50GB NVMe | ~3.60 EUR | Poor actual performance, oversubscribed |
| Oracle Cloud Free | 4 ARM | **24GB** | 200GB | **FREE** | Hard to provision, may be reclaimed |

**Recommendation:** **Hetzner CX22** — 4GB RAM for less money than OVH. Eliminates all memory pressure. The single highest-impact infra change.

## Alpine-Based Images

| Service | Current | Alpine/Minimal | Disk Savings |
|---|---|---|---|
| Postgres | Already alpine | — | — |
| API | debian:bookworm-slim (~80MB) | distroless/static (~2.5MB) | ~77MB |
| Stalwart | :latest (~100-150MB) | :latest-alpine (~50-80MB) | ~70MB |
| Tuwunel | Single Rust binary | Already minimal | — |
| Garage | Single binary | Already minimal | — |
| Dozzle | Single Go binary (~17MB) | Already minimal | — |

---

# Part 4: Backend Migration Strategy

## High-Level Migration Strategy

Do this in phases. Avoid combining framework + ORM + storage rewrites in one giant commit.

## Phase 0: Baseline and freeze

Before major rewrite:

- capture baseline API behavior (smoke tests)
- capture hot query plans (`EXPLAIN ANALYZE`) for search/matching
- capture memory/latency baseline in prod/staging

Quality gates to keep green throughout:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `backend/scripts/rust-code-analysis.sh`

## Phase 1: Axum bootstrap (drop Loco framework shell, keep SeaORM temporarily)

Objective: remove Loco runtime/boot/routing first while keeping DB access intact.

Replace:

- `backend/src/bin/main.rs` (Loco CLI entrypoint)
- `backend/src/app.rs` (Loco Hooks/app boot)
- `backend/src/controllers/migration_api/mod.rs` (Loco Routes wrapper)

Add:

- `AppState` (DB pool/connection, config, HTTP clients, shared services)
- plain Axum `Router`
- startup config loading + env validation
- tracing setup
- graceful shutdown

Temporary compatibility tactic:

- keep existing handler signatures where possible
- replace `loco_rs::prelude::*` imports with explicit Axum/Serde/Result types gradually

Expected scope:

- medium rewrite, low product risk if route paths/responses are preserved

## Phase 2: Replace Loco helpers (auth/hash/jwt/errors)

Objective: remove Loco utility dependencies that remain after Phase 1.

Key work:

- `backend/src/models/users.rs`
  - replace `loco_rs::hash::*` with `argon2` + `password-hash`
  - replace `loco_rs::auth::jwt` with `jsonwebtoken`
- replace `loco_rs::Error` / `Result` with local `AppError`
- replace any Loco-specific `Response` aliases with `axum::response::Response`

This phase is where "remove all Loco stuff" becomes real.

Scalability tweak to include in this phase:

- make auth/session writes idempotent where practical (helps retries once side effects are moved to workers/outbox)

## Phase 3: Storage simplification (Garage-only first, then OpenDAL removal)

Objective: simplify storage behavior before swapping client libraries.

Status update (implemented in current backend):

- Step A complete: `uploads_storage` is Garage-only (no local FS fallback)
- Step B complete: `OpenDAL` replaced behind existing storage API with `rust-s3`
- endpoint handlers kept unchanged (`uploads_support.rs` API preserved)

Step A (safe):

- make `uploads_storage` Garage-only
- remove local FS fallback (`Fs`) and `NODE_ENV` branching
- require `GARAGE_S3_*` always
- run Garage locally in development instead of local-disk uploads (higher parity, fewer code paths)
- keep module API unchanged:
  - `upload`
  - `read`
  - `exists`
  - `delete`
  - `signed_get_url`

Scalability tweaks for media path (add during Step A if possible):

- move toward direct-to-Garage uploads (API signs, client uploads)
- avoid proxying media bytes through API for normal flows
- use immutable object keys/variant names for better CDN/cache behavior
- keep originals private and serve cache-friendly variants directly
- move resize/variant generation to worker jobs

Step B (adapter swap):

- replace OpenDAL implementation in `backend/src/controllers/migration_api/uploads_storage.rs`
- keep `uploads_support.rs` and endpoint code unchanged
- current implementation uses `rust-s3` (`put/get/head/delete/presign`) with Garage custom region + optional public URL rewrite

Target client recommendation:

- start with `s3` crate for `put/get/delete/presign`
- keep a thin local adapter trait (`StorageBackend`) to avoid future lock-in

## Phase 4: Diesel introduction (parallel path)

Objective: add Diesel without breaking all existing endpoints at once.

Start with:

- `diesel` + `diesel-async` dependencies
- `schema.rs` generation
- one feature module migrated end-to-end (recommended: `sessions` or `otp_codes`)
- introduce a Postgres outbox table and worker polling loop (`FOR UPDATE SKIP LOCKED`) before broad feature rewrites

Keep SeaORM in parallel temporarily during transition.

This avoids rewriting every endpoint and ORM layer immediately.

Scalability tweak to include in Phase 4:

- design DB access around low Postgres connection counts (target `PgBouncer` + small backend pools)

## Phase 5: SeaORM -> Diesel module-by-module migration

Suggested order (lowest risk -> highest complexity):

1. `sessions`, `otp_codes`, `user_settings`
2. `users` auth reads/writes
3. `profiles`, `uploads`
4. `events`, `event_attendees`, `event_tags`
5. Matrix state tables (`matrix_dm_rooms`, room mapping logic)
6. complex matching/catalog helpers

Use Diesel for:

- standard CRUD
- transactional updates
- simple joins

Use raw SQL (via Diesel `sql_query`) for:

- FTS ranking (`ts_rank_cd`, `websearch_to_tsquery`)
- geo ranking (`earthdistance`, `cube`)
- future vector search / hybrid ranking (`pgvector`, RRF, etc.)

Scalability tweaks to include during module migration:

- convert hot multi-join endpoints to read models / precomputed tables when needed
- batch lookups and avoid N+1s explicitly (keep current `IN (...)` patterns)
- add partial indexes for real predicates (`deleted = false`, `status = 'going'`, etc.)
- partition append-only tables early (job attempts, notifications, audit, future embeddings)
- prefer UUIDv7/ULID for new high-write tables for index locality

## Phase 6: Remove SeaORM + Loco + migration crate coupling

After all DB/runtime paths are migrated:

- remove `loco-rs`
- remove `sea-orm`
- remove SeaORM entity generation (`backend/src/models/_entities/*`)
- replace current migration runner with Diesel migrations (or plain SQL migration runner)
- prune transitive bloat and remeasure build time/binary size

## Estimated Rewrite Size (rough)

This is a real rewrite. Expect:

- `2k-6k+` lines touched across backend depending on style cleanup
- multiple commits/PR-sized steps strongly recommended

Scalability redesign additions (can run in parallel with migration):

- Postgres outbox + worker binary: `2-5 days`
- Direct-to-Garage signed upload flow: `2-5 days` (depends on mobile/client changes)
- First read-model/precompute path (one feature area): `3-10 days`

The range depends on whether you:

- preserve endpoint signatures/types exactly
- refactor module layout simultaneously
- rewrite search/matching queries early vs later

## Acceptance Criteria

### Phase 1 complete (Axum shell)

- `loco-rs` no longer used for app boot/routing
- all current HTTP routes work with same paths/status codes
- SeaORM still passes tests

### Phase 3 complete (storage)

- uploads work in dev/prod via Garage only
- presigned URLs still work
- no `opendal` dependency remains

### Final complete

- no `loco-rs` dependency
- no `sea-orm` dependency
- no `opendal` dependency
- search (`tsvector` + geo) behavior preserved
- quality gates green
- Docker image under 20MB
- Compile time measurably reduced
- side effects (Matrix/mail/media) decoupled from request path via outbox/worker
- media not proxied through API in normal upload/download flows
- top hot endpoints have documented `EXPLAIN ANALYZE` plans
- at least one overload/degradation strategy implemented (bounded candidates / deferred recomputes)

---

# Part 5: Infrastructure Action Plan

## Immediate (free, high impact)

1. **Kernel tuning** — zram + sysctl (effectively doubles memory)
2. **Lower Docker memory limits** — Garage 256MB, Tuwunel 256MB, Dozzle 64MB
3. **Pin Tuwunel to v1.5.0**
4. **Mount custom PostgreSQL config** with low-memory settings
5. **Update Dozzle to v10.x**
6. **Add outbox + worker process skeleton** — foundation for cheap scale before feature growth
7. **Define direct-to-Garage media flow** (signed upload/download, immutable variants)
8. **Add worker observability + healthchecks** — outbox queue stats endpoint (pending/ready/failed/age) + worker heartbeat healthcheck to detect stalls early
   - add host-side alert script (`ops/check_outbox_alerts.sh`) using Docker health + outbox status endpoint
   - thresholds: worker unhealthy, `failedJobs > 0`, `oldestReadyJobAgeSeconds > 60`
   - run via cron/systemd timer every minute

### Outbox alerting (cheap/simple)

Host-side script: `ops/check_outbox_alerts.sh`

Checks:

- Docker Compose `worker` health is `healthy`
- `/api/v1/ops/outbox/status` is reachable with `OPS_STATUS_TOKEN`
- `failedJobs == 0`
- `oldestReadyJobAgeSeconds <= 60` (override with `OUTBOX_MAX_READY_AGE_SECONDS`)

Default inputs:

- `docker-compose.prod.yml`
- `OPS_STATUS_TOKEN` from environment or repo `.env`
- API URL `http://127.0.0.1:5150`

Example cron (every minute):

```cron
* * * * * cd /home/ubuntu/poziomki-rs && ./ops/check_outbox_alerts.sh >> /var/log/poziomki-outbox-alerts.log 2>&1
```

## Short-term (minimal effort)

9. **Replace node-exporter with Beszel** — actual monitoring UI, less RAM
10. **Switch Stalwart to alpine** — `stalwartlabs/stalwart:v0.15.5-alpine`
11. **cargo-chef + distroless** Docker build
12. **Add first read-model/precompute endpoint** (recommendations or heavy event listing)

## Medium-term (after MVP)

13. **Migrate VPS to Hetzner CX22** (4GB RAM, ~3.49 EUR/mo)
14. **Replace Stalwart with chasquid** or lettre direct (if OTP-only)
15. **Podman migration** — save ~150MB daemon overhead
16. **Evaluate Garage v2.0** when stable
17. **Evaluate Pingap** as Caddy replacement

---

# Sources

## Backend
- Axum 0.8.8: https://docs.rs/axum
- Axum 0.8 announcement: https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0
- Diesel: https://docs.rs/diesel
- diesel-async: https://docs.rs/diesel-async
- `diesel-async` migration support docs (includes `AsyncMigrationHarness`): https://docs.rs/diesel-async/latest/diesel_async/
- pgvector-rust: https://github.com/pgvector/pgvector-rust
- `pgvector` Postgres extension: https://github.com/pgvector/pgvector
- OpenDAL S3 service capabilities: https://opendal.apache.org/docs/rust/opendal/services/struct.S3.html
- s3 crate: https://docs.rs/crate/s3/0.1.15
- rust-s3: https://docs.rs/crate/rust-s3/latest
- rusty-s3: https://crates.io/crates/rusty-s3
- cargo-chef: https://github.com/LukeMathWalker/cargo-chef
- Distroless images: https://github.com/GoogleContainerTools/distroless
- jiff: https://github.com/BurntSushi/jiff
- zune-image: https://github.com/etemesi254/zune-image
- Rust serialization benchmarks: https://david.kolo.ski/rust_serialization_benchmark/
- TechEmpower R23: https://www.techempower.com/benchmarks/
- SeaORM 2.0: https://www.sea-ql.org/blog/2025-12-12-sea-orm-2.0/
- Pavex: https://pavex.dev/
- AWS S3 SDK breaks compat: https://xuanwo.io/links/2025/02/aws_s3_sdk_breaks_its_compatible_services/
- Rust Web Frameworks 2026: https://aarambhdevhub.medium.com/rust-web-frameworks-in-2026-axum-vs-actix-web-vs-rocket-vs-warp-vs-salvo-which-one-should-you-2db3792c79a2
- Rust ORMs in 2026: https://aarambhdevhub.medium.com/rust-orms-in-2026-diesel-vs-sqlx-vs-seaorm-vs-rusqlite-which-one-should-you-actually-use-706d0fe912f3
- Rust PNG outperforms C PNG: https://www.phoronix.com/news/Rust-PNG-Outperforms-C-PNG
- AVIF encoder comparison: https://catskull.net/libaom-vs-svtav1-vs-rav1e-2025.html
- Minimal Docker images for Rust: https://oneuptime.com/blog/post/2026-01-07-rust-minimal-docker-images/view
- Rust Dockerfile best practices: https://depot.dev/blog/rust-dockerfile-best-practices
- Ruma: https://ruma.dev/
- Jiff comparison docs: https://docs.rs/jiff/latest/jiff/_documentation/comparison/index.html
- rkyv: https://github.com/rkyv/rkyv
- Async Rust not safe with io_uring: https://tonbo.io/blog/async-rust-is-not-safe-with-io-uring

## Mobile
- Kotlin 2.3.0: https://kotlinlang.org/docs/whatsnew23.html
- Kotlin 2.2.20: https://kotlinlang.org/docs/whatsnew2220.html
- Kotlin 2.3.0 released: https://blog.jetbrains.com/kotlin/2025/12/kotlin-2-3-0-released/
- Kotlin releases: https://kotlinlang.org/docs/releases.html
- Compose Multiplatform 1.10.0: https://blog.jetbrains.com/kotlin/2026/01/compose-multiplatform-1-10-0/
- Compose Multiplatform 1.10.1: https://kotlinlang.org/docs/multiplatform/whats-new-compose-110.html
- Compose compatibility: https://kotlinlang.org/docs/multiplatform/compose-compatibility-and-versioning.html
- Ktor 3.4.0: https://ktor.io/docs/whats-new-340.html
- Ktor 3.4.0 blog: https://blog.jetbrains.com/kotlin/2026/01/ktor-3-4-0-is-now-available/
- Ktor client engines: https://ktor.io/docs/client-engines.html
- Navigation 3: https://kotlinlang.org/docs/multiplatform/compose-navigation-3.html
- Navigation 3 stable: https://android-developers.googleblog.com/2025/11/jetpack-navigation-3-is-stable.html
- Navigation in Compose KMP: https://kotlinlang.org/docs/multiplatform/compose-navigation.html
- SQLDelight: https://github.com/sqldelight/sqldelight/releases
- Room KMP: https://developer.android.com/kotlin/multiplatform/room
- SQLDelight vs Room: https://medium.com/@muralivitt/database-solutions-for-kmp-cmp-sqldelight-vs-room-ea9a52c7bce7
- KMP production readiness 2026: https://www.kmpship.app/blog/is-kotlin-multiplatform-production-ready-2026
- Matrix Rust SDK: https://github.com/matrix-org/matrix-rust-sdk/releases
- This Week in Matrix 2026-02-20: https://matrix.org/blog/2026/02/20/this-week-in-matrix-2026-02-20/
- This Week in Matrix 2026-01-23: https://matrix.org/blog/2026/01/23/this-week-in-matrix-2026-01-23/
- Koin 4.0: https://blog.insert-koin.io/koin-4-0-official-release-f4827bbcfce3
- Koin releases: https://github.com/InsertKoinIO/koin/releases
- Koin vs kotlin-inject: https://infinum.com/blog/koin-vs-kotlin-inject-dependency-injection/
- kotlin-inject KMP: https://github.com/evant/kotlin-inject/blob/main/docs/multiplatform.md
- Kodein DI: https://kosi-libs.org/kodein/7.25/getting-started.html
- Coil changelog: https://coil-kt.github.io/coil/changelog/
- Coil GitHub: https://github.com/coil-kt/coil
- Kamel: https://github.com/Kamel-Media/Kamel
- Landscapist Core: https://skydoves.medium.com/announcing-landscapist-core-a-new-image-loading-library-for-android-compose-multiplatform-6a4f408cba00
- Landscapist: https://github.com/skydoves/landscapist
- AGP 9 KMP migration: https://kotlinlang.org/docs/multiplatform/multiplatform-project-agp-9-migration.html
- AGP 9 release notes: https://developer.android.com/build/releases/gradle-plugin
- Update Kotlin projects for AGP 9: https://blog.jetbrains.com/kotlin/2026/01/update-your-projects-for-agp9/
- Android KMP Library Plugin: https://developer.android.com/kotlin/multiplatform/plugin
- Gradle 9.3.1: https://docs.gradle.org/current/release-notes.html
- Gradle 9.0: https://gradle.org/whats-new/gradle-9/
- Gradle performance guide: https://docs.gradle.org/current/userguide/performance.html
- Gradle configuration cache: https://docs.gradle.org/current/userguide/configuration_cache.html
- multiplatform-settings: https://github.com/russhwolf/multiplatform-settings
- DataStore for KMP: https://developer.android.com/kotlin/multiplatform/datastore
- Encrypted KV store in KMP: https://touchlab.co/encrypted-key-value-store-kotlin-multiplatform
- MapLibre Compose Maven Central: https://central.sonatype.com/artifact/org.maplibre.compose/maplibre-compose
- MapLibre Compose GitHub: https://github.com/maplibre/maplibre-compose
- MapLibre Compose Roadmap: https://maplibre.org/maplibre-compose/roadmap/
- Voyager: https://voyager.adriel.cafe/
- Decompose: https://github.com/arkivanov/Decompose

## Infrastructure
- Postgres 18 Docker 34% smaller: https://ardentperf.com/2025/04/07/waiting-for-postgres-18-docker-containers-34-smaller/
- PostgreSQL Memory Tuning: https://wiki.postgresql.org/wiki/Tuning_Your_PostgreSQL_Server
- PGlite: https://github.com/electric-sql/pglite
- Tuwunel: https://github.com/matrix-construct/tuwunel
- Tuwunel v1.5.0: https://newreleases.io/project/github/matrix-construct/tuwunel/release/v1.5.0
- Conduit vs Dendrite: https://matrixdocs.github.io/docs/servers/comparison
- Garage: https://git.deuxfleurs.fr/Deuxfleurs/garage/releases
- MinIO vs alternatives 2025: https://onidel.com/blog/minio-ceph-seaweedfs-garage-2025
- MinIO archived: https://www.infoq.com/news/2025/12/minio-s3-api-alternatives/
- Stalwart: https://stalw.art/docs/install/requirements/
- Chasquid: https://github.com/albertito/chasquid
- Four modern mail systems (SIDN): https://www.sidn.nl/en/news-and-blogs/four-modern-mail-systems-for-self-hosting
- Caddy vs Nginx benchmarks: https://blog.tjll.net/reverse-proxy-hot-dog-eating-contest-caddy-vs-nginx/
- Pingap: https://github.com/vicanso/pingap
- River reverse proxy: https://www.memorysafety.org/initiative/reverse-proxy/
- Caddy memory issues: https://github.com/caddyserver/caddy/issues/5366
- Beszel: https://github.com/henrygd/beszel
- Beszel guide: https://beszel.dev/guide/getting-started
- Beszel vs Prometheus/Grafana: https://techdecode.online/decode/beszel/
- Netdata alternatives: https://signoz.io/comparisons/netdata-alternatives/
- Dozzle 10.0: https://linuxiac.com/dozzle-10-0-real-time-docker-log-viewer-introduces-webhooks/
- Dozzle GitHub: https://github.com/amir20/dozzle
- Docker vs Podman benchmarks: https://sanj.dev/post/container-runtime-showdown-2025
- Podman vs Docker: https://last9.io/blog/podman-vs-docker/
- podman-compose: https://github.com/containers/podman-compose
- Hetzner pricing: https://costgoat.com/pricing/hetzner
- Hetzner cost-optimized plans: https://www.bitdoze.com/hetzner-cloud-cost-optimized-plans/
- Oracle Cloud Free: https://docs.oracle.com/en-us/iaas/Content/FreeTier/freetier_topic-Always_Free_Resources.htm
- Cheap VPS 2026: https://www.experte.com/server/cheap-vps
- Contabo vs Hetzner: https://www.vpsbenchmarks.com/compare/contabo_vs_hetzner
- zram vs zswap: https://onidel.com/blog/swapfile-zswap-zram-vps-2025
- Linux sysctl VM docs: https://docs.kernel.org/admin-guide/sysctl/vm.html
- tokio-uring: https://tokio.rs/blog/2021-07-tokio-uring
- io_uring and tokio-uring exploration: https://developerlife.com/2024/05/25/tokio-uring-exploration-rust/
- Stalwart Alpine Docker: https://hub.docker.com/layers/stalwartlabs/stalwart/latest-alpine/
- Alpine Docker: https://hub.docker.com/_/alpine

## Local code references

- `backend/src/app.rs`
- `backend/src/bin/main.rs`
- `backend/src/controllers/migration_api/mod.rs`
- `backend/src/controllers/migration_api/uploads_storage.rs`
- `backend/src/controllers/migration_api/uploads_support.rs`
- `backend/src/controllers/migration_api/uploads.rs`
- `backend/src/search.rs`
- `backend/src/models/users.rs`
- `backend/src/controllers/migration_api/auth_helpers.rs`
- `backend/src/controllers/migration_api/auth_account.rs`
