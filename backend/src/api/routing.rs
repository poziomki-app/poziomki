//! Walking-route proxy in front of a self-hosted OSRM instance.
//!
//! The mobile client sends start/end coordinates here; we forward to a
//! private OSRM daemon (typically `http://127.0.0.1:5000` on the same
//! host) and return a compact `{ geometryJson, distanceMeters,
//! durationSeconds }` payload. Coordinates never leave our VPS, which
//! is the whole reason we self-host instead of calling
//! `router.project-osrm.org` directly from the app.

use std::sync::OnceLock;
use std::time::Duration;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::common::{auth_or_respond, env_non_empty, error_response, ErrorSpec};
use crate::app::AppContext;

type Result<T> = crate::error::AppResult<T>;

const DEFAULT_OSRM_URL: &str = "http://127.0.0.1:5000";
const LAT_MIN: f64 = -90.0;
const LAT_MAX: f64 = 90.0;
const LNG_MIN: f64 = -180.0;
const LNG_MAX: f64 = 180.0;

#[derive(Deserialize)]
pub struct WalkQuery {
    #[serde(rename = "fromLat")]
    from_lat: f64,
    #[serde(rename = "fromLng")]
    from_lng: f64,
    #[serde(rename = "toLat")]
    to_lat: f64,
    #[serde(rename = "toLng")]
    to_lng: f64,
}

#[derive(Deserialize)]
struct OsrmResponse {
    code: String,
    #[serde(default)]
    routes: Vec<OsrmRoute>,
}

#[derive(Deserialize)]
struct OsrmRoute {
    geometry: serde_json::Value,
    distance: f64,
    duration: f64,
}

#[derive(Serialize)]
struct WalkResponse {
    #[serde(rename = "geometryJson")]
    geometry_json: String,
    #[serde(rename = "distanceMeters")]
    distance_meters: f64,
    #[serde(rename = "durationSeconds")]
    duration_seconds: f64,
}

fn osrm_base_url() -> String {
    env_non_empty("OSRM_URL").unwrap_or_else(|| DEFAULT_OSRM_URL.to_string())
}

fn osrm_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default()
    })
}

fn coord_in_range(lat: f64, lng: f64) -> bool {
    lat.is_finite()
        && lng.is_finite()
        && (LAT_MIN..=LAT_MAX).contains(&lat)
        && (LNG_MIN..=LNG_MAX).contains(&lng)
}

pub async fn walk(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Query(q): Query<WalkQuery>,
) -> Result<Response> {
    let _ = auth_or_respond!(headers);

    if !coord_in_range(q.from_lat, q.from_lng) || !coord_in_range(q.to_lat, q.to_lng) {
        return Ok(error_response(
            StatusCode::BAD_REQUEST,
            &headers,
            ErrorSpec {
                error: "invalid coordinates".to_string(),
                code: "BAD_REQUEST",
                details: None,
            },
        ));
    }

    let url = format!(
        "{}/route/v1/foot/{},{};{},{}?overview=full&geometries=geojson",
        osrm_base_url().trim_end_matches('/'),
        q.from_lng,
        q.from_lat,
        q.to_lng,
        q.to_lat,
    );

    let resp = match osrm_client().get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "osrm: upstream request failed");
            return Ok(error_response(
                StatusCode::BAD_GATEWAY,
                &headers,
                ErrorSpec {
                    error: "routing upstream unavailable".to_string(),
                    code: "BAD_GATEWAY",
                    details: None,
                },
            ));
        }
    };

    if !resp.status().is_success() {
        tracing::warn!(status = %resp.status(), "osrm: upstream non-2xx");
        return Ok(error_response(
            StatusCode::BAD_GATEWAY,
            &headers,
            ErrorSpec {
                error: "routing upstream error".to_string(),
                code: "BAD_GATEWAY",
                details: None,
            },
        ));
    }

    let parsed: OsrmResponse = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "osrm: response parse failed");
            return Ok(error_response(
                StatusCode::BAD_GATEWAY,
                &headers,
                ErrorSpec {
                    error: "routing parse error".to_string(),
                    code: "BAD_GATEWAY",
                    details: None,
                },
            ));
        }
    };

    if parsed.code != "Ok" {
        return Ok(error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "no route".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ));
    }

    let Some(route) = parsed.routes.into_iter().next() else {
        return Ok(error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "no route".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ));
    };

    let body = WalkResponse {
        geometry_json: route.geometry.to_string(),
        distance_meters: route.distance,
        duration_seconds: route.duration,
    };

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": body }))).into_response())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn rejects_out_of_range_coords() {
        assert!(!coord_in_range(f64::NAN, 0.0));
        assert!(!coord_in_range(0.0, f64::INFINITY));
        assert!(!coord_in_range(91.0, 0.0));
        assert!(!coord_in_range(-91.0, 0.0));
        assert!(!coord_in_range(0.0, 181.0));
        assert!(!coord_in_range(0.0, -181.0));
    }

    #[test]
    fn accepts_warsaw_coords() {
        assert!(coord_in_range(52.2297, 21.0122));
    }
}
