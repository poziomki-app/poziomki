# Infrastructure Optimization: 0 → 50k Users on OVH

> Research compiled February 2026. All pricing verified against current OVH/Hetzner pages.

---

## Table of Contents

1. [Current Architecture & Resource Footprint](#1-current-architecture--resource-footprint)
2. [Hosting: OVH vs Hetzner Pricing](#2-hosting-ovh-vs-hetzner-pricing)
3. [Scaling Phases & Cost Trajectory](#3-scaling-phases--cost-trajectory)
4. [Code Changes for Cost Reduction](#4-code-changes-for-cost-reduction)
   - 4.1 [Replace Meilisearch with PostgreSQL FTS](#41-replace-meilisearch-with-postgresql-fts)
   - 4.2 [Server-Side Image Resizing on Upload](#42-server-side-image-resizing-on-upload)
   - 4.3 [HTTP Caching Headers](#43-http-caching-headers)
   - 4.4 [Precompute Matching Scores](#44-precompute-matching-scores)
   - 4.5 [Add a CDN Layer for Images](#45-add-a-cdn-layer-for-images)
   - 4.6 [Move OTP State to PostgreSQL](#46-move-otp-state-to-postgresql)
   - 4.7 [Switch to UUIDv7 Primary Keys](#47-switch-to-uuidv7-primary-keys)
5. [PostgreSQL Tuning for Small Servers](#5-postgresql-tuning-for-small-servers)
6. [Garage S3 Production Configuration](#6-garage-s3-production-configuration)
7. [Matrix/Tuwunel Scaling Reality](#7-matrixtuwunel-scaling-reality)
8. [Summary: Impact Table](#8-summary-impact-table)
9. [Sources](#9-sources)

---

## 1. Current Architecture & Resource Footprint

Production stack (docker-compose.prod.yml) allocates **~3.3 GB RAM** across 6 containers:

| Service | RAM Limit | Role | Actual Idle Usage |
|---------|-----------|------|-------------------|
| PostgreSQL 18 | 1 GB | Primary database | ~50-100 MB |
| Loco API (Rust/Axum) | 512 MB | Stateless REST API | ~30-80 MB |
| Meilisearch v1.12 | 512 MB | Full-text + geo search | ~200-400 MB |
| Tuwunel | 512 MB | Matrix chat homeserver | ~70-150 MB |
| Garage v1.1.0 | 512 MB | S3-compatible object storage | ~20-100 MB |
| Stalwart | 256 MB | Self-hosted email (SMTP) | ~50-100 MB |

**Key insight:** The Rust API binary itself is extremely efficient (~30-80 MB). The cost story is about the supporting services. Meilisearch is the most replaceable (see Section 4.1).

---

## 2. Hosting: OVH vs Hetzner Pricing

### OVH VPS 2026 (launched January 2026)

All plans include **unlimited traffic**, free daily backups, Anti-DDoS, NVMe storage.

| Plan | vCores | RAM | NVMe | Bandwidth | EUR/mo |
|------|--------|-----|------|-----------|--------|
| **VPS-1** | 4 | 8 GB | 75 GB | 400 Mbps | **~4.49** |
| **VPS-2** | 6 | 12 GB | 100 GB | unlimited | **~5.70** |
| **VPS-3** | 8 | 24 GB | 200 GB | unlimited | **~13.99** |
| **VPS-4** | 12 | 48 GB | 300 GB | unlimited | **~18.60** |
| **VPS-5** | 12 | 48 GB | 300 GB | unlimited | **~29.00** |
| **VPS-6** | 24 | 96 GB | 400 GB | 3 Gbps | **~48.99** |

### OVH Public Cloud (B3 series, hourly-billed)

| Instance | vCPU | RAM | NVMe | USD/mo |
|----------|------|-----|------|--------|
| b3-8 | 2 | 8 GB | 50 GB | $37 |
| b3-16 | 4 | 16 GB | 100 GB | $74 |
| b3-32 | 8 | 32 GB | 200 GB | $148 |

**VPS 2026 is dramatically cheaper than Public Cloud.** VPS-1 at 4.49 EUR gets 4 vCores/8 GB; comparable Public Cloud b3-8 costs ~34 EUR.

### OVH Bare Metal (Advance series)

| Model | CPU | Cores | RAM | Storage | EUR/mo |
|-------|-----|-------|-----|---------|--------|
| Advance-1 | EPYC 4244P | 6c/12t | 32 GB DDR5 | 2x960 GB NVMe | ~85 |
| Advance-2 | EPYC 4344P | 8c/16t | 64 GB DDR5 | 2x960 GB NVMe | ~110 |
| Advance-3 | EPYC 4464P | 12c/24t | 64 GB DDR5 | 2x960 GB NVMe | ~165 |

All include unlimited traffic, Anti-DDoS, 500 GB backup, vRack private network (25 Gbps).

### OVH Object Storage

| Class | Cost/GB/mo | Egress |
|-------|-----------|--------|
| Standard (S3) | ~$0.008 | **FREE** (as of Jan 2026) |
| High Performance | ~$0.020 | **FREE** |

**Major change:** OVH removed egress fees on Object Storage entirely in late 2025. 1 TB = ~$8/month with zero egress.

### Hetzner Comparison

| Hetzner Plan | vCPU | RAM | Storage | Traffic | EUR/mo |
|-------------|------|-----|---------|---------|--------|
| CX23 (shared) | 2 | 4 GB | 40 GB | 20 TB | 2.99 |
| CPX31 (shared) | 4 | 8 GB | 160 GB | 3 TB | 15.99 |
| CCX13 (dedicated) | 2 | 8 GB | 80 GB | 1 TB | 11.99 |
| AX42 (bare metal) | 8c/16t | 64 GB | 2x512 GB NVMe | unlimited | 46.00 |

**Verdict:** OVH VPS-1 (4.49 EUR, 4 cores, 8 GB, unlimited traffic) is best value for small deployments. Hetzner wins on bare metal (AX42 at 46 EUR beats OVH Advance-1 at 85 EUR). For Poziomki, start on OVH VPS.

---

## 3. Scaling Phases & Cost Trajectory

### With optimizations applied (see Section 4)

| Phase | Users | OVH Plan | Services | EUR/mo |
|-------|-------|----------|----------|--------|
| MVP | 0-5k | VPS-1 (4c/8GB) | PG + API + Garage + Tuwunel | **~5** |
| Growth | 5-15k | VPS-2 (6c/12GB) | Same stack, PG tuned | **~6** |
| Scale | 15-30k | VPS-3 (8c/24GB) | + CDN ($1 BunnyCDN) | **~15** |
| Large | 30-50k | VPS-4 or split to 2x VPS | Monitor Tuwunel, bump RAM as needed | **~25-40** |

### Without optimizations (current architecture)

| Phase | Users | OVH Plan | EUR/mo |
|-------|-------|----------|--------|
| MVP | 0-2k | VPS-2 (6c/12GB) | ~6 |
| Growth | 2-10k | VPS-3 (8c/24GB) | ~14 |
| Scale | 10-30k | VPS-4 (12c/48GB) | ~19 |
| Large | 30-50k | Advance-1 bare metal | ~85 |

**Optimizations let you stay on cheaper VPS tiers ~3x longer.**

---

## 4. Code Changes for Cost Reduction

### 4.1 Replace Meilisearch with PostgreSQL FTS

**Saves: 512 MB RAM, one fewer service to operate**

Meilisearch indexes are 6-10x larger than raw data (8.6 MB source becomes 217 MB in Meilisearch). It recommends as much RAM as disk used. At 50k users, this means 1-2 GB RAM just for search.

PostgreSQL FTS with GIN indexes handles 50k-500k documents with sub-5ms queries.

#### Performance benchmarks (PostgreSQL FTS with GIN)

| Dataset | Without GIN | With GIN | Improvement |
|---------|------------|----------|-------------|
| 45k rows, simple query | N/A | **0.91 ms** | - |
| 45k rows, common term | N/A | **19.09 ms** | - |
| Generic dataset | ~8,154 ms | **~103 ms** | 98.7% |
| Optimized 10M rows | ~41.3 s | **~0.88 s** | ~50x |

#### Memory comparison at scale

| Documents | Meilisearch (disk+RAM) | PostgreSQL FTS (data+GIN) |
|-----------|----------------------|--------------------------|
| 50k | 500 MB - 1 GB | 50-100 MB |
| 100k | 1-2 GB | 100-200 MB |
| 500k | 5-10 GB | 500 MB - 1.5 GB |

#### Implementation approach

Replace the 4 Meilisearch indexes (profiles, events, tags, degrees) with:

**1. tsvector + GIN for text search:**
```sql
-- Add tsvector column to profiles
ALTER TABLE profiles ADD COLUMN search_vector tsvector
  GENERATED ALWAYS AS (
    setweight(to_tsvector('simple', coalesce(name, '')), 'A') ||
    setweight(to_tsvector('simple', coalesce(bio, '')), 'B') ||
    setweight(to_tsvector('simple', coalesce(program, '')), 'C')
  ) STORED;

CREATE INDEX idx_profiles_search ON profiles USING GIN (search_vector);

-- Query: SELECT * FROM profiles WHERE search_vector @@ plainto_tsquery('simple', 'query');
```

**2. pg_trgm for fuzzy/typo-tolerant fallback:**
```sql
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX idx_profiles_name_trgm ON profiles USING GIN (name gin_trgm_ops);

-- Fuzzy query: SELECT * FROM profiles WHERE name % 'query' ORDER BY similarity(name, 'query') DESC;
```

**3. earth_distance + cube for geo-radius queries (replaces `_geoRadius`):**
```sql
CREATE EXTENSION IF NOT EXISTS cube;
CREATE EXTENSION IF NOT EXISTS earthdistance;

-- GiST index for fast geo lookups
CREATE INDEX idx_events_geo ON events USING GIST (
  ll_to_earth(latitude, longitude)
) WHERE latitude IS NOT NULL AND longitude IS NOT NULL;

-- Radius query (e.g., 5km from user):
SELECT *, earth_distance(
  ll_to_earth(52.22, 21.01),
  ll_to_earth(latitude, longitude)
) AS distance_m
FROM events
WHERE earth_box(ll_to_earth(52.22, 21.01), 5000) @> ll_to_earth(latitude, longitude)
  AND earth_distance(ll_to_earth(52.22, 21.01), ll_to_earth(latitude, longitude)) <= 5000
ORDER BY distance_m;
```

Benchmarks: 12M row cities table went from 2,844 ms (no index) to **102 ms** (with GiST). At 50k events, expect sub-10 ms.

**4. Remove all Meilisearch code:**
- Delete `seed_search` background task
- Delete all `tokio::spawn` indexing calls on mutations
- Remove `meilisearch-sdk` from Cargo.toml
- Remove Meilisearch container from docker-compose.prod.yml

#### Future upgrade path

If search quality becomes a user complaint, consider **ParadeDB pg_search** (BM25 ranking, built-in fuzzy, faceting) — a Rust-based PostgreSQL extension built on Tantivy. Benchmarks show 265x speedup over native FTS at 10M rows. Stays within PostgreSQL, no additional service. Note: AGPL licensed.

---

### 4.2 Server-Side Image Resizing on Upload

**Saves: 70-90% bandwidth on image serving**

Currently `uploads.rs` stores raw uploads as-is. A typical phone photo is 3-5 MB. After resizing:

| Variant | Dimensions | File Size | Reduction |
|---------|-----------|-----------|-----------|
| ThumbHash | ~32x24 encoded | **25 bytes** | 99.999% |
| WebP thumbnail (q75) | 200px | **3-8 KB** | 99.8% |
| WebP standard (q80) | 800px | **25-60 KB** | 98-99% |
| WebP large (q80) | 1200px | **50-120 KB** | 97-98% |

At 50k users browsing profiles, this is the difference between 100 GB/mo and 10 GB/mo of transfer.

#### Recommended crate stack

```toml
image = { version = "0.25", default-features = false, features = ["jpeg", "png", "webp"] }
fast_image_resize = "5"   # 10-40x faster than `image` crate resize, matches libvips with AVX2
webp = "0.3"              # Lossy WebP encoding via libwebp (pure-Rust encoder only does lossless)
thumbhash = "0.1"         # 25-byte image placeholders, sub-ms to compute
```

#### Resize benchmarks (AMD Ryzen 9, 4928x3279 → 852x567)

| Library | Bilinear | Lanczos3 |
|---------|----------|----------|
| `image` crate | 83.28 ms | 189.93 ms |
| libvips (C) | 5.66 ms | 15.78 ms |
| fast_image_resize (AVX2) | **3.67 ms** | **13.21 ms** |

`fast_image_resize` matches or beats libvips with zero C dependencies.

#### Memory safety in 512 MB container

Decoded 12MP photo: ~36 MB (RGB8). Peak per image: ~45-50 MB. With 512 MB limit and ~80 MB baseline:

- Safe concurrent processing: 6-8 images
- **Recommended:** `tokio::sync::Semaphore` with 3-4 permits
- Set `image::Limits { max_image_width: Some(8000), max_image_height: Some(8000), max_alloc: Some(200 * 1024 * 1024) }`
- Reject uploads > 10 MB at the Axum layer before decoding

#### Architecture

```
Upload request → Axum multipart handler (async)
  → spawn_blocking (with semaphore permit)
    → Decode with image crate (with Limits)
    → Generate ThumbHash (~100px pre-resize)
    → Resize to 200px thumbnail (fast_image_resize, Bilinear)
    → Resize to 800px standard (fast_image_resize, Bilinear)
    → Encode each to WebP lossy (webp crate, q75/q80)
  → Upload variants to Garage S3 via opendal (async)
  → Save metadata + thumbhash (25 bytes BYTEA) to DB
  → Return JSON with URLs + inline thumbhash
```

Total processing: ~50-100 ms per image. No background job queue needed.

---

### 4.3 HTTP Caching Headers

**Saves: 30-50% of repeat API requests, reduces DB load**

Currently no caching headers on any API response. The mobile client already has a 5-minute staleness policy — the server should cooperate.

| Endpoint | Recommended Header | Rationale |
|----------|-------------------|-----------|
| `GET /tags`, `GET /degrees` | `Cache-Control: public, max-age=1800` | Essentially static catalog data |
| `GET /profiles/{id}` | `Cache-Control: private, max-age=60` | Changes rarely |
| `GET /events` (list) | `Cache-Control: private, max-age=60` | Moderate freshness |
| `GET /matching/*` | `Cache-Control: private, max-age=300` | Expensive to compute |
| Image uploads (Caddy) | `Cache-Control: public, max-age=31536000, immutable` | UUID filenames = content-addressed |

Add `ETag` headers based on `updated_at` timestamps for conditional requests (304 Not Modified).

The `immutable` directive tells browsers not to revalidate even on hard reload — perfect for UUID-named images that never change.

---

### 4.4 Precompute Matching Scores

**Saves: O(1) reads instead of O(n) queries per request**

`matching.rs:178` currently loads 200 profiles from the DB, loads all their tags, scores them in Rust, and sorts — on every single request. At 50k users, each request triggers 4+ queries.

#### Proposed approach

1. Add a `profile_recommendations` table:
```sql
CREATE TABLE profile_recommendations (
    profile_id UUID REFERENCES profiles(id) ON DELETE CASCADE,
    recommended_id UUID REFERENCES profiles(id) ON DELETE CASCADE,
    score REAL NOT NULL,
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (profile_id, recommended_id)
);
CREATE INDEX idx_rec_profile_score ON profile_recommendations (profile_id, score DESC);
```

2. Background task (run hourly or on profile/tag update):
   - For each profile, compute top-50 recommendations
   - Upsert into `profile_recommendations`

3. Matching endpoint becomes:
```sql
SELECT r.*, p.* FROM profile_recommendations r
JOIN profiles p ON p.id = r.recommended_id
WHERE r.profile_id = $1
ORDER BY r.score DESC
LIMIT 10;
```

Single indexed read. Sub-millisecond.

4. Same approach for event recommendations (precompute per-user event scores hourly).

---

### 4.5 Add a CDN Layer for Images

**Saves: 80%+ origin bandwidth for images**

Currently Caddy serves images from Garage with `Cache-Control: private, no-store`. Every image view hits the origin.

#### CDN options comparison

| Option | Monthly Cost (MVP) | Pros | Cons |
|--------|-------------------|------|------|
| **BunnyCDN + Garage** | $1/mo min | No ToS issues, simple setup, cheapest | Not free |
| **Cloudflare R2** (replace Garage) | $0 (10 GB free) | Zero egress, S3-compatible, CDN built-in | Vendor lock-in, 10 GB limit |
| **Cloudflare CDN + Garage** | $0 | Unlimited bandwidth | ToS risk for image-heavy external origin |
| **OVH Object Storage** | ~$8/TB, zero egress | Native OVH integration | No edge CDN, all traffic from origin DC |

**Best MVP path:** **BunnyCDN ($1/mo) + keep Garage**. Dead simple, no ToS risk, $0.01/GB in EU.

For images with UUID filenames, set on the S3/Garage side:
```
Cache-Control: public, max-age=31536000, immutable
```

This achieves 99%+ cache hit ratio at the CDN edge.

**Alternative:** If you want to eliminate Garage entirely, **Cloudflare R2** is S3-compatible (works with OpenDAL), has a 10 GB free tier with zero egress, and automatically serves through Cloudflare's CDN.

---

### 4.6 Move OTP State to PostgreSQL

**Enables: horizontal scaling of API instances**

Current in-memory OTP store (`state.rs`, LazyLock Mutex, 5000 entry cap) blocks running multiple API instances.

```sql
CREATE TABLE otp_codes (
    email TEXT PRIMARY KEY,
    code TEXT NOT NULL,
    attempts SMALLINT NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ NOT NULL,
    last_sent_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Auto-cleanup via PostgreSQL (or application-level on access)
CREATE INDEX idx_otp_expires ON otp_codes (expires_at);
```

Cost: zero (already have PostgreSQL). Enables running 2+ API instances behind a load balancer when needed.

---

### 4.7 Switch to UUIDv7 Primary Keys

**Improves: write performance, index efficiency**

UUIDv4 (random) causes catastrophic B-tree index fragmentation: 5,000-10,000+ page splits per million inserts vs 10-20 for sequential IDs.

UUIDv7 (time-ordered) has a monotonically increasing prefix — inserts append to the B-tree end. Behaves like BIGSERIAL for index performance while remaining globally unique.

```toml
# Cargo.toml
uuid = { version = "1", features = ["v7"] }
```

```rust
// Replace Uuid::new_v4() with:
uuid::Uuid::now_v7()
```

PostgreSQL 18 (which you're already using) has native `uuidv7()` generation. For application-generated UUIDs, the Rust `uuid` crate's `v7` feature is sufficient.

---

## 5. PostgreSQL Tuning for Small Servers

### By server size

#### 2 GB RAM (VPS-1 shared with other services)

```ini
shared_buffers = 256MB
effective_cache_size = 512MB
work_mem = 4MB
maintenance_work_mem = 64MB
random_page_cost = 1.1
effective_io_concurrency = 200
```

#### 4 GB available to PostgreSQL

```ini
shared_buffers = 1GB
effective_cache_size = 3GB
work_mem = 16MB               # Careful: multiplied by sorts x connections
maintenance_work_mem = 256MB
wal_buffers = 16MB
random_page_cost = 1.1        # SSD (default 4.0 assumes HDD)
effective_io_concurrency = 200 # SSD parallel IO
```

### Connection pooling

SeaORM's built-in sqlx pool is sufficient for a single Loco backend. No PgBouncer needed until horizontal scaling.

```rust
opt.max_connections(20)
   .min_connections(5)
   .connect_timeout(Duration::from_secs(8))
   .idle_timeout(Duration::from_secs(300))
   .max_lifetime(Duration::from_secs(1800))
   .sqlx_logging(false);         // Disable SQL logging in production
```

### Autovacuum tuning

```ini
autovacuum_vacuum_scale_factor = 0.1    # Vacuum at 10% dead rows (default 0.2)
autovacuum_analyze_scale_factor = 0.05  # Analyze at 5% changed (default 0.1)
autovacuum_vacuum_cost_limit = 800      # Higher for SSD (default 200)
```

### Essential indexes to add

```sql
-- Foreign key indexes (SeaORM doesn't auto-create these)
CREATE INDEX IF NOT EXISTS idx_profiles_user_id ON profiles (user_id);
CREATE INDEX IF NOT EXISTS idx_events_creator_id ON events (creator_id);
CREATE INDEX IF NOT EXISTS idx_uploads_owner_id ON uploads (owner_id);
CREATE INDEX IF NOT EXISTS idx_event_attendees_event ON event_attendees (event_id);
CREATE INDEX IF NOT EXISTS idx_event_attendees_profile ON event_attendees (profile_id);
CREATE INDEX IF NOT EXISTS idx_profile_tags_profile ON profile_tags (profile_id);
CREATE INDEX IF NOT EXISTS idx_profile_tags_tag ON profile_tags (tag_id);
CREATE INDEX IF NOT EXISTS idx_event_tags_event ON event_tags (event_id);
CREATE INDEX IF NOT EXISTS idx_event_tags_tag ON event_tags (tag_id);

-- Common query patterns
CREATE INDEX IF NOT EXISTS idx_events_starts_at ON events (starts_at DESC);
CREATE INDEX IF NOT EXISTS idx_uploads_filename ON uploads (filename) WHERE NOT deleted;
```

---

## 6. Garage S3 Production Configuration

### Recommended config (single-node MVP)

```toml
metadata_dir = "/var/lib/garage/meta"
data_dir = "/var/lib/garage/data"
db_engine = "lmdb"                          # 2x faster than SQLite
metadata_auto_snapshot_interval = "6h"       # Recovery from LMDB corruption
replication_factor = 1                       # Single node; change to 3 with cluster
consistency_mode = "consistent"
compression_level = 1                        # Zstd level 1: good ratio, low CPU
block_ram_buffer_max = "256MiB"
```

### Garage vs MinIO

| Metric | Garage | MinIO |
|--------|--------|-------|
| Idle RAM | 3-20 MiB | ~2 GB+ |
| Production RAM | 1-2 GB | 4-32 GB |
| Can run on 1 GB VPS | Yes | No |

Garage is 100x more memory-efficient at idle. Correct choice for small VPS.

### Filesystem

Use **XFS** for the data directory (best performance, no inode limits). Avoid EXT4 for data dir due to inode limitations at scale.

---

## 7. Matrix/Tuwunel Scaling Reality

### Why Tuwunel is the right choice

| Metric | Synapse (Python) | Dendrite (Go) | Tuwunel (Rust) |
|--------|-----------------|---------------|----------------|
| Romeo & Juliet benchmark | 1m 46s | 2m 45s | **4.2 seconds** |
| Min RAM (small) | 350 MB - 1 GB | 256-512 MB | **70-150 MB** |
| RAM at moderate scale | 4-16 GB | 2-4 GB | **1-2 GB** |
| External DB required | PostgreSQL | PostgreSQL/SQLite | None (embedded RocksDB) |

Tuwunel is the most efficient Matrix homeserver available — deployed nationally in Switzerland, single binary, no external database.

### What actually drives Matrix memory usage

Matrix RAM scales with **room state complexity**, not just user count. The worst-case numbers you'll find online come from users joining massive federated rooms (1,000+ members). That's not our use case.

**Poziomki's chat pattern is lightweight:**
- Federation is **disabled**
- Users are in small DMs and event group chats (2-20 people)
- No one joins 500-member public rooms
- Room state stays tiny

For this pattern, 512 MB should comfortably serve thousands of users. No published benchmarks exist for exactly this scenario, so **monitor real usage as you grow** rather than planning around worst-case extrapolations.

### Tuwunel optimization tips

- Reduce `cache_capacity_modifier` (trades speed for memory)
- Configure RocksDB memory budgets explicitly
- Keep `TUWUNEL_ALLOW_FEDERATION: false` (eliminates the biggest resource drain)
- Limit maximum room size if possible
- Bump the container RAM limit as needed — measure the actual growth curve

---

## 8. Summary: Impact Table

| Change | Effort | RAM Saved | Bandwidth Saved | Scaling Impact |
|--------|--------|-----------|----------------|----------------|
| Drop Meilisearch → PG FTS | Medium | **512 MB** | — | -1 service, simpler ops |
| Image resizing on upload | Small-Med | — | **70-90%** | Huge at scale |
| Cache-Control headers | Small | — | **30-50%** | Reduces DB load |
| Precompute matching scores | Medium | — | — | O(1) vs O(n) per request |
| CDN for images (BunnyCDN) | Small | — | **80%+ origin** | $1/mo, massive savings |
| OTP to PostgreSQL | Small | Marginal | — | Enables multi-instance |
| UUIDv7 primary keys | Small | — | — | Better write performance |
| PostgreSQL tuning | Small | — | — | 2-5x query improvement on SSD |

### Priority order (highest ROI first)

1. **Image resizing** — biggest bandwidth savings, small effort
2. **Cache-Control headers** — trivial to add, immediate impact
3. **CDN for images** — $1/mo, offloads most traffic from origin
4. **Replace Meilisearch** — free up 512 MB, simplify architecture
5. **PostgreSQL tuning** — tune `random_page_cost`, `effective_io_concurrency` for SSD
6. **UUIDv7 migration** — better index performance going forward
7. **Precompute matching** — important when matching queries become slow
8. **OTP to PostgreSQL** — important when you need horizontal scaling

---

## 9. Sources

### Hosting & Pricing
- [OVHcloud VPS 2026](https://www.ovhcloud.com/en/vps/)
- [OVHcloud Public Cloud Prices](https://us.ovhcloud.com/public-cloud/prices/)
- [OVHcloud Advance Bare Metal](https://us.ovhcloud.com/bare-metal/advance/)
- [OVHcloud Object Storage](https://us.ovhcloud.com/public-cloud/object-storage/)
- [OVH No More Egress Fees](https://lowendtalk.com/discussion/213725/ovh-no-more-egress-fees-on-object-storage)
- [Hetzner Cloud](https://www.hetzner.com/cloud/)
- [Hetzner Dedicated Servers](https://www.hetzner.com/dedicated-rootserver/)
- [Hetzner Object Storage](https://www.hetzner.com/storage/object-storage/)

### PostgreSQL FTS & Search
- [Supabase: Postgres Full Text Search vs the Rest](https://supabase.com/blog/postgres-full-text-search-vs-the-rest)
- [ParadeDB: Elasticsearch vs Postgres](https://www.paradedb.com/blog/elasticsearch_vs_postgres)
- [VectorChord: PostgreSQL BM25 FTS — Debunking the Slow Myth](https://blog.vectorchord.ai/postgresql-full-text-search-fast-when-done-right-debunking-the-slow-myth)
- [pganalyze: Understanding GIN Indexes](https://pganalyze.com/blog/gin-index)
- [PostIndustria: PostgreSQL Geo Queries Made Easy](https://postindustria.com/postgresql-geo-queries-made-easy/)
- [Meilisearch Storage Issue #4211](https://github.com/meilisearch/meilisearch/issues/4211)
- [We Replaced Elasticsearch with Postgres FTS — 800ms to 99ms](https://medium.com/@maahisoft20/we-replaced-elasticsearch-with-postgres-full-text-search-query-times-went-from-800ms-to-99ms-dbed2db5bd04)

### Image Processing
- [fast_image_resize benchmarks](https://github.com/Cykooz/fast_image_resize/blob/main/benchmarks-x86_64.md)
- [ThumbHash](https://evanw.github.io/thumbhash/)
- [WebP Compression Study — Google](https://developers.google.com/speed/webp/docs/webp_study)
- [image crate Limits struct](https://docs.rs/image/latest/image/struct.Limits.html)

### CDN
- [Cloudflare R2 Pricing](https://developers.cloudflare.com/r2/pricing/)
- [BunnyCDN Pricing](https://bunny.net/pricing/)
- [Cloudflare ToS Update](https://blog.cloudflare.com/updated-tos/)

### Matrix / Chat
- [Tuwunel GitHub](https://github.com/matrix-construct/tuwunel)
- [Element: Scaling to Millions Requires Synapse Pro](https://element.io/blog/scaling-to-millions-of-users-requires-synapse-pro/)
- [Matrix.org: Understanding Synapse Hosting](https://matrix.org/docs/older/understanding-synapse-hosting/)

### Garage S3
- [MinIO vs Garage vs SeaweedFS 2025](https://onidel.com/blog/minio-ceph-seaweedfs-garage-2025)
- [Garage Configuration Reference](https://garagehq.deuxfleurs.fr/documentation/reference-manual/configuration/)
- [Garage Benchmarks](https://garagehq.deuxfleurs.fr/documentation/design/benchmarks/)

### PostgreSQL Tuning
- [PostgreSQL Memory Tuning — EDB](https://www.enterprisedb.com/postgres-tutorials/how-tune-postgresql-memory)
- [Avoid UUID v4 Primary Keys](https://andyatkinson.com/avoid-uuid-version-4-primary-keys)
- [UUID v7 in PostgreSQL 18](https://betterstack.com/community/guides/databases/postgresql-18-uuid/)
- [Autovacuum Tuning Basics — EDB](https://www.enterprisedb.com/blog/autovacuum-tuning-basics)
- [SeaORM Connection Configuration](https://www.sea-ql.org/SeaORM/docs/install-and-config/connection/)
