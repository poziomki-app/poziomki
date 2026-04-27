use serde::{Deserialize, Deserializer};

fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct CreateProfileBody {
    pub(in crate::api) name: String,
    #[serde(default)]
    pub(in crate::api) bio: Option<String>,
    #[serde(default)]
    pub(in crate::api) status: Option<String>,
    #[serde(default)]
    pub(in crate::api) program: Option<String>,
    #[serde(default)]
    pub(in crate::api) profile_picture: Option<String>,
    #[serde(default)]
    pub(in crate::api) images: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::api) tags: Option<Vec<String>>,
    #[serde(default, rename = "tagIds")]
    pub(in crate::api) tag_ids: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::api) gradient_start: Option<String>,
    #[serde(default)]
    pub(in crate::api) gradient_end: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::option_option)]
pub(in crate::api) struct UpdateProfileBody {
    #[serde(default)]
    pub(in crate::api) name: Option<String>,
    #[serde(default)]
    pub(in crate::api) bio: Option<String>,
    #[serde(default)]
    pub(in crate::api) status: Option<String>,
    #[serde(default)]
    pub(in crate::api) program: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub(in crate::api) profile_picture: Option<Option<String>>,
    #[serde(default)]
    pub(in crate::api) images: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::api) tags: Option<Vec<String>>,
    #[serde(default, rename = "tagIds")]
    pub(in crate::api) tag_ids: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::api) gradient_start: Option<String>,
    #[serde(default)]
    pub(in crate::api) gradient_end: Option<String>,
}
