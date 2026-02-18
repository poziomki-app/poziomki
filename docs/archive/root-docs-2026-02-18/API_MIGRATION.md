# API Migration Plan (Elysia -> Rust)

## Goal
Migrate `poziomki/apps/api` to `poziomki-rs/backend` in incremental phases.

Primary constraints:
- Keep `/api/v1` contracts stable for **non-chat** endpoints during migration.
- Do **not** keep compatibility for legacy chat HTTP/WS APIs.
- Move chat to full Matrix-native implementation aligned with Element architecture.

Related document:
- Mobile chat port plan and source-to-target mapping: `CHAT_PORT_MAP.md`

## Source Inventory (Current Elysia API)
Mapped from:
- `poziomki/apps/api/src/app.ts`
- `poziomki/apps/api/src/features/**/index.ts`
- `poziomki/apps/api/src/features/**/model.ts`
- `poziomki/apps/api/src/features/chats/websocket/*`
- `poziomki/apps/api/src/plugins/{auth-guard,error-handler,rate-limit}.ts`

## Cross-Cutting Contract (Preserve in Phase 1-3)
Applies to platform/auth/profile/events/tags/degrees/matching/uploads:
- Base API prefix: `/api/v1`
- Health: `GET /health`
- Root info: `GET /`
- Success envelope: mostly `{ data: ... }` or `{ success: true }`
- Error envelope: `{ error, code, requestId, details? }`
- Auth style: Bearer session token (and Better Auth cookies for auth routes)
- Session refresh behavior: token lookup refreshes session expiry after update-age threshold
- Rate limit:
  - Auth routes: 5/min
  - General routes: 100/min
- Max body size: 10MB

## Endpoint Map (Compatibility Target for Non-Chat)

### Platform / Infra
| Method | Path | Auth | Current Behavior | Rust Target |
|---|---|---|---|---|
| GET | `/health` | No | `{ status: "ok" }` | Same |
| GET | `/` | No | API info (version/docs) | Same |
| GET | `/api/docs` | Dev-only | Scalar/OpenAPI UI | Same in non-prod |
| GET | `/api/docs/json` | Dev-only | OpenAPI JSON | Same in non-prod |

### Auth (`/api/v1/auth`)
| Method | Path | Auth | Request | Response | Rust Target |
|---|---|---|---|---|---|
| GET | `/get-session` | Optional | - | `{ session, user }` (not wrapped in `data`) | Same |
| POST | `/sign-up/email` | No | `{ email, name, password }` | `{ data: { user, token, ... } }` + cookies | Same |
| POST | `/sign-in/email` | No | `{ email, password, rememberMe? }` | `{ data: { user, token, ... } }` + cookies | Same |
| POST | `/verify-otp` | No | `{ email, otp }` | `{ data: {...} }` | Same |
| POST | `/resend-otp` | No | `{ email }` | `{ success: true }` | Same |
| POST | `/email-otp/verify-email` | No | `{ email, otp }` | `{ data: {...} }` | Same (compat alias) |
| POST | `/email-otp/send-verification-otp` | No | `{ email }` | `{ success: true }` | Same (compat alias) |
| POST | `/sign-out` | Optional | - | `{ success: true }` | Same |
| GET | `/sessions` | Yes | - | `{ data: Session[] }` | Same |
| DELETE | `/account` | Yes | `{ password }` | `{ success: true }` | Same |
| GET | `/export` | Yes | - | `{ data: UserDataExport }` | Same |

Auth details to preserve:
- Email domain restriction on sign-up.
- Sign-in should not leak account existence.
- Sign-out is idempotent (returns success without active session).
- OTP bypass in non-prod/test (`OTP_BYPASS_CODE`) remains test-only.

### Profiles (`/api/v1/profiles`)
| Method | Path | Auth | Request | Response | Rust Target |
|---|---|---|---|---|---|
| GET | `/me` | Yes | - | `{ data: FullProfile | null }` | Same |
| GET | `/:id` | Yes | - | `{ data: Profile }` | Same |
| GET | `/:id/full` | Yes | - | `{ data: FullProfile }` | Same |
| POST | `/` | Yes | `CreateProfileBody` | `{ data: FullProfile }` | Same |
| PATCH | `/:id` | Yes (owner) | `UpdateProfileBody` | `{ data: FullProfile }` | Same |
| DELETE | `/:id` | Yes (owner) | - | `{ success: true }` | Same |

### Degrees (`/api/v1/degrees`)
| Method | Path | Auth | Request | Response | Rust Target |
|---|---|---|---|---|---|
| GET | `/` | Optional | `search?, limit?` | `{ data: Degree[] }` | Same |

### Tags (`/api/v1/tags`)
| Method | Path | Auth | Request | Response | Rust Target |
|---|---|---|---|---|---|
| GET | `/` | Optional | `scope, search?, limit?` | `{ data: Tag[] }` | Same |
| POST | `/` | Yes | `CreateTagBody` | `{ data: Tag }` | Same |

### Events (`/api/v1/events`)
| Method | Path | Auth | Request | Response | Rust Target |
|---|---|---|---|---|---|
| GET | `/` | Yes + profile required | `limit?` | `{ data: Event[] }` | Same |
| GET | `/mine` | Yes + profile required | - | `{ data: Event[] }` | Same |
| GET | `/:id` | Yes + profile required | - | `{ data: Event }` | Same |
| GET | `/:id/attendees` | Yes + profile required | - | `{ data: Attendee[] }` | Same |
| POST | `/` | Yes + profile required | `CreateEventBody` | `{ data: Event }` | Same |
| PATCH | `/:id` | Yes + creator only | `UpdateEventBody` | `{ data: Event }` | Same |
| DELETE | `/:id` | Yes + creator only | - | `{ success: true }` | Same |
| POST | `/:id/attend` | Yes + profile required | `{ status? }` | `{ data: Event }` | Same |
| DELETE | `/:id/attend` | Yes + profile required | - | `{ data: Event }` | Same |

Event details to preserve:
- Enforce `endsAt > startsAt`.
- Event creator cannot `leave` event.
- Keep `conversationId` field, but it now stores Matrix `room_id` semantics.

### Matching (`/api/v1/matching`)
| Method | Path | Auth | Request | Response | Rust Target |
|---|---|---|---|---|---|
| GET | `/profiles` | Yes | `limit?` | `{ data: ProfileRecommendation[] }` | Same (placeholder algorithm first) |

### Uploads (`/api/v1/uploads`)
| Method | Path | Auth | Request | Response | Rust Target |
|---|---|---|---|---|---|
| GET | `/auth-check` | Yes + profile | `x-original-uri` header | `{ ok: true }` | Same |
| GET | `/:filename` | Mixed | - | Dev: file bytes, Prod: `{ url }` | Same |
| POST | `/` | Yes | multipart: `file`, `context?`, `contextId?` | `{ url, filename, size, type }` | Same |
| DELETE | `/:filename` | Yes | - | `{ success: true }` | Same |

Upload details to preserve:
- 10MB limit.
- MIME + magic-byte validation.
- Context rules (`chat_*` requires `contextId`).
- Storage backend: Garage S3 (S3-compatible) for upload object storage.
- Use presigned URLs for prod download/upload flows, keep API response contract unchanged.

## Legacy Chat API Inventory (Do Not Port)
These endpoints are mapped for migration completeness, but should not be implemented in Rust as compatibility facades.

### Legacy chats HTTP (`/api/v1/chats`)
| Method | Path | Existing Purpose | Matrix Equivalent | Rust Migration Decision |
|---|---|---|---|---|
| GET | `/` | List conversations | Room summaries from sync/room list | Not ported |
| GET | `/:id` | Conversation details | Room state + member list | Not ported |
| POST | `/personal` | Create/find DM | `createRoom` with DM semantics | Not ported |
| POST | `/group` | Create group chat | `createRoom` + invites | Not ported |
| POST | `/event` | Create/join event chat | Create/join event room | Not ported |
| DELETE | `/:id/leave` | Leave conversation | Room leave | Not ported |
| POST | `/:id/participants` | Add members | Room invite | Not ported |
| DELETE | `/:id/participants/:profileId` | Remove member | Kick/ban/unban via power levels | Not ported |
| GET | `/:id/messages` | Fetch messages | Timeline pagination | Not ported |
| POST | `/:id/messages` | Send message | `m.room.message` send | Not ported |
| PATCH | `/messages/:messageId` | Edit message | `m.replace` relation | Not ported |
| DELETE | `/messages/:messageId` | Delete message | Redaction | Not ported |
| POST | `/:id/read` | Read receipt | Read marker/receipt endpoints | Not ported |
| POST | `/messages/:messageId/reactions` | Add reaction | `m.reaction` | Not ported |
| DELETE | `/messages/:messageId/reactions/:emoji` | Remove reaction | Redact reaction event | Not ported |
| GET | `/messages/:messageId/reactions/:emoji/users` | Reaction users | Aggregations / event relations | Not ported |

### Legacy chat websocket (`/ws/chat`)
| Legacy Behavior | Matrix Equivalent | Rust Migration Decision |
|---|---|---|
| Tokenized socket connect (`/ws/chat?token=...`) | Matrix auth + sliding sync/sync | Not ported |
| `send_message`, `typing`, `mark_read`, reactions, subscribe/unsubscribe | Native Matrix room/timeline APIs | Not ported |
| Server-emitted custom events (`new_message`, `typing`, `read_receipt`, etc.) | Native Matrix timeline and ephemeral events | Not ported |

Cutover rule:
- Existing legacy chat endpoints should return `410 Gone` with migration message once Matrix path is enabled.

## Matrix-Native Chat Target (Element-like)
Chat capabilities move to Matrix APIs and SDK behavior, matching Element patterns from `CHAT_PORT_MAP.md`.

### Functional mapping
- Conversation list: Matrix room list + sync summaries.
- Open conversation: joined room timeline.
- Create DM/group/event chat: Matrix room creation and invite flows.
- Send/edit/delete: `m.room.message`, relations (`m.replace`), redaction.
- Reactions: `m.reaction` + relation aggregation.
- Read receipts: Matrix read markers/receipts.
- Typing: Matrix typing notifications.
- Timeline pagination and focus modes: same conceptual model as Element timeline controllers.

### Data mapping
- `conversationId` in domain models -> Matrix `room_id`.
- `profile.id` -> Matrix `user_id` mapping (stable table/service).
- Event chat is one Matrix room per `eventId`.

### Backend role for chat
Rust backend should not proxy message/timeline APIs. It may provide:
- Matrix provisioning/bootstrap endpoints (if required by auth model).
- Domain mapping persistence (`eventId <-> room_id`, `profileId <-> user_id`).
- Authorization/business hooks around event-to-room lifecycle.

Recommended backend endpoint namespace (keep versioning consistent):
- `/api/v1/matrix/config` (homeserver/discovery metadata for clients)
- `/api/v1/matrix/session` (token/bootstrap handshake, if app auth requires it)
- `/api/v1/matrix/events/:eventId/room` (event-to-room mapping lookup/provisioning)

### Design alignment requirement
- Chat UI should follow Element interaction and visual patterns (room list, timeline, composer, action list), as mapped in `CHAT_PORT_MAP.md`.
- API/backend work is not considered complete for chat migration until Matrix chat UX is validated against those Element references.

## Kotlin Compatibility Priority
Must be first-class in early phases:
- Auth: sign-up/sign-in/verify/resend/sign-out
- Profiles CRUD + `/profiles/me`
- Events CRUD + attend/leave + attendees
- Uploads POST
- Tags GET
- Degrees GET
- Matching GET

Chat migration requirement:
- Kotlin app chat features must migrate to Matrix SDK surface (per `CHAT_PORT_MAP.md`), not `ApiService` chat endpoints.

Important mismatch to preserve/normalize carefully:
- Kotlin models currently use `tagIds` for profile/event writes, while Elysia expects `tags` arrays.
- Support both during compatibility phase (`tagIds` alias -> normalize to `tags`).

## Incremental Delivery Plan

### Phase 0 - Contract Freeze and Fixtures
- Freeze Elysia contract for non-chat endpoints.
- Freeze legacy chat inventory as migration reference only (no compatibility implementation).
- Add request/response fixtures for high-traffic non-chat endpoints.

Exit criteria:
- Contract checklist approved.
- Non-chat compatibility test matrix complete.
- Chat cutover policy (`410 Gone` + Matrix migration path) approved.

### Phase 1 - Rust API Skeleton with Shared Contracts
- Add `/health`, `/`, `/api/v1/*` scaffolding for non-chat APIs.
- Implement shared middleware: request ID, error envelope, auth context extraction, rate limits.
- Keep envelope behavior exactly (`data` / `success` / auth special cases).

Exit criteria:
- Route surface exists in Rust.
- Contract tests pass for error shapes/status codes on unimplemented non-chat handlers.

### Phase 2 - Auth + Profiles + Tags + Degrees
- Implement auth endpoints with current bearer/cookie semantics.
- Implement profile/tag/degree endpoints and validations.
- Add compatibility aliases (`tagIds` and `tags`) where needed.

Exit criteria:
- Kotlin onboarding/auth/profile flows run against Rust backend.

### Phase 3 - Events + Matching + Uploads
- Implement events, attendance, and upload context access rules.
- Keep current matching logic first (newest profiles placeholder) for parity.
- Preserve upload validation and signed URL behavior.
- Integrate uploads with Garage S3 (bucket, object key strategy, presign, delete, access checks).

Exit criteria:
- Kotlin event and upload flows run against Rust backend.
- Upload objects are stored in Garage S3 in staging with successful upload/download/delete parity tests.

### Phase 4 - Matrix Chat Integration (No Legacy Chat API)
- Implement Matrix provisioning and domain mappings needed by product flows.
- Migrate mobile chat stack to Matrix SDK abstractions from `CHAT_PORT_MAP.md`.
- Implement Element-aligned chat UI/UX patterns from `CHAT_PORT_MAP.md` design references.
- Disable legacy chat endpoints in Rust (`/api/v1/chats`, `/ws/chat`) with `410 Gone`.

Exit criteria:
- Chat UX works end-to-end via Matrix.
- Chat UI reaches MVP design parity checks defined in `CHAT_PORT_MAP.md`.
- No production client depends on legacy chat HTTP/WS protocol.

### Phase 5 - Dual-Run, Cutover, Hardening
- Run Elysia and Rust in staged environments for non-chat parity checks.
- Move Kotlin app base URL to Rust for non-chat APIs.
- Complete Matrix chat rollout and monitor regressions.

Exit criteria:
- Error rate and latency within target.
- No blocking regressions in non-chat APIs or Matrix chat flows.

### Phase 6 - Cleanup and v1 Hardening
- Remove legacy chat code paths from old backend.
- Keep all active endpoints under `/api/v1`.
- Normalize API inconsistencies listed below with explicit deprecation windows.

## Known API Weirdness (Preserve for Non-Chat Compatibility)
1. Inconsistent envelopes:
- `/auth/get-session` is not wrapped in `{ data: ... }`.
- `GET /uploads/:filename` returns file bytes in dev but `{ url }` in prod.

2. Auth coupling complexity:
- Better Auth cookie flows + bearer session token coexist.

3. DTO inconsistencies:
- `tags` vs `tagIds` mismatch between backend schema and Kotlin models.

4. Legacy chat coupling (to remove):
- Custom chat semantics tied to local DB IDs and custom websocket protocol.

## Suggested Improvements (After Cutover, Keep `/api/v1`)
1. Keep legacy envelope behavior for existing routes, but require `{ data: ... }` for all new `/api/v1` endpoints.
2. Introduce explicit deprecation headers for legacy shapes/aliases:
- `Deprecation: true`
- `Sunset: <rfc-1123-date>`
- `Link: <migration-doc-url>; rel=\"deprecation\"`
3. Normalize `tags` payload naming to one canonical input (`tagIds`), while still accepting `tags` alias during deprecation window.
4. Make upload download behavior consistent across environments by converging on one mode (redirect or proxy), controlled by feature flag during rollout.
5. Keep S3 implementation vendor-neutral (AWS SDK-compatible API surface) while targeting Garage S3 in infra.
6. Publish a stable error code catalog (domain + reason), and guarantee machine-readable error bodies for every non-2xx response.
7. Add idempotency-key support for critical create/mutate endpoints (`sign-up`, event create, room provisioning) to protect clients from retries.
8. Keep chat transport exclusively Matrix-native; no reintroduction of custom chat WS protocol.

## API Quality Guardrails (Recommended)
1. Pagination: prefer opaque cursor tokens over mixed `before` semantics for new list endpoints in `/api/v1`.
2. Observability: include `requestId` on every response and propagate it through logs/traces.
3. Auth consistency: document and enforce one canonical bearer token flow for app APIs; keep cookies only where browser auth requires them.
4. Schema discipline: generate OpenAPI from source and run contract-diff checks in CI to detect accidental breaking changes.
5. Time semantics: use ISO-8601 UTC timestamps everywhere and document timezone handling explicitly.

## Test Strategy for Migration
- Reuse integration suite semantics from `poziomki/apps/api/tests/integration/*` for non-chat endpoints.
- Add Rust contract tests for mapped non-chat endpoints:
  - status code parity
  - response body shape parity
  - error code parity
- Add Matrix integration tests for chat flows:
  - create DM/group/event room
  - send/edit/redact/react/read/typing
  - timeline pagination
- Add deprecation tests that legacy chat routes return `410`.
- Add Kotlin smoke tests for end-to-end user flows.

## Immediate Next Tasks
1. Generate machine-readable non-chat contract fixtures from Elysia routes/models.
2. Implement Rust middleware stack for requestId/error/auth/rate-limit parity.
3. Define Matrix bootstrap/provisioning contract needed by app auth model.
4. Finalize Garage S3 upload design (bucket naming, presign TTLs, ACL model, local/dev fallback).
5. Start Phase 2 with Auth + Profiles as first production slice.
