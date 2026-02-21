# Matrix Implementation Mismatch Report (Merged)

> Status (2026-02-21): Historical research note. Some findings were superseded by later validation. Use `docs/chat/CHAT_IMPLEMENTATION_CHECKLIST.md` as the implementation authority and `docs/chat/README.md` for source precedence.

**Date:** 2026-02-20
**Scope:** `@poziomki-rs` Matrix implementation audit (mobile + backend + infra)
**Goal:** Identify where implementation diverges from Matrix SDK/spec and likely causes for message/avatar failures.

---

## Architecture Overview

The system has two Matrix-touching layers:

1. **Backend (Rust/Loco)** — Only bootstraps Matrix sessions via raw HTTP `reqwest` calls. Does NOT use matrix-rust-sdk. Handles: login/register, avatar upload, display name sync.
2. **Mobile (Kotlin)** — Uses the official `matrix-rust-components-kotlin` SDK (v26.2.6) for all real-time chat: sync, timelines, rooms, E2EE, send queues. This is the correct approach.

---

## CRITICAL FINDINGS

### 1. Wrong identifiers used for room invites (MESSAGES ROOT CAUSE)

`EventRepository.ensureEventRoom` builds invite list from attendee `userId` (app UUID). Matrix `createRoom.invite` requires MXIDs (`@user:server`), not app UUIDs.

`RustMatrixClient.normalizeUserId` only prepends `@` and appends `:domain` — it does **not** map app UUID to Matrix localpart (which is `poziomki_<alphanumeric_uuid>`).

**Result:** Invites target invalid/nonexistent Matrix users. Recipients never receive room membership, never see messages.

**This is likely the #1 reason messages are not received.**

Relevant files:
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/repository/EventRepository.kt`
- `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/RustMatrixClient.kt`
- `backend/src/controllers/migration_api/matrix_support.rs` (localpart derivation at line 347)

### 2. Legacy media upload endpoint vs authenticated media (AVATAR ROOT CAUSE)

`matrix_support.rs:248-251`:
```rust
let url = matrix_endpoint(
    homeserver,
    &format!("/_matrix/media/v3/upload?filename={filename}"),
);
```

This uses `/_matrix/media/v3/upload` — the **legacy unauthenticated** media endpoint. Tuwunel's default is `allow_legacy_media = false`. Should use the Matrix v1.11 authenticated endpoint: `/_matrix/client/v1/media/upload`.

The SDK's `getMediaThumbnail()` calls the authenticated media API for downloads. Upload/download endpoint mismatch may cause avatars to be stored but unresolvable.

Relevant files:
- `backend/src/controllers/migration_api/matrix_support.rs` (lines 240-265)

### 3. Avatar precedence masks valid Matrix avatars (AVATAR ROOT CAUSE)

UI prefers backend `profilePictureUrl` (Garage S3) over Matrix room/user avatar (`mxc://`). If the Garage URL is stale, forbidden, expired, or CORS-blocked, the Matrix avatar is **not used as fallback**. Same issue in event-chat sender avatar override path.

**Correct pattern:** Matrix avatar as default source in chat surfaces; Garage only as explicit override when required.

Relevant files:
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/MessagesScreen.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/MessageEventRow.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/UserAvatar.kt`

---

## HIGH PRIORITY FINDINGS

### 4. E2EE key distribution failure from device ID handling

Every call to `POST /api/v1/matrix/session` can create a **new device** on the homeserver. New devices don't have room encryption keys — messages appear as "Encrypted message" or invisible.

**Device ID normalization mismatch:**
- Mobile generates `POZ<random-uuid>` device IDs, saved to `device_id.txt`
- Backend `normalize_device_id` (matrix_support.rs:329-344) **uppercases** and strips characters
- Mobile stores the original pre-normalization value
- The SDK crypto store is keyed by device ID — if there's a mismatch, E2EE breaks silently

Relevant files:
- `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/RustMatrixClient.kt` (lines 119-120, 165-167, 578-584)
- `backend/src/controllers/migration_api/matrix_support.rs` (lines 329-344, 368-386)

### 5. Invitees are not guaranteed to join rooms

Client path is effectively joined-only. No reliable invite acceptance/auto-join flow found. If invite exists but user never joins, room timeline remains unavailable.

Relevant files:
- `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/RustMatrixClient.kt`
- `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/JoinedRustRoom.kt`

### 6. MXID domain can be wrong due to homeserver/public URL mismatch

Session/config can expose internal URL when `MATRIX_HOMESERVER_PUBLIC_URL` is unset. Client derives MXID domain from that URL authority. Tuwunel's configured `server_name` (e.g., `chat.poziomki.app`) may differ from internal host:port (e.g., `tuwunel:6167`). Result: generated MXIDs can be domain-mismatched.

Relevant files:
- `backend/src/controllers/migration_api/matrix_support.rs` (lines 95-108)
- `docker-compose.prod.tuwunel.yml`
- `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/RustMatrixClient.kt` (lines 631-650)

### 7. Avatar only synced at session creation time

`matrix.rs:117-130` — Avatar sync happens **once** during `create_session()`. If:
- User changes their profile picture after Matrix session was created
- Upload to Matrix fails silently (best-effort, logged as warning)
- Profile picture in the DB is a presigned URL that expired before backend could read it

...then the Matrix avatar will be permanently stale or missing.

Relevant files:
- `backend/src/controllers/migration_api/matrix.rs` (lines 117-130, 135-158)

---

## MEDIUM PRIORITY FINDINGS

### 8. MXC media loading uses thumbnail endpoint only

`MxcMediaFetcher` fetches `thumbnail` path only (256x256). No fallback to full media content endpoint if thumbnail retrieval fails. This can break avatar/image rendering depending on homeserver/media behavior.

Relevant files:
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/util/MxcMediaFetcher.kt`

### 9. Sliding sync version not verified

`RustMatrixClient.kt:175` uses `SlidingSyncVersion.NATIVE` (MSC4186). No Tuwunel env var explicitly enables this. If the deployed Tuwunel image doesn't support MSC4186, sync silently fails and no messages appear.

### 10. E2EE initialization timeout may be insufficient

```kotlin
withTimeoutOrNull(20_000) {
    runCatching { newClient.encryption().waitForE2eeInitializationTasks() }
}
```

If this times out, E2EE may not be ready. Messages sent to encrypted rooms will fail. The `runCatching` also swallows errors silently.

### 11. Push registration may be disabled by config defaults

Pusher registration requires `pushGatewayUrl` and `ntfyServer` config values that appear unset in default/prod compose baselines. This causes missed background delivery even if room membership is correct.

Relevant files:
- `docker-compose.prod.tuwunel.yml`
- `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/RustMatrixClient.kt` (lines 501-514)

### 12. Send queue errors swallowed

`RustMatrixClient.kt:183` — `enableAllSendQueues(true)` is called in `runCatching` which swallows errors. If this fails, outgoing messages queue up locally but never reach the server.

---

## "Purist" Gaps (Custom Logic Instead of SDK/Standard Paths)

### Custom backend Matrix HTTP flows
- Registration/login/profile/media are hand-written `reqwest` controller logic.
- Works, but increases maintenance and risk of drift vs SDK/spec behavior.
- Files: `matrix_support.rs`, `matrix.rs`

### Custom room draft storage
- Drafts use local `RoomComposerDraftStore` (SqlDelight/in-memory) rather than SDK-managed draft APIs.

### Distributed identity translation layer
- App UUID <-> Matrix ID conversion is split across backend (localpart derivation) and mobile (normalizeUserId).
- Not centralized or validated — currently a major correctness risk (see Finding #1).

---

## Avatar + Garage S3 Assessment

- Current UI behavior treats Garage URL as primary in chat contexts.
- Matrix avatar (`mxc://`) is secondary and often bypassed.
- If Garage signing/ACL/CORS/expiry is wrong, avatars fail despite Matrix containing valid avatar metadata.
- **Correct pattern:** Matrix avatar as default source in chat surfaces, Garage only as explicit override when required.

### mxc:// URL Resolution Chain

For avatars to display in the chat UI, this entire chain must work:
1. `RustTimeline.kt:358-362` extracts `senderAvatarUrl` from `ProfileDetails.Ready.avatarUrl` — this is an `mxc://` URL
2. `MessageEventRow.kt:230` passes it to `UserAvatar` component
3. `UserAvatar.kt` calls `SubcomposeAsyncImage(model = resolveImageUrl(picture))`
4. `ImageUrl.kt` — `resolveImageUrl()` passes `mxc://` URLs through unchanged
5. Coil dispatches to `MxcMediaFetcher` which calls `matrixClient.getMediaThumbnail()`
6. `RustMatrixClient.kt:327-334` calls the Rust SDK's `getMediaThumbnail()`

**Failure points:**
- `MxcMediaFetcher.kt:28-30` — waits 15s for Matrix client to be `Ready`. If not ready, throws `IOException`, Coil shows error fallback.
- `getMediaThumbnail()` returns `null` if client is null or SDK call fails.
- No fallback from thumbnail to full content endpoint.

---

## Suggested Fix Sequence (Priority Order)

| # | Fix | Impact | Effort |
|---|---|---|---|
| 1 | **Normalize attendee identifiers to real MXIDs** before `createRoom` invites (UUID -> `poziomki_<alnum>` localpart) | Messages not received | Medium |
| 2 | **Use authenticated media upload endpoint** (`/_matrix/client/v1/media/upload`) instead of legacy `/_matrix/media/v3/upload` | Avatars broken | Small |
| 3 | **Fix avatar resolution order** — prefer Matrix `mxc://` in chat, fall back to Garage only when mxc is absent | Avatars broken | Medium |
| 4 | **Implement invite-join flow** for invited rooms | Messages not received | Medium |
| 5 | **Fix device ID normalization** — ensure backend and mobile agree on same device ID format | E2EE failures | Small |
| 6 | **Harden MXID domain handling** — use canonical Matrix server name from config, not homeserver URL authority | Wrong MXID domain | Small |
| 7 | **Sync avatar on profile update** — not just at session creation | Stale avatars | Medium |
| 8 | **Add media fallback** — `thumbnail` -> full content endpoint in MxcMediaFetcher | Avatar edge cases | Small |
| 9 | **Verify sliding sync** — check Tuwunel logs for MSC4186 support | Silent sync failure | Small |
| 10 | **Add E2EE init logging** and increase timeout | Encrypted messages unreadable | Small |
| 11 | **Configure push registration** values in prod compose | Missed background delivery | Small |

---

## Reference: Key Files

### Backend
- `backend/src/controllers/migration_api/matrix.rs` — Session creation, avatar sync
- `backend/src/controllers/migration_api/matrix_support.rs` — All HTTP-based Matrix operations (auth, media, profile, localpart derivation)
- `backend/src/controllers/migration_api/uploads_storage.rs` — S3/filesystem storage abstraction
- `backend/src/controllers/migration_api/events_view.rs` — Event room mapping

### Mobile
- `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/RustMatrixClient.kt` — Matrix client wrapper, room creation, normalizeUserId
- `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/RustTimeline.kt` — Timeline event handling
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/repository/EventRepository.kt` — Event room invite logic (UUID vs MXID issue)
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/util/MxcMediaFetcher.kt` — mxc:// image fetching
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/util/ImageUrl.kt` — URL resolution logic
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/UserAvatar.kt` — Avatar display component
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/MessagesScreen.kt` — Avatar precedence in messages list

### Infrastructure
- `docker-compose.yml` — Dev config (Tuwunel on localhost)
- `docker-compose.prod.tuwunel.yml` — Prod config (Tuwunel on chat.poziomki.app)
- `garage.toml` — S3 storage config

### External References
- Matrix Rust SDK: https://github.com/matrix-org/matrix-rust-sdk
- Matrix Client-Server API (create room / invite): https://spec.matrix.org/latest/client-server-api/#post_matrixclientv3createroom
- Matrix Client-Server API (join room): https://spec.matrix.org/latest/client-server-api/#post_matrixclientv3roomsroomidjoin
- Matrix content repository/media endpoints: https://spec.matrix.org/latest/client-server-api/#content-repository
- Tuwunel documentation: https://matrix-construct.github.io/tuwunel/
- Matrix Rust SDK base: https://matrix-org.github.io/matrix-rust-sdk/matrix_sdk_base/
- Matrix Rust Components Kotlin: https://github.com/matrix-org/matrix-rust-components-kotlin
