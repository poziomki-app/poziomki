/*
 * NOTICE: Portions of this implementation are adapted from Element X Android Matrix client wrappers.
 * Copyright (c) 2025 Element Creations Ltd.
 * Copyright 2023-2025 New Vector Ltd.
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial.
 */
package com.poziomki.app.chat.matrix.impl

import android.content.Context
import com.poziomki.app.chat.cache.RoomTimelineCacheStore
import com.poziomki.app.chat.matrix.api.JoinedRoom
import com.poziomki.app.chat.matrix.api.MatrixClient
import com.poziomki.app.db.PoziomkiDatabase
import com.poziomki.app.chat.matrix.api.MatrixClientState
import com.poziomki.app.chat.matrix.api.MatrixEventSendStatus
import com.poziomki.app.chat.matrix.api.MatrixRoomSummary
import com.poziomki.app.chat.matrix.api.MatrixTimelineItem
import com.poziomki.app.chat.matrix.api.MatrixTimelineMode
import com.poziomki.app.network.ApiResult
import com.poziomki.app.network.ApiService
import com.poziomki.app.network.MatrixConfigData
import com.poziomki.app.session.SessionManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.delay
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.coroutines.withContext
import org.matrix.rustcomponents.sdk.ClientBuilder
import org.matrix.rustcomponents.sdk.CreateRoomParameters
import org.matrix.rustcomponents.sdk.RequestConfig
import org.matrix.rustcomponents.sdk.SqliteStoreBuilder
import org.matrix.rustcomponents.sdk.HttpPusherData
import org.matrix.rustcomponents.sdk.LatestEventValue
import org.matrix.rustcomponents.sdk.Membership
import org.matrix.rustcomponents.sdk.MembershipState
import org.matrix.rustcomponents.sdk.PushFormat
import org.matrix.rustcomponents.sdk.PusherIdentifiers
import org.matrix.rustcomponents.sdk.PusherKind
import org.matrix.rustcomponents.sdk.Room
import org.matrix.rustcomponents.sdk.RoomHero
import org.matrix.rustcomponents.sdk.RoomListDynamicEntriesController
import org.matrix.rustcomponents.sdk.RoomListEntriesListener
import org.matrix.rustcomponents.sdk.RoomListEntriesUpdate
import org.matrix.rustcomponents.sdk.RoomListEntriesWithDynamicAdaptersResult
import org.matrix.rustcomponents.sdk.RoomMember
import org.matrix.rustcomponents.sdk.RoomPreset
import org.matrix.rustcomponents.sdk.RoomVisibility
import org.matrix.rustcomponents.sdk.SlidingSyncVersion
import org.matrix.rustcomponents.sdk.TimelineItemContent
import org.matrix.rustcomponents.sdk.DateDividerMode
import org.matrix.rustcomponents.sdk.TimelineConfiguration
import org.matrix.rustcomponents.sdk.TimelineFilter
import org.matrix.rustcomponents.sdk.TimelineFocus
import uniffi.matrix_sdk.BackupDownloadStrategy
import uniffi.matrix_sdk_ui.LatestEventValueLocalState
import uniffi.matrix_sdk_ui.TimelineReadReceiptTracking
import java.io.File
import java.util.UUID

class RustMatrixClient(
    private val apiService: ApiService,
    private val sessionManager: SessionManager,
    private val appContext: Context,
    private val db: PoziomkiDatabase,
    private val roomTimelineCacheStore: RoomTimelineCacheStore,
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
    private var activeAppUserId: String? = null

    private val openedRooms = mutableMapOf<String, JoinedRustRoom>()
    private var cachedConfig: MatrixConfigData? = null
    private val dmRoomIdsByUserId = mutableMapOf<String, String>()
    private val dmUserIdsByRoomId = mutableMapOf<String, String>()
    private val dmAvatarUrlsByUserId = mutableMapOf<String, String>()
    private val openedRoomSummaryJobs = mutableMapOf<String, Job>()
    private val timelinePreviewByRoomId = mutableMapOf<String, TimelinePreview>()
    init {
        loadCachedPreviews()
    }

    private val roomSummaryComparator: Comparator<MatrixRoomSummary> =
        compareByDescending<MatrixRoomSummary> { it.latestTimestampMillis ?: Long.MIN_VALUE }
            .thenByDescending { it.unreadCount }
            .thenBy { it.roomId }

    override suspend fun ensureStarted(): Result<Unit> =
        startStopMutex.withLock {
            val currentAppUserId = sessionManager.userId.first()
            if (client != null && state.value is MatrixClientState.Ready && activeAppUserId == currentAppUserId) {
                return@withLock Result.success(Unit)
            }
            if (client != null && activeAppUserId != currentAppUserId) {
                cleanupInternal()
                _rooms.value = emptyList()
                _state.value = MatrixClientState.Idle
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

                    val sqliteStore = SqliteStoreBuilder(dataPath.absolutePath, cachePath.absolutePath)
                    val newClient =
                        ClientBuilder()
                            .homeserverUrl(homeserver)
                            .sqliteStore(sqliteStore)
                            .requestConfig(RequestConfig(
                                retryLimit = 3u,
                                timeout = 30_000u,
                                maxConcurrentRequests = null,
                                maxRetryTime = null,
                            ))
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

                    val newSyncService = newClient.syncService()
                        .withOfflineMode()
                        .withSharePos(true)
                        .finish()
                    newSyncService.start()

                    // Enable send queues so outgoing messages are flushed to the homeserver.
                    runCatching { newClient.enableAllSendQueues(true) }

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

                    runCatching {
                        newClient.setMediaRetentionPolicy(
                            uniffi.matrix_sdk_base.MediaRetentionPolicy(
                                maxCacheSize = 500uL * 1024uL * 1024uL,
                                maxFileSize = 20uL * 1024uL * 1024uL,
                                lastAccessExpiry = java.time.Duration.ofDays(30),
                                cleanupFrequency = java.time.Duration.ofDays(1),
                            ),
                        )
                    }

                    subscribeRoomList(newSyncService)

                    client = newClient
                    syncService = newSyncService
                    _state.value = MatrixClientState.Ready(newClient.userId(), homeserver, newClient.deviceId())
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
                activeAppUserId = currentAppUserId
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
        repeat(24) { attempt ->
            openedRooms[roomId]?.let { return it }

            val room =
                runCatching { innerClient.getRoom(roomId) }.getOrNull()
                    ?: runCatching {
                        // Backend may create a DM room before this device has seen it in sliding sync.
                        // Ask the SDK to hydrate/fetch the room by id as a fallback.
                        innerClient.joinRoomById(roomId)
                    }.getOrNull()

            if (room != null) {
                if (room.membership() == Membership.INVITED) {
                    runCatching { room.join() }
                        .onFailure { throwable ->
                            println("Matrix getJoinedRoom($roomId): join invite failed: ${throwable.message}")
                        }
                }

                if (room.membership() == Membership.JOINED) {
                    val ownUserId = runCatching { room.ownUserId() }.getOrNull()
                    if (!ownUserId.isNullOrBlank()) {
                        val roomInfo = runCatching { room.roomInfo() }.getOrNull()
                        val cachedDirectUserId = synchronized(dmUserIdsByRoomId) { dmUserIdsByRoomId[roomId] }
                        val shouldResolveDirectPeer = roomInfo?.isDirect == true || !cachedDirectUserId.isNullOrBlank()
                        if (shouldResolveDirectPeer) {
                            val directPeer =
                                runCatching { resolveDirectPeer(room, ownUserId) }
                                    .getOrNull()
                            val directPeerUserId = directPeer?.userId?.takeIf { it.isNotBlank() }
                            if (!directPeerUserId.isNullOrBlank()) {
                                val normalizedDirectUserId = normalizeUserId(directPeerUserId)
                                synchronized(dmRoomIdsByUserId) {
                                    dmRoomIdsByUserId[normalizedDirectUserId] = roomId
                                }
                                synchronized(dmUserIdsByRoomId) {
                                    dmUserIdsByRoomId[roomId] = directPeerUserId
                                }
                                directPeer.avatarUrl
                                    ?.takeIf { it.isNotBlank() }
                                    ?.let { avatarUrl ->
                                        synchronized(dmAvatarUrlsByUserId) {
                                            dmAvatarUrlsByUserId[normalizedDirectUserId] = avatarUrl
                                        }
                                    }
                            }
                        }
                    }

                    val liveTimeline =
                        runCatching {
                            room.timelineWithConfiguration(createLiveTimelineConfiguration())
                        }.onFailure { throwable ->
                                println("Matrix getJoinedRoom($roomId): timeline creation failed: ${throwable.message}")
                            }.getOrNull()
                    if (liveTimeline != null) {
                        val cachedTimelineItems =
                            runCatching {
                                withContext(Dispatchers.IO) {
                                    roomTimelineCacheStore.load(roomId = roomId)
                                }
                            }.getOrDefault(emptyList())
                        val wrappedRoom =
                            JoinedRustRoom(
                                innerRoom = room,
                                liveTimeline =
                                    ChatTimeline(
                                        inner = liveTimeline,
                                        mode = MatrixTimelineMode.Live,
                                        ownUserId = room.ownUserId(),
                                        coroutineScope = scope,
                                        roomId = roomId,
                                        timelineCacheStore = roomTimelineCacheStore,
                                        initialItems = cachedTimelineItems,
                                    ),
                                coroutineScope = scope,
                            )
                        openedRooms[roomId] = wrappedRoom
                        observeOpenedRoomTimelineSummary(room, wrappedRoom)
                        scope.launch(Dispatchers.Default) {
                            publishOpenedRoomSummary(room)
                        }
                        return wrappedRoom
                    }
                }
            }

            if (attempt == 0 || attempt % 3 == 2) {
                runCatching { refreshRooms() }
            }
            if (attempt < 23) {
                delay(500L)
            }
        }
        return null
    }

    private suspend fun publishOpenedRoomSummary(room: Room) {
        val mappedRoom =
            runCatching {
                val info = room.roomInfo()
                val latest = runCatching { room.latestEvent() }.getOrNull()
                val ownUserId = room.ownUserId()
                val cachedDirectUserId =
                    synchronized(dmUserIdsByRoomId) { dmUserIdsByRoomId[info.id] }
                val shouldResolveDirectPeer = info.isDirect || !cachedDirectUserId.isNullOrBlank()
                val directPeer = if (shouldResolveDirectPeer) {
                    runCatching { resolveDirectPeer(room, ownUserId) }.getOrNull()
                } else null
                val directHero = if (shouldResolveDirectPeer) {
                    runCatching {
                        pickDirectHeroCandidate(
                            ownUserId = ownUserId,
                            heroes = info.heroes + room.heroes(),
                            preferredUserId = directPeer?.userId ?: cachedDirectUserId,
                        )
                    }.getOrNull()
                } else null
                val heroUserId = if (shouldResolveDirectPeer) {
                    directPeer?.userId ?: directHero?.userId ?: cachedDirectUserId
                } else null
                val effectiveIsDirect = info.isDirect
                val normalizedDirectUserId = heroUserId?.let(::normalizeUserId)

                var resolvedAvatarUrl =
                    if (effectiveIsDirect) {
                        directPeer?.avatarUrl ?: directHero?.avatarUrl ?: info.avatarUrl
                    } else {
                        info.avatarUrl
                    }
                        ?: normalizedDirectUserId?.let { normalized ->
                            synchronized(dmAvatarUrlsByUserId) { dmAvatarUrlsByUserId[normalized] }
                        }

                if (resolvedAvatarUrl.isNullOrBlank() && effectiveIsDirect && !heroUserId.isNullOrBlank()) {
                    resolvedAvatarUrl =
                        runCatching { room.memberAvatarUrl(heroUserId) }
                            .getOrNull()
                            ?.takeIf { it.isNotBlank() }
                }

                if (!heroUserId.isNullOrBlank()) {
                    val normalized = normalizeUserId(heroUserId)
                    synchronized(dmRoomIdsByUserId) {
                        dmRoomIdsByUserId[normalized] = info.id
                    }
                    synchronized(dmUserIdsByRoomId) {
                        dmUserIdsByRoomId[info.id] = heroUserId
                    }
                    resolvedAvatarUrl?.takeIf { it.isNotBlank() }?.let { avatarUrl ->
                        synchronized(dmAvatarUrlsByUserId) {
                            dmAvatarUrlsByUserId[normalized] = avatarUrl
                        }
                    }
                }

                var mapped =
                    MatrixRoomSummary(
                    roomId = info.id,
                    displayName =
                        if (effectiveIsDirect) {
                            directPeer?.displayName
                                ?: directHero?.displayName
                                ?: info.displayName?.takeIf { !isMemberCountName(it) }
                                ?: room.displayName()?.takeIf { !isMemberCountName(it) }
                                ?: heroUserId?.matrixDisplayFallback()
                                ?: info.displayName
                                ?: room.displayName()
                                ?: info.id
                        } else {
                            info.displayName
                                ?: room.displayName()
                                ?: info.id
                        },
                    avatarUrl = resolvedAvatarUrl,
                    isDirect = effectiveIsDirect,
                    directUserId = heroUserId,
                    unreadCount =
                        info.numUnreadNotifications
                            .toLong()
                            .coerceAtMost(Int.MAX_VALUE.toLong())
                            .toInt(),
                    latestMessage = latest?.previewText(),
                    latestTimestampMillis = latest?.timestampMillis(),
                    latestMessageIsMine = latest?.isMine(ownUserId) == true,
                    latestMessageSendStatus = latest?.toRoomSendStatus(ownUserId),
                )
                mapped = applyTimelinePreviewOverride(mapped)
                mapped = applyExistingSummaryPreviewFallback(mapped, _rooms.value.firstOrNull { it.roomId == mapped.roomId })
                rememberPreviewSummary(mapped)
                mapped
            }.getOrNull() ?: return

        val updated =
            (_rooms.value + mappedRoom)
                .groupBy { it.roomId }
                .map { (_, candidates) ->
                    candidates.maxWithOrNull(roomSummaryComparator) ?: candidates.first()
                }.sortedWith(roomSummaryComparator)
        if (_rooms.value != updated) {
            _rooms.value = updated
        }
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

    override suspend fun getMediaThumbnail(
        mxcUrl: String,
        width: Long,
        height: Long,
    ): ByteArray? {
        val c = client ?: return null
        return runCatching {
            val mediaSource =
                org.matrix.rustcomponents.sdk.MediaSource
                    .fromUrl(mxcUrl)
            try {
                c.getMediaThumbnail(mediaSource, width.toULong(), height.toULong())
            } finally {
                mediaSource.destroy()
            }
        }.getOrNull()
    }

    override suspend fun getMediaContent(mxcUrl: String): ByteArray? {
        val c = client ?: return null
        return runCatching {
            val mediaSource =
                org.matrix.rustcomponents.sdk.MediaSource
                    .fromUrl(mxcUrl)
            try {
                c.getMediaContent(mediaSource)
            } finally {
                mediaSource.destroy()
            }
        }.getOrNull()
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
            roomList.entriesWithDynamicAdaptersWith(
                pageSize = 200u,
                enableLatestEventSorter = true,
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
        // Auto-join any rooms where we're invited
        for (room in roomsSnapshot) {
            if (room.membership() == Membership.INVITED) {
                scope.launch { runCatching { room.join() } }
            }
        }

        scope.launch(Dispatchers.Default) {
            val existingRoomsById = _rooms.value.associateBy { it.roomId }
            val mappedRooms = mutableListOf<MatrixRoomSummary>()
            for (room in roomsSnapshot) {
                val mappedRoom =
                    runCatching {
                        val info = room.roomInfo()
                        val latest = room.latestEvent()
                        val ownUserId = room.ownUserId()
                        val cachedDirectUserId =
                            synchronized(dmUserIdsByRoomId) { dmUserIdsByRoomId[info.id] }
                        val shouldResolveDirectPeer = info.isDirect || !cachedDirectUserId.isNullOrBlank()
                        val directPeer =
                            if (shouldResolveDirectPeer) {
                                resolveDirectPeer(room, ownUserId)
                            } else {
                                null
                            }
                        val directHero =
                            if (shouldResolveDirectPeer) {
                                pickDirectHeroCandidate(
                                    ownUserId = ownUserId,
                                    heroes = info.heroes + room.heroes(),
                                    preferredUserId = directPeer?.userId ?: cachedDirectUserId,
                                )
                            } else {
                                null
                            }

                        val heroUserId =
                            if (shouldResolveDirectPeer) {
                                directPeer?.userId ?: directHero?.userId ?: cachedDirectUserId
                            } else {
                                null
                            }
                        val effectiveIsDirect = info.isDirect

                        val normalizedDirectUserId = heroUserId?.let(::normalizeUserId)
                        var resolvedAvatarUrl =
                            if (effectiveIsDirect) {
                                directPeer?.avatarUrl ?: directHero?.avatarUrl ?: info.avatarUrl
                            } else {
                                info.avatarUrl
                            }
                                ?: normalizedDirectUserId?.let { normalized ->
                                    synchronized(dmAvatarUrlsByUserId) { dmAvatarUrlsByUserId[normalized] }
                                }

                        if (resolvedAvatarUrl.isNullOrBlank() && effectiveIsDirect && !heroUserId.isNullOrBlank()) {
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

                        var mapped =
                            MatrixRoomSummary(
                            roomId = info.id,
                            displayName =
                                if (effectiveIsDirect) {
                                    directPeer?.displayName
                                        ?: directHero?.displayName
                                        ?: info.displayName?.takeIf { !isMemberCountName(it) }
                                        ?: room.displayName()?.takeIf { !isMemberCountName(it) }
                                        ?: heroUserId?.matrixDisplayFallback()
                                        ?: info.displayName
                                        ?: room.displayName()
                                        ?: info.id
                                } else {
                                    info.displayName
                                        ?: room.displayName()
                                        ?: info.id
                                },
                            avatarUrl = resolvedAvatarUrl,
                            isDirect = effectiveIsDirect,
                            directUserId = heroUserId,
                            unreadCount =
                                info.numUnreadNotifications
                                    .toLong()
                                    .coerceAtMost(Int.MAX_VALUE.toLong())
                                    .toInt(),
                            latestMessage = latest.previewText(),
                            latestTimestampMillis = latest.timestampMillis(),
                            latestMessageIsMine = latest.isMine(ownUserId) == true,
                            latestMessageSendStatus = latest.toRoomSendStatus(ownUserId),
                        )
                        mapped = applyTimelinePreviewOverride(mapped)
                        mapped = applyExistingSummaryPreviewFallback(mapped, existingRoomsById[mapped.roomId])
                        rememberPreviewSummary(mapped)
                        mapped
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
                        dmUserIdsByRoomId[room.roomId] = directUserId
                    }
            }

            val sortedRooms = mappedRooms.sortedWith(roomSummaryComparator)
            if (_rooms.value != sortedRooms) {
                _rooms.value = sortedRooms
            }
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

    private fun createLiveTimelineConfiguration(
        trackReadReceipts: TimelineReadReceiptTracking = TimelineReadReceiptTracking.ALL_EVENTS,
    ): TimelineConfiguration = TimelineConfiguration(
        focus = TimelineFocus.Live(hideThreadedEvents = false),
        filter = TimelineFilter.All,
        internalIdPrefix = null,
        dateDividerMode = DateDividerMode.DAILY,
        trackReadReceipts = trackReadReceipts,
        reportUtds = false,
    )

    private fun cleanupInternal() {
        openedRooms.values.forEach { room -> room.close() }
        openedRooms.clear()
        openedRoomSummaryJobs.values.forEach { it.cancel() }
        openedRoomSummaryJobs.clear()
        synchronized(dmRoomIdsByUserId) { dmRoomIdsByUserId.clear() }
        synchronized(dmAvatarUrlsByUserId) { dmAvatarUrlsByUserId.clear() }
        synchronized(timelinePreviewByRoomId) { timelinePreviewByRoomId.clear() }

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
        activeAppUserId = null
    }

    private fun observeOpenedRoomTimelineSummary(
        room: Room,
        wrappedRoom: JoinedRustRoom,
    ) {
        val roomId = room.id()
        if (openedRoomSummaryJobs[roomId]?.isActive == true) return
        openedRoomSummaryJobs[roomId] =
            scope.launch(Dispatchers.Default) {
                wrappedRoom.liveTimeline.items.collectLatest { items ->
                    val latestMessageEvent =
                        items
                            .asReversed()
                            .firstNotNullOfOrNull { item ->
                                (item as? MatrixTimelineItem.Event)
                                    ?.takeIf { it.body.isNotBlank() }
                            } ?: return@collectLatest

                    val preview =
                        TimelinePreview(
                            message = latestMessageEvent.body,
                            timestampMillis = latestMessageEvent.timestampMillis,
                            isMine = latestMessageEvent.isMine,
                            sendStatus = latestMessageEvent.sendStatus,
                            readByCount = latestMessageEvent.readByCount,
                        )
                    synchronized(timelinePreviewByRoomId) {
                        timelinePreviewByRoomId[roomId] = preview
                    }
                    val currentSummary = _rooms.value.firstOrNull { it.roomId == roomId }
                    persistPreview(roomId, preview, currentSummary)

                    publishTimelinePreview(roomId, latestMessageEvent)
                }
            }
    }

    private fun publishTimelinePreview(
        roomId: String,
        event: MatrixTimelineItem.Event,
    ) {
        val currentRooms = _rooms.value
        val current = currentRooms.firstOrNull { it.roomId == roomId } ?: return
        val updatedSummary =
            current.copy(
                latestMessage = event.body,
                latestTimestampMillis = event.timestampMillis,
                latestMessageIsMine = event.isMine,
                latestMessageSendStatus = event.sendStatus,
                latestMessageReadByCount = event.readByCount,
            )
        if (updatedSummary == current) return

        val updated =
            currentRooms
                .map { summary -> if (summary.roomId == roomId) updatedSummary else summary }
                .sortedWith(roomSummaryComparator)
        if (_rooms.value != updated) {
            _rooms.value = updated
        }
    }

    private fun applyExistingSummaryPreviewFallback(
        summary: MatrixRoomSummary,
        previous: MatrixRoomSummary?,
    ): MatrixRoomSummary {
        if (hasMeaningfulPreview(summary.latestMessage)) return summary

        val previousSummary = previous ?: return summary
        if (!hasMeaningfulPreview(previousSummary.latestMessage)) return summary

        val previousTimestamp = previousSummary.latestTimestampMillis ?: return summary
        val previousMessage = previousSummary.latestMessage ?: return summary
        return summary.copy(
            latestMessage = previousMessage,
            latestTimestampMillis = previousTimestamp,
            latestMessageIsMine = previousSummary.latestMessageIsMine,
            latestMessageSendStatus = previousSummary.latestMessageSendStatus,
            latestMessageReadByCount = previousSummary.latestMessageReadByCount,
        )
    }

    private fun rememberPreviewSummary(summary: MatrixRoomSummary) {
        if (!hasMeaningfulPreview(summary.latestMessage)) return
        val message = summary.latestMessage ?: return
        val timestamp = summary.latestTimestampMillis ?: return
        val preview =
            TimelinePreview(
                message = message,
                timestampMillis = timestamp,
                isMine = summary.latestMessageIsMine,
                sendStatus = summary.latestMessageSendStatus,
                readByCount = summary.latestMessageReadByCount,
            )
        var changed = false
        synchronized(timelinePreviewByRoomId) {
            val existing = timelinePreviewByRoomId[summary.roomId]
            if (existing == null || existing.timestampMillis <= preview.timestampMillis) {
                timelinePreviewByRoomId[summary.roomId] = preview
                changed = existing != preview
            }
        }
        if (changed) {
            persistPreview(summary.roomId, preview, summary)
        }
    }

    private fun applyTimelinePreviewOverride(summary: MatrixRoomSummary): MatrixRoomSummary {
        val timelinePreview =
            synchronized(timelinePreviewByRoomId) { timelinePreviewByRoomId[summary.roomId] }
                ?: return summary

        if (!hasMeaningfulPreview(timelinePreview.message)) return summary

        val shouldOverride =
            summary.latestMessage.isNullOrBlank() ||
                isStatusPreview(summary.latestMessage) ||
                (summary.latestTimestampMillis ?: Long.MIN_VALUE) <= timelinePreview.timestampMillis
        if (!shouldOverride) return summary

        return summary.copy(
            latestMessage = timelinePreview.message,
            latestTimestampMillis = timelinePreview.timestampMillis,
            latestMessageIsMine = timelinePreview.isMine,
            latestMessageSendStatus = timelinePreview.sendStatus,
            latestMessageReadByCount = timelinePreview.readByCount,
        )
    }

    private fun hasMeaningfulPreview(message: String?): Boolean =
        !message.isNullOrBlank() && !isStatusPreview(message)

    private fun isStatusPreview(message: String?): Boolean =
        when (message?.trim()) {
            null, "" -> true
            "Rozpoczęto rozmowę",
            "Zaproszenie",
            "Zaproszenie wysłane",
            "Wiadomość",
            -> true
            else -> false
        }

    private fun loadCachedPreviews() {
        try {
            val rows = db.roomPreviewQueries.selectAll().executeAsList()
            val cachedSummaries = mutableListOf<MatrixRoomSummary>()
            synchronized(timelinePreviewByRoomId) {
                for (row in rows) {
                    timelinePreviewByRoomId[row.room_id] =
                        TimelinePreview(
                            message = row.message,
                            timestampMillis = row.timestamp_millis,
                            isMine = row.is_mine != 0L,
                            sendStatus = row.send_status?.toSendStatus(),
                            readByCount = row.read_by_count.toInt(),
                        )
                    if (row.display_name != null) {
                        cachedSummaries += MatrixRoomSummary(
                            roomId = row.room_id,
                            displayName = row.display_name,
                            avatarUrl = row.avatar_url,
                            isDirect = row.is_direct != 0L,
                            directUserId = row.direct_user_id,
                            unreadCount = 0,
                            latestMessage = row.message,
                            latestTimestampMillis = row.timestamp_millis,
                            latestMessageIsMine = row.is_mine != 0L,
                            latestMessageSendStatus = row.send_status?.toSendStatus(),
                            latestMessageReadByCount = row.read_by_count.toInt(),
                        )
                    }
                }
            }
            if (cachedSummaries.isNotEmpty()) {
                _rooms.value = cachedSummaries.sortedWith(roomSummaryComparator)
            }
        } catch (_: Exception) {
            // DB not yet migrated or corrupted — proceed with empty cache
        }
    }

    private fun persistPreview(
        roomId: String,
        preview: TimelinePreview,
        summary: MatrixRoomSummary? = null,
    ) {
        scope.launch(Dispatchers.IO) {
            try {
                db.roomPreviewQueries.upsertMessage(
                    room_id = roomId,
                    message = preview.message,
                    timestamp_millis = preview.timestampMillis,
                    is_mine = if (preview.isMine) 1L else 0L,
                    send_status = preview.sendStatus?.toDbString(),
                    read_by_count = preview.readByCount.toLong(),
                    updated_at = System.currentTimeMillis(),
                )
                if (summary != null) {
                    db.roomPreviewQueries.updateMetadata(
                        display_name = summary.displayName,
                        avatar_url = summary.avatarUrl,
                        is_direct = if (summary.isDirect) 1L else 0L,
                        direct_user_id = summary.directUserId,
                        unread_count = summary.unreadCount.toLong(),
                        room_id = roomId,
                    )
                }
            } catch (_: Exception) {
                // Best-effort persistence
            }
        }
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

        val serverName =
            (state.value as? MatrixClientState.Ready)
                ?.userId
                ?.substringAfter(':', "")
                ?.ifBlank { null }
                ?: return value
        return "@$value:$serverName"
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
        is LatestEventValue.RemoteInvite -> "Zaproszenie"
    }

private fun LatestEventValue.timestampMillis(): Long? =
    when (this) {
        LatestEventValue.None -> null
        is LatestEventValue.Remote -> this.timestamp.toLong()
        is LatestEventValue.Local -> this.timestamp.toLong()
        is LatestEventValue.RemoteInvite -> this.timestamp.toLong()
    }

private fun LatestEventValue.isMine(ownUserId: String): Boolean? =
    when (this) {
        LatestEventValue.None,
        is LatestEventValue.RemoteInvite,
        -> null

        is LatestEventValue.Remote -> this.isOwn || this.sender.sameMatrixUser(ownUserId)
        is LatestEventValue.Local -> this.sender.sameMatrixUser(ownUserId)
    }

private fun LatestEventValue.toRoomSendStatus(ownUserId: String): MatrixEventSendStatus? =
    when (this) {
        LatestEventValue.None,
        is LatestEventValue.RemoteInvite,
        -> null

        is LatestEventValue.Remote -> {
            if (this.isOwn || this.sender.sameMatrixUser(ownUserId)) MatrixEventSendStatus.Sent else null
        }

        is LatestEventValue.Local -> {
            when (this.state) {
                LatestEventValueLocalState.IS_SENDING -> MatrixEventSendStatus.Sending
                LatestEventValueLocalState.HAS_BEEN_SENT -> MatrixEventSendStatus.Sent
                LatestEventValueLocalState.CANNOT_BE_SENT -> MatrixEventSendStatus.Failed
            }
        }
    }

@Suppress("CyclomaticComplexMethod")
private fun TimelineItemContent.toPreviewText(): String? =
    when (this) {
        is TimelineItemContent.MsgLike -> {
            when (val kind = this.content.kind) {
                is org.matrix.rustcomponents.sdk.MsgLikeKind.Message ->
                    kind.content.body.takeIf { it.isNotBlank() }
                org.matrix.rustcomponents.sdk.MsgLikeKind.Redacted -> "Wiadomość usunięta"
                is org.matrix.rustcomponents.sdk.MsgLikeKind.Poll -> "Ankieta: ${kind.question}"
                is org.matrix.rustcomponents.sdk.MsgLikeKind.Sticker -> kind.body
                is org.matrix.rustcomponents.sdk.MsgLikeKind.UnableToDecrypt -> "Wiadomość zaszyfrowana"
                is org.matrix.rustcomponents.sdk.MsgLikeKind.Other -> "Nieobsługiwana wiadomość"
            }
        }

        is TimelineItemContent.RoomMembership -> {
            when (this.change) {
                org.matrix.rustcomponents.sdk.MembershipChange.JOINED,
                org.matrix.rustcomponents.sdk.MembershipChange.INVITATION_ACCEPTED,
                -> {
                    "Rozpoczęto rozmowę"
                }

                org.matrix.rustcomponents.sdk.MembershipChange.LEFT -> {
                    "${this.userDisplayName ?: this.userId} opuścił(a) rozmowę"
                }

                org.matrix.rustcomponents.sdk.MembershipChange.INVITED -> {
                    "Zaproszenie wysłane"
                }

                else -> {
                    null
                }
            }
        }

        TimelineItemContent.CallInvite -> {
            "Połączenie"
        }

        TimelineItemContent.RtcNotification,
        is TimelineItemContent.ProfileChange,
        is TimelineItemContent.State,
        is TimelineItemContent.FailedToParseMessageLike,
        is TimelineItemContent.FailedToParseState,
        -> {
            null
        }
    }

private data class DirectPeerCandidate(
    val userId: String,
    val displayName: String?,
    val avatarUrl: String?,
)

private data class TimelinePreview(
    val message: String,
    val timestampMillis: Long,
    val isMine: Boolean,
    val sendStatus: MatrixEventSendStatus?,
    val readByCount: Int,
)

private fun MatrixEventSendStatus.toDbString(): String =
    when (this) {
        MatrixEventSendStatus.Sending -> "sending"
        MatrixEventSendStatus.Sent -> "sent"
        MatrixEventSendStatus.Failed -> "failed"
    }

private fun String.toSendStatus(): MatrixEventSendStatus? =
    when (this) {
        "sending" -> MatrixEventSendStatus.Sending
        "sent" -> MatrixEventSendStatus.Sent
        "failed" -> MatrixEventSendStatus.Failed
        else -> null
    }

private suspend fun resolveDirectPeer(
    room: Room,
    ownUserId: String,
): DirectPeerCandidate? {
    val members = runCatching { room.membersNoSync() }.getOrElse { room.members() }
    return try {
        val others = mutableListOf<RoomMember>()
        while (true) {
            val chunk = members.nextChunk(64u) ?: break
            others +=
                chunk.filter { member ->
                    !member.userId.sameMatrixUser(ownUserId) &&
                        when (member.membership) {
                            MembershipState.Join,
                            MembershipState.Invite,
                            MembershipState.Knock,
                            -> true

                            else -> false
                        }
                }
        }

        val preferred =
            others.firstOrNull { it.membership == MembershipState.Join }
                ?: others.firstOrNull { it.membership == MembershipState.Invite }
                ?: others.firstOrNull()
                ?: return null

        DirectPeerCandidate(
            userId = preferred.userId,
            displayName = preferred.displayName,
            avatarUrl = preferred.avatarUrl,
        )
    } finally {
        members.close()
    }
}

private fun pickDirectHeroCandidate(
    ownUserId: String,
    heroes: List<RoomHero>,
    preferredUserId: String?,
): RoomHero? {
    val distinctHeroes =
        heroes
            .distinctBy { it.userId.trim().lowercase() }
            .filterNot { it.userId.sameMatrixUser(ownUserId) }
    return preferredUserId?.let { preferred ->
        distinctHeroes.firstOrNull { it.userId.sameMatrixUser(preferred) }
    } ?: distinctHeroes.firstOrNull()
}

private fun String.sameMatrixUser(other: String): Boolean = trim().equals(other.trim(), ignoreCase = true)

private val MEMBER_COUNT_PATTERN = Regex("^\\d+\\s+(people|person|members?|users?)$", RegexOption.IGNORE_CASE)

private fun isMemberCountName(value: String): Boolean = MEMBER_COUNT_PATTERN.matches(value.trim())

private fun String.matrixDisplayFallback(): String? {
    val localpart = substringAfter("@").substringBefore(":")
    return localpart.takeIf { it.isNotBlank() && it != this }
}
