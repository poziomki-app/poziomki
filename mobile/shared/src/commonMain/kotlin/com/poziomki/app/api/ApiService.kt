package com.poziomki.app.api

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

    suspend fun deleteEvent(id: String): ApiResult<Unit> = client.delete("/api/v1/events/$id")

    suspend fun attendEvent(id: String): ApiResult<Unit> = client.post("/api/v1/events/$id/attend")

    suspend fun leaveEvent(id: String): ApiResult<Unit> = client.delete("/api/v1/events/$id/attend")

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

    // Degrees

    suspend fun getDegrees(): ApiResult<List<Degree>> = client.get("/api/v1/degrees")

    // Matching

    suspend fun getMatchingProfiles(): ApiResult<List<MatchProfile>> = client.get("/api/v1/matching/profiles")

    // Matrix bootstrap

    suspend fun getMatrixConfig(): ApiResult<MatrixConfigData> = client.get("/api/v1/matrix/config")

    suspend fun createMatrixSession(deviceName: String = "Poziomki Mobile"): ApiResult<MatrixSessionData> =
        client.post("/api/v1/matrix/session", MatrixSessionRequest(deviceName = deviceName))
}
