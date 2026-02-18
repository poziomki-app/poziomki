# Matrix Integration Progress

## Completed
- Added Matrix SDK dependency (`org.matrix.rustcomponents:sdk-android`) and platform wiring in shared DI.
- Added Matrix bootstrap API models + service calls:
  - `GET /api/v1/matrix/config`
  - `POST /api/v1/matrix/session`
- Added Matrix API boundary in shared common code:
  - `MatrixClient`
  - `JoinedRoom`
  - `Timeline`
- Added Android Matrix wrappers:
  - `RustMatrixClient`
  - `JoinedRustRoom`
  - `RustTimeline`
- Added timeline controller skeleton (`TimelineController`).
- Fixed SDK compatibility issue for session restore (`Session` now includes homeserver/sliding sync fields).
- Added chat presentation viewmodels:
  - `MessagesViewModel`
  - `ChatViewModel`
  - `NewChatViewModel`
- Replaced placeholder chat screens with Matrix-backed UI flows:
  - `MessagesScreen`
  - `ChatScreen`
  - `NewChatScreen`
- Updated navigation to pass `Route.Chat.id` into `ChatScreen`.
- Registered new viewmodels in `AppModule`.
- Stabilized chat navigation semantics:
  - `Route.Chat(id)` now resolves to Matrix `roomId` before navigation.
  - Non-room targets are converted to DM room ids in `AppNavigation` via `MatrixClient.createDM`.
- Implemented focused timeline support:
  - `JoinedRustRoom.createFocusedTimeline(eventId)` now builds SDK `timelineWithConfiguration`.
  - `ChatViewModel` now switches timeline mode via `TimelineController` (Live <-> FocusedOnEvent).
  - `ChatScreen` now exposes focused mode actions (`kontekst`, `na zywo`).
- Switched Matrix store from in-memory to persistent session paths on Android:
  - `RustMatrixClient` now configures `ClientBuilder.sessionPaths(...)` under app files/cache.
- Implemented full chat interaction flow in `ChatScreen` + `ChatViewModel`:
  - composer modes (`new`, `reply`, `edit`)
  - timeline actions (`reply`, `edit`, `delete/redact`, `reaction toggle`, `context jump`)
  - reply preview rendering in timeline and composer
  - grouped message rendering + read-by count display for own messages
- Implemented event chat as Matrix group room flow:
  - `EventDetailScreen` now exposes event-chat action
  - `EventDetailViewModel.openEventChat(...)` now ensures/open event room via repository
  - `EventRepository.ensureEventRoom(...)` creates/reuses room and persists `conversationId`
- Extended event data model/storage for chat-room alignment:
  - `Event.conversationId`
  - `EventAttendee.userId`
  - SQLDelight tables updated (`event.conversation_id`, `event_attendee.user_id`)
  - migration API attendee response now includes `userId`
- Unified 1:1 and group room creation semantics:
  - `RustMatrixClient.createDM(...)` now uses the same room-creation path as groups
  - user-id normalization to Matrix MXID format is handled in client wrapper
- Implemented backend-managed Matrix session bootstrap for seamless app auth:
  - `POST /api/v1/matrix/session` now requires app auth and provisions/signs in Matrix users server-side
  - deterministic Matrix account mapping is derived from app user identity (no separate user-facing Matrix registration)
  - endpoint now returns Matrix session payload for SDK restore (`homeserver`, `accessToken`, `refreshToken`, `userId`, `deviceId`, `expiresAt`)
- Removed client fallback to direct Matrix JWT login:
  - `RustMatrixClient` now strictly uses backend-issued Matrix session bootstrap
  - eliminated Synapse JWT fallback path that produced `JWT login is not enabled` errors in chat startup
- Added typing-notification debounce in chat composer:
  - start typing notice after ~300ms debounce
  - stop typing notice after ~5s idle or immediately when draft is cleared/sent
- Added room-scoped draft persistence:
  - composer drafts are saved per Matrix room id while typing
  - drafts are restored when returning to a room and cleared on successful send
- Added chat attachment sending (Android):
  - composer now supports image picker and generic file picker actions
  - attachments are sent via Matrix timeline `sendImage` / `sendFile`
  - attachment send supports optional caption and reply target
- Current build validation:
  - `:shared:compileDebugKotlinAndroid` ✅
  - `:composeApp:compileDebugKotlinAndroid` ✅
  - `./gradlew ktlintCheck detekt` ✅
  - `cargo check` ✅
  - `cargo fmt --all -- --check` ✅
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` ✅

## Port Map Coverage (vs CHAT_PORT_MAP.md)

### P0: Must Port (MVP) — Complete

| # | Item | Status |
|---|------|--------|
| 1 | Matrix abstraction boundary (`MatrixClient`, `JoinedRoom`, `Timeline`) | Done |
| 2 | Android Rust SDK wrappers (`RustMatrixClient`, `JoinedRustRoom`, `RustTimeline`) | Done |
| 3 | TimelineController (live/focused mode, pagination) | Done |
| 4 | Chat ViewModels + UI state models | Done |
| 5 | Replace placeholder screens (room list, timeline, composer) | Done |

Core events: room list, open timeline, send text, toggle reaction, mark as read, typing notices, create DM, replies, edits, redactions — all implemented.

### P1: Should Port — In Progress

| # | Item | Status | Notes |
|---|------|--------|-------|
| 6a | Draft persistence (room-scoped) | Done | In-memory room-scoped draft store wired in ChatViewModel |
| 6b | Reply mode | Done | `ComposerMode.Reply` in ChatViewModel |
| 6c | Attachment send (image/file) | Done | Android picker + Matrix upload wired in chat composer |
| 6d | Typing debounce | Done | Debounced start/idle stop implemented in `ChatViewModel` |
| 7a | Date separators | Done | `DateDivider` in timeline model + UI |
| 7b | Loading indicators | Done | `isPaginatingBackwards` flow |
| 7c | New message indicator | Done | Prominent read-marker divider + scroll-to-bottom FAB with unread count |
| 7d | Typing indicator insertion | Done | Shown in chat screen |
| 8  | Action list panel (Element-style bottom sheet) | Done | Long-press message opens bottom sheet actions (reply/edit/delete/react/context) |

### P2: Nice-to-have — Not Started

Threads, polls, voice messages, pin events, identity/device trust UX, advanced moderation.

## Next Steps

### Design parity (P1 polish)
- [ ] Room list: avatar display, unread badge styling, last-message preview truncation
- [x] Timeline: Element-style long-press bottom sheet replacing three-dot menu
- [x] Timeline: prominent "new messages" divider at read marker position
- [x] Composer: typing debounce (send notice after ~300ms idle, stop after ~5s)
- [ ] Chat theme tokens mirroring Element design primitives (colors, spacing, typography)

### Feature gaps (P1 functionality)
- [x] Draft persistence: save/restore composer text per room across navigation
- [x] Media attachments: image/file picker and upload via Matrix content repository
- [x] New message indicator: scroll-to-bottom FAB with unread count

### Platform coverage
- [ ] iOS Matrix SDK: replace `NoopMatrixClient` with real implementation (pending `matrix-rust-components-swift` KMP bindings or native bridge)

### Testing (execution plan steps 9-10)
- [ ] Instrumentation tests for send/read/reaction flows
- [ ] Screenshot/UI regression checks against Element reference states

### Backend alignment
- [ ] Event-to-room mapping: backend-managed global mapping endpoint (currently mobile-local only)

## Monitoring Checklist
- [x] Route identity semantics are room-id only (`!roomId`) across all navigation sources.
- [x] Focused timeline is implemented and wired through `TimelineController`.
- [x] Matrix storage is persistent across app restarts.
- [x] Event detail opens/reuses Matrix group rooms.
- [x] Chat interactions support reply/edit/delete/reaction from UI.
- [x] 1:1 chats reuse group-room creation semantics in Matrix client layer.
- [x] Matrix session bootstrap uses single app account auth (no separate Matrix login flow for users).
- [ ] Element-style UX parity pass is complete for MVP screens.
- [ ] iOS Matrix chat functional.
- [ ] Instrumentation test coverage for chat flows.
