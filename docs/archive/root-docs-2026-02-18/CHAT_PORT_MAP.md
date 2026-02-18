# Chat Port Map (Element X -> Poziomki)

## Goal
Port a **practical chat slice** from Element X patterns into `poziomki-rs`, using Matrix Rust SDK practices and Element-style chat UX, without trying to import the whole Element X architecture.

This map is intentionally Matrix-native:
- Keep existing Poziomki app structure (Koin + Compose + KMP).
- Use Element X as reference for **API boundaries, timeline behavior, and presenter/state flow**.
- Use Element X as reference for **chat design language and interaction model**.
- Do not build or depend on legacy Poziomki chat endpoints (`/api/v1/chats`, `/ws/chat`); chat flows use Matrix SDK patterns.

## Current Poziomki Baseline
- Chat screens are placeholders:
  - `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/MessagesScreen.kt`
  - `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/ChatScreen.kt`
  - `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/NewChatScreen.kt`
- API client currently only talks to Elysia backend for non-chat APIs and has no Matrix chat API surface yet:
  - `mobile/shared/src/commonMain/kotlin/com/poziomki/app/api/ApiService.kt`

## Source Repos Fetched
- Element X focused slice: `../element-x-android-chat`
- Matrix Kotlin wrapper (`sdk-android`): `../matrix-rust-components-kotlin-sdk`

## Element Design References (Use for Chat UI)
Use these files as design/interaction references when building Poziomki chat UI:

Room list and list rows:
- `../element-x-android-chat/features/home/impl/src/main/kotlin/io/element/android/features/home/impl/roomlist/RoomListView.kt`
- `../element-x-android-chat/features/home/impl/src/main/kotlin/io/element/android/features/home/impl/components/RoomListContentView.kt`
- `../element-x-android-chat/features/home/impl/src/main/kotlin/io/element/android/features/home/impl/components/RoomSummaryRow.kt`

Messages shell and top bars:
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/MessagesView.kt`
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/topbars/MessagesViewTopBar.kt`

Timeline layout and message bubbles:
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/TimelineView.kt`
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/TimelineItemEventRow.kt`
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/MessageEventBubble.kt`
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/TimelineEventTimestampView.kt`
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/MessagesReactionButton.kt`
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/typing/TypingNotificationView.kt`

Composer and text entry:
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/messagecomposer/MessageComposerView.kt`
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/messagecomposer/MessageComposerPresenter.kt`
- `../element-x-android-chat/libraries/textcomposer/impl/src/main/kotlin/io/element/android/libraries/textcomposer/impl/components/markdown/MarkdownTextInput.kt`

Action list and contextual actions:
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/actionlist/ActionListView.kt`
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/actionlist/ActionListPresenter.kt`

Theme primitives and components:
- `../element-x-android-chat/libraries/designsystem/src/main/kotlin/io/element/android/libraries/designsystem/theme/ElementThemeApp.kt`
- `../element-x-android-chat/libraries/designsystem/src/main/kotlin/io/element/android/libraries/designsystem/theme/ColorAliases.kt`
- `../element-x-android-chat/libraries/designsystem/src/main/kotlin/io/element/android/libraries/designsystem/theme/ElementTypography.kt`
- `../element-x-android-chat/libraries/designsystem/src/main/kotlin/io/element/android/libraries/designsystem/components/avatar/Avatar.kt`
- `../element-x-android-chat/libraries/designsystem/src/main/kotlin/io/element/android/libraries/designsystem/atomic/organisms/RoomPreviewOrganism.kt`

## What To Port (Priority)

## P0: Must Port (MVP chat)

### 1. Matrix abstraction boundary (do this first)
Use these Element X interfaces as the model:
- `../element-x-android-chat/libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/MatrixClient.kt`
- `../element-x-android-chat/libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/room/JoinedRoom.kt`
- `../element-x-android-chat/libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/timeline/Timeline.kt`

Create in Poziomki:
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/MatrixClient.kt`
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/JoinedRoom.kt`
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/Timeline.kt`

Scope to port now:
- Room lookup/list
- Create DM / create room
- Timeline observe/paginate
- Send/edit/redact/reply
- Reactions
- Read receipts
- Typing notice

Do **not** port full API surface yet (OIDC, widgets, power-level editor, etc.).

### 2. Android Matrix implementation wrappers
Use these as implementation references:
- `../element-x-android-chat/libraries/matrix/impl/src/main/kotlin/io/element/android/libraries/matrix/impl/RustMatrixClient.kt`
- `../element-x-android-chat/libraries/matrix/impl/src/main/kotlin/io/element/android/libraries/matrix/impl/room/JoinedRustRoom.kt`
- `../element-x-android-chat/libraries/matrix/impl/src/main/kotlin/io/element/android/libraries/matrix/impl/timeline/RustTimeline.kt`

Create in Poziomki:
- `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/RustMatrixClient.kt`
- `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/JoinedRustRoom.kt`
- `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/RustTimeline.kt`

Dependency source:
- Element X uses `org.matrix.rustcomponents:sdk-android` (see `../element-x-android-chat/gradle/libs.versions.toml`).

### 3. Timeline controller concept
Port the design (not exact code):
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/TimelineController.kt`

Create in Poziomki:
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/timeline/TimelineController.kt`

Needed behavior:
- Live timeline mode
- Focused timeline mode (event permalink jump)
- Pagination state
- Switch back to live timeline

### 4. Chat presentation state/event pattern
Reference:
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/MessagesState.kt`
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/MessagesPresenter.kt`
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/TimelinePresenter.kt`
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/messagecomposer/MessageComposerPresenter.kt`

Create in Poziomki:
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/MessagesViewModel.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/ChatViewModel.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/NewChatViewModel.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/model/*` (UI states/events)

Core events to support first:
- Load room list
- Open room timeline
- Send text
- Toggle reaction
- Mark as read
- Start/stop typing
- Create DM

### 5. Replace placeholder screens
Target files to replace incrementally:
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/MessagesScreen.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/ChatScreen.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/NewChatScreen.kt`

MVP UI features:
- Conversation list
- Timeline list
- Composer text input
- Message bubble + timestamp
- Read receipts (simple)
- Reactions (simple)
- Element-style top bar and room identity treatment
- Element-style typing and loading states

## P1: Should Port (after MVP)

### 6. Message composer quality features
Reference:
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/messagecomposer/MessageComposerPresenter.kt`

Port subset:
- Draft persistence (room-scoped)
- Reply mode
- Attachment send hooks (image/file)
- Typing debounce behavior

### 7. Timeline post-processing concepts
Reference:
- `../element-x-android-chat/libraries/matrix/impl/src/main/kotlin/io/element/android/libraries/matrix/impl/timeline/RustTimeline.kt`

Port subset:
- Date separators
- Loading indicators
- New message indicator
- Typing indicator insertion

### 8. Chat actions panel
Reference:
- `../element-x-android-chat/features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/actionlist/*`

Port subset:
- Copy text
- Reply
- Edit own message
- Delete/redact
- Report (optional)

## P2: Nice-to-have / Later
- Threads
- Polls
- Voice messages
- Pin/unpin events and pinned list
- Identity / device trust UX
- Advanced moderation flows

## What To Follow vs What To Avoid

Follow:
- Interface-first matrix layer (`api` vs `impl` split)
- Flow-based timeline updates
- Explicit timeline mode model (live/focused/thread)
- Presenter/state/event separation
- Element visual hierarchy for room list, timeline, and composer
- Element interaction patterns for long-press actions, reply/edit affordances, and receipts

Avoid copying directly (for now):
- Appyx Node framework integration
- Full Metro/Hilt dependency graph patterns
- Full module granularity from Element X
- Enterprise/call/poll/thread features in first pass

## Poziomki Target Module Layout

Recommended new folders:
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/*`
- `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/*`
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/domain/*`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/*`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/messages/*`

DI wiring updates:
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/di/SharedModule.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/di/AppModule.kt`

## Backend Alignment
This mobile port should align with backend migration plan:
- Keep non-chat data via Poziomki backend (`/api/v1` auth/profile/events/uploads).
- For chat, use Matrix-native client sync and room APIs only (Element-like model).
- If backend support is needed, keep it limited to Matrix bootstrap/provisioning, not chat message facades.

## Execution Plan (Concrete)

1. Scaffold matrix API/impl boundaries in `mobile/shared` and register in Koin.
2. Add `ChatViewModel` + `MessagesViewModel` backed by Matrix room list/timeline flows.
3. Add chat theme/tokens in Poziomki design layer to mirror Element chat primitives.
4. Replace `MessagesScreen` with real room list and navigation (Element-style rows and states).
5. Replace `ChatScreen` with timeline + send text + read receipt (Element-style layout/spacing/actions).
6. Add reactions and basic action list.
7. Add `NewChatScreen` room creation + DM creation.
8. Wire draft persistence + typing notifications.
9. Add instrumentation tests for send/read/reaction flows.
10. Add screenshot/UI regression checks against selected Element reference states.

## Design Parity Acceptance Criteria (MVP)
- Room list rows match Element information hierarchy: avatar, title, preview, timestamp, unread indicators.
- Timeline message grouping and bubble spacing follow Element behavior for consecutive messages.
- Composer interaction follows Element patterns for reply/edit states and disabled/loading states.
- Action list affordances align with Element for long-press context actions.
- Typing/read receipt/reaction states are visible with Element-like emphasis and placement.

## Licensing Note
Element X is AGPL/commercial dual-licensed. Treat this map as architectural guidance; do not copy code into Poziomki without confirming license/commercial compliance.
