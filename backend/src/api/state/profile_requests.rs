use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct CreateProfileBody {
    pub(in crate::api) name: String,
    #[serde(default)]
    pub(in crate::api) bio: Option<String>,
    #[serde(default)]
    pub(in crate::api) program: Option<String>,
    #[serde(default)]
    pub(in crate::api) profile_picture: Option<String>,
    #[serde(default)]
    pub(in crate::api) images: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::api) bio_images: Option<Vec<String>>,
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
pub(in crate::api) struct UpdateProfileBody {
    #[serde(default)]
    pub(in crate::api) name: Option<String>,
    #[serde(default)]
    pub(in crate::api) bio: Option<String>,
    #[serde(default)]
    pub(in crate::api) program: Option<String>,
    #[serde(default)]
    pub(in crate::api) profile_picture: Option<String>,
    #[serde(default)]
    pub(in crate::api) images: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::api) bio_images: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::api) tags: Option<Vec<String>>,
    #[serde(default, rename = "tagIds")]
    pub(in crate::api) tag_ids: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::api) gradient_start: Option<String>,
    #[serde(default)]
    pub(in crate::api) gradient_end: Option<String>,
}
