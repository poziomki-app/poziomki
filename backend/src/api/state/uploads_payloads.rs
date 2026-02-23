use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct UploadResponse {
    pub(in crate::api) url: String,
    pub(in crate::api) filename: String,
    pub(in crate::api) size: usize,
    #[serde(rename = "type")]
    pub(in crate::api) mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::api) thumbnail_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::api) standard_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::api) thumbhash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::api) processing: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct UploadStatusResponse {
    pub(in crate::api) filename: String,
    pub(in crate::api) url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::api) thumbnail_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::api) standard_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::api) thumbhash: Option<String>,
    pub(in crate::api) processing: bool,
    pub(in crate::api) has_variants: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct DirectUploadPresignBody {
    #[serde(default)]
    pub(in crate::api) context: Option<String>,
    #[serde(default)]
    pub(in crate::api) context_id: Option<String>,
    #[serde(rename = "type")]
    pub(in crate::api) mime_type: String,
    pub(in crate::api) size: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct DirectUploadCompleteBody {
    pub(in crate::api) filename: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct DirectUploadPresignResponse {
    pub(in crate::api) upload_url: String,
    pub(in crate::api) method: &'static str,
    pub(in crate::api) filename: String,
    #[serde(rename = "type")]
    pub(in crate::api) mime_type: String,
    pub(in crate::api) expires_in: u64,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct UrlResponse {
    pub(in crate::api) url: String,
}

pub(in crate::api) type UploadUrlResponse = UrlResponse;
