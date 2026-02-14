# Matrix Implementation Gap Analysis (12/02/2026)

## Executive Summary

This document compares Poziomki's Matrix implementation against Element X Android and Matrix Rust Components Kotlin SDK to identify missing features and implementation gaps.

---

## 1. Core Matrix Client API Surface

### Poziomki Has
- `MatrixClient` interface (basic)
- `JoinedRoom` interface (basic)
- `Timeline` interface (basic)
- Session bootstrap via backend (`POST /api/v1/matrix/session`)
- Rust SDK wrappers (`RustMatrixClient`, `JoinedRustRoom`, `RustTimeline`)

### Missing from Element X's `MatrixClient`

| Feature | Element X | Poziomki | Priority |
|---------|-----------|----------|----------|
| `sessionVerificationService` | Full verification flow | None | P1 |
| `encryptionService` | Backup/recovery/identity | None | P1 |
| `spaceService` | Spaces hierarchy | None | P2 |
| `notificationService` | Push notifications | None | P1 |
| `notificationSettingsService` | Per-room notification rules | None | P1 |
| `pushersService` | Push device registration | None | P1 |
| `roomDirectoryService` | Public room search | None | P2 |
| `mediaPreviewService` | Media preview config | None | P2 |
| `matrixMediaLoader` | Full media download/caching | Basic | P1 |
| `ignoredUsersFlow` | Block/ignore users | None | P1 |
| `createRoom()` | Full room creation params | Basic DM only | P1 |
| `joinRoomByIdOrAlias()` | Alias-based join | None | P2 |
| `knockRoom()` | Knock feature | None | P2 |
| `getRoomPreview()` | Room preview before join | None | P2 |
| `resolveRoomAlias()` | Alias resolution | None | P2 |
| `setAllSendQueuesEnabled()` | Send queue control | None | P1 |
| `sendQueueDisabledFlow()` | Send failure tracking | None | P1 |
| `canLinkNewDevice()` | QR device linking | None | P2 |
| `createLinkMobileHandler()` | Mobile QR scanner | None | P2 |
| `createLinkDesktopHandler()` | Desktop QR display | None | P2 |
| `deactivateAccount()` | Account deletion | None | P2 |
| `getRecentEmojis()` | Emoji picker MRU | None | P2 |
| `performDatabaseVacuum()` | DB optimization | None | P2 |
| `getMaxFileUploadSize()` | Upload limits | None | P1 |

---

## 2. JoinedRoom API Gaps

### Poziomki Has
- `liveTimeline`
- Basic timeline creation
- `typingNotice()`
- `inviteUserById()`

### Missing from Element X's `JoinedRoom`

| Feature | Element X | Poziomki | Priority |
|---------|-----------|----------|----------|
| `syncUpdateFlow` | Room sync state tracking | None | P1 |
| `roomTypingMembersFlow` | Other users typing | Partial | P1 |
| `identityStateChangesFlow` | E2EE identity changes | None | P1 |
| `roomNotificationSettingsStateFlow` | Notification prefs | None | P1 |
| `knockRequestsFlow` | Knock request management | None | P2 |
| `editMessage()` | Message editing | Via timeline only | P1 |
| `updateAvatar()` | Room avatar | None | P2 |
| `removeAvatar()` | Room avatar removal | None | P2 |
| `updateCanonicalAlias()` | Room alias management | None | P2 |
| `updateRoomVisibility()` | Directory visibility | None | P2 |
| `updateHistoryVisibility()` | History visibility | None | P2 |
| `publishRoomAliasInRoomDirectory()` | Alias publishing | None | P2 |
| `enableEncryption()` | E2EE toggle | Always enabled | N/A |
| `updateJoinRule()` | Join rules | None | P2 |
| `updateUsersRoles()` | Power levels | None | P2 |
| `updatePowerLevels()` | Fine-grained permissions | None | P2 |
| `resetPowerLevels()` | Permission reset | None | P2 |
| `setName()` | Room name | None | P2 |
| `setTopic()` | Room topic | None | P2 |
| `reportContent()` | Content reporting | None | P2 |
| `kickUser()` | Kick member | None | P1 |
| `banUser()` / `unbanUser()` | Ban management | None | P1 |
| `generateWidgetWebViewUrl()` | Widget/Call integration | None | P2 |
| `getWidgetDriver()` | Widget driver | None | P2 |
| `setSendQueueEnabled()` | Per-room send queue | None | P1 |
| `ignoreDeviceTrustAndResend()` | UTD recovery | None | P1 |
| `withdrawVerificationAndResend()` | Identity recovery | None | P1 |
| `subscribeToSendQueueUpdates()` | Send queue state | None | P1 |

---

## 3. Timeline API Gaps

### Poziomki Has
- Live timeline mode
- Focused timeline mode
- Basic pagination
- `sendMessage()`
- `toggleReaction()`
- `redactEvent()`
- `sendReadReceipt()` / `markAsRead()`

### Missing from Element X's `Timeline`

| Feature | Element X | Poziomki | Priority |
|---------|-----------|----------|----------|
| `Mode.PinnedEvents` | Pinned messages view | None | P2 |
| `Mode.Media` | Media gallery view | None | P2 |
| `Mode.Thread` | Thread view | None | P2 |
| `membershipChangeEventReceived` | Membership updates | None | P1 |
| `onSyncedEventReceived` | Sync tracking | None | P1 |
| `forwardPaginationStatus` | Forward pagination | None | P1 |
| `sendImage()` | Image upload | ✅ Has | Done |
| `sendVideo()` | Video upload | None | P1 |
| `sendAudio()` | Audio upload | None | P1 |
| `sendFile()` | File upload | ✅ Has | Done |
| `sendLocation()` | Location sharing | None | P2 |
| `sendVoiceMessage()` | Voice messages | None | P2 |
| `editCaption()` | Media caption edit | None | P2 |
| `forwardEvent()` | Message forwarding | None | P2 |
| `createPoll()` | Poll creation | None | P2 |
| `editPoll()` | Poll editing | None | P2 |
| `sendPollResponse()` | Poll voting | None | P2 |
| `endPoll()` | Poll end | None | P2 |
| `loadReplyDetails()` | Reply context load | None | P1 |
| `pinEvent()` / `unpinEvent()` | Pinned messages | None | P2 |
| `getLatestEventId()` | Latest event tracking | None | P1 |
| `cancelSend()` | Cancel pending send | None | P1 |

---

## 4. Encryption & Verification

### Poziomki Status ✅ Core E2EE Works
- **E2EE is functional** - Rust SDK handles encryption transparently
- Rooms created with `isEncrypted = true` (see `RustMatrixClient.kt:339`)
- Messages encrypt/decrypt automatically
- Crypto store persisted at `matrix-sdk/{namespace}/data`

### What's Missing: UX Flows

| Feature | Element X | Poziomki | Need? |
|---------|-----------|----------|-------|
| Device verification UI | Cross-signing flow | None | Yes - for multi-device trust |
| Key backup setup | Onboarding prompt | None | Yes - prevents data loss |
| Recovery key management | Export/restore | None | Yes - for device loss |
| UTD (Unable To Decrypt) handling | Retry/resend UI | None | Yes - error recovery |
| Identity state changes | Warning banners | None | Nice-to-have |

### Element X Services (for reference)

```
EncryptionService:
- backupStateStateFlow / recoveryStateStateFlow
- enableBackups() / enableRecovery()
- resetRecoveryKey() / recover(recoveryKey)
- getUserIdentity() / pinUserIdentity()

SessionVerificationService:
- verificationFlowState (SAS emoji flow)
- sessionVerifiedStatus
- requestCurrentSessionVerification()
- approveVerification() / declineVerification()
```

**Recommendation**: Port Element X's verification/backup flows as-is. The Rust SDK provides all the primitives; Element X has the UX patterns.

---

## 5. Push Notifications

### Poziomki Status
- No push notification integration
- No pusher registration
- No notification handling

### Element X Push Features

```
PushService:
- Pusher registration (FCM/UnifiedPush)
- Push gateway integration
- Notification rendering
- Notification channels
- Background sync
- Battery optimization handling
```

**Missing components**:
- `libraries/push/api/*` - Push service interface
- `libraries/push/impl/*` - Push implementation
- `PushersService` integration
- `NotificationService` integration
- `NotificationSettingsService` integration

---

## 6. Room List & Synchronization

### Poziomki Has
- Basic room list via `RoomListEntriesWithDynamicAdaptersResult`
- Room summary display
- **Sliding Sync** (native) - `RustMatrixClient.kt:147` uses `SlidingSyncVersion.NATIVE`

### Missing from Element X

| Feature | Description | Priority |
|---------|-------------|----------|
| `DynamicRoomList` | Paginated/filtered room list | P1 |
| `RoomListFilter` | Room filtering | P1 |
| `syncIndicator` | Sync state indicator | P1 |
| `subscribeToVisibleRooms()` | Visible room subscription | P1 |

---

## 7. Draft Persistence

### Poziomki Status ✅ Has In-Memory Draft
- **Has `RoomComposerDraftStore`** - in-memory implementation
- **Has `ComposerMode`** - supports NewMessage, Reply, Edit (see `ChatViewModel.kt`)
- Draft survives navigation within session
- **Missing**: Persistent Rust SDK-backed storage (`Room.saveDraft()`)

### Element X Draft System (for reference)

```kotlin
ComposerDraft:
- plainText: String
- htmlText: String?
- draftType: ComposerDraftType

ComposerDraftType:
- NewMessage
- Reply(eventId)
- Edit(eventId)
```

**Gap**: Need Rust SDK-backed `ComposerDraftService` for persistence across app restarts

---

## 8. Media Handling

### Poziomki Status ✅ Has Basic Media Upload
- **Has `sendImage()`** - `RustTimeline.kt:164-201`
- **Has `sendFile()`** - `RustTimeline.kt:203-236`
- Used in `ChatViewModel.sendImageAttachment()`, `sendFileAttachment()`
- **Missing**: `sendVideo()`, `sendAudio()`, `sendVoiceMessage()`
- **Missing**: Media download (`MatrixMediaLoader`)

### Element X Media Features (for reference)

```
MatrixMediaLoader:
- loadMediaFile()
- loadMediaThumbnail()
- loadMediaContent()

MediaUploadHandler:
- upload progress tracking
- cancellation

MediaPreviewService:
- Media preview configuration
- Size limits
```

**Gap**: Media download and video/audio upload

---

## 9. Spaces

### Poziomki Status
- No space support

### Element X Space Features

```
SpaceService:
- topLevelSpacesFlow
- spaceFiltersFlow
- joinedParents()
- getSpaceRoom()
- spaceRoomList()
- editableSpaces()
- getLeaveSpaceHandle()
- addChildToSpace()
- removeChildFromSpace()
```

---

## 10. Calls & Widgets

### Poziomki Status
- No call support
- No widget support

### Element X Call Features

```
ElementCallEntryPoint:
- Place/receive calls
- Call notifications
- PiP mode
- Call widget integration

MatrixWidgetDriver:
- Widget lifecycle
- Message passing
```

---

## 11. Location Sharing

### Poziomki Status
- No location sharing

### Element X Location Features

```
SendLocationEntryPoint:
- Share current location
- Share pin location
- Map display
- Location permissions
```

---

## 12. Knock/Membership

### Poziomki Status
- No knock support
- No membership request handling

### Element X Knock Features

```
KnockRequest:
- accept()
- decline()
- declineAndBan()
- markAsSeen()
```

---

## 13. Threads

### Poziomki Status
- No thread support

### Element X Thread Features

- Thread timeline mode
- Thread summary display
- Thread notifications

---

## 14. Voice Messages

### Poziomki Status
- No voice message support

### Element X Voice Features

- Voice recording
- Waveform display
- Voice playback
- Voice transcription

---

## 15. Polls

### Poziomki Status
- No poll support

### Element X Poll Features

- Poll creation
- Poll voting
- Poll editing
- Poll end
- Poll results display

---

## 16. Backend Integration Gaps

### Poziomki Backend Has
- Session bootstrap endpoint
- Matrix user provisioning
- Deterministic user mapping

### Missing Backend Features

| Feature | Priority |
|---------|----------|
| Media proxy (MSC3916) | P1 |
| Well-known delegation | P1 |
| Identity server integration | P2 |
| Integration manager | P2 |

---

## Priority Roadmap

### P0: Critical (Already Done per MATRIX_PROGRESS.md)
- [x] Matrix abstraction boundary
- [x] Android Rust SDK wrappers
- [x] TimelineController
- [x] Chat ViewModels
- [x] Basic timeline operations

### P1: Essential for Production
1. **Media Download** - `MatrixMediaLoader` for viewing images/files
2. **Media Upload (Video/Audio)** - Complete media sending
3. **Push Notifications** - Users expect notifications
4. **Device Verification UX** - Cross-signing for multi-device trust
5. **Key Backup/Recovery** - Prevents data loss on device change
6. **Draft Persistence** - Rust SDK-backed storage for app restarts
7. **Send Queue Management** - Offline resilience
8. **UTD Error Handling** - Unable-to-decrypt recovery flow

### Already Done ✅
- Draft (in-memory) - `RoomComposerDraftStore`
- ComposerMode (Reply/Edit) - `ChatViewModel`
- Media upload (image/file) - `RustTimeline.sendImage()`, `sendFile()`
- Sliding Sync - Native mode enabled
- Core E2EE - Automatic via Rust SDK

### P2: Nice-to-have
- Spaces
- Calls
- Location sharing
- Threads
- Voice messages
- Polls
- Knock
- Widgets

---

## Architecture Recommendations

1. **Follow Element X API boundaries exactly** - Use the same interface patterns for compatibility

2. **Implement EncryptionService first** - Without E2EE, Poziomki cannot participate in most Matrix rooms

3. **Add Push notification infrastructure** - Critical for user engagement

4. **Media handling pipeline** - Upload/download with progress tracking

5. **Draft persistence layer** - Room-scoped composer state

6. **Send queue resilience** - Handle offline scenarios gracefully

---

## API Surface Comparison Summary

| Layer | Element X | Poziomki | Gap |
|-------|-----------|----------|-----|
| MatrixClient | ~40 methods | 6 methods | 85% |
| JoinedRoom | ~25 methods | 5 methods | 80% |
| Timeline | ~25 methods | 12 methods | 52% |
| Core E2EE | Rust SDK | Rust SDK ✅ | 0% |
| Verification UX | Full flow | None | 100% |
| Backup/Recovery UX | Full flow | None | 100% |
| PushService | Full | None | 100% |
| Media Upload | Full | Image/File ✅ | 50% |
| Media Download | Full | None | 100% |
| Draft | Persistent | In-memory ✅ | 50% |
| Sliding Sync | Native | Native ✅ | 0% |

---

## References

- Element X Android: `element-x-android-chat/`
- Matrix Rust SDK Kotlin: `matrix-rust-components-kotlin-sdk/`
- Current Progress: `MATRIX_PROGRESS.md`
- Port Map: `CHAT_PORT_MAP.md`

---

## Implementation Approach: Follow Element or Build Custom?

### Recommendation: Follow Element X Patterns

**Why follow Element X:**
1. **Rust SDK is the same** - Both use `org.matrix.rustcomponents:sdk-android`, Element X just wraps it with better UX
2. **Proven UX flows** - Verification, backup, media upload all tested at scale
3. **API boundaries ready** - `EncryptionService`, `SessionVerificationService` interfaces are clean
4. **Reduced risk** - Avoid reinventing complex crypto UX flows

**What to copy directly:**
- `EncryptionService` interface + `RustEncryptionService` impl
- `SessionVerificationService` + verification UI flow
- Media upload/download patterns (`MediaUploadHandler`, `MatrixMediaLoader`)
- Draft persistence (`ComposerDraft`, `VolatileComposerDraftStore`)

**What to adapt:**
- UI theming (use Poziomki design system)
- Navigation (Poziomki uses Koin + Compose, not Appyx)
- Feature scope (skip calls/widgets/spaces for now)

### What NOT to port

| Skip | Reason |
|------|--------|
| Appyx Node framework | Poziomki uses different navigation |
| Metro/Hilt DI | Poziomki uses Koin |
| Full module granularity | Keep simpler structure |
| Enterprise features | Not needed for MVP |
| Call/Widget infrastructure | P2, complex |
| Spaces | P2, niche |
