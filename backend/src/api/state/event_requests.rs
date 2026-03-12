use serde::{Deserialize, Deserializer};

use super::shared::AttendeeStatus;

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
pub(in crate::api) struct EventsQuery {
    #[serde(default)]
    pub(in crate::api) limit: Option<u8>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct CreateEventBody {
    pub(in crate::api) title: String,
    #[serde(default)]
    pub(in crate::api) description: Option<String>,
    #[serde(default)]
    pub(in crate::api) cover_image: Option<String>,
    #[serde(default)]
    pub(in crate::api) location: Option<String>,
    pub(in crate::api) starts_at: String,
    #[serde(default)]
    pub(in crate::api) ends_at: Option<String>,
    #[serde(default)]
    pub(in crate::api) latitude: Option<f64>,
    #[serde(default)]
    pub(in crate::api) longitude: Option<f64>,
    #[serde(default)]
    pub(in crate::api) tags: Option<Vec<String>>,
    #[serde(default, rename = "tagIds")]
    pub(in crate::api) tag_ids: Option<Vec<String>>,
    #[serde(default)]
    pub(in crate::api) max_attendees: Option<i32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::option_option)]
pub(in crate::api) struct UpdateEventBody {
    #[serde(default)]
    pub(in crate::api) title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub(in crate::api) description: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub(in crate::api) cover_image: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub(in crate::api) location: Option<Option<String>>,
    #[serde(default)]
    pub(in crate::api) starts_at: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub(in crate::api) ends_at: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub(in crate::api) latitude: Option<Option<f64>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub(in crate::api) longitude: Option<Option<f64>>,
    #[serde(default)]
    pub(in crate::api) tags: Option<Vec<String>>,
    #[serde(default, rename = "tagIds")]
    pub(in crate::api) tag_ids: Option<Vec<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub(in crate::api) max_attendees: Option<Option<i32>>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::api) struct AttendEventBody {
    #[serde(default)]
    pub(in crate::api) status: Option<AttendeeStatus>,
}
