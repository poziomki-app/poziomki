use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub(in crate::api) struct MatchingQuery {
    #[serde(default)]
    pub(in crate::api) limit: Option<u8>,
    #[serde(default)]
    pub(in crate::api) lat: Option<f64>,
    #[serde(default)]
    pub(in crate::api) lng: Option<f64>,
    #[serde(default, rename = "radiusM")]
    pub(in crate::api) radius_m: Option<u32>,
}
