use serde::Serialize;

use super::catalog_responses::MatchingTagResponse;

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct ProfileRecommendation {
    pub(in crate::api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::api) user_id: String,
    pub(in crate::api) name: String,
    pub(in crate::api) bio: Option<String>,
    pub(in crate::api) age: u8,
    #[serde(rename = "profilePicture")]
    pub(in crate::api) profile_picture: Option<String>,
    pub(in crate::api) program: Option<String>,
    #[serde(rename = "gradientStart")]
    pub(in crate::api) gradient_start: Option<String>,
    #[serde(rename = "gradientEnd")]
    pub(in crate::api) gradient_end: Option<String>,
    #[serde(rename = "createdAt")]
    pub(in crate::api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::api) updated_at: String,
    pub(in crate::api) tags: Vec<MatchingTagResponse>,
    pub(in crate::api) score: f64,
}
