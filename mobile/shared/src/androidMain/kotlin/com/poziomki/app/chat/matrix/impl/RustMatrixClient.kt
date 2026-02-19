/*
 * NOTICE: Portions of this implementation are adapted from Element X Android Matrix client wrappers.
 * Copyright (c) 2025 Element Creations Ltd.
 * Copyright 2023-2025 New Vector Ltd.
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial.
 */
package com.poziomki.app.chat.matrix.impl

import android.content.Context
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.MatrixConfigData
import com.poziomki.app.chat.matrix.api.JoinedRoom
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.chat.matrix.api.MatrixClientState
import com.poziomki.app.chat.matrix.api.MatrixRoomSummary
import com.poziomki.app.chat.matrix.api.MatrixTimelineMode
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withTimeoutOrNull
import org.matrix.rustcomponents.sdk.ClientBuilder
import org.matrix.rustcomponents.sdk.CreateRoomParameters
import org.matrix.rustcomponents.sdk.HttpPusherData
import org.matrix.rustcomponents.sdk.LatestEventValue
import org.matrix.rustcomponents.sdk.Membership
import org.matrix.rustcomponents.sdk.PushFormat
import org.matrix.rustcomponents.sdk.PusherIdentifiers
import org.matrix.rustcomponents.sdk.PusherKind
import org.matrix.rustcomponents.sdk.Room
import org.matrix.rustcomponents.sdk.RoomListDynamicEntriesController
import org.matrix.rustcomponents.sdk.RoomListEntriesListener
import org.matrix.rustcomponents.sdk.RoomListEntriesUpdate
import org.matrix.rustcomponents.sdk.RoomListEntriesWithDynamicAdaptersResult
import org.matrix.rustcomponents.sdk.RoomPreset
import org.matrix.rustcomponents.sdk.RoomVisibility
import org.matrix.rustcomponents.sdk.SlidingSyncVersion
import org.matrix.rustcomponents.sdk.TimelineItemContent
import uniffi.matrix_sdk.BackupDownloadStrategy
import java.io.File
import java.net.URI
import java.util.UUID

class RustMatrixClient(
    private val apiService: ApiService,
    private val sessionManager: SessionManager,
    private val appContext: Context,
) : MatrixClient {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val startStopMutex = Mutex()
    private val roomCreationMutex = Mutex()
    private val dmCreationMutex = Mutex()

    private val _state = MutableStateFlow<MatrixClientState>(MatrixClientState.Idle)
    override val state: StateFlow<MatrixClientState> = _state

    private val _rooms = MutableStateFlow<List<MatrixRoomSummary>>(emptyList())
    override val rooms: StateFlow<List<MatrixRoomSummary>> = _rooms

    private var client: org.matrix.rustcomponents.sdk.Client? = null
    private var syncService: org.matrix.rustcomponents.sdk.SyncService? = null
    private var roomListSubscription: RoomListSubscription? = null

    private val openedRooms = mutableMapOf<String, JoinedRustRoom>()
    private var cachedConfig: MatrixConfigData? = null
    private val dmRoomIdsByUserId = mutableMapOf<String, String>()
    private val dmAvatarUrlsByUserId = mutableMapOf<String, String>()

    override suspend fun ensureStarted(): Result<Unit> =
        startStopMutex.withLock {
            if (client != null && state.value is MatrixClientState.Ready) {
                return@withLock Result.success(Unit)
            }

            _state.value = MatrixClientState.Connecting

            val config =
                when (val configResult = apiService.getMatrixConfig()) {
                    is ApiResult.Success -> {
                        configResult.data
                    }

                    is ApiResult.Error -> {
                        val message = "Failed to load Matrix config: ${configResult.message}"
                        _state.value = MatrixClientState.Error(message)
                        return@withLock Result.failure(IllegalStateException(message))
                    }
                }

            cachedConfig = config
            val homeserver = config.homeserver
            if (homeserver.isNullOrBlank()) {
                val message = "Matrix homeserver is not configured"
                _state.value = MatrixClientState.Error(message)
                return@withLock Result.failure(IllegalStateException(message))
            }

            val jwt = sessionManager.getToken()
            if (jwt.isNullOrBlank()) {
                val message = "User session token is missing"
                _state.value = MatrixClientState.Error(message)
                return@withLock Result.failure(IllegalStateException(message))
            }

            val storeNamespace = matrixStoreNamespace()
            val dataPath = File(appContext.filesDir, "matrix-sdk/$storeNamespace/data").apply { mkdirs() }
            val cachePath = File(appContext.cacheDir, "matrix-sdk/$storeNamespace/cache").apply { mkdirs() }

            suspend fun attemptConnect(retried: Boolean = false): Result<Unit> =
                runCatching {
                    val matrixDeviceId = loadOrCreateMatrixDeviceId(storeNamespace)

                    val newClient =
                        ClientBuilder()
                            .homeserverUrl(homeserver)
                            .sessionPaths(dataPath.absolutePath, cachePath.absolutePath)
                            .autoEnableBackups(true)
                            .autoEnableCrossSigning(true)
                            .backupDownloadStrategy(BackupDownloadStrategy.AFTER_DECRYPTION_FAILURE)
                            .build()

                    val matrixSession =
                        when (
                            val sessionResult =
                                apiService.createMatrixSession(
                                    deviceName = "Poziomki Mobile",
                                    deviceId = matrixDeviceId,
                                )
                        ) {
                            is ApiResult.Success -> {
                                sessionResult.data
                            }

                            is ApiResult.Error -> {
                                val message = "Failed to initialize secure messaging: ${sessionResult.message}"
                                _state.value = MatrixClientState.Error(message)
                                return Result.failure(IllegalStateException(message))
                            }
                        }

                    val accessToken = matrixSession.accessToken
                    val refreshToken = matrixSession.refreshToken
                    val userId = matrixSession.userId
                    val deviceId = matrixSession.deviceId

                    if (
                        accessToken.isNullOrBlank() ||
                        refreshToken.isNullOrBlank() ||
                        userId.isNullOrBlank() ||
                        deviceId.isNullOrBlank()
                    ) {
                        val message = "Failed to initialize secure messaging session"
                        _state.value = MatrixClientState.Error(message)
                        return Result.failure(IllegalStateException(message))
                    }

                    saveMatrixDeviceId(storeNamespace, deviceId)

                    newClient.restoreSession(
                        org.matrix.rustcomponents.sdk.Session(
                            accessToken = accessToken,
                            refreshToken = refreshToken,
                            userId = userId,
                            deviceId = deviceId,
                            homeserverUrl = homeserver,
                            oidcData = null,
                            slidingSyncVersion = SlidingSyncVersion.NATIVE,
                        ),
                    )

                    val newSyncService = newClient.syncService().finish()
                    newSyncService.start()

                    // Give E2EE startup tasks time to recover keys before we expose timelines.
                    withTimeoutOrNull(20_000) {
                        runCatching { newClient.encryption().waitForE2eeInitializationTasks() }
                    }

                    runCatching {
                        val encryption = newClient.encryption()
                        if (
                            encryption.backupExistsOnServer() &&
                            encryption.backupState() != org.matrix.rustcomponents.sdk.BackupState.ENABLED
                        ) {
                            encryption.enableBackups()
                        }
                    }

                    subscribeRoomList(newSyncService)

                    client = newClient
                    syncService = newSyncService
                    _state.value = MatrixClientState.Ready(newClient.userId(), homeserver, newClient.deviceId(), accessToken)
                }.onFailure { throwable ->
                    cleanupInternal()
                    if (!retried) {
                        parseMismatchedStoreDeviceId(throwable.message)?.let { storedDeviceId ->
                            saveMatrixDeviceId(storeNamespace, storedDeviceId)
                            return attemptConnect(retried = true)
                        }
                    }
                    if (!retried && shouldResetCryptoStore(throwable)) {
                        wipeMatrixStore(storeNamespace)
                        return attemptConnect(retried = true)
                    }
                    _state.value = MatrixClientState.Error(throwable.message ?: "Failed to initialize Matrix")
                }

            attemptConnect()

            if (_state.value is MatrixClientState.Ready) {
                syncDisplayName()
                registerPusherIfConfigured()
                Result.success(Unit)
            } else {
                Result.failure(IllegalStateException((_state.value as? MatrixClientState.Error)?.message ?: "Failed to initialize Matrix"))
            }
        }

    override suspend fun refreshRooms(): Result<Unit> =
        runCatching {
            ensureStarted().getOrThrow()
            roomListSubscription?.controller?.setFilter(org.matrix.rustcomponents.sdk.RoomListEntriesDynamicFilterKind.NonLeft)
            Unit
        }

    override suspend fun getJoinedRoom(roomId: String): JoinedRoom? {
        ensureStarted().getOrElse { return null }

        openedRooms[roomId]?.let { return it }

        val innerClient = client ?: return null
        val room = innerClient.getRoom(roomId) ?: return null
        if (room.membership() != Membership.JOINED) return null

        val liveTimeline = runCatching { room.timeline() }.getOrElse { return null }
        val wrappedRoom =
            JoinedRustRoom(
                innerRoom = room,
                liveTimeline = RustTimeline(liveTimeline, MatrixTimelineMode.Live, room.ownUserId(), scope),
                coroutineScope = scope,
            )
        openedRooms[roomId] = wrappedRoom
        return wrappedRoom
    }

    override suspend fun createDM(
        userId: String,
        displayName: String?,
    ): Result<String> =
        dmCreationMutex.withLock {
            runCatching {
                ensureStarted().getOrThrow()
                val innerClient = client ?: error("Matrix client is not initialized")
                val normalizedUserId = normalizeUserId(userId)
                val cachedRoomId = synchronized(dmRoomIdsByUserId) { dmRoomIdsByUserId[normalizedUserId] }
                if (!cachedRoomId.isNullOrBlank()) return@runCatching cachedRoomId

                val existing = innerClient.getDmRoom(normalizedUserId)
                if (existing != null) {
                    val existingRoomId = existing.id()
                    synchronized(dmRoomIdsByUserId) {
                        dmRoomIdsByUserId[normalizedUserId] = existingRoomId
                    }
                    return@runCatching existingRoomId
                }

                val existingFromSummary =
                    _rooms.value
                        .firstOrNull { summary ->
                            summary.isDirect && summary.directUserId?.let(::normalizeUserId) == normalizedUserId
                        }?.roomId
                if (!existingFromSummary.isNullOrBlank()) {
                    synchronized(dmRoomIdsByUserId) {
                        dmRoomIdsByUserId[normalizedUserId] = existingFromSummary
                    }
                    return@runCatching existingFromSummary
                }

                val createdRoomId =
                    createRoomInternal(
                        name = displayName,
                        invitedUserIds = listOf(normalizedUserId),
                        isDirect = true,
                        preset = RoomPreset.TRUSTED_PRIVATE_CHAT,
                    )
                synchronized(dmRoomIdsByUserId) {
                    dmRoomIdsByUserId[normalizedUserId] = createdRoomId
                }
                createdRoomId
            }
        }

    override suspend fun createRoom(
        name: String,
        invitedUserIds: List<String>,
    ): Result<String> =
        roomCreationMutex.withLock {
            runCatching {
                ensureStarted().getOrThrow()
                createRoomInternal(
                    name = name.ifBlank { null },
                    invitedUserIds = invitedUserIds.map(::normalizeUserId),
                    isDirect = false,
                    preset = RoomPreset.PRIVATE_CHAT,
                )
            }
        }

    override suspend fun stop() {
        startStopMutex.withLock {
            cleanupInternal()
            _rooms.value = emptyList()
            _state.value = MatrixClientState.Idle
        }
    }

    private suspend fun subscribeRoomList(sync: org.matrix.rustcomponents.sdk.SyncService) {
        val roomListService = sync.roomListService()
        val roomList = roomListService.allRooms()
        val roomBuffer = mutableListOf<Room>()

        val result =
            roomList.entriesWithDynamicAdapters(
                200u,
                object : RoomListEntriesListener {
                    override fun onUpdate(roomEntriesUpdate: List<RoomListEntriesUpdate>) {
                        synchronized(roomBuffer) {
                            applyRoomListUpdates(roomBuffer, roomEntriesUpdate)
                        }
                        val snapshot = synchronized(roomBuffer) { roomBuffer.toList() }
                        publishRoomSummaries(snapshot)
                    }
                },
            )

        val controller = result.controller()
        controller.setFilter(org.matrix.rustcomponents.sdk.RoomListEntriesDynamicFilterKind.NonLeft)
        val streamHandle = result.entriesStream()

        roomListSubscription =
            RoomListSubscription(
                roomListResult = result,
                streamHandle = streamHandle,
                controller = controller,
            )
    }

    @Suppress("LongMethod", "CyclomaticComplexMethod")
    private fun publishRoomSummaries(roomsSnapshot: List<Room>) {
        scope.launch(Dispatchers.Default) {
            val mappedRooms = mutableListOf<MatrixRoomSummary>()
            for (room in roomsSnapshot) {
                val mappedRoom =
                    runCatching {
                        val info = room.roomInfo()
                        val latest = room.latestEvent()
                        val directHero = info.heroes.firstOrNull() ?: room.heroes().firstOrNull()

                        val heroUserId =
                            if (info.isDirect) {
                                directHero?.userId
                            } else {
                                null
                            }

                        val normalizedDirectUserId = heroUserId?.let(::normalizeUserId)
                        var resolvedAvatarUrl =
                            info.avatarUrl
                                ?: if (info.isDirect) {
                                    directHero?.avatarUrl
                                } else {
                                    null
                                }
                                ?: normalizedDirectUserId?.let { normalized ->
                                    synchronized(dmAvatarUrlsByUserId) { dmAvatarUrlsByUserId[normalized] }
                                }

                        if (resolvedAvatarUrl.isNullOrBlank() && info.isDirect && !heroUserId.isNullOrBlank()) {
                            resolvedAvatarUrl =
                                runCatching { room.memberAvatarUrl(heroUserId) }
                                    .getOrNull()
                                    ?.takeIf { it.isNotBlank() }
                            if (!resolvedAvatarUrl.isNullOrBlank() && !normalizedDirectUserId.isNullOrBlank()) {
                                synchronized(dmAvatarUrlsByUserId) {
                                    dmAvatarUrlsByUserId[normalizedDirectUserId] = resolvedAvatarUrl
                                }
                            }
                        }

                        MatrixRoomSummary(
                            roomId = info.id,
                            displayName = info.displayName ?: directHero?.displayName ?: room.displayName() ?: info.id,
                            avatarUrl = resolvedAvatarUrl,
                            isDirect = info.isDirect,
                            directUserId = heroUserId,
                            unreadCount =
                                info.numUnreadNotifications
                                    .toLong()
                                    .coerceAtMost(Int.MAX_VALUE.toLong())
                                    .toInt(),
                            latestMessage = latest.previewText(),
                            latestTimestampMillis = latest.timestampMillis(),
                        )
                    }.getOrNull()

                if (mappedRoom != null) {
                    mappedRooms += mappedRoom
                }
            }

            synchronized(dmRoomIdsByUserId) {
                mappedRooms
                    .asSequence()
                    .filter { it.isDirect && !it.directUserId.isNullOrBlank() }
                    .forEach { room ->
                        val directUserId = room.directUserId ?: return@forEach
                        dmRoomIdsByUserId[normalizeUserId(directUserId)] = room.roomId
                    }
            }

            _rooms.value = mappedRooms.sortedByDescending { it.latestTimestampMillis ?: Long.MIN_VALUE }
        }
    }

    override suspend fun registerPusher(
        ntfyEndpoint: String,
        gatewayUrl: String,
    ): Result<Unit> =
        runCatching {
            val c = client ?: error("Matrix client is not initialized")
            c.setPusher(
                identifiers =
                    PusherIdentifiers(
                        pushkey = ntfyEndpoint,
                        appId = "app.poziomki",
                    ),
                kind =
                    PusherKind.Http(
                        HttpPusherData(
                            url = gatewayUrl,
                            format = PushFormat.EVENT_ID_ONLY,
                            defaultPayload = null,
                        ),
                    ),
                appDisplayName = "Poziomki",
                deviceDisplayName = "Poziomki Android",
                profileTag = null,
                lang = "en",
            )
        }

    override suspend fun unregisterPusher(ntfyEndpoint: String): Result<Unit> =
        runCatching {
            val c = client ?: error("Matrix client is not initialized")
            c.deletePusher(
                PusherIdentifiers(
                    pushkey = ntfyEndpoint,
                    appId = "app.poziomki",
                ),
            )
        }

    private fun syncDisplayName() {
        scope.launch {
            val c = client ?: return@launch
            val profile = (apiService.getMyProfile() as? ApiResult.Success)?.data ?: return@launch
            val currentName = runCatching { c.displayName() }.getOrNull()
            if (currentName == profile.name) return@launch
            runCatching { c.setDisplayName(profile.name) }
        }
    }

    private fun registerPusherIfConfigured() {
        val config = cachedConfig ?: return
        val gatewayUrl = config.pushGatewayUrl ?: return
        val ntfyServer = config.ntfyServer ?: return
        val readyState = state.value as? MatrixClientState.Ready ?: return
        val ntfyEndpoint = "$ntfyServer/poz_${readyState.deviceId}"

        scope.launch {
            registerPusher(ntfyEndpoint, gatewayUrl).onFailure { error ->
                // Best-effort — don't block startup
                println("Failed to register pusher: ${error.message}")
            }
        }
    }

    private fun cleanupInternal() {
        openedRooms.values.forEach { room -> room.close() }
        openedRooms.clear()
        synchronized(dmRoomIdsByUserId) { dmRoomIdsByUserId.clear() }
        synchronized(dmAvatarUrlsByUserId) { dmAvatarUrlsByUserId.clear() }

        roomListSubscription?.streamHandle?.cancel()
        roomListSubscription?.roomListResult?.close()
        roomListSubscription = null

        runBlocking {
            runCatching { syncService?.stop() }
        }
        syncService?.close()
        syncService = null

        client?.close()
        client = null
    }

    private suspend fun matrixStoreNamespace(): String =
        sessionManager
            .userId
            .first()
            ?.replace(Regex("[^A-Za-z0-9_.-]"), "_")
            ?.ifBlank { null }
            ?: "default"

    private fun shouldResetCryptoStore(error: Throwable): Boolean {
        val message = error.message?.lowercase() ?: return false
        return message.contains("account in the store doesn't match") ||
            message.contains("account in store doesn't match")
    }

    private fun wipeMatrixStore(storeNamespace: String) {
        File(appContext.filesDir, "matrix-sdk/$storeNamespace").deleteRecursively()
        File(appContext.cacheDir, "matrix-sdk/$storeNamespace").deleteRecursively()
    }

    private fun loadMatrixDeviceId(storeNamespace: String): String? {
        val file = matrixDeviceIdFile(storeNamespace)
        return runCatching {
            if (file.exists()) {
                normalizeMatrixDeviceId(file.readText())
            } else {
                null
            }
        }.getOrNull()
    }

    private fun saveMatrixDeviceId(
        storeNamespace: String,
        rawDeviceId: String?,
    ) {
        val normalized = normalizeMatrixDeviceId(rawDeviceId) ?: return
        val file = matrixDeviceIdFile(storeNamespace)
        file.parentFile?.mkdirs()
        runCatching { file.writeText(normalized) }
    }

    private fun matrixDeviceIdFile(storeNamespace: String): File = File(appContext.filesDir, "matrix-sdk/$storeNamespace/device_id.txt")

    private fun loadOrCreateMatrixDeviceId(storeNamespace: String): String {
        loadMatrixDeviceId(storeNamespace)?.let { return it }

        val created = "POZ${UUID.randomUUID().toString().replace("-", "").take(16).uppercase()}"
        saveMatrixDeviceId(storeNamespace, created)
        return created
    }

    private fun normalizeMatrixDeviceId(raw: String?): String? {
        if (raw.isNullOrBlank()) return null
        val filtered = raw.trim().uppercase().filter { it.isLetterOrDigit() || it == '_' || it == '-' || it == '.' }
        val bounded = filtered.take(64)
        return bounded.ifBlank { null }
    }

    private fun parseMismatchedStoreDeviceId(message: String?): String? {
        if (message.isNullOrBlank()) return null
        val regex =
            Regex(
                pattern = """expected\s+.+:([A-Za-z0-9_.-]+),\s*got\s+.+:([A-Za-z0-9_.-]+)""",
                options = setOf(RegexOption.IGNORE_CASE),
            )
        val match = regex.find(message) ?: return null
        val storedDeviceId = match.groupValues.getOrNull(2)
        return normalizeMatrixDeviceId(storedDeviceId)
    }

    private suspend fun createRoomInternal(
        name: String?,
        invitedUserIds: List<String>,
        isDirect: Boolean,
        preset: RoomPreset,
    ): String {
        val innerClient = client ?: error("Matrix client is not initialized")
        return innerClient.createRoom(
            CreateRoomParameters(
                name = name,
                topic = null,
                isEncrypted = true,
                isDirect = isDirect,
                visibility = RoomVisibility.Private,
                preset = preset,
                invite = invitedUserIds.takeIf { it.isNotEmpty() },
                avatar = null,
                powerLevelContentOverride = null,
                joinRuleOverride = null,
                historyVisibilityOverride = null,
                canonicalAlias = null,
                isSpace = false,
            ),
        )
    }

    private fun normalizeUserId(rawValue: String): String {
        val value = rawValue.trim()
        if (value.isEmpty()) return value
        if (value.startsWith("@")) return value

        val homeserver =
            (state.value as? MatrixClientState.Ready)
                ?.homeserver
                ?.let(::extractServerNameFromHomeserver)
                ?: return value
        return "@$value:$homeserver"
    }

    private fun extractServerNameFromHomeserver(homeserver: String): String? {
        val normalized = if (homeserver.contains("://")) homeserver else "https://$homeserver"
        return runCatching {
            val uri = URI(normalized)
            uri.authority
        }.getOrNull()?.ifBlank { null }
    }
}

private data class RoomListSubscription(
    val roomListResult: RoomListEntriesWithDynamicAdaptersResult,
    val streamHandle: org.matrix.rustcomponents.sdk.TaskHandle,
    val controller: RoomListDynamicEntriesController,
)

private fun applyRoomListUpdates(
    roomBuffer: MutableList<Room>,
    updates: List<RoomListEntriesUpdate>,
) {
    updates.forEach { update ->
        when (update) {
            is RoomListEntriesUpdate.Append -> {
                roomBuffer.addAll(update.values)
            }

            is RoomListEntriesUpdate.Clear -> {
                roomBuffer.clear()
            }

            is RoomListEntriesUpdate.PushFront -> {
                roomBuffer.add(0, update.value)
            }

            is RoomListEntriesUpdate.PushBack -> {
                roomBuffer.add(update.value)
            }

            RoomListEntriesUpdate.PopFront -> {
                if (roomBuffer.isNotEmpty()) roomBuffer.removeAt(0)
            }

            RoomListEntriesUpdate.PopBack -> {
                if (roomBuffer.isNotEmpty()) roomBuffer.removeAt(roomBuffer.lastIndex)
            }

            is RoomListEntriesUpdate.Insert -> {
                val index = update.index.toInt().coerceIn(0, roomBuffer.size)
                roomBuffer.add(index, update.value)
            }

            is RoomListEntriesUpdate.Set -> {
                val index = update.index.toInt()
                if (index in roomBuffer.indices) {
                    roomBuffer[index] = update.value
                }
            }

            is RoomListEntriesUpdate.Remove -> {
                val index = update.index.toInt()
                if (index in roomBuffer.indices) {
                    roomBuffer.removeAt(index)
                }
            }

            is RoomListEntriesUpdate.Truncate -> {
                val newSize = update.length.toInt().coerceAtLeast(0)
                if (newSize < roomBuffer.size) {
                    roomBuffer.subList(newSize, roomBuffer.size).clear()
                }
            }

            is RoomListEntriesUpdate.Reset -> {
                roomBuffer.clear()
                roomBuffer.addAll(update.values)
            }
        }
    }
}

private fun LatestEventValue.previewText(): String? =
    when (this) {
        LatestEventValue.None -> null
        is LatestEventValue.Remote -> this.content.toPreviewText()
        is LatestEventValue.Local -> this.content.toPreviewText()
        is LatestEventValue.RemoteInvite -> "Room invite"
    }

private fun LatestEventValue.timestampMillis(): Long? =
    when (this) {
        LatestEventValue.None -> null
        is LatestEventValue.Remote -> this.timestamp.toLong()
        is LatestEventValue.Local -> this.timestamp.toLong()
        is LatestEventValue.RemoteInvite -> this.timestamp.toLong()
    }

private fun TimelineItemContent.toPreviewText(): String? =
    when (this) {
        is TimelineItemContent.MsgLike -> {
            when (val kind = this.content.kind) {
                is org.matrix.rustcomponents.sdk.MsgLikeKind.Message -> kind.content.body
                org.matrix.rustcomponents.sdk.MsgLikeKind.Redacted -> "Message removed"
                is org.matrix.rustcomponents.sdk.MsgLikeKind.Poll -> "Poll: ${kind.question}"
                is org.matrix.rustcomponents.sdk.MsgLikeKind.Sticker -> kind.body
                is org.matrix.rustcomponents.sdk.MsgLikeKind.UnableToDecrypt -> "Encrypted message"
                is org.matrix.rustcomponents.sdk.MsgLikeKind.Other -> "Unsupported message"
            }
        }

        // State events should not appear as room preview text
        TimelineItemContent.CallInvite,
        TimelineItemContent.RtcNotification,
        is TimelineItemContent.ProfileChange,
        is TimelineItemContent.RoomMembership,
        is TimelineItemContent.State,
        is TimelineItemContent.FailedToParseMessageLike,
        is TimelineItemContent.FailedToParseState,
        -> {
            null
        }
    }
