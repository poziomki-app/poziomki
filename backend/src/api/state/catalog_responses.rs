use serde::Serialize;

use super::shared::TagScope;

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct TagResponse {
    pub(in crate::api) id: String,
    pub(in crate::api) name: String,
    pub(in crate::api) scope: TagScope,
    pub(in crate::api) category: Option<String>,
    pub(in crate::api) emoji: Option<String>,
    #[serde(rename = "parentId")]
    pub(in crate::api) parent_id: Option<String>,
    #[serde(rename = "onboardingOrder")]
    pub(in crate::api) onboarding_order: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct ScopedTagResponse {
    pub(in crate::api) id: String,
    pub(in crate::api) name: String,
    pub(in crate::api) scope: TagScope,
    #[serde(rename = "parentId")]
    pub(in crate::api) parent_id: Option<String>,
}

pub(in crate::api) type EventTagResponse = ScopedTagResponse;
pub(in crate::api) type MatchingTagResponse = ScopedTagResponse;

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct TagSuggestionResponse {
    pub(in crate::api) tag: TagResponse,
    pub(in crate::api) score: f64,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct IdNameResponse {
    pub(in crate::api) id: String,
    pub(in crate::api) name: String,
}

pub(in crate::api) type DegreeResponse = IdNameResponse;
