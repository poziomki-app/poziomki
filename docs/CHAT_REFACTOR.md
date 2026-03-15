# Chat Refactor Map

Comprehensive audit of the WebSocket + Postgres chat system after migrating from Matrix.
Ordered by priority. Checked items are done in this PR.

## Bugs (correctness)

- [x] **Session expiry not checked in WS auth** — `ws.rs:authenticate()` never checked `sessions.expires_at`, allowing expired tokens to connect
- [x] **Missing membership checks on react/read/typing** — `ws.rs` handlers let any authenticated user react/read-receipt/type in any conversation
- [x] **History pagination same-timestamp gaps** — `messages.rs:load_history()` used `created_at < before_ts`, skipping messages with identical timestamps forever
- [x] **Inconsistent initial page ordering** — initial history page used `created_at DESC` only while paginated pages used `(created_at DESC, id DESC)`, causing duplicates/gaps at timestamp boundaries
- [x] **Event conversation seeded with wrong creator** — `mod.rs:resolve_event_conversation()` passed requesting user's ID instead of event creator's user_id
- [x] **"Interested" attendees get chat membership** — `write_handler.rs:event_attend()` synced chat membership for all non-pending statuses including "interested"
- [x] **`mark_read` cross-conversation vulnerability** — accepted any valid message UUID without checking it belonged to the target conversation, corrupting unread counts and leaking message IDs
- [x] **Silent write failures (mobile)** — `WsConnection.send()` was a no-op when disconnected; optimistic UI showed sent messages that never transmitted
- [x] **Read receipt count drift (mobile)** — `WsTimeline.onReadReceipt()` blindly incremented count without deduplicating by userId
- [x] **Push device ID collision (mobile)** — `WsChatClient.deviceId()` returned `hashCode()` of userId, identical across all devices for same user
- [x] **No reconnect backfill (mobile)** — after WS drop/reconnect, missed messages were never fetched for opened rooms
- [x] **`sendReply` missing optimistic UI** — reply messages had no optimistic item, causing visible delay compared to `sendMessage`
- [x] **`paginateBackwards` hangs indefinitely** — `deferred.await()` had no timeout; if the server never responded, pagination blocked forever
- [x] **No client_id idempotency** — partial unique index added in migration `2026-03-15-000000`

## Dead Matrix Code

- [x] `ChatClient` interface methods `getMediaThumbnail(mxcUrl)` / `getMediaContent(mxcUrl)` — removed from interface and all implementations
- [x] `ChatClientState.Ready.homeserver` field — always empty string, removed
- [x] `ImageUrl.kt` — removed `mxc://` URL checks in `hasSupportedImageScheme()` and `resolveImageUrl()`
- [x] `proguard-rules.pro` — removed 21 lines keeping `org.matrix.rustcomponents`, `uniffi`, and `JNA` classes
- [x] `.env.example` — removed 7 commented-out Matrix config vars
- [ ] `detekt-baseline.xml` — 54 stale suppression entries referencing deleted Matrix files (regenerate baseline)

## Validation & Input Safety

- [x] **Empty message body** — added validation rejecting empty/whitespace-only bodies (unless attachment present)
- [ ] **Emoji validation in reactions** — no validation that `emoji` is actually a valid emoji string
- [x] **Message body length limit** — 10KB server-side cap on message body
- [x] **Message kind validation** — only `text`, `image`, `file` accepted

## Error Handling

- [ ] **Spawn-and-forget push notifications** — `tokio::spawn` in `handle_send()` with no panic guard or error propagation
- [ ] **`println!` in production** — `WsConnection.kt:73` uses `println` instead of structured logging (no logging framework in KMP)
- [ ] **Silent cache corruption** — `SqlDelightRoomTimelineCacheStore` silently deletes corrupted cache with no logging
- [ ] **Auth frame type not validated** — `WsConnection.kt` doesn't handle unexpected frame types (Binary/Close) after auth send

## Performance

- [x] **N+1 queries in `list_for_user`** — replaced per-conversation loop with batch queries (DISTINCT ON, raw SQL unread counts, batch profile load)
- [x] **`reqwest::Client` created per push** — shared a static `OnceLock<Client>` for connection reuse
- [x] **`send_to_user` hub memory leak** — empty entries left in DashMap after all senders disconnected; added cleanup matching broadcast/is_online
- [ ] **Avatar URL resolution repeated** — same `signed_url(filename, "thumb", "webp")` pattern duplicated across multiple files
- [ ] **Hub broadcast copies** — every broadcast clones the entire member vec; could use iterator

## Code Organization

- [ ] **`message_to_payload` does 4 queries** — sender profile, attachment URL, reply, reactions — could batch
- [ ] **Writer task cleanup** — `let _ = (writer_hub, writer_user_id)` at ws.rs:61 is a no-op; variables captured but discarded
- [ ] **Heartbeat without pong timeout** — client sends ping every 30s but never validates pong response

## Concurrency

- [x] **Writer task / unregister race** — abort + await before unregister (`ws.rs:82-83`)
- [x] **Typing state non-atomic update** — `WsJoinedRoom.onTyping()` used read-modify-write on `MutableStateFlow`; replaced with atomic `update {}`
- [x] **`pendingHistoryDeferred` not nulled on timeout** — `paginateBackwards` left stale deferred after timeout, causing next request to overwrite
- [x] **`readReceiptUsers` map grows unbounded** — deleted messages never cleaned up from receipt tracking map
- [ ] **Unsynchronized `openedRooms` map** — `WsChatClient.openedRooms` is a plain `mutableMapOf` accessed from multiple coroutines
- [ ] **Typing indicator timeout race** — rapid typing events can cancel/restart timeout job without proper sequencing

## Security & Authorization

- [x] **Cross-conversation reply leakage** — `reply_to_id` not validated against target conversation; attacker could reference messages from other conversations
- [x] **Removed members can edit/delete old messages** — `handle_edit`/`handle_delete` had no membership check; added same pattern as `handle_react`
- [x] **Deleted event leaves orphaned chat** — conversations with `event_id` FK used `ON DELETE SET NULL`; changed to `ON DELETE CASCADE`
- [x] **Read watermark can move backwards** — `mark_read` unconditionally set watermark; now only advances forward via timestamp comparison
- [x] **Auth failures not logged server-side** — WS auth errors returned to client but not logged; added `tracing::warn!` for all auth failure paths

## Bugs (functional)

- [x] **First-message delivery broken for unknown rooms** — `updateRoomLatestMessage` silently dropped messages for rooms not in the list; now triggers debounced `ListConversations` refresh
- [x] **Reconnect backfill prepends in wrong order** — `backfillOnReconnect` appended new messages on top of stale items; now clears items before requesting fresh history
- [x] **Offline startup clears cache before connection** — `ensureStarted` called `clearAll()` before connecting; moved to after successful connection
- [x] **Client actions falsely report success** — `edit`/`redact`/`toggleReaction`/`sendReadReceipt` ignored `send()` return value; now return `Result.failure` when disconnected
- [x] **Push targeting** — push only to offline users via `hub.offline_users()`; online users see unread badge via WS, client-side `ActiveChat.roomId` suppresses when viewing the chat
- [x] **WsChatClient singleton scope** — scope lives as long as the singleton; `Idle` state guard in `handleServerMessage` handles stop/restart without cancelling collectors

## Reliability

- [x] **Push HTTP timeout** — ntfy HTTP client had no timeout; added 10s timeout via `Client::builder()`
- [ ] **Non-atomic conversation creation** — self-healing via `on_conflict`, low priority
- [x] **WS send rate limiting** — 10 sends/second per connection; token bucket in reader loop

## Known Limitations

- [ ] **Single-instance ChatHub** — in-memory only, requires Redis pub/sub for multi-replica
