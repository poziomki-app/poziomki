# Chat Refactor Map

Comprehensive audit of the WebSocket + Postgres chat system after migrating from Matrix.
Ordered by priority. Checked items are done in this PR.

## Bugs (correctness)

- [x] **Session expiry not checked in WS auth** — `ws.rs:authenticate()` never checked `sessions.expires_at`, allowing expired tokens to connect
- [x] **Missing membership checks on react/read/typing** — `ws.rs` handlers let any authenticated user react/read-receipt/type in any conversation
- [x] **History pagination same-timestamp gaps** — `messages.rs:load_history()` used `created_at < before_ts`, skipping messages with identical timestamps forever
- [x] **Event conversation seeded with wrong creator** — `mod.rs:resolve_event_conversation()` passed requesting user's ID instead of event creator's user_id
- [x] **"Interested" attendees get chat membership** — `write_handler.rs:event_attend()` synced chat membership for all non-pending statuses including "interested"
- [x] **Silent write failures (mobile)** — `WsConnection.send()` was a no-op when disconnected; optimistic UI showed sent messages that never transmitted
- [x] **Read receipt count drift (mobile)** — `WsTimeline.onReadReceipt()` blindly incremented count without deduplicating by userId
- [x] **Push device ID collision (mobile)** — `WsChatClient.deviceId()` returned `hashCode()` of userId, identical across all devices for same user
- [x] **No reconnect backfill (mobile)** — after WS drop/reconnect, missed messages were never fetched for opened rooms
- [ ] **No client_id idempotency** — no unique index on `client_id`, retried sends can create duplicates (low risk, client deduplicates via matching)

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
- [ ] **Message body length limit** — no server-side cap on message body length

## Error Handling

- [ ] **Spawn-and-forget push notifications** — `tokio::spawn` in `handle_send()` with no panic guard or error propagation
- [ ] **`println!` in production** — `WsConnection.kt:73` uses `println` instead of structured logging (no logging framework in KMP)
- [ ] **Silent cache corruption** — `SqlDelightRoomTimelineCacheStore` silently deletes corrupted cache with no logging
- [ ] **Auth frame type not validated** — `WsConnection.kt` doesn't handle unexpected frame types (Binary/Close) after auth send

## Performance

- [ ] **N+1 queries in `list_for_user`** — per-conversation queries for latest message, unread count, DM profile, sender name
- [ ] **Avatar URL resolution repeated** — same `signed_url(filename, "thumb", "webp")` pattern duplicated across multiple files
- [ ] **Hub broadcast copies** — every broadcast clones the entire member vec; could use iterator

## Code Organization

- [ ] **`list_for_user` is 164 lines** — nested queries and transformations should be broken into helpers
- [ ] **`message_to_payload` does 4 queries** — sender profile, attachment URL, reply, reactions — could batch
- [ ] **Writer task cleanup** — `let _ = (writer_hub, writer_user_id)` at ws.rs:61 is a no-op; variables captured but discarded
- [ ] **Heartbeat without pong timeout** — client sends ping every 30s but never validates pong response

## Concurrency

- [ ] **Unsynchronized `openedRooms` map** — `WsChatClient.openedRooms` is a plain `mutableMapOf` accessed from multiple coroutines
- [ ] **Writer task / unregister race** — `hub.unregister()` called before `writer_task.abort()`, writer could send to unregistered user briefly
- [ ] **Typing indicator timeout race** — rapid typing events can cancel/restart timeout job without proper sequencing
