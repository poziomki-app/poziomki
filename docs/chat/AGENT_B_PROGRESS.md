# Agent B Progress - chat-p0-mobile

Last updated: 2026-02-21
Branch: `chat-p0-mobile`
Worktree: `../poziomki-rs-agent-b`

## Scope
- Mobile integration + UX only (`mobile/*`)
- No backend edits (`backend/src/controllers/migration_api/*` untouched)

## P0 Checklist Tracking

- [x] `P0-EVT-02` Mobile event room authority migration path
  - Implemented backend-first event room resolver call (`GET /api/v1/matrix/events/{eventId}/room`) in `EventRepository.ensureEventRoom(...)`.
  - Removed local legacy room creation fallback; event-room authority is backend-only.

- [x] `P0-EVT-03` Attendee-only access for event chat entry points
  - Added guard in `EventDetailViewModel.openEventChat(...)`.
  - Disabled event chat CTA in `EventDetailScreen` for non-attendees.
  - Added explicit non-attendee UI state in `EventChatScreen`.

- [x] `P0-DM-02` Mobile DM start through backend canonical mapping
  - Added `ChatRoomRepository.resolveDirectRoom(...)`.
  - Navigation "Wiadomość" flow now goes through backend canonical resolver only (`POST /api/v1/matrix/dms`).

- [x] `P0-UX-01` Distinguish event rooms in messages categories
  - `MessagesViewModel` now tracks event room IDs from `EventRepository.observeEventConversationIds()`.
  - `MessagesScreen` filters now:
    - `Wydarzenia`: only event rooms
    - `Grupy`: non-direct non-event rooms

- [~] `P0-EVT-04` Join/leave membership sync end-to-end
  - Mobile-side reconciliation added:
    - after attend success, app now does Matrix refresh and best-effort auto-join for invited event room using existing `getJoinedRoom()` behavior.
    - after leave success, app refreshes Matrix room list so left-state changes are reflected quickly.
  - Event-room classification now uses attended events only, preventing stale/non-attendee event rooms from appearing under `Wiadomości -> Wydarzenia`.
  - Still waiting on Agent A backend contract finalization for full end-to-end closure.

## Files Changed
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/api/Models.kt`
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/api/ApiService.kt`
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/repository/ChatRoomRepository.kt`
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/repository/EventRepository.kt`
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/di/SharedModule.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/navigation/AppNavigation.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/event/EventDetailViewModel.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/event/EventDetailScreen.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/event/EventChatScreen.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/MessagesViewModel.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/MessagesScreen.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/messages/MessagesUiState.kt`

## Validation Status
- `./gradlew :shared:compileCommonMainKotlinMetadata` ✅
- `./gradlew :shared:allMetadataJar` ✅
- `./gradlew :composeApp:allMetadataJar` ⚠️ fails on pre-existing unrelated errors:
  - `ui/component/LocationPickerSheet.kt` unresolved `format`
  - `ui/screen/onboarding/ProfilePreviewDialog.kt` unknown `decorFitsSystemWindows`
  - `ui/screen/profile/ProfileEditScreen.kt` unknown `decorFitsSystemWindows`
  - `util/MxcMediaFetcher.kt` unresolved `SYSTEM`
- Android compile blocked by environment (`ANDROID_HOME` / `local.properties` sdk.dir missing) ⚠️

## Contract/Integration Follow-ups
- Finalize explicit invite-policy UX for edge cases (`P1-INV-01`).
- Add mobile-side automated regression tests for DM/event invariants (`P1-TEST-01` follow-up).

## Maintainability Refactor (2026-02-21)
- Extracted shared Matrix room resolution helpers to remove duplicated fallback/status logic:
  - `mobile/shared/src/commonMain/kotlin/com/poziomki/app/api/MatrixRoomResolution.kt`
- Refactored repositories to use shared helpers and reduce branching complexity:
  - `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/repository/ChatRoomRepository.kt`
  - `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/repository/EventRepository.kt`
- Split `MessagesScreen` logic into focused files:
  - filtering: `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/messages/MessagesRoomFiltering.kt`
  - tabs/filter model: `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/messages/MessagesRoomFilter.kt`
  - avatar resolution: `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/messages/MessagesAvatarResolver.kt`
  - room row rendering + timestamp: `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/messages/RoomRow.kt`
  - viewmodel mappers: `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/messages/MessagesStateMappers.kt`
- Split `EventChatScreen` into orchestrator + dedicated UI/helpers:
  - header: `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/event/EventChatHeader.kt`
  - loading/not-found/join-required states: `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/event/EventChatStateViews.kt`
  - attendee avatar override mapping: `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/event/EventChatAvatarOverrides.kt`

### Additional Validation
- `./gradlew :shared:compileCommonMainKotlinMetadata` ✅
- `./gradlew :composeApp:compileCommonMainKotlinMetadata` ⚠️ still fails on pre-existing unrelated errors (`LocationPickerSheet`, `ProfilePreviewDialog`, `ProfileEditScreen`, `MxcMediaFetcher`)

## EventRepository Refactor Pass 2 (2026-02-21)
- Extracted Matrix event-room orchestration out of `EventRepository` into:
  - `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/repository/EventRoomManager.kt`
- `EventRoomManager` now owns:
  - backend-first event room resolution (`GET /matrix/events/{eventId}/room` + status handling)
  - conversation id persistence updates
  - attend/leave Matrix membership reconciliation refresh flow
- `EventRepository` now delegates:
  - `ensureEventRoom(...)` -> `eventRoomManager.ensureEventRoom(...)`
  - attend success -> `eventRoomManager.reconcileMembershipAfterAttend(...)`
  - leave success -> `eventRoomManager.reconcileMembershipAfterLeave(...)`

### Refactor Validation
- `./gradlew :shared:compileCommonMainKotlinMetadata` ✅ after extraction

## Post-Merge Canonicalization Pass (2026-02-21)
- Removed mobile temporary compatibility paths that could create canonical-room drift:
  - removed `/api/v1/matrix/dm` probe and fallback; DM resolution now uses `/api/v1/matrix/dms` only.
  - removed local legacy event-room creation fallback; event-room resolution is backend-authoritative.
- Simplified DI/contracts after fallback removal:
  - `ChatRoomRepository` no longer depends on `MatrixClient`.
  - `EventRepository.ensureEventRoom(...)` / `EventRoomManager.ensureEventRoom(...)` signatures reduced to required inputs only.
- Added backend regression tests (compile-validated) for invariants:
  - DM symmetry for both directions.
  - Event chat access symmetry across attend/leave transitions.

## EventRepository Refactor Pass 3 (2026-02-21)
- Extracted event mutation/offline-queue logic from `EventRepository` into:
  - `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/repository/EventMutationManager.kt`
- `EventMutationManager` now owns:
  - `createEvent`, `updateEvent`, `attendEvent`, `leaveEvent`, `deleteEvent`
  - optimistic updates, rollback/restore, retry policy and pending operation enqueueing
  - event upsert helper used by refresh paths
- `EventRepository` now acts as orchestration facade:
  - read/observe/refresh methods remain
  - delegates writes to `eventMutationManager`
  - delegates room concerns to `eventRoomManager`
- Size reduction:
  - `EventRepository.kt`: `473` -> `179` lines

### Refactor Validation
- `./gradlew :shared:compileCommonMainKotlinMetadata` ✅ after pass 3 extraction
