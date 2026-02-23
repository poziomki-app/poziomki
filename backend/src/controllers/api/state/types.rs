use serde::{Deserialize, Deserializer, Serialize};

/// Deserialize a field as `Some(value)` when present in JSON (even if null),
/// and `None` when absent. Used with `Option<Option<T>>` to distinguish
/// "field absent" (outer None) from "field set to null" (Some(None)).
fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::api) struct SignUpBody {
    pub(in crate::controllers::api) email: String,
    pub(in crate::controllers::api) name: String,
    pub(in crate::controllers::api) password: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::controllers::api) struct SignInBody {
    pub(in crate::controllers::api) email: String,
    pub(in crate::controllers::api) password: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::api) struct VerifyOtpBody {
    pub(in crate::controllers::api) email: String,
    pub(in crate::controllers::api) otp: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::api) struct ResendOtpBody {
    pub(in crate::controllers::api) email: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::api) struct DeleteAccountBody {
    pub(in crate::controllers::api) password: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(in crate::controllers::api) enum TagScope {
    Interest,
    Activity,
    Event,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(in crate::controllers::api) enum AttendeeStatus {
    Going,
    Interested,
    Invited,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(in crate::controllers::api) enum UploadContext {
    ProfilePicture,
    ProfileGallery,
    EventCover,
    ChatCover,
    ChatAttachment,
    Bio,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::api) struct CreateTagBody {
    pub(in crate::controllers::api) name: String,
    pub(in crate::controllers::api) scope: TagScope,
    #[serde(default)]
    pub(in crate::controllers::api) category: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) emoji: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::api) struct TagsQuery {
    #[serde(default)]
    pub(in crate::controllers::api) scope: Option<TagScope>,
    #[serde(default)]
    pub(in crate::controllers::api) search: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) limit: Option<u8>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::api) struct DegreesQuery {
    #[serde(default)]
    pub(in crate::controllers::api) search: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) limit: Option<u8>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::api) struct EventsQuery {
    #[serde(default)]
    pub(in crate::controllers::api) limit: Option<u8>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::api) struct MatchingQuery {
    #[serde(default)]
    pub(in crate::controllers::api) limit: Option<u8>,
    #[serde(default)]
    pub(in crate::controllers::api) lat: Option<f64>,
    #[serde(default)]
    pub(in crate::controllers::api) lng: Option<f64>,
    #[serde(default, rename = "radiusM")]
    pub(in crate::controllers::api) radius_m: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::controllers::api) struct CreateProfileBody {
    pub(in crate::controllers::api) name: String,
    pub(in crate::controllers::api) age: u8,
    #[serde(default)]
    pub(in crate::controllers::api) bio: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) program: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) profile_picture: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) images: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::controllers::api) tags: Option<Vec<String>>,
    #[serde(default, rename = "tagIds")]
    pub(in crate::controllers::api) tag_ids: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::controllers::api) gradient_start: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) gradient_end: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::controllers::api) struct UpdateProfileBody {
    #[serde(default)]
    pub(in crate::controllers::api) name: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) age: Option<u8>,
    #[serde(default)]
    pub(in crate::controllers::api) bio: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) program: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) profile_picture: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) images: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::controllers::api) tags: Option<Vec<String>>,
    #[serde(default, rename = "tagIds")]
    pub(in crate::controllers::api) tag_ids: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::controllers::api) gradient_start: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) gradient_end: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub(in crate::controllers::api) struct CreateEventBody {
    pub(in crate::controllers::api) title: String,
    #[serde(default)]
    pub(in crate::controllers::api) description: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) cover_image: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) location: Option<String>,
    pub(in crate::controllers::api) starts_at: String,
    #[serde(default)]
    pub(in crate::controllers::api) ends_at: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) latitude: Option<f64>,
    #[serde(default)]
    pub(in crate::controllers::api) longitude: Option<f64>,
    #[serde(default)]
    pub(in crate::controllers::api) tags: Option<Vec<String>>,
    #[serde(default, rename = "tagIds")]
    pub(in crate::controllers::api) tag_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::option_option, dead_code)]
pub(in crate::controllers::api) struct UpdateEventBody {
    #[serde(default)]
    pub(in crate::controllers::api) title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub(in crate::controllers::api) description: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub(in crate::controllers::api) cover_image: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub(in crate::controllers::api) location: Option<Option<String>>,
    #[serde(default)]
    pub(in crate::controllers::api) starts_at: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub(in crate::controllers::api) ends_at: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub(in crate::controllers::api) latitude: Option<Option<f64>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub(in crate::controllers::api) longitude: Option<Option<f64>>,
    #[serde(default)]
    pub(in crate::controllers::api) tags: Option<Vec<String>>,
    #[serde(default, rename = "tagIds")]
    pub(in crate::controllers::api) tag_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::controllers::api) struct AttendEventBody {
    #[serde(default)]
    pub(in crate::controllers::api) status: Option<AttendeeStatus>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct SuccessResponse {
    pub(in crate::controllers::api) success: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct DataResponse<T> {
    pub(in crate::controllers::api) data: T,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct SessionResponse {
    pub(in crate::controllers::api) session: Option<SessionView>,
    pub(in crate::controllers::api) user: Option<UserView>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct SessionView {
    pub(in crate::controllers::api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::controllers::api) user_id: String,
    #[serde(rename = "expiresAt")]
    pub(in crate::controllers::api) expires_at: String,
    #[serde(rename = "createdAt")]
    pub(in crate::controllers::api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::controllers::api) updated_at: String,
    #[serde(rename = "ipAddress")]
    pub(in crate::controllers::api) ip_address: Option<String>,
    #[serde(rename = "userAgent")]
    pub(in crate::controllers::api) user_agent: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct UserView {
    pub(in crate::controllers::api) id: String,
    pub(in crate::controllers::api) email: String,
    pub(in crate::controllers::api) name: String,
    #[serde(rename = "emailVerified")]
    pub(in crate::controllers::api) email_verified: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct TagResponse {
    pub(in crate::controllers::api) id: String,
    pub(in crate::controllers::api) name: String,
    pub(in crate::controllers::api) scope: TagScope,
    pub(in crate::controllers::api) category: Option<String>,
    pub(in crate::controllers::api) emoji: Option<String>,
    #[serde(rename = "onboardingOrder")]
    pub(in crate::controllers::api) onboarding_order: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct ScopedTagResponse {
    pub(in crate::controllers::api) id: String,
    pub(in crate::controllers::api) name: String,
    pub(in crate::controllers::api) scope: TagScope,
}

pub(in crate::controllers::api) type EventTagResponse = ScopedTagResponse;
pub(in crate::controllers::api) type MatchingTagResponse = ScopedTagResponse;

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct IdNameResponse {
    pub(in crate::controllers::api) id: String,
    pub(in crate::controllers::api) name: String,
}

pub(in crate::controllers::api) type DegreeResponse = IdNameResponse;

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct ProfileResponse {
    pub(in crate::controllers::api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::controllers::api) user_id: String,
    pub(in crate::controllers::api) name: String,
    pub(in crate::controllers::api) bio: Option<String>,
    pub(in crate::controllers::api) age: u8,
    #[serde(rename = "profilePicture")]
    pub(in crate::controllers::api) profile_picture: Option<String>,
    pub(in crate::controllers::api) images: Vec<String>,
    pub(in crate::controllers::api) program: Option<String>,
    #[serde(rename = "gradientStart")]
    pub(in crate::controllers::api) gradient_start: Option<String>,
    #[serde(rename = "gradientEnd")]
    pub(in crate::controllers::api) gradient_end: Option<String>,
    #[serde(rename = "createdAt")]
    pub(in crate::controllers::api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::controllers::api) updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct ProfilePreview {
    pub(in crate::controllers::api) id: String,
    pub(in crate::controllers::api) name: String,
    #[serde(rename = "profilePicture")]
    pub(in crate::controllers::api) profile_picture: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct FullProfileResponse {
    pub(in crate::controllers::api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::controllers::api) user_id: String,
    pub(in crate::controllers::api) name: String,
    pub(in crate::controllers::api) bio: Option<String>,
    pub(in crate::controllers::api) age: u8,
    #[serde(rename = "profilePicture")]
    pub(in crate::controllers::api) profile_picture: Option<String>,
    pub(in crate::controllers::api) images: Vec<String>,
    pub(in crate::controllers::api) program: Option<String>,
    #[serde(rename = "gradientStart")]
    pub(in crate::controllers::api) gradient_start: Option<String>,
    #[serde(rename = "gradientEnd")]
    pub(in crate::controllers::api) gradient_end: Option<String>,
    pub(in crate::controllers::api) tags: Vec<TagResponse>,
    #[serde(rename = "createdAt")]
    pub(in crate::controllers::api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::controllers::api) updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct EventResponse {
    pub(in crate::controllers::api) id: String,
    pub(in crate::controllers::api) title: String,
    pub(in crate::controllers::api) description: Option<String>,
    #[serde(rename = "coverImage")]
    pub(in crate::controllers::api) cover_image: Option<String>,
    pub(in crate::controllers::api) location: Option<String>,
    pub(in crate::controllers::api) latitude: Option<f64>,
    pub(in crate::controllers::api) longitude: Option<f64>,
    #[serde(rename = "startsAt")]
    pub(in crate::controllers::api) starts_at: String,
    #[serde(rename = "endsAt")]
    pub(in crate::controllers::api) ends_at: Option<String>,
    #[serde(rename = "createdAt")]
    pub(in crate::controllers::api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::controllers::api) updated_at: String,
    pub(in crate::controllers::api) creator: ProfilePreview,
    #[serde(rename = "attendeesCount")]
    pub(in crate::controllers::api) attendees_count: usize,
    #[serde(rename = "attendeesPreview")]
    pub(in crate::controllers::api) attendees_preview: Vec<ProfilePreview>,
    pub(in crate::controllers::api) tags: Vec<EventTagResponse>,
    #[serde(rename = "isAttending")]
    pub(in crate::controllers::api) is_attending: bool,
    #[serde(rename = "conversationId")]
    pub(in crate::controllers::api) conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::controllers::api) score: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct AttendeeFullInfo {
    pub(in crate::controllers::api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::controllers::api) user_id: String,
    pub(in crate::controllers::api) name: String,
    #[serde(rename = "profilePicture")]
    pub(in crate::controllers::api) profile_picture: Option<String>,
    pub(in crate::controllers::api) status: AttendeeStatus,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct ProfileRecommendation {
    pub(in crate::controllers::api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::controllers::api) user_id: String,
    pub(in crate::controllers::api) name: String,
    pub(in crate::controllers::api) bio: Option<String>,
    pub(in crate::controllers::api) age: u8,
    #[serde(rename = "profilePicture")]
    pub(in crate::controllers::api) profile_picture: Option<String>,
    pub(in crate::controllers::api) program: Option<String>,
    #[serde(rename = "gradientStart")]
    pub(in crate::controllers::api) gradient_start: Option<String>,
    #[serde(rename = "gradientEnd")]
    pub(in crate::controllers::api) gradient_end: Option<String>,
    #[serde(rename = "createdAt")]
    pub(in crate::controllers::api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::controllers::api) updated_at: String,
    pub(in crate::controllers::api) tags: Vec<MatchingTagResponse>,
    pub(in crate::controllers::api) score: f64,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct UploadResponse {
    pub(in crate::controllers::api) url: String,
    pub(in crate::controllers::api) filename: String,
    pub(in crate::controllers::api) size: usize,
    #[serde(rename = "type")]
    pub(in crate::controllers::api) mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::controllers::api) thumbnail_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::controllers::api) standard_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::controllers::api) thumbhash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::controllers::api) processing: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::controllers::api) struct UploadStatusResponse {
    pub(in crate::controllers::api) filename: String,
    pub(in crate::controllers::api) url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::controllers::api) thumbnail_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::controllers::api) standard_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::controllers::api) thumbhash: Option<String>,
    pub(in crate::controllers::api) processing: bool,
    pub(in crate::controllers::api) has_variants: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::controllers::api) struct DirectUploadPresignBody {
    #[serde(default)]
    pub(in crate::controllers::api) context: Option<String>,
    #[serde(default)]
    pub(in crate::controllers::api) context_id: Option<String>,
    #[serde(rename = "type")]
    pub(in crate::controllers::api) mime_type: String,
    pub(in crate::controllers::api) size: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::controllers::api) struct DirectUploadCompleteBody {
    pub(in crate::controllers::api) filename: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::controllers::api) struct DirectUploadPresignResponse {
    pub(in crate::controllers::api) upload_url: String,
    pub(in crate::controllers::api) method: &'static str,
    pub(in crate::controllers::api) filename: String,
    #[serde(rename = "type")]
    pub(in crate::controllers::api) mime_type: String,
    pub(in crate::controllers::api) expires_in: u64,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct UrlResponse {
    pub(in crate::controllers::api) url: String,
}

pub(in crate::controllers::api) type UploadUrlResponse = UrlResponse;

#[derive(Clone, Debug, Serialize)]
pub(in crate::controllers::api) struct SessionListItem {
    pub(in crate::controllers::api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::controllers::api) user_id: String,
    #[serde(rename = "expiresAt")]
    pub(in crate::controllers::api) expires_at: String,
    #[serde(rename = "createdAt")]
    pub(in crate::controllers::api) created_at: String,
    #[serde(rename = "ipAddress")]
    pub(in crate::controllers::api) ip_address: Option<String>,
    #[serde(rename = "userAgent")]
    pub(in crate::controllers::api) user_agent: Option<String>,
}
