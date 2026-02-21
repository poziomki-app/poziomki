# Matrix Research Consensus

> Status (2026-02-21): Historical research note. Use `docs/chat/CHAT_IMPLEMENTATION_CHECKLIST.md` for execution tasks and `docs/chat/README.md` for doc precedence.

**Date:** 2026-02-20
**Purpose:** Resolve divergences between mismatch1 and mismatch2 reports using Matrix spec + Tuwunel docs as ground truth.

---

## 1. Media Endpoints — Spec Verdict

**Both reports were partially wrong.**

### Upload
- `POST /_matrix/media/v3/upload` is **correct and NOT deprecated** (MSC3916 explicitly states this)
- `/_matrix/client/v1/media/upload` **does not exist** in the spec — any code targeting it will fail
- Upload was already authenticated (Bearer token), so MSC3916 didn't need to move it

### Download / Thumbnail
- `GET /_matrix/client/v1/media/download/{serverName}/{mediaId}` — **correct, authenticated** (Matrix v1.11+)
- `GET /_matrix/client/v1/media/thumbnail/{serverName}/{mediaId}` — **correct, authenticated**
- `GET /_matrix/media/v3/download/...` and `/_matrix/media/v3/thumbnail/...` — **deprecated, unauthenticated**

### Tuwunel Defaults
- `allow_legacy_media = false` — legacy download/thumbnail endpoints **disabled by default**
- Upload at `/_matrix/media/v3/upload` is **always available** (it's the only upload path)
- No storage difference between endpoints — `mxc://` URI is canonical regardless

### Action Taken
- Reverted incorrect change to `/_matrix/client/v1/media/upload` — restored `/_matrix/media/v3/upload`
- Removed unnecessary fallback logic added by other agent (fallback targets non-existent endpoint)
- The mobile SDK's `getMediaThumbnail()` uses the authenticated download path internally — this is correct

### Remaining Avatar Issue
The upload endpoint was never the problem. Avatar failures are caused by:
1. Avatar only synced once at session creation (not on profile update)
2. UI preferring Garage URLs over `mxc://` URLs (avatar precedence)
3. Thumbnail-only fetching with no fullsize fallback in `MxcMediaFetcher`

---

## 2. UUID vs MXID in Room Invites — CONFIRMED REAL

**mismatch2 was correct. This is the #1 root cause for messages not being received.**

### The Bug (Verified)
Two code paths for creating rooms:

| Path | UUID Conversion | Result |
|------|----------------|--------|
| **DM creation** (`AppNavigation.kt`) | `matrixLocalpartFromUserId(userId)` before `createDM()` | Correct MXID |
| **Event room** (`EventDetailViewModel.kt`) | Raw UUIDs passed to `ensureEventRoom()` → `createRoom()` | **Broken MXID** |

### What Happens
1. `EventDetailViewModel` collects `attendeeUserIds` as raw app UUIDs (e.g., `abc-123-def`)
2. Passes them to `EventRepository.ensureEventRoom()` without conversion
3. `RustMatrixClient.createRoom()` calls `normalizeUserId("abc-123-def")`
4. `normalizeUserId` just prepends `@` and appends `:homeserver` → `@abc-123-def:chat.poziomki.app`
5. This is **not a valid Matrix user** — the correct MXID would be `@poziomki_abc123def:chat.poziomki.app`

### The Fix
In `EventRepository.ensureEventRoom()` (or `EventDetailViewModel`), convert attendee UUIDs using `matrixLocalpartFromUserId()` before passing to `createRoom()`. The function already exists in `MatrixIds.kt`.

### Files
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/event/EventDetailViewModel.kt` — passes raw UUIDs
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/repository/EventRepository.kt` — `ensureEventRoom()` doesn't convert
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/util/MatrixIds.kt` — has `matrixLocalpartFromUserId()` (the correct converter)

---

## 3. Invite/Join Semantics — Spec Verdict

**mismatch2 was correct. The app needs an explicit join flow.**

### Matrix Spec
- `createRoom` with `invite: [userId]` sets the user's membership to `invite`, **NOT** `join`
- The invited user **must explicitly call** `POST /_matrix/client/v3/rooms/{roomId}/join`
- The Matrix Rust SDK does **NOT** auto-join — the app must call `room.join()` explicitly
- For bots, the pattern is an event handler on `StrippedRoomMemberEvent` with retry logic

### `trusted_private_chat` vs `private_chat`
- **Identical invite behavior** — both require explicit join
- Only difference: `trusted_private_chat` gives invitees admin power level (100) on join
- Both use `history_visibility: shared` and `join_rules: invite`

### E2EE and Invites
- With `history_visibility: shared`, invited users ARE included in Megolm key distribution
- Alice's client *should* share session keys with Bob's devices at invite time
- Bob can decrypt messages sent after he received the key, but NOT messages from before the invite
- In practice, this is fragile — key distribution failures mean "Encrypted message" in the UI

### Required Fix
The app needs to handle invited rooms. Options:
1. **Auto-join in background** — register a handler that automatically accepts invites for Poziomki-created rooms
2. **UI for invite acceptance** — show pending invites and let user accept/reject

Option 1 is simpler for MVP and matches the expected UX (users don't expect to "accept" a chat invite from within the same app).

---

## 4. MXID Domain Handling — Spec Verdict

**Both reports identified the issue. The fix is clear from the spec.**

### The Problem
`extractServerNameFromHomeserver()` in `RustMatrixClient.kt` parses the homeserver URL authority to get the server_name. Per the Matrix spec, **server_name and homeserver URL are separate concepts**:
- server_name: `chat.poziomki.app` (appears in MXIDs)
- homeserver URL: `https://chat.poziomki.app` or `http://tuwunel:6167` (HTTP endpoint)

Currently works by coincidence (URL authority matches server_name). Breaks if:
- `MATRIX_HOMESERVER_PUBLIC_URL` is empty → falls back to `http://tuwunel:6167` → MXIDs become `@user:tuwunel:6167` (broken)
- URL includes a port that differs from server_name

### The Correct Fix
Extract server_name from the user's own MXID (returned by the homeserver in the session response), not from URL parsing:

```kotlin
private fun normalizeUserId(rawValue: String): String {
    val value = rawValue.trim()
    if (value.isEmpty()) return value
    if (value.startsWith("@")) return value

    // Extract server_name from our own MXID, not from URL authority.
    // The homeserver returns the canonical MXID in the session response.
    val serverName =
        (state.value as? MatrixClientState.Ready)
            ?.userId  // e.g., "@poziomki_abc123:chat.poziomki.app"
            ?.substringAfter(':', "")
            ?.ifBlank { null }
            ?: return value
    return "@$value:$serverName"
}
```

`MatrixClientState.Ready` already stores `userId` from `newClient.userId()`. The `extractServerNameFromHomeserver` function can then be removed.

---

## 5. Device ID Normalization — Consensus

**mismatch1 was correct. Already fixed.**

The backend was uppercasing and stripping characters from device IDs. The mobile generates clean `POZ<hex>` IDs. The SDK crypto store keys on the exact device ID from the homeserver — any normalization mismatch breaks E2EE.

Fix applied: `normalize_device_id` now just trims and length-bounds (no uppercasing, no character filtering).

---

## Revised Priority Order

| # | Issue | Impact | Status |
|---|---|---|---|
| 1 | **UUID→MXID in event room invites** | Messages not received (event rooms) | Needs fix |
| 2 | **No invite-join flow** | Invited users never see rooms | Needs fix |
| 3 | **MXID domain from URL authority** | Wrong MXIDs if public URL unset | Needs fix |
| 4 | **Avatar precedence** (Garage over mxc://) | Avatars broken | Other agent working on it |
| 5 | **Device ID normalization** | E2EE failures | **Fixed** |
| 6 | **Media upload endpoint** | Was correct all along | **Verified correct** |
| 7 | **Avatar sync on profile update** | Stale avatars | Needs fix |
| 8 | **MxcMediaFetcher fullsize fallback** | Avatar edge cases | Other agent |
| 9 | **E2EE init timeout logging** | Silent failures | Nice to have |
| 10 | **Push gateway config** | Missed notifications | Nice to have |

---

## Corrections to Previous Reports

### mismatch1 was wrong about:
- `/_matrix/client/v1/media/upload` does NOT exist — the upload endpoint is `/_matrix/media/v3/upload` and is correct
- Media upload was never the avatar root cause

### mismatch2 was wrong about:
- Nothing significant — its findings were all confirmed by spec research

### Both reports missed:
- The invite-join flow being a separate, critical issue from the UUID→MXID issue
- The MXID domain fix should use own userId, not a config value
