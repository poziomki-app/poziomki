# Matrix Chat Status

Last verified: 2026-02-18

This is an implementation-state snapshot for Matrix chat integration in Poziomki RS.
Canonical execution playbook: `docs/chat/CHAT_IMPLEMENTATION_CHECKLIST.md`.
Docs index and precedence: `docs/chat/README.md`.
Two-agent parallel mode: `docs/chat/AGENT_SPLIT_PLAN.md`.

## 1. Scope

This document consolidates:
- port objectives from `docs/archive/root-docs-2026-02-18/CHAT_PORT_MAP.md`
- implementation progress from `docs/archive/root-docs-2026-02-18/MATRIX_PROGRESS.md`
- gap analysis from `docs/archive/root-docs-2026-02-18/MATRIX_12_02.md`

All three source docs are archived under `docs/archive/root-docs-2026-02-18/`.

## 2. Verification Method

Status is derived from:
- current code presence and interface/implementation checks in `mobile/shared` and `mobile/composeApp`
- current backend routing state in `backend/src/controllers/migration_api`
- latest quality-gate command runs from 2026-02-18

Notes:
- This is implementation-state verification, not a full runtime test matrix.
- iOS targets are disabled on this machine, so iOS runtime behavior is inferred from source.

## 3. Confirmed Implemented

## 3.1 Matrix boundary and Android wrappers

Implemented:
- `MatrixClient`, `JoinedRoom`, `Timeline` interfaces in shared common code.
- Android implementations:
  - `RustMatrixClient`
  - `JoinedRustRoom`
  - `RustTimeline`

## 3.2 Core room and timeline flows

Implemented:
- room list and room open flows
- live timeline mode
- focused timeline mode (`createFocusedTimeline`)
- backwards pagination
- send text, reply, edit, redact
- toggle reactions
- read receipts and typing notice calls

Evidence includes:
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/JoinedRoom.kt`
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/Timeline.kt`
- `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/JoinedRustRoom.kt`
- `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/RustTimeline.kt`

## 3.3 Composer and UX state model

Implemented:
- `ComposerMode` states (`NewMessage`, `Reply`, `Edit`)
- room-scoped in-memory draft behavior
- long-press action list flow in chat UI
- unread indicator/read-marker style behavior in timeline UI

Evidence includes:
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/model/ChatUiModels.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/ChatViewModel.kt`
- `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/ChatContent.kt`

## 3.4 Media send basics

Implemented:
- image send (`sendImage`)
- generic file send (`sendFile`)

Not yet implemented:
- video upload API path
- audio/voice upload API path
- structured media download layer (`MatrixMediaLoader` equivalent)

## 4. Confirmed Gaps

## HIGH

1. iOS Matrix chat is still disabled by design-time noop binding.
- `mobile/shared/src/iosMain/kotlin/com/poziomki/app/di/PlatformModule.ios.kt:24`
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/NoopMatrixClient.kt`

2. Backend event-to-room mapping endpoint is present but still returns not implemented.
- Route wiring: `backend/src/controllers/migration_api/mod.rs:258`
- Handler: `backend/src/controllers/migration_api/mod.rs:150`

This keeps event room authority partly client-local and risks divergence across devices/clients.

## MEDIUM

3. Security/trust UX layers are missing (verification, backup/recovery guidance).
- Core E2EE primitives exist via Rust SDK.
- User-facing device verification/recovery flows are not implemented in app UX.

4. Push notification integration for Matrix remains incomplete.
- No end-to-end pusher registration and notification delivery pipeline documented as complete.

5. Draft persistence is memory-scoped, not durable across app restarts via SDK-backed draft store.

## LOW

6. Advanced chat features are still deferred:
- threads
- polls
- voice messages
- pin management
- moderation/admin tooling

## 5. Delivery Priorities

## P0 - Hard blockers for cross-platform reliability

1. Replace iOS `NoopMatrixClient` with real implementation path.
2. Implement backend `eventId <-> roomId` mapping endpoint and migrate mobile to server-authoritative flow.

## P1 - User-facing reliability and trust

1. Add durable draft persistence.
2. Add media download path and complete video/audio send coverage.
3. Add verification and recovery UX around existing E2EE primitives.
4. Add push notification/pusher pipeline.

## P2 - UX parity and advanced features

1. Complete Element-style parity polish (room list hierarchy, tokens, states).
2. Add advanced Matrix features when P0/P1 are stable.

## 6. Acceptance Criteria for "Matrix MVP Complete"

Matrix MVP is considered complete only when:
- Android and iOS both use real Matrix client implementations.
- Event-to-room mapping is backend-authoritative.
- Room list, timeline, composer, reactions, receipts, and typing are stable in production paths.
- Quality gates are green on chat code paths.

## 7. Related Docs

- Deep technical reference: `docs/chat/MATRIX_REFERENCE.md`
- Backend migration plan: `docs/backend/API_MIGRATION.md`
- Project-wide roadmap and quality status: `docs/ENGINEERING_ROADMAP.md`
