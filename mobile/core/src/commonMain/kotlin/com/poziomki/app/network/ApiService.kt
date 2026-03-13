package com.poziomki.app.network

import io.ktor.http.encodeURLQueryComponent

private const val MATRIX_DIRECT_ROOMS_PATH = "/api/v1/matrix/dms"

private fun matrixEventRoomPath(eventId: String): String = "/api/v1/matrix/events/$eventId/room"

private fun matrixDirectRoomRequest(targetUserId: String): MatrixDirectRoomRequest = MatrixDirectRoomRequest(targetUserId = targetUserId)

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

    // Matrix bootstrap

    suspend fun getMatrixConfig(): ApiResult<MatrixConfigData> = client.get("/api/v1/matrix/config")

    suspend fun createMatrixSession(
        deviceName: String = "Poziomki Mobile",
        deviceId: String? = null,
    ): ApiResult<MatrixSessionData> =
        client.post(
            "/api/v1/matrix/session",
            MatrixSessionRequest(deviceName = deviceName, deviceId = deviceId),
        )

    suspend fun getMatrixEventRoom(eventId: String): ApiResult<MatrixRoomResolveData> = client.get(matrixEventRoomPath(eventId))

    suspend fun resolveMatrixDirectRoom(targetUserId: String): ApiResult<MatrixRoomResolveData> =
        client.post(MATRIX_DIRECT_ROOMS_PATH, matrixDirectRoomRequest(targetUserId))
}
