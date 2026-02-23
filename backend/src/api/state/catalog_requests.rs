use serde::Deserialize;

use super::shared::TagScope;

#[derive(Clone, Debug, Deserialize)]
pub(in crate::api) struct CreateTagBody {
    pub(in crate::api) name: String,
    pub(in crate::api) scope: TagScope,
    #[serde(default)]
    pub(in crate::api) category: Option<String>,
    #[serde(default)]
    pub(in crate::api) emoji: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::api) struct TagsQuery {
    #[serde(default)]
    pub(in crate::api) scope: Option<TagScope>,
    #[serde(default)]
    pub(in crate::api) search: Option<String>,
    #[serde(default)]
    pub(in crate::api) limit: Option<u8>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::api) struct DegreesQuery {
    #[serde(default)]
    pub(in crate::api) search: Option<String>,
    #[serde(default)]
    pub(in crate::api) limit: Option<u8>,
}
