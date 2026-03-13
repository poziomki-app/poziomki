use serde::Serialize;

use super::catalog_responses::EventTagResponse;
use super::profile_responses::ProfilePreview;
use super::shared::AttendeeStatus;

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct EventResponse {
    pub(in crate::api) id: String,
    pub(in crate::api) title: String,
    pub(in crate::api) description: Option<String>,
    #[serde(rename = "coverImage")]
    pub(in crate::api) cover_image: Option<String>,
    pub(in crate::api) location: Option<String>,
    pub(in crate::api) latitude: Option<f64>,
    pub(in crate::api) longitude: Option<f64>,
    #[serde(rename = "startsAt")]
    pub(in crate::api) starts_at: String,
    #[serde(rename = "endsAt")]
    pub(in crate::api) ends_at: Option<String>,
    #[serde(rename = "createdAt")]
    pub(in crate::api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::api) updated_at: String,
    pub(in crate::api) creator: ProfilePreview,
    #[serde(rename = "attendeesCount")]
    pub(in crate::api) attendees_count: usize,
    #[serde(rename = "maxAttendees")]
    pub(in crate::api) max_attendees: Option<i32>,
    #[serde(rename = "attendeesPreview")]
    pub(in crate::api) attendees_preview: Vec<ProfilePreview>,
    pub(in crate::api) tags: Vec<EventTagResponse>,
    #[serde(rename = "isAttending")]
    pub(in crate::api) is_attending: bool,
    #[serde(rename = "isSaved")]
    pub(in crate::api) is_saved: bool,
    #[serde(rename = "isPending")]
    pub(in crate::api) is_pending: bool,
    #[serde(rename = "requiresApproval")]
    pub(in crate::api) requires_approval: bool,
    #[serde(rename = "conversationId")]
    pub(in crate::api) conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::api) score: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct AttendeeFullInfo {
    #[serde(rename = "profileId")]
    pub(in crate::api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::api) user_id: String,
    pub(in crate::api) name: String,
    #[serde(rename = "profilePicture")]
    pub(in crate::api) profile_picture: Option<String>,
    pub(in crate::api) status: AttendeeStatus,
    #[serde(rename = "isCreator")]
    pub(in crate::api) is_creator: bool,
}
