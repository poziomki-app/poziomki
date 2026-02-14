# Matrix Implementation Reference Guide

Detailed API mappings, file references, and implementation patterns from Element X to port to Poziomki.

---

## 1. Media Upload/Download

### API Layer

**Element X Interface:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/media/MatrixMediaLoader.kt
```
```kotlin
interface MatrixMediaLoader {
    suspend fun loadMediaContent(source: MediaSource): Result<ByteArray>
    suspend fun loadMediaThumbnail(source: MediaSource, width: Long, height: Long): Result<ByteArray>
    suspend fun downloadMediaFile(source: MediaSource, mimeType: String?, filename: String?, useCache: Boolean = true): Result<MediaFile>
}
```

**Element X Implementation:**
```
libraries/matrix/impl/src/main/kotlin/io/element/android/libraries/matrix/impl/media/RustMediaLoader.kt
```

**Timeline Upload Methods:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/timeline/Timeline.kt
```
```kotlin
suspend fun sendImage(file: File, thumbnailFile: File?, imageInfo: ImageInfo, caption: String?, formattedCaption: String?, inReplyToEventId: EventId?): Result<MediaUploadHandler>
suspend fun sendVideo(file: File, thumbnailFile: File?, videoInfo: VideoInfo, caption: String?, formattedCaption: String?, inReplyToEventId: EventId?): Result<MediaUploadHandler>
suspend fun sendAudio(file: File, audioInfo: AudioInfo, caption: String?, formattedCaption: String?, inReplyToEventId: EventId?): Result<MediaUploadHandler>
suspend fun sendFile(file: File, fileInfo: FileInfo, caption: String?, formattedCaption: String?, inReplyToEventId: EventId?): Result<MediaUploadHandler>
```

**Media Upload Handler:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/media/MediaUploadHandler.kt
```
```kotlin
interface MediaUploadHandler {
    suspend fun await(): Result<Unit>
    fun cancel()
}
```

### Data Models

| File | Purpose |
|------|---------|
| `ImageInfo.kt` | Width, height, blurhash, mimetype |
| `VideoInfo.kt` | Duration, width, height, blurhash |
| `AudioInfo.kt` | Duration, mimetype |
| `FileInfo.kt` | Mimetype, size, filename |
| `ThumbnailInfo.kt` | Width, height, mimetype |
| `MediaSource.kt` | mxc:// URL wrapper |

### Feature Layer (Attachments)

**Attachment Picker:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/attachments/Attachment.kt
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/attachments/preview/AttachmentsPreviewPresenter.kt
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/attachments/preview/AttachmentsPreviewView.kt
```

**Media Optimization:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/attachments/video/MediaOptimizationSelectorPresenter.kt
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/attachments/video/VideoMetadataExtractor.kt
```

**Media Upload Library:**
```
libraries/mediaupload/api/src/main/kotlin/io/element/android/libraries/mediaupload/api/MediaSender.kt
libraries/mediaupload/impl/src/main/kotlin/io/element/android/libraries/mediaupload/impl/MediaSenderImpl.kt
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/media/MatrixMediaLoader.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/media/MediaUploadHandler.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/media/ImageInfo.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/media/VideoInfo.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/media/AudioInfo.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/media/FileInfo.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/media/MediaSource.kt
mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/media/RustMediaLoader.kt
```

Extend `Timeline.kt`:
```kotlin
suspend fun sendImage(file: File, imageInfo: ImageInfo, caption: String?): Result<Unit>
suspend fun sendVideo(file: File, videoInfo: VideoInfo, caption: String?): Result<Unit>
suspend fun sendFile(file: File, fileInfo: FileInfo): Result<Unit>
```

### Rust SDK Methods Used

```kotlin
// From org.matrix.rustcomponents.sdk
Room.sendImage()
Room.sendVideo()
Room.sendFile()
Room.sendAudio()
Client.mediaRequest() // For downloads
```

### UX Components

**Composer Attachment Bottom Sheet:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/messagecomposer/AttachmentsBottomSheet.kt
```

**Events:**
```
MessageComposerEvent.PickAttachmentSource.FromGallery
MessageComposerEvent.PickAttachmentSource.FromFiles
MessageComposerEvent.PickAttachmentSource.PhotoFromCamera
MessageComposerEvent.PickAttachmentSource.VideoFromCamera
```

---

## 2. Push Notifications

### API Layer

**Push Service Interface:**
```
libraries/push/api/src/main/kotlin/io/element/android/libraries/push/api/PushService.kt
```
```kotlin
interface PushService {
    suspend fun getCurrentPushProvider(sessionId: SessionId): PushProvider?
    fun getAvailablePushProviders(): List<PushProvider>
    suspend fun registerWith(matrixClient: MatrixClient, pushProvider: PushProvider, distributor: Distributor): Result<Unit>
    suspend fun ensurePusherIsRegistered(matrixClient: MatrixClient): Result<Unit>
    suspend fun testPush(sessionId: SessionId): Boolean
    val pushCounter: Flow<Int>
    fun getPushHistoryItemsFlow(): Flow<List<PushHistoryItem>>
}
```

**Matrix Pusher Service:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/pusher/PushersService.kt
```
```kotlin
interface PushersService {
    suspend fun setHttpPusher(setHttpPusherData: SetHttpPusherData): Result<Unit>
    suspend fun unsetHttpPusher(unsetHttpPusherData: UnsetHttpPusherData): Result<Unit>
}
```

**Pusher Data Models:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/pusher/SetHttpPusherData.kt
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/pusher/UnsetHttpPusherData.kt
```

### Implementation Layer

```
libraries/push/impl/src/main/kotlin/io/element/android/libraries/push/impl/DefaultPushService.kt
libraries/push/impl/src/main/kotlin/io/element/android/libraries/push/impl/DefaultPusherSubscriber.kt
libraries/push/impl/src/main/kotlin/io/element/android/libraries/push/impl/pushgateway/PushGatewayAPI.kt
```

### Notification System

**Notification Service:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/notification/NotificationService.kt
```

**Notification Rendering:**
```
libraries/push/impl/src/main/kotlin/io/element/android/libraries/push/impl/notifications/DefaultNotificationDrawerManager.kt
libraries/push/impl/src/main/kotlin/io/element/android/libraries/push/impl/notifications/NotificationRenderer.kt
libraries/push/impl/src/main/kotlin/io/element/android/libraries/push/impl/notifications/NotificationDataFactory.kt
```

**Notification Settings:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/notificationsettings/NotificationSettingsService.kt
```

### Background Processing

```
libraries/push/impl/src/main/kotlin/io/element/android/libraries/push/impl/workmanager/SyncNotificationWorkManagerRequest.kt
libraries/push/impl/src/main/kotlin/io/element/android/libraries/push/impl/workmanager/FetchNotificationsWorker.kt
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/push/PushService.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/push/PushersService.kt
mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/push/RustPushersService.kt
mobile/shared/src/androidMain/kotlin/com/poziomki/app/push/PushNotificationService.kt
```

### Rust SDK Methods

```kotlin
// From org.matrix.rustcomponents.sdk
Client.setPusher()
Client.getPushers()
Client.notificationClient()
```

### Key Dependencies

- FCM (Firebase Cloud Messaging) or UnifiedPush
- WorkManager for background sync
- Notification channels (Android O+)

---

## 3. Device Verification UX (Cross-Signing)

### API Layer

**Session Verification Service:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/verification/SessionVerificationService.kt
```
```kotlin
interface SessionVerificationService {
    val verificationFlowState: StateFlow<VerificationFlowState>
    val sessionVerifiedStatus: StateFlow<SessionVerifiedStatus>
    val needsSessionVerification: Flow<Boolean>
    
    suspend fun requestCurrentSessionVerification()
    suspend fun requestUserVerification(userId: UserId)
    suspend fun cancelVerification()
    suspend fun approveVerification()
    suspend fun declineVerification()
    suspend fun startVerification()
    suspend fun acknowledgeVerificationRequest(verificationRequest: VerificationRequest.Incoming)
    suspend fun acceptVerificationRequest()
    fun setListener(listener: SessionVerificationServiceListener?)
}
```

**Verification Flow States:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/verification/SessionVerificationData.kt
```
```kotlin
sealed interface VerificationFlowState {
    data object Initial : VerificationFlowState
    data object DidAcceptVerificationRequest : VerificationFlowState
    data object DidStartSasVerification : VerificationFlowState
    data class DidReceiveVerificationData(val data: SessionVerificationData) : VerificationFlowState
    data object DidFinish : VerificationFlowState
    data object DidCancel : VerificationFlowState
    data object DidFail : VerificationFlowState
}

sealed interface SessionVerificationData {
    data class Emojis(val emojis: List<VerificationEmoji>) : SessionVerificationData
    data class Decimals(val decimals: List<Int>) : SessionVerificationData
}
```

### Implementation

```
libraries/matrix/impl/src/main/kotlin/io/element/android/libraries/matrix/impl/verification/RustSessionVerificationService.kt
```

Key implementation details:
- Uses `SessionVerificationController` from Rust SDK
- Implements `SessionVerificationControllerDelegate` for callbacks
- Emoji comparison flow (SAS verification)
- Decimal fallback

### UX Flow

**Verification Screen:**
```
features/ftue/impl/src/main/kotlin/io/element/android/features/ftue/impl/sessionverification/SessionVerificationPresenter.kt
features/ftue/impl/src/main/kotlin/io/element/android/features/ftue/impl/sessionverification/SessionVerificationView.kt
```

**Emoji Display:**
```
features/ftue/impl/src/main/kotlin/io/element/android/features/ftue/impl/sessionverification/SasVerificationStateProvider.kt
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/verification/SessionVerificationService.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/verification/VerificationFlowState.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/verification/SessionVerificationData.kt
mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/verification/RustSessionVerificationService.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/verification/SessionVerificationScreen.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/verification/SessionVerificationViewModel.kt
```

### Rust SDK Methods

```kotlin
// From org.matrix.rustcomponents.sdk
Client.getSessionVerificationController()
SessionVerificationController.requestDeviceVerification()
SessionVerificationController.startSasVerification()
SessionVerificationController.approveVerification()
SessionVerificationController.declineVerification()
SessionVerificationController.cancelVerification()
Encryption.verificationState()
Encryption.verificationStateListener()
```

### UI States

| State | UI Action |
|-------|-----------|
| `Initial` | Show "Start Verification" button |
| `DidAcceptVerificationRequest` | Show "Waiting for other device..." |
| `DidStartSasVerification` | Transition to emoji display |
| `DidReceiveVerificationData(Emojis)` | Show emoji grid for comparison |
| `DidFinish` | Show success, navigate away |
| `DidCancel` | Show cancelled message |
| `DidFail` | Show error, offer retry |

---

## 4. Key Backup/Recovery

### API Layer

**Encryption Service:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/encryption/EncryptionService.kt
```
```kotlin
interface EncryptionService {
    val backupStateStateFlow: StateFlow<BackupState>
    val recoveryStateStateFlow: StateFlow<RecoveryState>
    val enableRecoveryProgressStateFlow: StateFlow<EnableRecoveryProgress>
    val isLastDevice: StateFlow<Boolean>
    val hasDevicesToVerifyAgainst: StateFlow<AsyncData<Boolean>>
    
    suspend fun enableBackups(): Result<Unit>
    suspend fun enableRecovery(waitForBackupsToUpload: Boolean): Result<Unit>
    suspend fun resetRecoveryKey(): Result<String>
    suspend fun disableRecovery(): Result<Unit>
    suspend fun doesBackupExistOnServer(): Result<Boolean>
    suspend fun recover(recoveryKey: String): Result<Unit>
    fun waitForBackupUploadSteadyState(): Flow<BackupUploadState>
}
```

**Backup/Recovery States:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/encryption/BackupState.kt
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/encryption/RecoveryState.kt
```
```kotlin
enum class BackupState {
    UNKNOWN, WAITING_FOR_SYNC, ENABLING, ENABLED, ERROR, NOT_SETUP
}

enum class RecoveryState {
    UNKNOWN, WAITING_FOR_SYNC, ENABLED, INCOMPLETE, DISABLED, ERROR
}

sealed interface EnableRecoveryProgress {
    data object Starting : EnableRecoveryProgress
    data object CreatingRecoveryKey : EnableRecoveryProgress
    data class Done(val recoveryKey: String) : EnableRecoveryProgress
}
```

### Implementation

```
libraries/matrix/impl/src/main/kotlin/io/element/android/libraries/matrix/impl/encryption/RustEncryptionService.kt
```

### UX Components

**Recovery Key Banner:**
```
features/home/impl/src/main/kotlin/io/element/android/libraries/home/impl/components/SetUpRecoveryKeyBanner.kt
```

**Security Banner State:**
```
features/home/impl/src/main/kotlin/io/element/android/features/home/impl/roomlist/RoomListState.kt
```
```kotlin
enum class SecurityBannerState {
    None,
    RecoveryKeyConfirmation,  // Show recovery setup prompt
}
```

**Recovery Key Entry:**
```
features/ftue/impl/src/main/kotlin/io/element/android/features/ftue/impl/enterrecoverykey/EnterRecoveryKeyPresenter.kt
```

### Recovery Flow Detection

In `RoomListPresenter.kt`:
```kotlin
val recoveryState by encryptionService.recoveryStateStateFlow.collectAsState()

when (recoveryState) {
    RecoveryState.INCOMPLETE -> SecurityBannerState.RecoveryKeyConfirmation
    else -> SecurityBannerState.None
}
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/encryption/EncryptionService.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/encryption/BackupState.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/encryption/RecoveryState.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/encryption/EnableRecoveryProgress.kt
mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/encryption/RustEncryptionService.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/recovery/RecoveryKeySetupScreen.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/recovery/EnterRecoveryKeyScreen.kt
```

### Rust SDK Methods

```kotlin
// From org.matrix.rustcomponents.sdk
Encryption.enableBackups()
Encryption.enableRecovery()
Encryption.disableRecovery()
Encryption.recover(recoveryKey: String)
Encryption.resetRecoveryKey()
Encryption.backupState()
Encryption.recoveryState()
Encryption.backupStateListener()
Encryption.recoveryStateListener()
Encryption.waitForBackupUploadSteadyState()
```

### UX Flow

```
┌─────────────────────────────────────┐
│ User signs in on new device         │
└─────────────────┬───────────────────┘
                  ▼
┌─────────────────────────────────────┐
│ Check recoveryState                 │
│ - INCOMPLETE: Show recovery banner  │
│ - DISABLED: Prompt setup (optional) │
│ - ENABLED: No action needed         │
└─────────────────┬───────────────────┘
                  ▼
┌─────────────────────────────────────┐
│ User taps "Set up recovery"         │
│ - enableBackups()                   │
│ - enableRecovery()                  │
│ - Show progress (CreatingKey->Done) │
│ - Display recovery key to save      │
└─────────────────────────────────────┘
```

---

## 5. Draft Persistence

### API Layer

**Draft Model:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/room/draft/ComposerDraft.kt
```
```kotlin
data class ComposerDraft(
    val plainText: String,
    val htmlText: String?,
    val draftType: ComposerDraftType
)
```

```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/room/draft/ComposerDraftType.kt
```
```kotlin
sealed interface ComposerDraftType {
    data object NewMessage : ComposerDraftType
    data class Reply(val eventId: EventId) : ComposerDraftType
    data class Edit(val eventId: EventId) : ComposerDraftType
}
```

**Draft Service:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/draft/ComposerDraftService.kt
```
```kotlin
interface ComposerDraftService {
    suspend fun loadDraft(roomId: RoomId, threadRoot: ThreadId?, isVolatile: Boolean): ComposerDraft?
    suspend fun updateDraft(roomId: RoomId, threadRoot: ThreadId?, draft: ComposerDraft?, isVolatile: Boolean)
}
```

### Implementation

**Default Service:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/draft/DefaultComposerDraftService.kt
```

**Stores:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/draft/ComposerDraftStore.kt
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/draft/MatrixComposerDraftStore.kt
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/draft/VolatileComposerDraftStore.kt
```

### Two Types of Drafts

| Type | Purpose | Persistence |
|------|---------|-------------|
| **Persistent** | Survives app restart | Matrix SDK store |
| **Volatile** | Temporary (edit mode swap) | In-memory |

### Rust SDK Integration

```
libraries/matrix/impl/src/main/kotlin/io/element/android/libraries/matrix/impl/room/draft/ComposerDraftMapper.kt
```
```kotlin
internal fun ComposerDraft.into(): RustComposerDraft {
    return RustComposerDraft(
        plainText = plainText,
        htmlText = htmlText,
        draftType = draftType.into(),
        attachments = emptyList(), // TODO
    )
}
```

### Usage in Composer

In `MessageComposerPresenter.kt`:
```kotlin
// Load draft on init
LaunchedEffect(Unit) {
    val draft = draftService.loadDraft(roomId = room.roomId, threadRoot = null, isVolatile = false)
    if (draft != null) {
        applyDraft(draft, markdownTextEditorState, richTextEditorState)
    }
}

// Save draft on mode change (edit mode)
fun setMode(newComposerMode: MessageComposerMode) {
    if (newComposerMode.isEditing) {
        val draft = createDraftFromState(...)
        updateDraft(draft, isVolatile = true)
    }
}
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/draft/ComposerDraft.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/draft/ComposerDraftType.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/draft/ComposerDraftService.kt
mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/draft/RustComposerDraftStore.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/draft/VolatileDraftStore.kt
```

### Rust SDK Methods

```kotlin
// From org.matrix.rustcomponents.sdk
Room.saveDraft(draft: ComposerDraft)
Room.loadDraft(): ComposerDraft?
Room.clearDraft()
```

---

## 6. Send Queue Management

### API Layer

**Send Queue Update:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/room/SendQueueUpdate.kt
```
```kotlin
sealed interface SendQueueUpdate {
    data object Enabled : SendQueueUpdate
    data class Error(val error: SendQueueError) : SendQueueUpdate
}
```

**MatrixClient Methods:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/MatrixClient.kt
```
```kotlin
suspend fun setAllSendQueuesEnabled(enabled: Boolean)
fun sendQueueDisabledFlow(): Flow<RoomId>
```

**JoinedRoom Methods:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/room/JoinedRoom.kt
```
```kotlin
suspend fun setSendQueueEnabled(enabled: Boolean)
fun subscribeToSendQueueUpdates(): Flow<SendQueueUpdate>
suspend fun ignoreDeviceTrustAndResend(devices: Map<UserId, List<DeviceId>>, sendHandle: SendHandle): Result<Unit>
suspend fun withdrawVerificationAndResend(userIds: List<UserId>, sendHandle: SendHandle): Result<Unit>
```

### Implementation

```
libraries/matrix/impl/src/main/kotlin/io/element/android/libraries/matrix/impl/room/SendQueueUpdatesExt.kt
```

### Send Queue Error Handling

**UTD (Unable To Decrypt) Recovery:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/crypto/sendfailure/resolve/ResolveVerifiedUserSendFailurePresenter.kt
```

**Error Types:**
```kotlin
sealed interface SendQueueError {
    // Device not verified
    data class CrossSignVerificationRequired(...) : SendQueueError
    // User identity changed
    data class IdentityVerificationRequired(...) : SendQueueError
}
```

### UX Flow

```
┌─────────────────────────────────────┐
│ Message fails to send               │
│ (network, UTD, verification)        │
└─────────────────┬───────────────────┘
                  ▼
┌─────────────────────────────────────┐
│ Send queue enters error state       │
│ - Show error indicator on message   │
│ - Disable send queue for room       │
│ - Emit via sendQueueDisabledFlow    │
└─────────────────┬───────────────────┘
                  ▼
┌─────────────────────────────────────┐
│ User taps "Retry" or "Resolve"      │
│ - If UTD: show verification prompt  │
│ - If network: wait for connection   │
│ - Re-enable send queue              │
└─────────────────────────────────────┘
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/room/SendQueueUpdate.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/room/SendQueueError.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/chat/SendQueueErrorBanner.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/chat/MessageSendErrorView.kt
```

### Rust SDK Methods

```kotlin
// From org.matrix.rustcomponents.sdk
Client.setSendQueueEnabled(enabled: Boolean)
Room.setSendQueueEnabled(enabled: Boolean)
Room.sendQueueUpdates(): Flow<SendQueueUpdate>
Room.ignoreDeviceTrustAndResend(...)
Room.withdrawVerificationAndResend(...)
```

---

## 7. UTD Error Handling

### API Layer

**UTD Cause:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/timeline/item/event/UtdCause.kt
```
```kotlin
sealed interface UtdCause {
    data object Unknown : UtdCause
    data object MegolmV1AesSha2 : UtdCause  // Normal E2EE
    data object InsecureDevice : UtdCause
    data object HistoricallyInaccessibleDevice : UtdCause
    data object HistoricallyInaccessibleCrossSigning : UtdCause
    data object CrossSigningNotSetup : UtdCause
    data object verificationViolation : UtdCause
    data object HistoricallyInaccessibleNoKeyBackup : UtdCause
    data object InaccessibleWithHook : UtdCause
}
```

**Message Shield:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/timeline/item/event/MessageShield.kt
```
```kotlin
sealed interface MessageShield {
    data object Unknown : MessageShield
    data class Redacted(val code: ShieldCode) : MessageShield
}
```

### UX Components

**Encrypted Message View:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/event/TimelineItemEncryptedView.kt
```

**UTD Display:**
```kotlin
@Composable
fun TimelineItemEncryptedView(
    utdCause: UtdCause,
    onRetry: () -> Unit,
) {
    when (utdCause) {
        UtdCause.InsecureDevice -> {
            // "Your device is not verified"
            // Button: "Verify this device"
        }
        UtdCause.HistoricallyInaccessibleNoKeyBackup -> {
            // "No key backup available"
            // Historical messages can't be decrypted
        }
        UtdCause.CrossSigningNotSetup -> {
            // "Cross-signing not set up"
        }
        // etc.
    }
}
```

### Identity State Changes

```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/encryption/identity/IdentityState.kt
```
```kotlin
sealed interface IdentityState {
    data object Verified : IdentityState
    data object Pinned : IdentityState  // TOFU verified
    data object VerificationViolation : IdentityState  // Identity changed
}
```

**Identity Change Handling:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/crypto/identity/IdentityChangeStatePresenter.kt
```

### Recovery Actions

```kotlin
// In JoinedRoom
suspend fun ignoreDeviceTrustAndResend(
    devices: Map<UserId, List<DeviceId>>,
    sendHandle: SendHandle
): Result<Unit>

suspend fun withdrawVerificationAndResend(
    userIds: List<UserId>,
    sendHandle: SendHandle
): Result<Unit>
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/timeline/UtdCause.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/timeline/MessageShield.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/encryption/IdentityState.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/chat/EncryptedMessageView.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/chat/IdentityChangeBanner.kt
```

---

## Quick Reference: File Locations

### Element X Source Paths

| Category | Path |
|----------|------|
| Matrix API | `libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/` |
| Matrix Impl | `libraries/matrix/impl/src/main/kotlin/io/element/android/libraries/matrix/impl/` |
| Messages Feature | `features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/` |
| Home/Room List | `features/home/impl/src/main/kotlin/io/element/android/features/home/impl/` |
| FTUE (Onboarding) | `features/ftue/impl/src/main/kotlin/io/element/android/features/ftue/impl/` |
| Push | `libraries/push/api/src/main/kotlin/io/element/android/libraries/push/api/` |
| Design System | `libraries/designsystem/src/main/kotlin/io/element/android/libraries/designsystem/` |

### Poziomki Target Paths

| Category | Path |
|----------|------|
| Matrix API | `mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/` |
| Matrix Impl | `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/` |
| Chat UI | `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/` |
| Components | `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/` |

---

## 8. Timeline Items Architecture

### API Layer

**Timeline Item Types:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/timeline/item/
```

```kotlin
// MatrixTimelineItem is the main sealed interface
sealed interface MatrixTimelineItem {
    data class Event(val event: EventTimelineItem) : MatrixTimelineItem
    data class Virtual(val model: VirtualTimelineItemModel) : MatrixTimelineItem
}
```

**Event Timeline Item:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/timeline/item/event/EventTimelineItem.kt
```
```kotlin
data class EventTimelineItem(
    val eventId: EventId?,
    val transactionId: TransactionId?,
    val isEditable: Boolean,
    val canBeRepliedTo: Boolean,
    val isOwn: Boolean,
    val isRemote: Boolean,
    val localSendState: LocalEventSendState?,
    val reactions: ImmutableList<EventReaction>,
    val receipts: ImmutableList<Receipt>,
    val sender: UserId,
    val senderProfile: ProfileDetails,
    val timestamp: Long,
    val content: EventContent,
    val origin: TimelineItemEventOrigin?,
    val forwarder: UserId?,
    val forwarderProfile: ProfileDetails?,
)
```

**Event Content Types:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/timeline/item/event/EventContent.kt
```
```kotlin
sealed interface EventContent

data class MessageContent(
    val body: String,
    val inReplyTo: InReplyTo?,
    val isEdited: Boolean,
    val threadInfo: EventThreadInfo?,
    val type: MessageType,
) : EventContent

data object RedactedContent : EventContent
data class StickerContent(...) : EventContent
data class PollContent(...) : EventContent
data class UnableToDecryptContent(...) : EventContent
data class RoomMembershipContent(...) : EventContent
data class ProfileChangeContent(...) : EventContent
data class StateContent(...) : EventContent
```

**Message Types:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/timeline/item/event/MessageType.kt
```
```kotlin
sealed interface MessageType

// Text messages
data class TextMessageType(val body: String, val formatted: FormattedBody?) : MessageType
data class EmoteMessageType(val body: String, val formatted: FormattedBody?) : MessageType
data class NoticeMessageType(val body: String, val formatted: FormattedBody?) : MessageType

// Media messages (with attachment)
sealed interface MessageTypeWithAttachment : MessageType {
    val filename: String
    val caption: String?
    val formattedCaption: FormattedBody?
}

data class ImageMessageType(...) : MessageTypeWithAttachment
data class VideoMessageType(...) : MessageTypeWithAttachment
data class AudioMessageType(...) : MessageTypeWithAttachment
data class VoiceMessageType(...) : MessageTypeWithAttachment
data class FileMessageType(...) : MessageTypeWithAttachment

// Location
data class LocationMessageType(val body: String, val geoUri: String, val description: String?) : MessageType
```

### Virtual Timeline Items

```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/timeline/item/virtual/VirtualTimelineItem.kt
```
```kotlin
sealed interface VirtualTimelineItemModel {
    data class DaySeparator(val formattedDate: String) : VirtualTimelineItemModel
    data object ReadMarker : VirtualTimelineItemModel
    data object LoadingIndicator : VirtualTimelineItemModel
    data object RoomBeginning : VirtualTimelineItemModel
    data class TypingNotification(val users: List<UserId>) : VirtualTimelineItemModel
}
```

### Local Send State

```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/timeline/item/event/LocalEventSendState.kt
```
```kotlin
sealed interface LocalEventSendState {
    data object NotSentYet : LocalEventSendState
    data class Sending(val progress: Float?) : LocalEventSendState
    data object Sent : LocalEventSendState
    data class SendingFailed(val error: Throwable) : LocalEventSendState
}
```

### UX: Timeline Components

**Event Row:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/TimelineItemEventRow.kt
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/MessageEventBubble.kt
```

**Content Views by Type:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/event/
├── TimelineItemTextView.kt
├── TimelineItemImageView.kt
├── TimelineItemVideoView.kt
├── TimelineItemAudioView.kt
├── TimelineItemFileView.kt
├── TimelineItemLocationView.kt
├── TimelineItemPollView.kt
├── TimelineItemEncryptedView.kt
├── TimelineItemRedactedView.kt
├── TimelineItemStateView.kt
└── TimelineItemStickerView.kt
```

**Virtual Item Views:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/virtual/
├── TimelineItemDaySeparatorView.kt
├── TimelineItemReadMarkerView.kt
├── TimelineLoadingMoreIndicator.kt
└── TimelineItemRoomBeginningView.kt
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/timeline/MatrixTimelineItem.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/timeline/EventTimelineItem.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/timeline/EventContent.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/timeline/MessageType.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/timeline/VirtualTimelineItem.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/timeline/LocalSendState.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/timeline/TimelineItemRow.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/timeline/MessageBubble.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/timeline/MessageContentView.kt
```

---

## 9. Message Composer (Full Reference)

### API Layer

**Composer Modes:**
```
libraries/textcomposer/impl/src/main/kotlin/io/element/android/libraries/textcomposer/model/MessageComposerMode.kt
```
```kotlin
sealed interface MessageComposerMode {
    data object Normal : MessageComposerMode
    data class Reply(val replyToDetails: InReplyToDetails, val hideImage: Boolean) : MessageComposerMode
    data class Edit(val eventOrTransactionId: EventOrTransactionId, val content: String) : MessageComposerMode
    data class EditCaption(val eventOrTransactionId: EventOrTransactionId, val content: String) : MessageComposerMode
    data object Attachment : MessageComposerMode  // Caption mode for media

    val isEditing: Boolean
        get() = this is Edit || this is EditCaption
    val inThread: Boolean
        get() = false  // Override in thread mode
}
```

**Text Editor State:**
```
libraries/textcomposer/impl/src/main/kotlin/io/element/android/libraries/textcomposer/model/TextEditorState.kt
```
```kotlin
sealed interface TextEditorState {
    val isRoomEncrypted: Boolean?
    fun hasFocus(): Boolean
    suspend fun requestFocus()

    data class Rich(
        val richTextEditorState: RichTextEditorState,
        override val isRoomEncrypted: Boolean?
    ) : TextEditorState

    data class Markdown(
        val state: MarkdownTextEditorState,
        override val isRoomEncrypted: Boolean?
    ) : TextEditorState
}
```

**Message Model:**
```
libraries/textcomposer/impl/src/main/kotlin/io/element/android/libraries/textcomposer/model/Message.kt
```
```kotlin
data class Message(
    val html: String?,
    val markdown: String,
    val intentionalMentions: List<IntentionalMention>
)
```

### Main Composer Component

```
libraries/textcomposer/impl/src/main/kotlin/io/element/android/libraries/textcomposer/TextComposer.kt
```

Key props:
```kotlin
@Composable
fun TextComposer(
    state: TextEditorState,
    voiceMessageState: VoiceMessageState,
    composerMode: MessageComposerMode,
    onRequestFocus: () -> Unit,
    onSendMessage: () -> Unit,
    onResetComposerMode: () -> Unit,
    onAddAttachment: () -> Unit,
    onDismissTextFormatting: () -> Unit,
    onVoiceRecorderEvent: (VoiceMessageRecorderEvent) -> Unit,
    onVoicePlayerEvent: (VoiceMessagePlayerEvent) -> Unit,
    onSendVoiceMessage: () -> Unit,
    onDeleteVoiceMessage: () -> Unit,
    onError: (Throwable) -> Unit,
    onTyping: (Boolean) -> Unit,
    onReceiveSuggestion: (Suggestion?) -> Unit,
    onSelectRichContent: ((Uri) -> Unit)?,
    resolveMentionDisplay: (text: String, url: String) -> TextDisplay,
    resolveAtRoomMentionDisplay: () -> TextDisplay,
    modifier: Modifier = Modifier,
    showTextFormatting: Boolean = false,
)
```

### Composer State

```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/messagecomposer/MessageComposerState.kt
```
```kotlin
data class MessageComposerState(
    val textEditorState: TextEditorState,
    val isFullScreen: Boolean,
    val mode: MessageComposerMode,
    val showAttachmentSourcePicker: Boolean,
    val showTextFormatting: Boolean,
    val canShareLocation: Boolean,
    val suggestions: ImmutableList<ResolvedSuggestion>,
    val resolveMentionDisplay: (String, String) -> TextDisplay,
    val resolveAtRoomMentionDisplay: () -> TextDisplay,
    val eventSink: (MessageComposerEvent) -> Unit,
)
```

### Composer Events

```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/messagecomposer/MessageComposerEvent.kt
```
```kotlin
sealed interface MessageComposerEvent {
    data object ToggleFullScreenState : MessageComposerEvent
    data object CloseSpecialMode : MessageComposerEvent
    data object SendMessage : MessageComposerEvent
    data class SendUri(val uri: Uri) : MessageComposerEvent
    data class SetMode(val composerMode: MessageComposerMode) : MessageComposerEvent
    data object AddAttachment : MessageComposerEvent
    data object DismissAttachmentMenu : MessageComposerEvent
    // Attachment sources
    data object PickAttachmentSourceFromGallery : MessageComposerEvent
    data object PickAttachmentSourceFromFiles : MessageComposerEvent
    data object PickAttachmentSourcePhotoFromCamera : MessageComposerEvent
    data object PickAttachmentSourceVideoFromCamera : MessageComposerEvent
    data object PickAttachmentSourceLocation : MessageComposerEvent
    data object PickAttachmentSourcePoll : MessageComposerEvent
    data class ToggleTextFormatting(val enabled: Boolean) : MessageComposerEvent
    data class Error(val error: Throwable) : MessageComposerEvent
    data class TypingNotice(val isTyping: Boolean) : MessageComposerEvent
    data class SuggestionReceived(val suggestion: Suggestion?) : MessageComposerEvent
    data class InsertSuggestion(val resolvedSuggestion: ResolvedSuggestion) : MessageComposerEvent
    data object SaveDraft : MessageComposerEvent
}
```

### Typing Notifications

**In Presenter:**
```kotlin
// Typing debounce - send after user stops
val sendTypingNotifications by remember {
    sessionPreferencesStore.isSendTypingNotificationsEnabled()
}.collectAsState(initial = true)

// In handleEvent
is MessageComposerEvent.TypingNotice -> {
    if (sendTypingNotifications) {
        localCoroutineScope.launch {
            room.typingNotice(event.isTyping)
        }
    }
}

// Cleanup on dispose
DisposableEffect(Unit) {
    onDispose {
        sessionCoroutineScope.launch {
            if (sendTypingNotifications) {
                room.typingNotice(false)
            }
        }
    }
}
```

### Mentions & Suggestions

**Suggestion Model:**
```
libraries/textcomposer/impl/src/main/kotlin/io/element/android/libraries/textcomposer/model/Suggestion.kt
```
```kotlin
data class Suggestion(val pattern: SuggestionPattern)

// Pattern types from wysiwyg
// @ for user mentions
// # for room aliases
```

**Resolved Suggestion:**
```
libraries/textcomposer/impl/src/main/kotlin/io/element/android/libraries/textcomposer/mentions/ResolvedSuggestion.kt
```
```kotlin
sealed interface ResolvedSuggestion {
    data class AtRoom(val roomMember: RoomMember) : ResolvedSuggestion
    data class Member(val roomMember: RoomMember) : ResolvedSuggestion
    data class Alias(val roomAlias: RoomAlias) : ResolvedSuggestion
}
```

**Suggestions Processor:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/messagecomposer/suggestions/SuggestionsProcessor.kt
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/composer/MessageComposerMode.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/composer/TextEditorState.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/composer/Message.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/composer/Suggestion.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/composer/TextComposer.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/composer/ComposerModeView.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/composer/SuggestionsPicker.kt
```

---

## 10. Room List & Sync State

### API Layer

**Room List Service:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/roomlist/RoomListService.kt
```
```kotlin
interface RoomListService {
    sealed interface State {
        data object Idle : State
        data object Running : State
        data object Error : State
        data object Terminated : State
    }

    sealed interface SyncIndicator {
        data object Show : SyncIndicator
        data object Hide : SyncIndicator
    }

    fun createRoomList(pageSize: Int, source: RoomList.Source, coroutineScope: CoroutineScope): DynamicRoomList
    suspend fun subscribeToVisibleRooms(roomIds: List<RoomId>)
    val allRooms: RoomList
    val syncIndicator: StateFlow<SyncIndicator>
    val state: StateFlow<State>
}
```

**Room Summary:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/roomlist/RoomSummary.kt
```
```kotlin
data class RoomSummary(
    val identifier: RoomListIdentifier,
    val roomId: RoomId,
    val name: String,
    val canonicalAlias: RoomAlias?,
    val isDm: Boolean,
    val isSpace: Boolean,
    val avatarUrl: String?,
    val lastMessage: RoomSummaryDetails?,
    val unread: RoomUnreadDetails?,
    val inviter: RoomMember?,
    val hasRoomCall: Boolean,
    val isFavorite: Boolean,
    val currentUserMembership: CurrentUserMembership,
)
```

**Room Info:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/room/RoomInfo.kt
```
```kotlin
data class RoomInfo(
    val id: RoomId,
    val name: String?,
    val rawName: String?,
    val topic: String?,
    val avatarUrl: String?,
    val isPublic: Boolean?,
    val isDirect: Boolean,
    val isEncrypted: Boolean?,
    val joinRule: JoinRule?,
    val isSpace: Boolean,
    val isFavorite: Boolean,
    val canonicalAlias: RoomAlias?,
    val alternativeAliases: ImmutableList<RoomAlias>,
    val currentUserMembership: CurrentUserMembership,
    val inviter: RoomMember?,
    val activeMembersCount: Long,
    val invitedMembersCount: Long,
    val joinedMembersCount: Long,
    val roomPowerLevels: RoomPowerLevels?,
    val highlightCount: Long,
    val notificationCount: Long,
    val userDefinedNotificationMode: RoomNotificationMode?,
    val hasRoomCall: Boolean,
    val activeRoomCallParticipants: ImmutableList<UserId>,
    val isMarkedUnread: Boolean,
    val numUnreadMessages: Long,
    val numUnreadNotifications: Long,
    val numUnreadMentions: Long,
    val heroes: ImmutableList<MatrixUser>,
    val pinnedEventIds: ImmutableList<EventId>,
    val creators: ImmutableList<UserId>,
    val historyVisibility: RoomHistoryVisibility,
    val successorRoom: SuccessorRoom?,
    val roomVersion: String?,
    val privilegedCreatorRole: Boolean,
)
```

### Sync Service

```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/sync/SyncService.kt
```
```kotlin
interface SyncService {
    suspend fun startSync(): Result<Unit>
    suspend fun stopSync(): Result<Unit>
    val syncState: StateFlow<SyncState>
    val isOnline: StateFlow<Boolean>
}
```

**Sync State:**
```kotlin
enum class SyncState {
    IDLE,
    RUNNING,
    PAUSED,
    STOPPED,
    ERROR,
}
```

### Room List Presenter Pattern

```
features/home/impl/src/main/kotlin/io/element/android/features/home/impl/roomlist/RoomListPresenter.kt
```

Key patterns:
```kotlin
@Composable
override fun present(): RoomListState {
    // Security banner from encryption state
    val recoveryState by encryptionService.recoveryStateStateFlow.collectAsState()
    
    // Room summaries from data source
    val roomSummaries by produceState(initialValue = AsyncData.Loading()) {
        roomListDataSource.roomSummariesFlow.collect { value = AsyncData.Success(it) }
    }
    
    // Loading state
    val loadingState by roomListDataSource.loadingState.collectAsState()
    
    // Compute content state
    val contentState = when {
        showEmpty -> RoomListContentState.Empty(securityBannerState = ...)
        showSkeleton -> RoomListContentState.Skeleton(count = 16)
        else -> RoomListContentState.Rooms(
            securityBannerState = securityBannerState,
            summaries = roomSummaries.dataOrNull().orEmpty().toImmutableList(),
            ...
        )
    }
    
    return RoomListState(
        contentState = contentState,
        eventSink = ::handleEvent,
    )
}
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/roomlist/RoomListService.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/roomlist/RoomSummary.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/sync/SyncService.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/sync/SyncState.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/room/RoomInfo.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/main/MessagesViewModel.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/roomlist/RoomSummaryRow.kt
```

---

## 11. Action List (Context Menu)

### API Layer

**Timeline Item Actions:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/actionlist/model/TimelineItemAction.kt
```
```kotlin
enum class TimelineItemAction {
    // Messaging
    Reply,
    ReplyInThread,
    Forward,
    Edit,
    EditCaption,
    AddCaption,
    RemoveCaption,
    Redact,
    
    // Reactions
    React,  // Custom reaction picker
    
    // Polls
    EndPoll,
    EditPoll,
    
    // Pins
    Pin,
    Unpin,
    
    // Copy/Share
    CopyText,
    CopyCaption,
    CopyLink,
    
    // Moderation
    ReportContent,
    
    // Debug
    ViewSource,
}
```

### Action List State

```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/actionlist/ActionListState.kt
```
```kotlin
data class ActionListState(
    val target: Target,
    val eventSink: (ActionListEvent) -> Unit,
) {
    sealed interface Target {
        data object None : Target
        data class Loading(val event: TimelineItem.Event) : Target
        data class Success(
            val event: TimelineItem.Event,
            val sentTimeFull: String,
            val displayEmojiReactions: Boolean,
            val verifiedUserSendFailure: VerifiedUserSendFailure,
            val actions: ImmutableList<TimelineItemAction>,
            val recentEmojis: ImmutableList<String>,
        ) : Target
    }
}
```

### Action Computation Logic

```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/actionlist/ActionListPresenter.kt
```

Key logic:
```kotlin
private fun buildActions(
    timelineItem: TimelineItem.Event,
    usersEventPermissions: UserEventPermissions,
    isDeveloperModeEnabled: Boolean,
    isEventPinned: Boolean,
    isThreadsEnabled: Boolean,
): List<TimelineItemAction> {
    val canRedact = timelineItem.isMine && usersEventPermissions.canRedactOwn || 
                    !timelineItem.isMine && usersEventPermissions.canRedactOther
    
    return buildSet {
        // Reply/Thread
        if (timelineItem.canBeRepliedTo && usersEventPermissions.canSendMessage) {
            if (isThreadsEnabled && timelineMode !is Timeline.Mode.Thread) {
                add(TimelineItemAction.ReplyInThread)
                add(TimelineItemAction.Reply)
            } else {
                add(TimelineItemAction.Reply)
            }
        }
        
        // Forward
        if (timelineItem.isRemote && timelineItem.content.canBeForwarded()) {
            add(TimelineItemAction.Forward)
        }
        
        // Edit
        if (timelineItem.isEditable && usersEventPermissions.canSendMessage) {
            if (timelineItem.content is TimelineItemEventContentWithAttachment) {
                if (timelineItem.content.caption == null) {
                    add(TimelineItemAction.AddCaption)
                } else {
                    add(TimelineItemAction.EditCaption)
                    add(TimelineItemAction.RemoveCaption)
                }
            } else {
                add(TimelineItemAction.Edit)
            }
        }
        
        // Pin/Unpin
        if (usersEventPermissions.canPinUnpin && timelineItem.isRemote) {
            if (isEventPinned) add(TimelineItemAction.Unpin)
            else add(TimelineItemAction.Pin)
        }
        
        // Copy
        if (timelineItem.content.canBeCopied()) {
            add(TimelineItemAction.CopyText)
        }
        
        // Copy link
        if (timelineItem.isRemote) {
            add(TimelineItemAction.CopyLink)
        }
        
        // Report (not own messages)
        if (!timelineItem.isMine) {
            add(TimelineItemAction.ReportContent)
        }
        
        // Redact/Delete
        if (canRedact) {
            add(TimelineItemAction.Redact)
        }
    }.sortedWith(comparator)
}
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/timeline/TimelineItemAction.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/timeline/ActionListBottomSheet.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/timeline/ActionListPresenter.kt
```

---

## 12. Read Receipts

### API Layer

**Receipt Model:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/timeline/item/event/Receipt.kt
```
```kotlin
data class Receipt(
    val userId: UserId,
    val timestamp: Long,
    val type: ReceiptType,
)

enum class ReceiptType {
    READ,
    READ_PRIVATE,
}
```

**Timeline Receipt Methods:**
```kotlin
// In Timeline.kt
suspend fun sendReadReceipt(eventId: EventId, receiptType: ReceiptType): Result<Unit>
suspend fun markAsRead(receiptType: ReceiptType): Result<Unit>
```

### UX Components

**Read Receipt View:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/receipt/TimelineItemReadReceiptView.kt
```

**Read Receipt Bottom Sheet:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/receipt/bottomsheet/ReadReceiptBottomSheet.kt
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/receipt/bottomsheet/ReadReceiptBottomSheetPresenter.kt
```

### Read Receipt Logic

In `TimelinePresenter.kt`:
```kotlin
private fun CoroutineScope.sendReadReceiptIfNeeded(
    firstVisibleIndex: Int,
    timelineItems: ImmutableList<TimelineItem>,
    lastReadReceiptId: MutableState<EventId?>,
    readReceiptType: ReceiptType,
) = launch {
    // If at bottom, mark entire room as read
    if (firstVisibleIndex == 0) {
        timelineController.invokeOnCurrentTimeline {
            markAsRead(receiptType = readReceiptType)
        }
    } else {
        // Send receipt for visible event
        val eventId = getLastEventIdBeforeOrAt(firstVisibleIndex, timelineItems)
        if (eventId != null && eventId != lastReadReceiptId.value) {
            lastReadReceiptId.value = eventId
            timelineController.invokeOnCurrentTimeline {
                sendReadReceipt(eventId = eventId, receiptType = readReceiptType)
            }
        }
    }
}
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/timeline/Receipt.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/timeline/ReceiptType.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/timeline/ReadReceiptView.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/timeline/ReadReceiptBottomSheet.kt
```

---

## 13. Reactions

### API Layer

**Event Reaction:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/timeline/item/event/EventReaction.kt
```
```kotlin
data class EventReaction(
    val key: String,  // Emoji
    val senders: ImmutableList<ReactionSender>,
    val isHighlighted: Boolean,  // Current user reacted
)

data class ReactionSender(
    val senderId: UserId,
    val timestamp: Long,
)
```

**Timeline Method:**
```kotlin
suspend fun toggleReaction(emoji: String, eventOrTransactionId: EventOrTransactionId): Result<Boolean>
```

### UX Components

**Reactions View:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/TimelineItemReactionsView.kt
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/MessagesReactionButton.kt
```

**Reaction Summary (Bottom Sheet):**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/reactionsummary/ReactionSummaryPresenter.kt
```

**Custom Reaction Picker:**
```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/customreaction/CustomReactionPresenter.kt
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/timeline/components/customreaction/picker/EmojiPickerPresenter.kt
```

### Reaction Flow

```
┌─────────────────────────────────────┐
│ Long press message                  │
└─────────────────┬───────────────────┘
                  ▼
┌─────────────────────────────────────┐
│ ActionList shows:                   │
│ - Suggested emojis (👍👎🔥❤️👏)      │
│ - Recent emojis (from MRU)          │
│ - "More reactions" button           │
└─────────────────┬───────────────────┘
                  ▼
┌─────────────────────────────────────┐
│ User taps emoji                     │
│ -> toggleReaction(emoji, eventId)   │
│ -> If already reacted: remove       │
│ -> If not reacted: add              │
└─────────────────────────────────────┘
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/timeline/EventReaction.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/timeline/ReactionsView.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/timeline/ReactionButton.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/timeline/EmojiPicker.kt
```

---

## 14. Room Members

### API Layer

**Room Member:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/room/RoomMember.kt
```
```kotlin
data class RoomMember(
    val userId: UserId,
    val displayName: String?,
    val avatarUrl: String?,
    val membership: RoomMembershipState,
    val isNameAmbiguous: Boolean,
    val powerLevel: Long,
    val isIgnored: Boolean,
    val role: Role,
    val membershipChangeReason: String?,
) {
    sealed interface Role {
        data class Owner(val isCreator: Boolean) : Role
        data object Admin : Role
        data object Moderator : Role
        data object User : Role
    }
    
    val disambiguatedDisplayName: String
}

enum class RoomMembershipState {
    BAN,
    INVITE,
    JOIN,
    KNOCK,
    LEAVE;
}
```

**Members State:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/room/RoomMembersState.kt
```
```kotlin
sealed interface RoomMembersState {
    data object Unknown : RoomMembersState
    data class Loaded(val members: ImmutableList<RoomMember>) : RoomMembersState
}
```

**JoinedRoom Member Methods:**
```kotlin
val membersStateFlow: StateFlow<RoomMembersState>
suspend fun updateMembers()
suspend fun inviteUserById(id: UserId): Result<Unit>
suspend fun kickUser(userId: UserId, reason: String?): Result<Unit>
suspend fun banUser(userId: UserId, reason: String?): Result<Unit>
suspend fun unbanUser(userId: UserId, reason: String?): Result<Unit>
```

### Member List Fetcher

```
libraries/matrix/impl/src/main/kotlin/io/element/android/libraries/matrix/impl/room/member/RoomMemberListFetcher.kt
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/room/RoomMember.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/room/RoomMembershipState.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/room/RoomMembersState.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/room/MemberListView.kt
```

---

## 15. Typing Notifications

### API Layer

**JoinedRoom Method:**
```kotlin
suspend fun typingNotice(isTyping: Boolean): Result<Unit>
```

**Flow for Other Users:**
```kotlin
val roomTypingMembersFlow: Flow<List<UserId>>
```

### UX Component

```
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/typing/TypingNotificationPresenter.kt
features/messages/impl/src/main/kotlin/io/element/android/features/messages/impl/typing/TypingNotificationView.kt
```

**State:**
```kotlin
data class TypingNotificationState(
    val typingUsers: ImmutableList<UserId>,
    val isDirect: Boolean,
)
```

### Insertion into Timeline

In `TimelinePresenter.kt`, typing notification is added as a virtual item:
```kotlin
// Typing notification appears as VirtualTimelineItem at top of timeline
data class TypingNotification(val users: List<UserId>) : VirtualTimelineItemModel
```

### Poziomki Target Files

Create:
```
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/timeline/TypingNotificationView.kt
```

Extend `JoinedRoom.kt`:
```kotlin
val roomTypingMembersFlow: Flow<List<UserId>>
```

---

## 16. Room Creation & DM

### API Layer

**MatrixClient Methods:**
```kotlin
suspend fun createRoom(createRoomParams: CreateRoomParameters): Result<RoomId>
suspend fun createDM(userId: UserId): Result<RoomId>
suspend fun findDM(userId: UserId): Result<RoomId?>
```

**Create Room Parameters:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/createroom/CreateRoomParameters.kt
```
```kotlin
data class CreateRoomParameters(
    val name: String?,
    val topic: String?,
    val isEncrypted: Boolean,
    val isDirect: Boolean,
    val visibility: RoomVisibility,
    val preset: RoomPreset,
    val invite: List<UserId>?,
    val powerLevelContentOverride: PowerLevelContentOverride?,
    val joinRuleOverride: JoinRuleOverride?,
    val historyVisibilityOverride: RoomHistoryVisibility?,
    val canonicalAlias: RoomAlias?,
    val isSpace: Boolean,
)
```

### Rust SDK Usage

In `RustMatrixClient.kt`:
```kotlin
private suspend fun createRoomInternal(
    name: String?,
    invitedUserIds: List<String>,
    isDirect: Boolean,
    preset: RoomPreset,
): String {
    return innerClient.createRoom(
        CreateRoomParameters(
            name = name,
            topic = null,
            isEncrypted = true,  // Always encrypt
            isDirect = isDirect,
            visibility = RoomVisibility.Private,
            preset = preset,
            invite = invitedUserIds.takeIf { it.isNotEmpty() },
            // ...
        ),
    )
}
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/room/CreateRoomParameters.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/NewChatViewModel.kt
```

---

## 17. In-Reply-To (Message Replies)

### API Layer

**InReplyTo Model:**
```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/timeline/item/event/InReplyTo.kt
```
```kotlin
sealed interface InReplyTo {
    data class NotLoaded(val eventId: EventId) : InReplyTo
    data class Loading(val eventId: EventId) : InReplyTo
    data class Ready(
        val eventId: EventId,
        val senderId: UserId,
        val senderProfile: ProfileDetails?,
        val content: EventContent,
        val threadInfo: EventThreadInfo?,
    ) : InReplyTo
    data object Error : InReplyTo
}
```

**Timeline Methods:**
```kotlin
suspend fun replyMessage(
    repliedToEventId: EventId,
    body: String,
    htmlBody: String?,
    intentionalMentions: List<IntentionalMention>,
): Result<Unit>

suspend fun loadReplyDetails(eventId: EventId): InReplyTo
```

### UX Components

**Reply Preview in Composer:**
```
libraries/textcomposer/impl/src/main/kotlin/io/element/android/libraries/textcomposer/ComposerModeView.kt
```

**In-Reply-To Details:**
```
libraries/matrix/ui/src/main/kotlin/io/element/android/libraries/matrix/ui/messages/reply/InReplyToDetails.kt
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/timeline/InReplyTo.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/timeline/ReplyPreview.kt
```

---

## 18. Sliding Sync (Native)

### API Layer

**Version Check:**
```kotlin
suspend fun currentSlidingSyncVersion(): Result<SlidingSyncVersion>

enum class SlidingSyncVersion {
    NATIVE,  // Sliding sync proxy or native
    LEGACY,
}
```

### Rust SDK Setup

In `RustMatrixClient.kt`:
```kotlin
newClient.restoreSession(
    Session(
        // ...
        slidingSyncVersion = SlidingSyncVersion.NATIVE,
    ),
)
```

### Room List Integration

Sliding sync is transparent - `RoomListService` handles it internally:
- Efficient room list updates
- Partial sync support
- Better battery life

---

## 19. Intentional Mentions

### API Layer

```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/room/IntentionalMention.kt
```
```kotlin
sealed interface IntentionalMention {
    data class User(val userId: UserId) : IntentionalMention
    data object Room : IntentionalMention  // @room mention
}
```

### Usage

When sending messages:
```kotlin
suspend fun sendMessage(
    body: String,
    htmlBody: String?,
    intentionalMentions: List<IntentionalMention>,
): Result<Unit>
```

The SDK tracks mentions in the rich text editor and extracts them automatically.

---

## 20. Feature Flags System

### API Layer

**Feature Flag Service:**
```
libraries/featureflag/api/src/main/kotlin/io/element/android/libraries/featureflag/api/FeatureFlagService.kt
```
```kotlin
interface FeatureFlagService {
    suspend fun isFeatureEnabled(feature: Feature): Boolean
    fun isFeatureEnabledFlow(feature: Feature): Flow<Boolean>
}

enum class Feature {
    TimelineTopics,        // Topics in timeline
    VoiceMessage,          // Voice messages
    LocationSharing,       // Location sharing
    Polls,                 // Polls
    VoiceBroadcast,        // Voice broadcasts
    Widgets,               // Widgets
    Calls,                 // Element Call
    Threads,               // Threads
    Knock,                 // Knock feature
    SdkIntegrity,          // SDK integrity checks
    CachingIdentityOnLogin, // Identity caching
    UnreadBackwards,       // Backward unread marker
    IosNotifications,      // iOS-specific
}
```

**Developer Preferences:**
```
libraries/featureflag/impl/src/main/kotlin/io/element/android/libraries/featureflag/impl/DeveloperPreferencesStore.kt
```
```kotlin
interface DeveloperPreferencesStore {
    val featuresEnabledState: Flow<Map<Feature, Boolean>>
    suspend fun setFeatureEnabled(feature: Feature, enabled: Boolean)
}
```

### Usage Pattern

```kotlin
@Composable
fun TimelineScreen(
    featureFlagService: FeatureFlagService,
) {
    val pollsEnabled by featureFlagService.isFeatureEnabledFlow(Feature.Polls)
        .collectAsState(initial = false)
    
    if (pollsEnabled) {
        // Show poll creation button
    }
}
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/featureflag/FeatureFlagService.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/featureflag/Feature.kt
mobile/shared/src/androidMain/kotlin/com/poziomki/app/featureflag/FeatureFlagServiceImpl.kt
```

---

## 21. Session & App Preferences

### Session Preferences

**Session Preferences Store:**
```
libraries/session-storage/api/src/main/kotlin/io/element/android/libraries/session-storage/api/SessionPreferencesStore.kt
```
```kotlin
interface SessionPreferencesStore {
    // Read receipts
    suspend fun isReadReceiptsEnabled(): Boolean
    suspend fun setReadReceiptsEnabled(enabled: Boolean)
    
    // Typing notifications
    suspend fun isSendTypingNotificationsEnabled(): Boolean
    suspend fun setSendTypingNotificationsEnabled(enabled: Boolean)
    
    // Timeline settings
    suspend fun isShowTimestampOnMessagesEnabled(): Boolean
    suspend fun setShowTimestampOnMessagesEnabled(enabled: Boolean)
    
    // Media settings
    suspend fun isAutoPlayAnimatedImagesEnabled(): Boolean
    suspend fun setAutoPlayAnimatedImagesEnabled(enabled: Boolean)
    
    // Notification settings
    suspend fun isNotificationsEnabled(): Boolean
    suspend fun setNotificationsEnabled(enabled: Boolean)
    
    // Data
    val preferencesFlow: Flow<SessionPreferences>
    suspend fun clear()
}
```

**Session Preferences Model:**
```kotlin
data class SessionPreferences(
    val readReceiptsEnabled: Boolean = true,
    val sendTypingNotificationsEnabled: Boolean = true,
    val showTimestampOnMessagesEnabled: Boolean = false,
    val autoPlayAnimatedImagesEnabled: Boolean = true,
    val notificationsEnabled: Boolean = true,
)
```

### App Preferences

**App Preferences Store:**
```
libraries/preferences/api/src/main/kotlin/io/element/android/libraries/preferences/api/AppPreferencesStore.kt
```
```kotlin
interface AppPreferencesStore {
    // Theme
    suspend fun getTheme(): Theme
    suspend fun setTheme(theme: Theme)
    
    // Developer mode
    suspend fun isDeveloperModeEnabled(): Boolean
    suspend fun setDeveloperModeEnabled(enabled: Boolean)
    
    // Analytics
    suspend fun isAnalyticsEnabled(): Boolean
    suspend fun setAnalyticsEnabled(enabled: Boolean)
    
    // Onboarding
    suspend fun isOnboardingShown(): Boolean
    suspend fun setOnboardingShown(shown: Boolean)
    
    // Flows
    val themeFlow: Flow<Theme>
    val developerModeFlow: Flow<Boolean>
}

enum class Theme {
    System,
    Light,
    Dark,
}
```

### Usage Pattern

```kotlin
@Composable
fun MessageComposer(
    sessionPreferencesStore: SessionPreferencesStore,
) {
    val sendTyping by remember {
        sessionPreferencesStore.isSendTypingNotificationsEnabled()
    }.collectAsState(initial = true)
    
    // On text change
    if (sendTyping) {
        room.typingNotice(isTyping = true)
    }
}
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/preferences/SessionPreferencesStore.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/preferences/AppPreferencesStore.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/preferences/SessionPreferences.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/preferences/Theme.kt
mobile/shared/src/androidMain/kotlin/com/poziomki/app/preferences/DataStorePreferences.kt
```

---

## 22. Architecture Patterns

### Presenter Pattern

**Base Presenter Interface:**
```
libraries/architecture/src/main/kotlin/io/element/android/libraries/architecture/Presenter.kt
```
```kotlin
interface Presenter<State> {
    @Composable
    fun present(): State
}
```

**State-Event Pattern:**
```kotlin
data class SomeState(
    val data: AsyncData<Data>,
    val eventSink: (SomeEvent) -> Unit,
)

sealed interface SomeEvent {
    data object Load : SomeEvent
    data class Update(val value: String) : SomeEvent
    data object Reset : SomeEvent
}
```

**Example Presenter Implementation:**
```kotlin
class RoomListPresenter @Inject constructor(
    private val roomListDataSource: RoomListDataSource,
    private val encryptionService: EncryptionService,
) : Presenter<RoomListState> {
    
    @Composable
    override fun present(): RoomListState {
        val coroutineScope = rememberCoroutineScope()
        
        // Collect state
        val roomSummaries by produceState(initialValue = AsyncData.Loading()) {
            roomListDataSource.roomSummariesFlow.collect { 
                value = AsyncData.Success(it) 
            }
        }
        
        val recoveryState by encryptionService.recoveryStateStateFlow
            .collectAsState()
        
        // Handle events
        fun handleEvent(event: RoomListEvent) {
            when (event) {
                is RoomListEvent.LoadMore -> coroutineScope.launch {
                    roomListDataSource.loadMore()
                }
                is RoomListEvent.SelectRoom -> {
                    // Navigate to room
                }
            }
        }
        
        return RoomListState(
            roomSummaries = roomSummaries,
            securityBannerState = when (recoveryState) {
                RecoveryState.INCOMPLETE -> SecurityBannerState.RecoveryKeyConfirmation
                else -> SecurityBannerState.None
            },
            eventSink = ::handleEvent,
        )
    }
}
```

### AsyncData Pattern

**AsyncData Sealed Interface:**
```
libraries/architecture/src/main/kotlin/io/element/android/libraries/architecture/AsyncData.kt
```
```kotlin
sealed interface AsyncData<out T> {
    data object Loading : AsyncData<Nothing>
    data class Success<T>(val data: T) : AsyncData<T>
    data class Failure(val error: Throwable) : AsyncData<Nothing>
    
    fun isLoading(): Boolean = this is Loading
    fun isSuccess(): Boolean = this is Success
    fun isFailure(): Boolean = this is Failure
    
    fun dataOrNull(): T? = (this as? Success)?.data
    fun errorOrNull(): Throwable? = (this as? Failure)?.error
    
    companion object {
        operator fun <T> invoke(block: suspend () -> T): Flow<AsyncData<T>> = flow {
            emit(Loading)
            try {
                emit(Success(block()))
            } catch (e: Exception) {
                emit(Failure(e))
            }
        }
    }
}
```

**Usage in UI:**
```kotlin
@Composable
fun <T> AsyncDataView(
    asyncData: AsyncData<T>,
    onSuccess: @Composable (T) -> Unit,
    loading: @Composable () -> Unit = { CircularProgressIndicator() },
    onError: @Composable (Throwable) -> Unit = { ErrorView(it) },
) {
    when (asyncData) {
        is AsyncData.Loading -> loading()
        is AsyncData.Success -> onSuccess(asyncData.data)
        is AsyncData.Failure -> onError(asyncData.error)
    }
}
```

### AsyncAction Pattern

**AsyncAction for One-Time Operations:**
```
libraries/architecture/src/main/kotlin/io/element/android/libraries/architecture/AsyncAction.kt
```
```kotlin
sealed interface AsyncAction<out T> {
    data object Uninitialized : AsyncAction<Nothing>
    data object Loading : AsyncAction<Nothing>
    data class Success<T>(val data: T) : AsyncAction<T>
    data class Failure(val error: Throwable) : AsyncAction<Nothing>
    
    fun isUninitialized(): Boolean = this is Uninitialized
    fun isLoading(): Boolean = this is Loading
    fun isSuccess(): Boolean = this is Success
    fun isFailure(): Boolean = this is Failure
}
```

**Usage for Form Submission:**
```kotlin
@Composable
fun LoginFormPresenter(
    loginUseCase: LoginUseCase,
): LoginFormState {
    val coroutineScope = rememberCoroutineScope()
    var loginAction by remember { mutableStateOf<AsyncAction<Unit>>(AsyncAction.Uninitialized) }
    
    fun handleEvent(event: LoginFormEvent) {
        when (event) {
            is LoginFormEvent.Submit -> coroutineScope.launch {
                loginAction = AsyncAction.Loading
                loginAction = loginUseCase(event.username, event.password)
                    .fold(
                        onSuccess = { AsyncAction.Success(Unit) },
                        onFailure = { AsyncAction.Failure(it) },
                    )
            }
            is LoginFormEvent.Reset -> {
                loginAction = AsyncAction.Uninitialized
            }
        }
    }
    
    return LoginFormState(
        loginAction = loginAction,
        eventSink = ::handleEvent,
    )
}
```

### Impression Event Tracking

```kotlin
@Composable
fun <T> rememberImpressionEvent(
    item: T,
    trigger: ImpressionTrigger = ImpressionTrigger.Visible,
    onImpression: (T) -> Unit,
): Modifier {
    return Modifier.onVisible(
        onVisible = { onImpression(item) },
    )
}
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/architecture/Presenter.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/architecture/AsyncData.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/architecture/AsyncAction.kt
```

---

## 23. Design System Components

### Core Components

**Buttons:**
```
libraries/designsystem/src/main/kotlin/io/element/android/libraries/designsystem/components/buttons/
```
```kotlin
// Primary button
@Composable
fun Button(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    loading: Boolean = false,
    destructive: Boolean = false,
)

// Icon button
@Composable
fun IconButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    icon: IconSource,
)

// FAB
@Composable
fun FloatingActionButton(
    onClick: () -> Unit,
    icon: IconSource,
    contentDescription: String?,
)
```

**Inputs:**
```
libraries/designsystem/src/main/kotlin/io/element/android/libraries/designsystem/components/form/
```
```kotlin
@Composable
fun TextField(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    label: String? = null,
    placeholder: String? = null,
    leadingIcon: IconSource? = null,
    trailingIcon: IconSource? = null,
    isError: Boolean = false,
    errorMessage: String? = null,
    singleLine: Boolean = true,
)

@Composable
fun SearchField(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    placeholder: String = "Search...",
)
```

**Dialogs:**
```
libraries/designsystem/src/main/kotlin/io/element/android/libraries/designsystem/components/dialogs/
```
```kotlin
@Composable
fun AlertDialog(
    onDismissRequest: () -> Unit,
    title: String?,
    content: @Composable ColumnScope.() -> Unit,
    buttons: @Composable RowScope.() -> Unit,
)

@Composable
fun ConfirmDialog(
    onDismissRequest: () -> Unit,
    onConfirmation: () -> Unit,
    title: String,
    text: String,
    confirmText: String = "Confirm",
    dismissText: String = "Cancel",
    destructive: Boolean = false,
)
```

**List Items:**
```
libraries/designsystem/src/main/kotlin/io/element/android/libraries/designsystem/components/listItems/
```
```kotlin
@Composable
fun ListItem(
    headline: String,
    modifier: Modifier = Modifier,
    leadingContent: @Composable (() -> Unit)? = null,
    trailingContent: @Composable (() -> Unit)? = null,
    supportingText: String? = null,
    onClick: (() -> Unit)? = null,
)

@Composable
fun ListItemSwitch(
    headline: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
    supportingText: String? = null,
    enabled: Boolean = true,
)
```

### Avatars

```
libraries/designsystem/src/main/kotlin/io/element/android/libraries/designsystem/components/avatar/
```
```kotlin
@Composable
fun Avatar(
    avatarData: AvatarData,
    modifier: Modifier = Modifier,
    contentDescription: String? = null,
)

data class AvatarData(
    val id: String,
    val name: String?,
    val url: String?,
    val size: AvatarSize,
)

enum class AvatarSize {
    Tiny,      // 16dp
    ExtraTiny, // 20dp
    Small,     // 24dp
    Medium,    // 32dp
    Large,     // 44dp
    Larger,    // 56dp
    ExtraLarge, // 80dp
    Custom,    // Custom size
}
```

### Typography & Colors

```
libraries/designsystem/src/main/kotlin/io/element/android/libraries/designsystem/theme/
```

```kotlin
// Typography
@Immutable
data class Typography(
    val fontFamily: FontFamily,
    val headlineLarge: TextStyle,
    val headlineMedium: TextStyle,
    val headlineSmall: TextStyle,
    val titleLarge: TextStyle,
    val titleMedium: TextStyle,
    val titleSmall: TextStyle,
    val bodyLarge: TextStyle,
    val bodyMedium: TextStyle,
    val bodySmall: TextStyle,
    val labelLarge: TextStyle,
    val labelMedium: TextStyle,
    val labelSmall: TextStyle,
)

// Colors
@Immutable
data class ElementColors(
    val primary: Color,
    val onPrimary: Color,
    val secondary: Color,
    val onSecondary: Color,
    val error: Color,
    val onError: Color,
    val background: Color,
    val onBackground: Color,
    val surface: Color,
    val onSurface: Color,
    val border: Color,
    val borderVariant: Color,
    // Semantic colors
    val textPrimary: Color,
    val textSecondary: Color,
    val textTertiary: Color,
    val accent: Color,
    val success: Color,
    val warning: Color,
    val info: Color,
)
```

### Poziomki Target Files

Create or extend:
```
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/theme/
├── Theme.kt
├── Color.kt
├── Typography.kt
├── Shape.kt

mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/
├── button/Button.kt
├── button/IconButton.kt
├── input/TextField.kt
├── input/SearchField.kt
├── dialog/AlertDialog.kt
├── dialog/ConfirmDialog.kt
├── list/ListItem.kt
├── avatar/Avatar.kt
```

---

## 24. Error Handling Patterns

### Error Kind Classification

```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/error/ErrorKind.kt
```
```kotlin
sealed interface ErrorKind {
    // Network
    data object Network : ErrorKind
    data object Timeout : ErrorKind
    
    // Server
    data object Server : ErrorKind
    data object RateLimited : ErrorKind
    
    // Auth
    data object Unauthorized : ErrorKind
    data object Forbidden : ErrorKind
    
    // Room
    data object RoomNotFound : ErrorKind
    data object UserNotInRoom : ErrorKind
    
    // Crypto
    data object VerificationRequired : ErrorKind
    data object CrossSigningNotSetup : ErrorKind
    
    // Unknown
    data object Unknown : ErrorKind
}

fun Throwable.toErrorKind(): ErrorKind {
    return when (this) {
        is UnknownHostException -> ErrorKind.Network
        is SocketTimeoutException -> ErrorKind.Timeout
        is HttpException -> when (code()) {
            401 -> ErrorKind.Unauthorized
            403 -> ErrorKind.Forbidden
            404 -> ErrorKind.RoomNotFound
            429 -> ErrorKind.RateLimited
            in 500..599 -> ErrorKind.Server
            else -> ErrorKind.Unknown
        }
        else -> ErrorKind.Unknown
    }
}
```

### Client Exception

```
libraries/matrix/api/src/main/kotlin/io/element/android/libraries/matrix/api/error/ClientException.kt
```
```kotlin
sealed class ClientException : Exception {
    // Generic
    data class Generic(val cause: Throwable) : ClientException()
    
    // Auth
    data class Unauthorized(val message: String) : ClientException()
    data class Forbidden(val message: String) : ClientException()
    
    // Network
    data class NetworkError(val cause: Throwable) : ClientException()
    data class Timeout(val cause: Throwable) : ClientException()
    
    // Room
    data class RoomNotFound(val roomId: RoomId) : ClientException()
    
    // Crypto
    data class VerificationRequired(val deviceId: DeviceId?) : ClientException()
    
    // Sliding Sync
    data class SlidingSyncError(val message: String) : ClientException()
}
```

### Error Display Pattern

```kotlin
@Composable
fun ErrorMessage(
    error: Throwable,
    onRetry: (() -> Unit)? = null,
    modifier: Modifier = Modifier,
) {
    val errorKind = remember(error) { error.toErrorKind() }
    
    Column(
        modifier = modifier,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Icon(
            imageVector = when (errorKind) {
                ErrorKind.Network, ErrorKind.Timeout -> Icons.Outlined.WifiOff
                ErrorKind.Server -> Icons.Outlined.CloudOff
                ErrorKind.VerificationRequired -> Icons.Outlined.VerifiedUser
                else -> Icons.Outlined.Error
            },
            contentDescription = null,
        )
        
        Text(
            text = when (errorKind) {
                ErrorKind.Network -> "No internet connection"
                ErrorKind.Timeout -> "Request timed out"
                ErrorKind.Server -> "Server error. Try again later."
                ErrorKind.RateLimited -> "Too many requests. Wait a moment."
                ErrorKind.Unauthorized -> "Session expired. Please log in."
                ErrorKind.Forbidden -> "You don't have permission."
                ErrorKind.VerificationRequired -> "Device verification required"
                else -> "An error occurred"
            },
        )
        
        if (onRetry != null) {
            Button(onClick = onRetry) {
                Text("Retry")
            }
        }
    }
}
```

### Global Error Handler

```kotlin
@Composable
fun rememberGlobalErrorHandler(): (Throwable) -> Unit {
    val context = LocalContext.current
    val snackbarHostState = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()
    
    return { error ->
        scope.launch {
            val message = when (error.toErrorKind()) {
                ErrorKind.Network -> "No internet connection"
                ErrorKind.Timeout -> "Request timed out"
                else -> error.message ?: "An error occurred"
            }
            snackbarHostState.showSnackbar(
                message = message,
                duration = SnackbarDuration.Short,
            )
        }
    }
}
```

### Poziomki Target Files

Create:
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/error/ErrorKind.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/error/ClientException.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/error/ErrorExtensions.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/error/ErrorMessage.kt
mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/error/ErrorView.kt
```

---

## 25. Analytics Service

### API Layer

```
libraries/analytics/api/src/main/kotlin/io/element/android/libraries/analytics/api/AnalyticsService.kt
```
```kotlin
interface AnalyticsService {
    val userProperties: UserProperties
    val consentState: StateFlow<AnalyticsConsentState>
    
    suspend fun setConsentState(state: AnalyticsConsentState)
    suspend fun setUserProperty(property: UserProperty, value: Any?)
    suspend fun track(event: AnalyticsEvent)
    suspend fun screen(name: String, properties: Map<String, Any?> = emptyMap())
    suspend fun flush()
}

sealed interface AnalyticsConsentState {
    data object Unknown : AnalyticsConsentState
    data object Declined : AnalyticsConsentState
    data class Accepted(val userAskedForConsent: Boolean) : AnalyticsConsentState
}
```

### Analytics Events

```
libraries/analytics/api/src/main/kotlin/io/element/android/libraries/analytics/api/events/
```
```kotlin
sealed interface AnalyticsEvent {
    // Composer
    data class Composer(
        val inThread: Boolean,
        val isEditing: Boolean,
        val isReply: Boolean,
        val startsThread: Boolean,
    ) : AnalyticsEvent
    
    // Room
    data class RoomCreated(val isDM: Boolean, val isSpace: Boolean) : AnalyticsEvent
    data class JoinedRoom(val isDM: Boolean, val isSpace: Boolean, val activeMembers: Int) : AnalyticsEvent
    
    // Timeline
    data object MessageSent : AnalyticsEvent
    data object ImageSent : AnalyticsEvent
    data object VideoSent : AnalyticsEvent
    data object FileSent : AnalyticsEvent
    data object ReactionAdded : AnalyticsEvent
    
    // E2EE
    data class VerificationStarted(val method: String) : AnalyticsEvent
    data object VerificationCompleted : AnalyticsEvent
    data object RecoveryKeyCreated : AnalyticsEvent
    data object RecoveryKeyEntered : AnalyticsEvent
    
    // Navigation
    data class ScreenView(val screenName: String) : AnalyticsEvent
    
    // Errors
    data class Error(val domain: String, val name: String) : AnalyticsEvent
}
```

### Usage Pattern

```kotlin
@Composable
fun MessageComposer(
    analyticsService: AnalyticsService,
    room: JoinedRoom,
) {
    fun onSendMessage(message: Message) {
        analyticsService.track(
            AnalyticsEvent.Composer(
                inThread = false,
                isEditing = false,
                isReply = composerMode is MessageComposerMode.Reply,
                startsThread = false,
            )
        )
        room.sendMessage(message)
    }
}

@Composable
fun ScreenTracker(
    screenName: String,
    analyticsService: AnalyticsService,
) {
    LaunchedEffect(screenName) {
        analyticsService.screen(screenName)
    }
}
```

### Poziomki Target Files

Create (if analytics needed):
```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/analytics/AnalyticsService.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/analytics/AnalyticsEvent.kt
mobile/shared/src/commonMain/kotlin/com/poziomki/app/analytics/AnalyticsConsentState.kt
mobile/shared/src/androidMain/kotlin/com/poziomki/app/analytics/PostHogAnalyticsService.kt
```

---

## Complete File Structure Summary

### Poziomki Target Architecture

```
mobile/shared/src/commonMain/kotlin/com/poziomki/app/
├── chat/
│   ├── matrix/
│   │   ├── api/
│   │   │   ├── MatrixClient.kt
│   │   │   ├── JoinedRoom.kt
│   │   │   ├── NotJoinedRoom.kt
│   │   │   ├── BaseRoom.kt
│   │   │   ├── RoomMember.kt
│   │   │   ├── RoomInfo.kt
│   │   │   ├── RoomMembersState.kt
│   │   │   ├── draft/
│   │   │   │   ├── ComposerDraft.kt
│   │   │   │   └── ComposerDraftType.kt
│   │   │   ├── encryption/
│   │   │   │   ├── EncryptionService.kt
│   │   │   │   ├── BackupState.kt
│   │   │   │   ├── RecoveryState.kt
│   │   │   │   └── IdentityState.kt
│   │   │   ├── media/
│   │   │   │   ├── MatrixMediaLoader.kt
│   │   │   │   ├── MediaUploadHandler.kt
│   │   │   │   ├── ImageInfo.kt
│   │   │   │   ├── VideoInfo.kt
│   │   │   │   ├── AudioInfo.kt
│   │   │   │   └── FileInfo.kt
│   │   │   ├── push/
│   │   │   │   ├── PushService.kt
│   │   │   │   └── PushersService.kt
│   │   │   ├── roomlist/
│   │   │   │   ├── RoomListService.kt
│   │   │   │   └── RoomSummary.kt
│   │   │   ├── sync/
│   │   │   │   ├── SyncService.kt
│   │   │   │   └── SyncState.kt
│   │   │   ├── timeline/
│   │   │   │   ├── Timeline.kt
│   │   │   │   ├── MatrixTimelineItem.kt
│   │   │   │   ├── EventContent.kt
│   │   │   │   ├── MessageType.kt
│   │   │   │   ├── VirtualTimelineItem.kt
│   │   │   │   ├── Receipt.kt
│   │   │   │   ├── EventReaction.kt
│   │   │   │   ├── InReplyTo.kt
│   │   │   │   ├── UtdCause.kt
│   │   │   │   └── SendQueueUpdate.kt
│   │   │   └── verification/
│   │   │       ├── SessionVerificationService.kt
│   │   │       ├── VerificationFlowState.kt
│   │   │       └── SessionVerificationData.kt
│   │   └── impl/ (Android-specific)
│   │       ├── RustMatrixClient.kt
│   │       ├── JoinedRustRoom.kt
│   │       ├── RustTimeline.kt
│   │       ├── media/RustMediaLoader.kt
│   │       ├── encryption/RustEncryptionService.kt
│   │       ├── verification/RustSessionVerificationService.kt
│   │       └── draft/RustComposerDraftStore.kt
│   ├── composer/
│   │   ├── MessageComposerMode.kt
│   │   ├── TextEditorState.kt
│   │   ├── Message.kt
│   │   └── Suggestion.kt
│   └── timeline/
│       └── TimelineItemAction.kt
└── data/
    └── sync/
        └── SyncEngine.kt

mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/
├── screen/
│   ├── chat/
│   │   ├── ChatScreen.kt
│   │   ├── ChatViewModel.kt
│   │   ├── NewChatScreen.kt
│   │   ├── NewChatViewModel.kt
│   │   └── model/ChatUiModels.kt
│   ├── main/
│   │   ├── MessagesScreen.kt
│   │   └── MessagesViewModel.kt
│   ├── verification/
│   │   ├── SessionVerificationScreen.kt
│   │   └── SessionVerificationViewModel.kt
│   └── recovery/
│       ├── RecoveryKeySetupScreen.kt
│       └── EnterRecoveryKeyScreen.kt
└── component/
    ├── timeline/
    │   ├── TimelineItemRow.kt
    │   ├── MessageBubble.kt
    │   ├── MessageContentView.kt
    │   ├── ReactionsView.kt
    │   ├── ReadReceiptView.kt
    │   ├── TypingNotificationView.kt
    │   ├── ReplyPreview.kt
    │   ├── ActionListBottomSheet.kt
    │   └── EmojiPicker.kt
    ├── composer/
    │   ├── TextComposer.kt
    │   ├── ComposerModeView.kt
    │   └── SuggestionsPicker.kt
    ├── roomlist/
    │   └── RoomSummaryRow.kt
    └── chat/
        ├── SendQueueErrorBanner.kt
        └── EncryptedMessageView.kt
```

---

## Implementation Roadmap

### Phase 1: Core Architecture (Foundation)

**Week 1-2**
| Task | Files | Dependency |
|------|-------|------------|
| AsyncData/AsyncAction patterns | `architecture/AsyncData.kt`, `AsyncAction.kt` | None |
| Error handling | `error/ErrorKind.kt`, `ClientException.kt` | None |
| Session preferences | `preferences/SessionPreferencesStore.kt` | None |
| Feature flags | `featureflag/FeatureFlagService.kt` | None |

### Phase 2: Draft & Media (Essential UX)

**Week 3-4**
| Task | Files | Dependency |
|------|-------|------------|
| Draft persistence (Rust SDK) | Extend `draft/RoomComposerDraftStore.kt` with Rust `Room.saveDraft()` | ✅ Has in-memory |
| Media download | `api/media/MatrixMediaLoader.kt` | Rust SDK |
| Video upload | Extend `api/Timeline.kt` with `sendVideo()` | ✅ Has sendImage/sendFile |
| Audio upload | Extend `api/Timeline.kt` with `sendAudio()` | Rust SDK |

### Phase 3: E2EE UX (Trust & Recovery)

**Week 5-6**
| Task | Files | Dependency |
|------|-------|------------|
| Encryption service | `api/encryption/EncryptionService.kt`, `BackupState.kt`, `RecoveryState.kt` | Rust SDK |
| Encryption impl | `impl/encryption/RustEncryptionService.kt` | Encryption service |
| Recovery key UI | `ui/screen/recovery/RecoveryKeySetupScreen.kt` | Encryption impl |
| Verification service | `api/verification/SessionVerificationService.kt` | Rust SDK |
| Verification impl | `impl/verification/RustSessionVerificationService.kt` | Verification service |
| Verification UI | `ui/screen/verification/SessionVerificationScreen.kt` | Verification impl |

### Phase 4: Push & Send Queue (Resilience)

**Week 7-8**
| Task | Files | Dependency |
|------|-------|------------|
| Pushers service | `api/push/PushersService.kt` | Rust SDK |
| Push service | `api/push/PushService.kt` | Pushers service |
| Send queue update | `api/room/SendQueueUpdate.kt`, `SendQueueError.kt` | Rust SDK |
| UTD handling | `api/timeline/UtdCause.kt`, `api/encryption/IdentityState.kt` | Send queue |
| Error banners | `ui/component/chat/SendQueueErrorBanner.kt` | UTD handling |

### Phase 5: Polish & Testing

**Week 9-10**
| Task | Description |
|------|-------------|
| Design system alignment | Match Element X component styling |
| Error messages | User-friendly error strings |
| Analytics (optional) | Event tracking infrastructure |
| Integration tests | E2E flow tests |
| Performance profiling | Timeline scrolling, media caching |

---

## Rust SDK Method Reference

### Client Methods
```kotlin
// Session
suspend fun restoreSession(session: Session): Client
suspend fun login(...): Client
suspend fun logout()

// Rooms
suspend fun createRoom(params: CreateRoomParameters): RoomId
suspend fun createDM(userId: UserId): RoomId
suspend fun findDM(userId: UserId): RoomId?
suspend fun joinRoomByIdOrAlias(id: String)
suspend fun knockRoom(id: String, message: String?)

// Media
suspend fun mediaRequest(request: MediaRequest): ByteArray
fun getMaxFileUploadSize(): Long

// Push
suspend fun setPusher(pusher: Pusher)
suspend fun getPushers(): List<Pusher>

// Sync
fun syncService(): SyncService
fun roomListService(): RoomListService

// Crypto
fun encryption(): Encryption
fun getSessionVerificationController(): SessionVerificationController

// Send Queue
suspend fun setSendQueuesEnabled(enabled: Boolean)
fun sendQueueDisabledFlow(): Flow<RoomId>
```

### Room Methods
```kotlin
// Timeline
suspend fun timeline(): Timeline
suspend fun timelineFocused(eventId: EventId): Timeline

// Send
suspend fun send(msg: String, html: String?, mentions: List<IntentionalMention>)
suspend fun sendImage(file: File, imageInfo: ImageInfo): MediaUploadHandler
suspend fun sendVideo(file: File, videoInfo: VideoInfo): MediaUploadHandler
suspend fun sendFile(file: File, fileInfo: FileInfo): MediaUploadHandler

// Draft
suspend fun saveDraft(draft: ComposerDraft)
suspend fun loadDraft(): ComposerDraft?

// Members
suspend fun members(): List<RoomMember>
suspend fun inviteUser(userId: UserId)
suspend fun kickUser(userId: UserId, reason: String?)
suspend fun banUser(userId: UserId, reason: String?)

// Settings
suspend fun setName(name: String)
suspend fun setTopic(topic: String)
suspend fun updateAvatar(url: String)

// Send Queue
suspend fun setSendQueueEnabled(enabled: Boolean)
fun sendQueueUpdates(): Flow<SendQueueUpdate>
suspend fun ignoreDeviceTrustAndResend(...)
suspend fun withdrawVerificationAndResend(...)

// Typing
suspend fun typingNotice(isTyping: Boolean)
fun typingMembersFlow(): Flow<List<UserId>>
```

### Timeline Methods
```kotlin
// Items
fun itemsFlow(): Flow<List<TimelineItem>>
suspend fun paginate(direction: PaginationDirection, count: Int)

// Send
suspend fun sendMessage(body: String, html: String?, mentions: List<IntentionalMention>)
suspend fun toggleReaction(emoji: String, eventId: EventId): Boolean
suspend fun redactEvent(eventId: EventId, reason: String?)

// Read receipts
suspend fun sendReadReceipt(eventId: EventId, receiptType: ReceiptType)
suspend fun markAsRead(receiptType: ReceiptType)

// Reply/Edit
suspend fun replyMessage(replyToId: EventId, body: String, html: String?)
suspend fun editMessage(eventId: EventId, body: String, html: String?)
suspend fun loadReplyDetails(eventId: EventId): InReplyTo

// Media
suspend fun sendImage(...)
suspend fun sendVideo(...)
suspend fun sendFile(...)
```

### Encryption Methods
```kotlin
// Backup
suspend fun enableBackups()
suspend fun enableRecovery(waitForBackups: Boolean)
suspend fun disableRecovery()
suspend fun recover(recoveryKey: String)
suspend fun resetRecoveryKey(): String
fun backupState(): BackupState
fun recoveryState(): RecoveryState
fun backupStateListener(): Flow<BackupState>

// Identity
suspend fun getUserIdentity(userId: UserId): UserIdentity?
suspend fun pinUserIdentity(userId: UserId)
```

### SessionVerificationController Methods
```kotlin
suspend fun requestDeviceVerification(deviceId: DeviceId)
suspend fun requestUserVerification(userId: UserId)
suspend fun startSasVerification()
suspend fun approveVerification()
suspend fun declineVerification()
suspend fun cancelVerification()
fun verificationState(): VerificationState
fun verificationStateListener(): Flow<VerificationState>
```

---

## Quick Links

### Element X Source Repos
- Main: `element-x-android-chat/`
- Matrix API: `libraries/matrix/api/`
- Matrix Impl: `libraries/matrix/impl/`
- Features: `features/*/impl/`
- Design System: `libraries/designsystem/`

### Poziomki Target Paths
- Matrix API: `mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/`
- Matrix Impl: `mobile/shared/src/androidMain/kotlin/com/poziomki/app/chat/matrix/impl/`
- UI Screens: `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/`
- UI Components: `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/component/`

### Key Documents
- Gap Analysis: `MATRIX_12_02.md`
- Progress: `MATRIX_PROGRESS.md`
- Port Map: `CHAT_PORT_MAP.md`

---

## Verification Report (2026-02-12)

### What Poziomki Actually Has

| Feature | Status | File Location |
|---------|--------|---------------|
| **MatrixClient** | ✅ Basic | `api/MatrixClient.kt` (58 lines) |
| **JoinedRoom** | ✅ Basic | `api/JoinedRoom.kt` (25 lines) |
| **Timeline** | ✅ Basic | `api/Timeline.kt` (108 lines) |
| **Rust SDK Wrappers** | ✅ Working | `impl/RustMatrixClient.kt`, `impl/JoinedRustRoom.kt`, `impl/RustTimeline.kt` |
| **Draft Store** | ✅ In-Memory | `draft/RoomComposerDraftStore.kt`, `draft/InMemoryRoomComposerDraftStore.kt` |
| **ComposerMode** | ✅ Full | `ChatUiModels.kt` - NewMessage, Reply, Edit |
| **sendImage** | ✅ Implemented | `impl/RustTimeline.kt:164-201` |
| **sendFile** | ✅ Implemented | `impl/RustTimeline.kt:203-236` |
| **Sliding Sync** | ✅ Native | `impl/RustMatrixClient.kt:147` uses `SlidingSyncVersion.NATIVE` |
| **E2EE** | ✅ Automatic | Rooms created with `isEncrypted = true` (`impl/RustMatrixClient.kt:339`) |
| **Typing Notifications** | ✅ Implemented | `impl/JoinedRustRoom.kt:43-51` |
| **Typing Debounce** | ✅ Implemented | `ChatViewModel.kt:341-384` - start/stop timers |
| **Reactions** | ✅ Implemented | `api/Timeline.kt:97-100`, `impl/RustTimeline.kt:259-262` |
| **Edit Message** | ✅ Implemented | `api/Timeline.kt:87-90`, `impl/RustTimeline.kt:238-248` |
| **Redact** | ✅ Implemented | `api/Timeline.kt:92-95`, `impl/RustTimeline.kt:250-257` |
| **Reply** | ✅ Implemented | `api/Timeline.kt:66-69`, `impl/RustTimeline.kt:155-162` |
| **Read Receipts** | ✅ Implemented | `api/Timeline.kt:102-104`, `impl/RustTimeline.kt:264-274` |
| **DM Creation** | ✅ Implemented | `api/MatrixClient.kt:46-49`, `impl/RustMatrixClient.kt:203-219` |
| **Room Creation** | ✅ Implemented | `api/MatrixClient.kt:51-54`, `impl/RustMatrixClient.kt:221-233` |
| **Focused Timeline** | ✅ Implemented | `api/JoinedRoom.kt:17`, `impl/JoinedRustRoom.kt:70-95` |
| **TimelineController** | ✅ Implemented | `timeline/TimelineController.kt` - mode management |
| **Media Caption** | ✅ Implemented | `ChatViewModel.kt:395` - caption with attachment |

### What's Actually Missing (Priority Order)

| P1 Feature | Element X Has | Poziomki | Rust SDK Support |
|------------|---------------|----------|------------------|
| **Encryption Service** | `EncryptionService` interface | ❌ None | ✅ `Encryption` class with `backupState()`, `recoveryState()`, `enableRecovery()` |
| **Session Verification** | `SessionVerificationService` | ❌ None | ✅ `SessionVerificationController` with `requestDeviceVerification()`, `startSasVerification()` |
| **Push Notifications** | `PushService`, `PushersService` | ❌ None | ✅ `setPusher()`, notification client |
| **Send Queue Management** | `SendQueueUpdate`, error recovery | ❌ None | ✅ `setSendQueuesEnabled()`, `sendQueueDisabledFlow()` |
| **UTD Error Handling** | `UtdCause`, `MessageShield` | ❌ None | ✅ Part of `TimelineItemContent.UnableToDecrypt` |
| **Draft Persistence** | `ComposerDraft` with type (Reply/Edit) | ⚠️ In-memory only | ✅ `Room.saveDraft()`, `Room.loadDraft()` |
| **Media Download** | `MatrixMediaLoader` | ❌ None | ✅ `Client.mediaRequest()` |
| **Video Upload** | `sendVideo()` | ❌ None | ✅ `Timeline.sendVideo()` |
| **Audio Upload** | `sendAudio()` | ❌ None | ✅ `Timeline.sendAudio()` |
| **Room Members Full** | `membersStateFlow`, `kickUser`, `banUser` | ❌ None | ✅ `Room.members()`, `kickUser()`, `banUser()` |
| **Notification Settings** | `NotificationSettingsService` | ❌ None | ✅ Via Rust SDK |

### Corrections to Documentation

1. **Draft System**: Poziomki HAS a draft store (`RoomComposerDraftStore`) - it's in-memory only, not missing. Need to add persistent Rust SDK-backed implementation.

2. **Media Upload**: Poziomki HAS `sendImage()` and `sendFile()` in Timeline API. Missing: `sendVideo()`, `sendAudio()`, `sendVoiceMessage()`.

3. **E2EE Clarification**: Core E2EE works (encryption/decryption automatic). Missing: user-facing verification flows, key backup/recovery UI.

### Element X API Surface vs Poziomki

| Interface | Element X Methods | Poziomki Methods | Coverage |
|-----------|-------------------|------------------|----------|
| `MatrixClient` | ~40 | 6 | 15% |
| `JoinedRoom` | ~25 | 5 | 20% |
| `Timeline` | ~25 | 12 | 48% |

### Rust SDK Method Availability (Verified)

```kotlin
// ENCRYPTION - Available in Rust SDK
Encryption.backupState(): BackupState
Encryption.recoveryState(): RecoveryState
Encryption.enableBackups()
Encryption.enableRecovery(waitForBackupsToUpload: Boolean, passphrase: String?): String
Encryption.recover(recoveryKey: String)
Encryption.resetRecoveryKey(): String
Encryption.backupExistsOnServer(): Boolean

// VERIFICATION - Available in Rust SDK
Client.getSessionVerificationController(): SessionVerificationController
SessionVerificationController.requestDeviceVerification()
SessionVerificationController.startSasVerification()
SessionVerificationController.approveVerification()
SessionVerificationController.declineVerification()
SessionVerificationController.cancelVerification()
SessionVerificationController.setDelegate(delegate: SessionVerificationControllerDelegate)

// SEND QUEUE - Available in Rust SDK
Client.setSendQueuesEnabled(enabled: Boolean)
Client.sendQueueDisabledFlow(): Flow<RoomId>
Room.setSendQueueEnabled(enabled: Boolean)
Room.subscribeToSendQueueUpdates(): Flow<SendQueueUpdate>
Room.ignoreDeviceTrustAndResend(devices: Map<UserId, List<DeviceId>>, sendHandle: SendHandle)
Room.withdrawVerificationAndResend(userIds: List<UserId>, sendHandle: SendHandle)

// PUSH - Available in Rust SDK
Client.setPusher(pusher: Pusher)
Client.getPushers(): List<Pusher>

// DRAFT - Available in Rust SDK
Room.saveDraft(draft: ComposerDraft)
Room.loadDraft(): ComposerDraft?
Room.clearDraft()

// MEDIA - Available in Rust SDK
Timeline.sendImage(...)
Timeline.sendVideo(...)
Timeline.sendAudio(...)
Timeline.sendFile(...)
Timeline.sendVoiceMessage(...)
Client.mediaRequest(request: MediaRequest): ByteArray
```

### Updated Priority Roadmap

**Already Done ✅**
- Draft (in-memory) - `RoomComposerDraftStore`
- ComposerMode (Reply/Edit) - `ChatUiModels.kt`
- Media upload (image/file) - `RustTimeline.sendImage()`, `sendFile()`
- Media caption with attachment - `ChatViewModel.kt:395`
- Sliding Sync - Native mode enabled
- Core E2EE - Automatic via Rust SDK
- Typing notifications with debounce - `ChatViewModel.kt:341-384`
- Focused timeline - `JoinedRustRoom.createFocusedTimeline()`

**P1a: Immediate (User Experience)**
1. ❌ Media download (`MatrixMediaLoader`) - view images/files in chat
2. ❌ Video/audio upload - complete media sending
3. ⚠️ Draft persistence (add Rust SDK-backed for app restarts)

**P1b: Security (Multi-device Trust)**
4. ❌ `EncryptionService` wrapper + UI
5. ❌ `SessionVerificationService` + emoji verification UI
6. ❌ UTD error handling

**P1c: Resilience**
7. ❌ Send queue management
8. ❌ Push notifications

**P2: Nice-to-have**
- Spaces, Calls, Location, Threads, Voice, Polls, Knock, Widgets
