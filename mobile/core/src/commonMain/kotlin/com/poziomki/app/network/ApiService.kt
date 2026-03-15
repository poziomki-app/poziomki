package com.poziomki.app.network

import io.ktor.http.encodeURLQueryComponent

class ApiService(
    private val client: ApiClient,
) {
    // Auth — cookies handled automatically by Ktor HttpCookies plugin

    suspend fun signUp(
        email: String,
        password: String,
        name: String,
    ): ApiResult<AuthResponse> = client.post("/api/v1/auth/sign-up/email", SignUpRequest(email, password, name))

    suspend fun signIn(
        email: String,
        password: String,
    ): ApiResult<AuthResponse> = client.post("/api/v1/auth/sign-in/email", SignInRequest(email, password))

    suspend fun verifyOtp(
        email: String,
        otp: String,
    ): ApiResult<OtpResponse> = client.post("/api/v1/auth/verify-otp", VerifyOtpRequest(email, otp))

    suspend fun resendOtp(email: String): ApiResult<SuccessResponse> = client.post("/api/v1/auth/resend-otp", ResendOtpRequest(email))

    suspend fun signOut(): ApiResult<SuccessResponse> = client.post("/api/v1/auth/sign-out")

    suspend fun exportData(): ApiResult<kotlinx.serialization.json.JsonObject> = client.get("/api/v1/auth/export")

    suspend fun deleteAccount(password: String): ApiResult<SuccessResponse> =
        client.delete("/api/v1/auth/account", DeleteAccountRequest(password))

    // Profiles

    suspend fun getMyProfile(): ApiResult<Profile> = client.get("/api/v1/profiles/me")

    suspend fun getProfile(id: String): ApiResult<Profile> = client.get("/api/v1/profiles/$id")

    suspend fun getProfileFull(id: String): ApiResult<ProfileWithTags> = client.get("/api/v1/profiles/$id/full")

    suspend fun createProfile(request: CreateProfileRequest): ApiResult<Profile> = client.post("/api/v1/profiles", request)

    suspend fun updateProfile(
        id: String,
        request: UpdateProfileRequest,
    ): ApiResult<Profile> = client.patch("/api/v1/profiles/$id", request)

    // Events

    suspend fun getEvents(limit: Int = 20): ApiResult<List<Event>> = client.get("/api/v1/events?limit=$limit")

    suspend fun getMyEvents(): ApiResult<List<Event>> = client.get("/api/v1/events/mine")

    suspend fun getEvent(id: String): ApiResult<Event> = client.get("/api/v1/events/$id")

    suspend fun getEventAttendees(id: String): ApiResult<List<EventAttendee>> = client.get("/api/v1/events/$id/attendees")

    suspend fun createEvent(request: CreateEventRequest): ApiResult<Event> = client.post("/api/v1/events", request)

    suspend fun updateEvent(
        id: String,
        request: UpdateEventRequest,
    ): ApiResult<Event> = client.patch("/api/v1/events/$id", request)

    suspend fun deleteEvent(id: String): ApiResult<SuccessResponse> = client.delete("/api/v1/events/$id")

    suspend fun attendEvent(id: String): ApiResult<Event> =
        client.post(
            "/api/v1/events/$id/attend",
            AttendEventRequest(),
        )

    suspend fun leaveEvent(id: String): ApiResult<Event> = client.delete("/api/v1/events/$id/attend")

    suspend fun saveEvent(id: String): ApiResult<Event> = client.post("/api/v1/events/$id/save")

    suspend fun unsaveEvent(id: String): ApiResult<Event> = client.delete("/api/v1/events/$id/save")

    suspend fun approveAttendee(
        eventId: String,
        profileId: String,
    ): ApiResult<SuccessResponse> = client.post("/api/v1/events/$eventId/attendees/$profileId/approve")

    suspend fun rejectAttendee(
        eventId: String,
        profileId: String,
    ): ApiResult<SuccessResponse> = client.post("/api/v1/events/$eventId/attendees/$profileId/reject")

    // Uploads

    suspend fun uploadImage(
        bytes: ByteArray,
        fileName: String,
        context: String = "profile_gallery",
    ): ApiResult<UploadResponse> = client.uploadFile(bytes, fileName, context)

    // Tags

    suspend fun getTags(scope: String? = null): ApiResult<List<Tag>> {
        val query = scope?.let { "?scope=$it" } ?: ""
        return client.get("/api/v1/tags$query")
    }

    suspend fun searchTags(
        scope: String,
        search: String,
    ): ApiResult<List<Tag>> {
        val encoded = search.encodeURLQueryComponent()
        return client.get("/api/v1/tags?scope=$scope&search=$encoded")
    }

    suspend fun createTag(request: CreateTagRequest): ApiResult<Tag> = client.post("/api/v1/tags", request)

    suspend fun suggestTags(
        scope: String,
        title: String,
        description: String? = null,
    ): ApiResult<List<TagSuggestion>> =
        client.post(
            "/api/v1/tags/suggestions",
            TagSuggestionsRequest(scope = scope, title = title, description = description),
        )

    // Degrees

    suspend fun getDegrees(): ApiResult<List<Degree>> = client.get("/api/v1/degrees")

    // Matching

    suspend fun getMatchingProfiles(): ApiResult<List<MatchProfile>> = client.get("/api/v1/matching/profiles")

    suspend fun getMatchingEvents(
        limit: Int = 20,
        lat: Double? = null,
        lng: Double? = null,
        radiusM: Int? = null,
    ): ApiResult<List<Event>> {
        val sb = StringBuilder("/api/v1/matching/events?limit=$limit")
        if (lat != null && lng != null) {
            sb.append("&lat=$lat&lng=$lng")
            if (radiusM != null) sb.append("&radiusM=$radiusM")
        }
        return client.get(sb.toString())
    }

    // Message search

    suspend fun searchMessageRooms(query: String): ApiResult<MessageSearchResults> {
        val encoded = query.encodeURLQueryComponent()
        return client.get("/api/v1/messages/search?q=$encoded")
    }

    // Search

    suspend fun search(
        query: String,
        limit: Int = 10,
        lat: Double? = null,
        lng: Double? = null,
        radiusM: Int? = null,
    ): ApiResult<SearchResults> {
        val encoded = query.encodeURLQueryComponent()
        val sb = StringBuilder("/api/v1/search?q=$encoded&limit=$limit")
        if (lat != null && lng != null) {
            sb.append("&lat=$lat&lng=$lng")
            if (radiusM != null) sb.append("&radiusM=$radiusM")
        }
        return client.get(sb.toString())
    }

    // Settings

    suspend fun updateSettings(request: UpdateSettingsRequest): ApiResult<UserSettingsResponse> = client.patch("/api/v1/settings", request)

    // Chat (WebSocket backend)

    suspend fun getChatConfig(): ApiResult<ChatConfigData> = client.get("/api/v1/chat/config")

    suspend fun resolveChatDm(targetUserId: String): ApiResult<ChatConversationResolveData> =
        client.post("/api/v1/chat/dms", ChatDmRequest(userId = targetUserId))

    suspend fun getChatEventConversation(eventId: String): ApiResult<ChatConversationResolveData> =
        client.get("/api/v1/chat/events/$eventId/conversation")

    suspend fun registerChatPush(
        deviceId: String,
        ntfyTopic: String,
    ): ApiResult<SuccessResponse> =
        client.post(
            "/api/v1/chat/push/register",
            ChatPushRequest(deviceId = deviceId, ntfyTopic = ntfyTopic),
        )

    suspend fun unregisterChatPush(deviceId: String): ApiResult<SuccessResponse> =
        client.post("/api/v1/chat/push/unregister", ChatPushUnregisterRequest(deviceId = deviceId))
}
