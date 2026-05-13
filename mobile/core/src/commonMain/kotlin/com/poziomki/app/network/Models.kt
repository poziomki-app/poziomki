package com.poziomki.app.network

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonPrimitive

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
    val token: String? = null,
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
    val profilePicture: String? = null,
    val thumbhash: String? = null,
    val images: List<String> = emptyList(),
    val program: String? = null,
    val gradientStart: String? = null,
    val gradientEnd: String? = null,
    val createdAt: String? = null,
    val updatedAt: String? = null,
)

@Serializable
data class ProfileWithTags(
    val id: String,
    val userId: String,
    val name: String,
    val bio: String? = null,
    val profilePicture: String? = null,
    val thumbhash: String? = null,
    val images: List<String> = emptyList(),
    val program: String? = null,
    val gradientStart: String? = null,
    val gradientEnd: String? = null,
    val tags: List<Tag> = emptyList(),
    val isBookmarked: Boolean = false,
)

@Serializable
data class CreateProfileRequest(
    val name: String,
    val bio: String? = null,
    val program: String? = null,
    val tagIds: List<String> = emptyList(),
)

@Serializable
data class UpdateProfileRequest(
    val name: String? = null,
    val bio: String? = null,
    val program: String? = null,
    val profilePicture: JsonElement? = null,
    val images: List<String>? = null,
    val tagIds: List<String>? = null,
    val gradientStart: String? = null,
    val gradientEnd: String? = null,
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
    val category: String? = null,
    val location: String? = null,
    val latitude: Double? = null,
    val longitude: Double? = null,
    val startsAt: String,
    val endsAt: String? = null,
    val createdAt: String? = null,
    val attendeesCount: Int = 0,
    val maxAttendees: Int? = null,
    val isAttending: Boolean = false,
    val isSaved: Boolean = false,
    val isPending: Boolean = false,
    val requiresApproval: Boolean = false,
    val creator: EventCreator? = null,
    val attendeesPreview: List<EventAttendeePreview> = emptyList(),
    val tags: List<Tag> = emptyList(),
    val conversationId: String? = null,
    val labels: List<String> = emptyList(),
    val isOnline: Boolean = false,
    val meetingUrl: String? = null,
    val score: Double = 0.0,
)

@Serializable
data class CreateEventRequest(
    val title: String,
    val description: String? = null,
    val coverImage: String? = null,
    val category: String? = null,
    val location: String? = null,
    val startsAt: String,
    val endsAt: String? = null,
    val latitude: Double? = null,
    val longitude: Double? = null,
    val maxAttendees: Int? = null,
    val tagIds: List<String> = emptyList(),
    val requiresApproval: Boolean? = null,
    val isOnline: Boolean? = null,
    val meetingUrl: String? = null,
)

@Serializable
data class UpdateEventRequest(
    val title: String? = null,
    val description: String? = null,
    val coverImage: String? = null,
    val category: String? = null,
    val location: String? = null,
    val startsAt: String? = null,
    val endsAt: String? = null,
    val latitude: Double? = null,
    val longitude: Double? = null,
    val tagIds: List<String>? = null,
    val maxAttendees: JsonElement = JsonNull,
    val requiresApproval: Boolean? = null,
    val isOnline: Boolean? = null,
    val meetingUrl: String? = null,
) {
    companion object {
        fun maxAttendeesValue(value: Int?): JsonElement = value?.let { JsonPrimitive(it) } ?: JsonNull
    }
}

@Serializable
data class AttendEventRequest(
    val status: String? = null,
)

@Serializable
data class EventAttendee(
    val profileId: String,
    val userId: String? = null,
    val name: String,
    val profilePicture: String? = null,
    val status: String,
    val isCreator: Boolean = false,
)

// Tag models

@Serializable
data class Tag(
    val id: String,
    val name: String,
    val scope: String,
    val category: String? = null,
    val emoji: String? = null,
    val parentId: String? = null,
)

@Serializable
data class CreateTagRequest(
    val name: String,
    val scope: String,
    val category: String? = null,
    val parentId: String? = null,
)

@Serializable
data class TagSuggestion(
    val tag: Tag,
    val score: Double = 0.0,
)

@Serializable
data class TagSuggestionsRequest(
    val scope: String,
    val title: String,
    val description: String? = null,
)

// Upload models

@Serializable
data class UploadResponse(
    val id: String,
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
    val userId: String = "",
    val name: String,
    val bio: String? = null,
    val profilePicture: String? = null,
    val thumbhash: String? = null,
    val images: List<String> = emptyList(),
    val program: String? = null,
    val gradientStart: String? = null,
    val gradientEnd: String? = null,
    val tags: List<Tag> = emptyList(),
    val score: Double = 0.0,
)

// Chat (WebSocket backend) models

@Serializable
data class ChatConfigData(
    @SerialName("chatMode")
    val chatMode: String = "ws",
    @SerialName("pushProvider")
    val pushProvider: String? = null,
)

@Serializable
data class ChatConversationResolveData(
    val conversationId: String,
)

@Serializable
data class ChatDmRequest(
    val userId: String,
)

@Serializable
data class ChatPushRequest(
    val deviceId: String,
    val fcmToken: String,
    val platform: String,
)

@Serializable
data class ChatPushUnregisterRequest(
    val deviceId: String,
)

@Serializable
data class ReportConversationRequest(
    val reason: String,
    val description: String? = null,
)

// Message search models

@Serializable
data class MessageSearchResults(
    @SerialName("room_ids")
    val roomIds: List<String> = emptyList(),
)

// Search models

@Serializable
data class SearchResults(
    val profiles: List<SearchProfile> = emptyList(),
    val events: List<SearchEvent> = emptyList(),
    val tags: List<SearchTag> = emptyList(),
)

@Serializable
data class SearchProfile(
    val id: String,
    val name: String,
    val bio: String? = null,
    val program: String? = null,
    @SerialName("profile_picture")
    val profilePicture: String? = null,
    val tags: List<String> = emptyList(),
)

@Serializable
data class SearchGeoPoint(
    val lat: Double,
    val lng: Double,
)

@Serializable
data class SearchEvent(
    val id: String,
    val title: String,
    val description: String? = null,
    val location: String? = null,
    @SerialName("starts_at")
    val startsAt: String,
    @SerialName("cover_image")
    val coverImage: String? = null,
    @SerialName("creator_name")
    val creatorName: String,
    val geo: SearchGeoPoint? = null,
)

@Serializable
data class SearchTag(
    val id: String,
    val name: String,
    val scope: String,
    val category: String? = null,
    val emoji: String? = null,
    val parentId: String? = null,
)

@Serializable
data class SearchDegree(
    val id: String,
    val name: String,
)

// Account models

@Serializable
data class DeleteAccountRequest(
    val password: String,
)

@Serializable
data class ChangePasswordRequest(
    val currentPassword: String,
    val newPassword: String,
)

@Serializable
data class RequestEmailChangeRequest(
    val newEmail: String,
    val currentPassword: String,
)

@Serializable
data class ConfirmEmailChangeRequest(
    val newEmail: String,
    val code: String,
)

@Serializable
data class EmailChangeResponse(
    val success: Boolean,
    val email: String,
)

// Forgot password models

@Serializable
data class ForgotPasswordRequest(
    val email: String,
)

@Serializable
data class ForgotPasswordVerifyRequest(
    val email: String,
    val otp: String,
)

@Serializable
data class ResetPasswordRequest(
    val email: String,
    val resetToken: String,
    val newPassword: String,
)

@Serializable
data class ResetTokenResponse(
    val resetToken: String,
)

// Geocoding models

data class GeocodingResult(
    val name: String,
    val lat: Double,
    val lng: Double,
)

// Settings models

@Serializable
data class UpdateSettingsRequest(
    val theme: String? = null,
    val language: String? = null,
    val notificationsEnabled: Boolean? = null,
    val privacyShowProgram: Boolean? = null,
    val privacyDiscoverable: Boolean? = null,
)

@Serializable
data class UserSettingsResponse(
    val theme: String,
    val language: String,
    val notificationsEnabled: Boolean,
    val privacyShowProgram: Boolean,
    val privacyDiscoverable: Boolean,
)

// Routing models

@Serializable
data class WalkingRouteResponse(
    @SerialName("geometryJson")
    val geometryJson: String,
    @SerialName("distanceMeters")
    val distanceMeters: Double,
    @SerialName("durationSeconds")
    val durationSeconds: Double,
)
