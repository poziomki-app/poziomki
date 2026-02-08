package com.poziomki.app.api

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class ApiResponse<T>(
    val data: T? = null,
)

@Serializable
data class ApiErrorResponse(
    val error: String,
    val code: String,
    val requestId: String? = null,
)

sealed class ApiResult<out T> {
    data class Success<T>(
        val data: T,
    ) : ApiResult<T>()

    data class Error(
        val message: String,
        val code: String,
        val status: Int,
    ) : ApiResult<Nothing>()
}

// Auth models — matches Better Auth response format

@Serializable
data class SignUpRequest(
    val email: String,
    val password: String,
    val name: String,
)

@Serializable
data class SignInRequest(
    val email: String,
    val password: String,
)

@Serializable
data class VerifyOtpRequest(
    val email: String,
    val otp: String,
)

@Serializable
data class ResendOtpRequest(
    val email: String,
)

@Serializable
data class AuthResponse(
    val token: String? = null,
    val user: AuthUser? = null,
)

@Serializable
data class AuthUser(
    val id: String,
    val email: String,
    val name: String,
    val emailVerified: Boolean,
    val image: String? = null,
)

@Serializable
data class OtpResponse(
    val user: AuthUser? = null,
    val status: Boolean? = null,
)

@Serializable
data class SuccessResponse(
    val success: Boolean? = null,
)

// Profile models

@Serializable
data class Profile(
    val id: String,
    val userId: String,
    val name: String,
    val bio: String? = null,
    val age: Int,
    val profilePicture: String? = null,
    val images: List<String> = emptyList(),
    val program: String? = null,
    val createdAt: String? = null,
    val updatedAt: String? = null,
)

@Serializable
data class ProfileWithTags(
    val id: String,
    val userId: String,
    val name: String,
    val bio: String? = null,
    val age: Int,
    val profilePicture: String? = null,
    val images: List<String> = emptyList(),
    val program: String? = null,
    val tags: List<Tag> = emptyList(),
)

@Serializable
data class CreateProfileRequest(
    val name: String,
    val age: Int,
    val bio: String? = null,
    val program: String? = null,
    val tagIds: List<String> = emptyList(),
)

@Serializable
data class UpdateProfileRequest(
    val name: String? = null,
    val bio: String? = null,
    val age: Int? = null,
    val program: String? = null,
    val profilePicture: String? = null,
    val images: List<String>? = null,
    val tagIds: List<String>? = null,
)

// Event models

@Serializable
data class EventCreator(
    val id: String,
    val name: String,
    val profilePicture: String? = null,
)

@Serializable
data class EventAttendeePreview(
    val id: String,
    val name: String,
    val profilePicture: String? = null,
)

@Serializable
data class Event(
    val id: String,
    val title: String,
    val description: String? = null,
    val coverImage: String? = null,
    val location: String? = null,
    val startsAt: String,
    val endsAt: String? = null,
    val creatorId: String? = null,
    val createdAt: String? = null,
    val attendeesCount: Int = 0,
    val isAttending: Boolean = false,
    val creator: EventCreator? = null,
    val attendeesPreview: List<EventAttendeePreview> = emptyList(),
    val conversationId: String? = null,
)

@Serializable
data class CreateEventRequest(
    val title: String,
    val description: String? = null,
    val location: String? = null,
    val startsAt: String,
    val endsAt: String? = null,
    val tagIds: List<String> = emptyList(),
)

@Serializable
data class UpdateEventRequest(
    val title: String? = null,
    val description: String? = null,
    val location: String? = null,
    val startsAt: String? = null,
    val endsAt: String? = null,
)

@Serializable
data class EventAttendee(
    val profileId: String,
    val userId: String? = null,
    val name: String,
    val profilePicture: String? = null,
    val status: String,
)

// Tag models

@Serializable
data class Tag(
    val id: String,
    val name: String,
    val scope: String,
    val category: String? = null,
    val emoji: String? = null,
)

// Upload models

@Serializable
data class UploadResponse(
    val url: String,
    val filename: String,
    val size: Long,
    val type: String,
)

// Degree models

@Serializable
data class Degree(
    val id: String,
    val name: String,
)

// Matching models

@Serializable
data class MatchProfile(
    val id: String,
    val name: String,
    val bio: String? = null,
    val age: Int,
    val profilePicture: String? = null,
    val images: List<String> = emptyList(),
    val program: String? = null,
    val tags: List<Tag> = emptyList(),
    val score: Double = 0.0,
)

// Matrix bootstrap models

@Serializable
data class MatrixConfigEnvelope(
    val data: MatrixConfigData,
)

@Serializable
data class MatrixConfigData(
    val homeserver: String? = null,
    @SerialName("chat_mode")
    val chatMode: String = "matrix-native",
)

@Serializable
data class MatrixSessionRequest(
    val deviceName: String = "Poziomki Mobile",
)

@Serializable
data class MatrixSessionEnvelope(
    val data: MatrixSessionData? = null,
)

@Serializable
data class MatrixSessionData(
    val homeserver: String? = null,
    val accessToken: String? = null,
    val refreshToken: String? = null,
    val userId: String? = null,
    val deviceId: String? = null,
    val expiresAt: Long? = null,
)
