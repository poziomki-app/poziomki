use serde::Serialize;

use super::catalog_responses::TagResponse;

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct ProfileResponse {
    pub(in crate::api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::api) user_id: String,
    pub(in crate::api) name: String,
    pub(in crate::api) bio: Option<String>,
    pub(in crate::api) status: Option<String>,
    #[serde(rename = "statusEmoji", skip_serializing_if = "Option::is_none")]
    pub(in crate::api) status_emoji: Option<String>,
    #[serde(rename = "statusExpiresAt", skip_serializing_if = "Option::is_none")]
    pub(in crate::api) status_expires_at: Option<String>,
    #[serde(rename = "profilePicture")]
    pub(in crate::api) profile_picture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::api) thumbhash: Option<String>,
    pub(in crate::api) images: Vec<String>,
    pub(in crate::api) program: Option<String>,
    #[serde(rename = "gradientStart")]
    pub(in crate::api) gradient_start: Option<String>,
    #[serde(rename = "gradientEnd")]
    pub(in crate::api) gradient_end: Option<String>,
    pub(in crate::api) xp: i32,
    #[serde(rename = "streakCurrent")]
    pub(in crate::api) streak_current: i32,
    #[serde(rename = "streakLongest")]
    pub(in crate::api) streak_longest: i32,
    #[serde(rename = "createdAt")]
    pub(in crate::api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::api) updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct ProfilePreview {
    pub(in crate::api) id: String,
    pub(in crate::api) name: String,
    #[serde(rename = "profilePicture")]
    pub(in crate::api) profile_picture: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct FullProfileResponse {
    pub(in crate::api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::api) user_id: String,
    pub(in crate::api) name: String,
    pub(in crate::api) bio: Option<String>,
    pub(in crate::api) status: Option<String>,
    #[serde(rename = "statusEmoji", skip_serializing_if = "Option::is_none")]
    pub(in crate::api) status_emoji: Option<String>,
    #[serde(rename = "statusExpiresAt", skip_serializing_if = "Option::is_none")]
    pub(in crate::api) status_expires_at: Option<String>,
    #[serde(rename = "profilePicture")]
    pub(in crate::api) profile_picture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::api) thumbhash: Option<String>,
    pub(in crate::api) images: Vec<String>,
    pub(in crate::api) program: Option<String>,
    #[serde(rename = "gradientStart")]
    pub(in crate::api) gradient_start: Option<String>,
    #[serde(rename = "gradientEnd")]
    pub(in crate::api) gradient_end: Option<String>,
    pub(in crate::api) tags: Vec<TagResponse>,
    #[serde(rename = "isBookmarked")]
    pub(in crate::api) is_bookmarked: bool,
    pub(in crate::api) xp: i32,
    #[serde(rename = "streakCurrent")]
    pub(in crate::api) streak_current: i32,
    #[serde(rename = "streakLongest")]
    pub(in crate::api) streak_longest: i32,
    #[serde(rename = "createdAt")]
    pub(in crate::api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::api) updated_at: String,
}
