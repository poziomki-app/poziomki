use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::migration_api) struct SignUpBody {
    pub(in crate::controllers::migration_api) email: String,
    pub(in crate::controllers::migration_api) name: String,
    pub(in crate::controllers::migration_api) password: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::controllers::migration_api) struct SignInBody {
    pub(in crate::controllers::migration_api) email: String,
    pub(in crate::controllers::migration_api) password: String,
    #[serde(default)]
    pub(in crate::controllers::migration_api) remember_me: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::migration_api) struct VerifyOtpBody {
    pub(in crate::controllers::migration_api) email: String,
    pub(in crate::controllers::migration_api) otp: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::migration_api) struct ResendOtpBody {
    pub(in crate::controllers::migration_api) email: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::migration_api) struct DeleteAccountBody {
    pub(in crate::controllers::migration_api) password: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(in crate::controllers::migration_api) enum TagScope {
    Interest,
    Activity,
    Event,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(in crate::controllers::migration_api) enum AttendeeStatus {
    Going,
    Interested,
    Invited,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(in crate::controllers::migration_api) enum UploadContext {
    ProfilePicture,
    ProfileGallery,
    EventCover,
    ChatCover,
    ChatAttachment,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::migration_api) struct CreateTagBody {
    pub(in crate::controllers::migration_api) name: String,
    pub(in crate::controllers::migration_api) scope: TagScope,
    #[serde(default)]
    pub(in crate::controllers::migration_api) category: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) emoji: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::migration_api) struct TagsQuery {
    pub(in crate::controllers::migration_api) scope: TagScope,
    #[serde(default)]
    pub(in crate::controllers::migration_api) search: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) limit: Option<u8>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::migration_api) struct DegreesQuery {
    #[serde(default)]
    pub(in crate::controllers::migration_api) search: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) limit: Option<u8>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::migration_api) struct EventsQuery {
    #[serde(default)]
    pub(in crate::controllers::migration_api) limit: Option<u8>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::migration_api) struct MatchingQuery {
    #[serde(default)]
    pub(in crate::controllers::migration_api) limit: Option<u8>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::controllers::migration_api) struct CreateProfileBody {
    pub(in crate::controllers::migration_api) name: String,
    pub(in crate::controllers::migration_api) age: u8,
    #[serde(default)]
    pub(in crate::controllers::migration_api) bio: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) program: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) profile_picture: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) images: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) tags: Option<Vec<String>>,
    #[serde(default, rename = "tagIds")]
    pub(in crate::controllers::migration_api) tag_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::controllers::migration_api) struct UpdateProfileBody {
    #[serde(default)]
    pub(in crate::controllers::migration_api) name: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) age: Option<u8>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) bio: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) program: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) profile_picture: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) images: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) tags: Option<Vec<String>>,
    #[serde(default, rename = "tagIds")]
    pub(in crate::controllers::migration_api) tag_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::controllers::migration_api) struct CreateEventBody {
    pub(in crate::controllers::migration_api) title: String,
    #[serde(default)]
    pub(in crate::controllers::migration_api) description: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) cover_image: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) location: Option<String>,
    pub(in crate::controllers::migration_api) starts_at: String,
    #[serde(default)]
    pub(in crate::controllers::migration_api) ends_at: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) tags: Option<Vec<String>>,
    #[serde(default, rename = "tagIds")]
    pub(in crate::controllers::migration_api) tag_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::option_option)]
pub(in crate::controllers::migration_api) struct UpdateEventBody {
    #[serde(default)]
    pub(in crate::controllers::migration_api) title: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) description: Option<Option<String>>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) cover_image: Option<Option<String>>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) location: Option<Option<String>>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) starts_at: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) ends_at: Option<Option<String>>,
    #[serde(default)]
    pub(in crate::controllers::migration_api) tags: Option<Vec<String>>,
    #[serde(default, rename = "tagIds")]
    pub(in crate::controllers::migration_api) tag_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::migration_api) struct AttendEventBody {
    #[serde(default)]
    pub(in crate::controllers::migration_api) status: Option<AttendeeStatus>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct SuccessResponse {
    pub(in crate::controllers::migration_api) success: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct AuthCheckResponse {
    pub(in crate::controllers::migration_api) ok: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct DataResponse<T> {
    pub(in crate::controllers::migration_api) data: T,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct SessionResponse {
    pub(in crate::controllers::migration_api) session: Option<SessionView>,
    pub(in crate::controllers::migration_api) user: Option<UserView>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct SessionView {
    pub(in crate::controllers::migration_api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::controllers::migration_api) user_id: String,
    pub(in crate::controllers::migration_api) token: String,
    #[serde(rename = "expiresAt")]
    pub(in crate::controllers::migration_api) expires_at: String,
    #[serde(rename = "createdAt")]
    pub(in crate::controllers::migration_api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::controllers::migration_api) updated_at: String,
    #[serde(rename = "ipAddress")]
    pub(in crate::controllers::migration_api) ip_address: Option<String>,
    #[serde(rename = "userAgent")]
    pub(in crate::controllers::migration_api) user_agent: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct UserView {
    pub(in crate::controllers::migration_api) id: String,
    pub(in crate::controllers::migration_api) email: String,
    pub(in crate::controllers::migration_api) name: String,
    #[serde(rename = "emailVerified")]
    pub(in crate::controllers::migration_api) email_verified: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct TagResponse {
    pub(in crate::controllers::migration_api) id: String,
    pub(in crate::controllers::migration_api) name: String,
    pub(in crate::controllers::migration_api) scope: TagScope,
    pub(in crate::controllers::migration_api) category: Option<String>,
    pub(in crate::controllers::migration_api) emoji: Option<String>,
    #[serde(rename = "onboardingOrder")]
    pub(in crate::controllers::migration_api) onboarding_order: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct EventTagResponse {
    pub(in crate::controllers::migration_api) id: String,
    pub(in crate::controllers::migration_api) name: String,
    pub(in crate::controllers::migration_api) scope: TagScope,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct DegreeResponse {
    pub(in crate::controllers::migration_api) id: String,
    pub(in crate::controllers::migration_api) name: String,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct ProfileResponse {
    pub(in crate::controllers::migration_api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::controllers::migration_api) user_id: String,
    pub(in crate::controllers::migration_api) name: String,
    pub(in crate::controllers::migration_api) bio: Option<String>,
    pub(in crate::controllers::migration_api) age: u8,
    #[serde(rename = "profilePicture")]
    pub(in crate::controllers::migration_api) profile_picture: Option<String>,
    pub(in crate::controllers::migration_api) images: Vec<String>,
    pub(in crate::controllers::migration_api) program: Option<String>,
    #[serde(rename = "createdAt")]
    pub(in crate::controllers::migration_api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::controllers::migration_api) updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct ProfilePreview {
    pub(in crate::controllers::migration_api) id: String,
    pub(in crate::controllers::migration_api) name: String,
    #[serde(rename = "profilePicture")]
    pub(in crate::controllers::migration_api) profile_picture: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct FullProfileResponse {
    pub(in crate::controllers::migration_api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::controllers::migration_api) user_id: String,
    pub(in crate::controllers::migration_api) name: String,
    pub(in crate::controllers::migration_api) bio: Option<String>,
    pub(in crate::controllers::migration_api) age: u8,
    #[serde(rename = "profilePicture")]
    pub(in crate::controllers::migration_api) profile_picture: Option<String>,
    pub(in crate::controllers::migration_api) images: Vec<String>,
    pub(in crate::controllers::migration_api) program: Option<String>,
    pub(in crate::controllers::migration_api) tags: Vec<TagResponse>,
    #[serde(rename = "createdAt")]
    pub(in crate::controllers::migration_api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::controllers::migration_api) updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct EventResponse {
    pub(in crate::controllers::migration_api) id: String,
    pub(in crate::controllers::migration_api) title: String,
    pub(in crate::controllers::migration_api) description: Option<String>,
    #[serde(rename = "coverImage")]
    pub(in crate::controllers::migration_api) cover_image: Option<String>,
    pub(in crate::controllers::migration_api) location: Option<String>,
    #[serde(rename = "startsAt")]
    pub(in crate::controllers::migration_api) starts_at: String,
    #[serde(rename = "endsAt")]
    pub(in crate::controllers::migration_api) ends_at: Option<String>,
    #[serde(rename = "createdAt")]
    pub(in crate::controllers::migration_api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::controllers::migration_api) updated_at: String,
    pub(in crate::controllers::migration_api) creator: ProfilePreview,
    #[serde(rename = "attendeesCount")]
    pub(in crate::controllers::migration_api) attendees_count: usize,
    #[serde(rename = "attendeesPreview")]
    pub(in crate::controllers::migration_api) attendees_preview: Vec<ProfilePreview>,
    pub(in crate::controllers::migration_api) tags: Vec<EventTagResponse>,
    #[serde(rename = "isAttending")]
    pub(in crate::controllers::migration_api) is_attending: bool,
    #[serde(rename = "conversationId")]
    pub(in crate::controllers::migration_api) conversation_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct AttendeeFullInfo {
    pub(in crate::controllers::migration_api) id: String,
    pub(in crate::controllers::migration_api) name: String,
    #[serde(rename = "profilePicture")]
    pub(in crate::controllers::migration_api) profile_picture: Option<String>,
    pub(in crate::controllers::migration_api) status: AttendeeStatus,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct MatchingTagResponse {
    pub(in crate::controllers::migration_api) id: String,
    pub(in crate::controllers::migration_api) name: String,
    pub(in crate::controllers::migration_api) scope: TagScope,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct ProfileRecommendation {
    pub(in crate::controllers::migration_api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::controllers::migration_api) user_id: String,
    pub(in crate::controllers::migration_api) name: String,
    pub(in crate::controllers::migration_api) bio: Option<String>,
    pub(in crate::controllers::migration_api) age: u8,
    #[serde(rename = "profilePicture")]
    pub(in crate::controllers::migration_api) profile_picture: Option<String>,
    pub(in crate::controllers::migration_api) program: Option<String>,
    #[serde(rename = "createdAt")]
    pub(in crate::controllers::migration_api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::controllers::migration_api) updated_at: String,
    pub(in crate::controllers::migration_api) tags: Vec<MatchingTagResponse>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct UploadResponse {
    pub(in crate::controllers::migration_api) url: String,
    pub(in crate::controllers::migration_api) filename: String,
    pub(in crate::controllers::migration_api) size: usize,
    #[serde(rename = "type")]
    pub(in crate::controllers::migration_api) mime_type: String,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct UploadUrlResponse {
    pub(in crate::controllers::migration_api) url: String,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::migration_api) struct SessionListItem {
    pub(in crate::controllers::migration_api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::controllers::migration_api) user_id: String,
    #[serde(rename = "expiresAt")]
    pub(in crate::controllers::migration_api) expires_at: String,
    #[serde(rename = "createdAt")]
    pub(in crate::controllers::migration_api) created_at: String,
    #[serde(rename = "ipAddress")]
    pub(in crate::controllers::migration_api) ip_address: Option<String>,
    #[serde(rename = "userAgent")]
    pub(in crate::controllers::migration_api) user_agent: Option<String>,
}

#[derive(Clone, Debug)]
pub(in crate::controllers::migration_api) struct UserRecord {
    pub(in crate::controllers::migration_api) id: String,
    pub(in crate::controllers::migration_api) email: String,
    pub(in crate::controllers::migration_api) name: String,
    pub(in crate::controllers::migration_api) password: String,
    pub(in crate::controllers::migration_api) email_verified: bool,
    pub(in crate::controllers::migration_api) created_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub(in crate::controllers::migration_api) struct SessionRecord {
    pub(in crate::controllers::migration_api) id: String,
    pub(in crate::controllers::migration_api) user_id: String,
    pub(in crate::controllers::migration_api) token: String,
    pub(in crate::controllers::migration_api) created_at: DateTime<Utc>,
    pub(in crate::controllers::migration_api) updated_at: DateTime<Utc>,
    pub(in crate::controllers::migration_api) expires_at: DateTime<Utc>,
    pub(in crate::controllers::migration_api) ip_address: Option<String>,
    pub(in crate::controllers::migration_api) user_agent: Option<String>,
}

#[derive(Clone, Debug)]
pub(in crate::controllers::migration_api) struct TagRecord {
    pub(in crate::controllers::migration_api) id: String,
    pub(in crate::controllers::migration_api) name: String,
    pub(in crate::controllers::migration_api) scope: TagScope,
    pub(in crate::controllers::migration_api) category: Option<String>,
    pub(in crate::controllers::migration_api) emoji: Option<String>,
    pub(in crate::controllers::migration_api) onboarding_order: Option<String>,
}

#[derive(Clone, Debug)]
pub(in crate::controllers::migration_api) struct ProfileRecord {
    pub(in crate::controllers::migration_api) id: String,
    pub(in crate::controllers::migration_api) user_id: String,
    pub(in crate::controllers::migration_api) name: String,
    pub(in crate::controllers::migration_api) bio: Option<String>,
    pub(in crate::controllers::migration_api) age: u8,
    pub(in crate::controllers::migration_api) profile_picture: Option<String>,
    pub(in crate::controllers::migration_api) images: Vec<String>,
    pub(in crate::controllers::migration_api) program: Option<String>,
    pub(in crate::controllers::migration_api) tag_ids: Vec<String>,
    pub(in crate::controllers::migration_api) created_at: DateTime<Utc>,
    pub(in crate::controllers::migration_api) updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub(in crate::controllers::migration_api) struct DegreeRecord {
    pub(in crate::controllers::migration_api) id: String,
    pub(in crate::controllers::migration_api) name: String,
}

#[derive(Clone, Debug)]
pub(in crate::controllers::migration_api) struct EventRecord {
    pub(in crate::controllers::migration_api) id: String,
    pub(in crate::controllers::migration_api) title: String,
    pub(in crate::controllers::migration_api) description: Option<String>,
    pub(in crate::controllers::migration_api) cover_image: Option<String>,
    pub(in crate::controllers::migration_api) location: Option<String>,
    pub(in crate::controllers::migration_api) starts_at: DateTime<Utc>,
    pub(in crate::controllers::migration_api) ends_at: Option<DateTime<Utc>>,
    pub(in crate::controllers::migration_api) creator_id: String,
    pub(in crate::controllers::migration_api) conversation_id: Option<String>,
    pub(in crate::controllers::migration_api) tag_ids: Vec<String>,
    pub(in crate::controllers::migration_api) created_at: DateTime<Utc>,
    pub(in crate::controllers::migration_api) updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub(in crate::controllers::migration_api) struct UploadRecord {
    pub(in crate::controllers::migration_api) owner_id: Option<String>,
    pub(in crate::controllers::migration_api) context: UploadContext,
    pub(in crate::controllers::migration_api) context_id: Option<String>,
    pub(in crate::controllers::migration_api) mime_type: String,
    pub(in crate::controllers::migration_api) deleted: bool,
}

#[derive(Default)]
pub(in crate::controllers::migration_api) struct MigrationState {
    pub(in crate::controllers::migration_api) users: HashMap<String, UserRecord>,
    pub(in crate::controllers::migration_api) users_by_email: HashMap<String, String>,
    pub(in crate::controllers::migration_api) sessions_by_token: HashMap<String, SessionRecord>,
    pub(in crate::controllers::migration_api) profiles: HashMap<String, ProfileRecord>,
    pub(in crate::controllers::migration_api) profiles_by_user: HashMap<String, String>,
    pub(in crate::controllers::migration_api) tags: HashMap<String, TagRecord>,
    pub(in crate::controllers::migration_api) degrees: Vec<DegreeRecord>,
    pub(in crate::controllers::migration_api) events: HashMap<String, EventRecord>,
    pub(in crate::controllers::migration_api) event_attendees:
        HashMap<(String, String), AttendeeStatus>,
    pub(in crate::controllers::migration_api) uploads: HashMap<String, UploadRecord>,
    pub(in crate::controllers::migration_api) upload_blobs: HashMap<String, Vec<u8>>,
    pub(in crate::controllers::migration_api) otp_by_email: HashMap<String, String>,
}
