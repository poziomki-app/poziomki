use meilisearch_sdk::client::Client;
use serde::{Deserialize, Serialize};

// --- Geo types ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint {
    pub lat: f64,
    pub lng: f64,
}

// --- Document types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDocument {
    pub id: String,
    pub name: String,
    pub bio: Option<String>,
    pub age: i16,
    pub program: Option<String>,
    pub profile_picture: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDocument {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub starts_at: String,
    pub cover_image: Option<String>,
    pub creator_name: String,
    #[serde(rename = "_geo", skip_serializing_if = "Option::is_none")]
    pub geo: Option<GeoPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagDocument {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub category: Option<String>,
    pub emoji: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegreeDocument {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub profiles: Vec<ProfileDocument>,
    pub events: Vec<EventDocument>,
    pub tags: Vec<TagDocument>,
    pub degrees: Vec<DegreeDocument>,
}

// --- Client ---

pub fn create_client() -> Result<Client, meilisearch_sdk::errors::Error> {
    let url = std::env::var("MEILI_URL").unwrap_or_else(|_| "http://localhost:7700".to_string());
    let key = match std::env::var("MEILI_MASTER_KEY") {
        Ok(k) if !k.trim().is_empty() => k,
        _ => {
            tracing::warn!("MEILI_MASTER_KEY not set — search will not work in production");
            String::new()
        }
    };
    Client::new(url, Some(key))
}

// --- Index configuration ---

pub async fn configure_indexes(client: &Client) {
    configure_profiles_index(client).await;
    configure_events_index(client).await;
    configure_tags_index(client).await;
    configure_degrees_index(client).await;
}

async fn configure_profiles_index(client: &Client) {
    let index = client.index("profiles");
    let _ = index
        .set_searchable_attributes(["name", "bio", "program", "tags"])
        .await;
    let _ = index
        .set_filterable_attributes(["age", "program", "tags"])
        .await;
    let _ = index.set_sortable_attributes(["name"]).await;
}

async fn configure_events_index(client: &Client) {
    let index = client.index("events");
    let _ = index
        .set_searchable_attributes(["title", "description", "location", "creator_name"])
        .await;
    let _ = index
        .set_filterable_attributes(["starts_at", "location", "_geo"])
        .await;
    let _ = index.set_sortable_attributes(["starts_at", "_geo"]).await;
}

async fn configure_tags_index(client: &Client) {
    let index = client.index("tags");
    let _ = index.set_searchable_attributes(["name", "category"]).await;
    let _ = index.set_filterable_attributes(["scope"]).await;
    let _ = index.set_sortable_attributes(["name"]).await;
}

async fn configure_degrees_index(client: &Client) {
    let index = client.index("degrees");
    let _ = index.set_searchable_attributes(["name"]).await;
    let _ = index.set_sortable_attributes(["name"]).await;
}

// --- Sync helpers (fire-and-forget, log warnings on failure) ---

pub fn index_profile(client: &Client, doc: ProfileDocument) {
    let index = client.index("profiles");
    tokio::spawn(async move {
        if let Err(e) = index.add_or_replace(&[doc], Some("id")).await {
            tracing::warn!("Failed to index profile in Meilisearch: {e}");
        }
    });
}

pub fn index_event(client: &Client, doc: EventDocument) {
    let index = client.index("events");
    tokio::spawn(async move {
        if let Err(e) = index.add_or_replace(&[doc], Some("id")).await {
            tracing::warn!("Failed to index event in Meilisearch: {e}");
        }
    });
}

pub fn index_tag(client: &Client, doc: TagDocument) {
    let index = client.index("tags");
    tokio::spawn(async move {
        if let Err(e) = index.add_or_replace(&[doc], Some("id")).await {
            tracing::warn!("Failed to index tag in Meilisearch: {e}");
        }
    });
}

pub fn index_degree(client: &Client, doc: DegreeDocument) {
    let index = client.index("degrees");
    tokio::spawn(async move {
        if let Err(e) = index.add_or_replace(&[doc], Some("id")).await {
            tracing::warn!("Failed to index degree in Meilisearch: {e}");
        }
    });
}

pub fn delete_profile(client: &Client, id: String) {
    let index = client.index("profiles");
    tokio::spawn(async move {
        if let Err(e) = index.delete_document(&id).await {
            tracing::warn!("Failed to delete profile from Meilisearch: {e}");
        }
    });
}

pub fn delete_event(client: &Client, id: String) {
    let index = client.index("events");
    tokio::spawn(async move {
        if let Err(e) = index.delete_document(&id).await {
            tracing::warn!("Failed to delete event from Meilisearch: {e}");
        }
    });
}

// --- Geo search params ---

pub struct GeoSearchParams {
    pub lat: f64,
    pub lng: f64,
    pub radius_m: u32,
}

// --- Multi-search ---

pub async fn search_all(
    client: &Client,
    query: &str,
    limit: usize,
    geo: Option<&GeoSearchParams>,
) -> Result<SearchResults, meilisearch_sdk::errors::Error> {
    let profiles_idx = client.index("profiles");
    let events_idx = client.index("events");
    let tags_idx = client.index("tags");
    let degrees_idx = client.index("degrees");

    let mut pq = profiles_idx.search();
    pq.with_query(query).with_limit(limit);
    let mut eq = events_idx.search();
    eq.with_query(query).with_limit(limit);

    let geo_filter;
    let geo_sort;
    let geo_sort_arr;
    if let Some(g) = geo {
        geo_filter = format!("_geoRadius({}, {}, {})", g.lat, g.lng, g.radius_m);
        eq.with_filter(&geo_filter);
        geo_sort = format!("_geoPoint({}, {}):asc", g.lat, g.lng);
        geo_sort_arr = [geo_sort.as_str()];
        eq.with_sort(&geo_sort_arr);
    }

    let mut tq = tags_idx.search();
    tq.with_query(query).with_limit(limit);
    let mut dq = degrees_idx.search();
    dq.with_query(query).with_limit(limit);

    let (profiles_res, events_res, tags_res, degrees_res) = tokio::join!(
        pq.execute::<ProfileDocument>(),
        eq.execute::<EventDocument>(),
        tq.execute::<TagDocument>(),
        dq.execute::<DegreeDocument>(),
    );

    Ok(SearchResults {
        profiles: profiles_res
            .map(|r| r.hits.into_iter().map(|h| h.result).collect())
            .unwrap_or_default(),
        events: events_res
            .map(|r| r.hits.into_iter().map(|h| h.result).collect())
            .unwrap_or_default(),
        tags: tags_res
            .map(|r| r.hits.into_iter().map(|h| h.result).collect())
            .unwrap_or_default(),
        degrees: degrees_res
            .map(|r| r.hits.into_iter().map(|h| h.result).collect())
            .unwrap_or_default(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn sample_event(geo: Option<GeoPoint>) -> EventDocument {
        EventDocument {
            id: "evt-1".to_string(),
            title: "Test Event".to_string(),
            description: Some("A description".to_string()),
            location: Some("Warsaw".to_string()),
            starts_at: "2030-01-01T12:00:00Z".to_string(),
            cover_image: None,
            creator_name: "Alice".to_string(),
            geo,
        }
    }

    #[test]
    fn geo_point_serializes_correctly() {
        let point = GeoPoint {
            lat: 52.2297,
            lng: 21.0122,
        };
        let json = serde_json::to_value(&point).unwrap();
        assert_eq!(json["lat"], 52.2297);
        assert_eq!(json["lng"], 21.0122);
    }

    #[test]
    fn geo_point_roundtrips_through_json() {
        let point = GeoPoint {
            lat: 52.2297,
            lng: 21.0122,
        };
        let json = serde_json::to_string(&point).unwrap();
        let deserialized: GeoPoint = serde_json::from_str(&json).unwrap();
        assert_eq!(point, deserialized);
    }

    #[test]
    fn event_document_with_geo_serializes_as_underscore_geo() {
        let doc = sample_event(Some(GeoPoint {
            lat: 52.2297,
            lng: 21.0122,
        }));
        let json = serde_json::to_value(&doc).unwrap();
        assert!(json.get("_geo").is_some(), "_geo field must be present");
        assert_eq!(json["_geo"]["lat"], 52.2297);
        assert_eq!(json["_geo"]["lng"], 21.0122);
        assert!(
            json.get("geo").is_none(),
            "plain 'geo' field must not appear"
        );
    }

    #[test]
    fn event_document_without_geo_omits_field() {
        let doc = sample_event(None);
        let json = serde_json::to_value(&doc).unwrap();
        assert!(
            json.get("_geo").is_none(),
            "_geo must be absent when None (skip_serializing_if)"
        );
    }

    #[test]
    fn event_document_deserializes_with_geo() {
        let raw = serde_json::json!({
            "id": "evt-2",
            "title": "Geo Event",
            "description": null,
            "location": "Krakow",
            "starts_at": "2030-06-15T18:00:00Z",
            "cover_image": null,
            "creator_name": "Bob",
            "_geo": { "lat": 50.0647, "lng": 19.9450 }
        });
        let doc: EventDocument = serde_json::from_value(raw).unwrap();
        let geo = doc.geo.expect("geo must be Some");
        assert!((geo.lat - 50.0647).abs() < f64::EPSILON);
        assert!((geo.lng - 19.9450).abs() < f64::EPSILON);
    }

    #[test]
    fn event_document_deserializes_without_geo() {
        let raw = serde_json::json!({
            "id": "evt-3",
            "title": "No Geo",
            "description": null,
            "location": null,
            "starts_at": "2030-06-15T18:00:00Z",
            "cover_image": null,
            "creator_name": "Charlie",
        });
        let doc: EventDocument = serde_json::from_value(raw).unwrap();
        assert!(doc.geo.is_none());
    }

    #[test]
    fn geo_filter_format_matches_meilisearch_syntax() {
        let params = GeoSearchParams {
            lat: 52.2297,
            lng: 21.0122,
            radius_m: 5000,
        };
        let filter = format!(
            "_geoRadius({}, {}, {})",
            params.lat, params.lng, params.radius_m
        );
        assert_eq!(filter, "_geoRadius(52.2297, 21.0122, 5000)");
    }

    #[test]
    fn geo_sort_format_matches_meilisearch_syntax() {
        let params = GeoSearchParams {
            lat: 52.2297,
            lng: 21.0122,
            radius_m: 5000,
        };
        let sort = format!("_geoPoint({}, {}):asc", params.lat, params.lng);
        assert_eq!(sort, "_geoPoint(52.2297, 21.0122):asc");
    }

    #[test]
    fn search_results_includes_geo_in_events() {
        let results = SearchResults {
            profiles: vec![],
            events: vec![
                sample_event(Some(GeoPoint {
                    lat: 52.0,
                    lng: 21.0,
                })),
                sample_event(None),
            ],
            tags: vec![],
            degrees: vec![],
        };
        let json = serde_json::to_value(&results).unwrap();
        let events = json["events"].as_array().unwrap();
        assert!(events[0].get("_geo").is_some());
        assert!(events[1].get("_geo").is_none());
    }

    #[test]
    fn geo_point_negative_coordinates() {
        let point = GeoPoint {
            lat: -33.8688,
            lng: -151.2093,
        };
        let json = serde_json::to_value(&point).unwrap();
        assert_eq!(json["lat"], -33.8688);
        assert_eq!(json["lng"], -151.2093);
        let deserialized: GeoPoint = serde_json::from_value(json).unwrap();
        assert_eq!(point, deserialized);
    }

    #[test]
    fn geo_point_zero_coordinates() {
        let point = GeoPoint { lat: 0.0, lng: 0.0 };
        let json = serde_json::to_value(&point).unwrap();
        assert_eq!(json["lat"], 0.0);
        assert_eq!(json["lng"], 0.0);
    }

    #[test]
    fn geo_filter_with_large_radius() {
        let params = GeoSearchParams {
            lat: 0.0,
            lng: 0.0,
            radius_m: 1_000_000,
        };
        let filter = format!(
            "_geoRadius({}, {}, {})",
            params.lat, params.lng, params.radius_m
        );
        assert_eq!(filter, "_geoRadius(0, 0, 1000000)");
    }
}
