use meilisearch_sdk::client::Client;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, FromQueryResult,
    QueryFilter, Statement,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::models::_entities::{profile_tags, tags};

// MEILI_COMPAT_REMOVE: Meilisearch compatibility code in this file can be removed
// wholesale once Postgres search fully replaces Meilisearch in production.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchBackend {
    Postgres,
    Meilisearch,
}

#[must_use]
pub fn configured_backend() -> SearchBackend {
    match std::env::var("SEARCH_BACKEND") {
        Ok(value) if value.eq_ignore_ascii_case("meili") => SearchBackend::Meilisearch,
        Ok(value) if value.eq_ignore_ascii_case("meilisearch") => SearchBackend::Meilisearch,
        _ => SearchBackend::Postgres,
    }
}

#[must_use]
pub fn meili_compat_enabled() -> bool {
    if let Ok(raw) = std::env::var("SEARCH_MEILI_COMPAT") {
        let normalized = raw.trim().to_ascii_lowercase();
        return normalized == "1" || normalized == "true" || normalized == "yes";
    }

    matches!(configured_backend(), SearchBackend::Meilisearch)
}

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

pub struct GeoSearchParams {
    pub lat: f64,
    pub lng: f64,
    pub radius_m: u32,
}

pub async fn search_all(
    db: &DatabaseConnection,
    query: &str,
    limit: usize,
    geo: Option<&GeoSearchParams>,
) -> loco_rs::Result<SearchResults> {
    match configured_backend() {
        SearchBackend::Postgres => search_all_postgres(db, query, limit, geo).await,
        SearchBackend::Meilisearch => {
            // MEILI_COMPAT_REMOVE
            let client = create_client().map_err(|e| {
                loco_rs::Error::Message(format!("Failed to create Meilisearch client: {e}"))
            })?;
            search_all_meilisearch(&client, query, limit, geo)
                .await
                .map_err(|e| loco_rs::Error::Message(format!("Search query failed: {e}")))
        }
    }
}

async fn search_all_postgres(
    db: &DatabaseConnection,
    query: &str,
    limit: usize,
    geo: Option<&GeoSearchParams>,
) -> loco_rs::Result<SearchResults> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(loco_rs::Error::Message(
            "SEARCH_BACKEND=postgres requires a PostgreSQL database".to_string(),
        ));
    }

    let pattern = format!("%{}%", query.to_ascii_lowercase());
    let limit_i64 = i64::try_from(limit).unwrap_or(50);
    let events_limit = geo.map_or(limit, |_| limit.saturating_mul(5).min(250));
    let events_limit_i64 = i64::try_from(events_limit).unwrap_or(250);

    let profiles_fut = search_profiles_postgres(db, &pattern, limit_i64);
    let events_fut = search_events_postgres(db, &pattern, events_limit_i64, limit, geo);
    let tags_fut = search_tags_postgres(db, &pattern, limit_i64);
    let degrees_fut = search_degrees_postgres(db, &pattern, limit_i64);

    let (profiles, events, tags, degrees) =
        tokio::try_join!(profiles_fut, events_fut, tags_fut, degrees_fut)?;

    Ok(SearchResults {
        profiles,
        events,
        tags,
        degrees,
    })
}

async fn search_profiles_postgres(
    db: &DatabaseConnection,
    pattern: &str,
    limit_i64: i64,
) -> loco_rs::Result<Vec<ProfileDocument>> {
    let profile_rows = ProfileSearchRow::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r"
        SELECT
            p.id,
            p.name,
            p.bio,
            p.age,
            p.program,
            p.profile_picture,
            p.updated_at
        FROM profiles p
        WHERE
            LOWER(p.name) LIKE $1
            OR LOWER(COALESCE(p.bio, '')) LIKE $1
            OR LOWER(COALESCE(p.program, '')) LIKE $1
            OR EXISTS (
                SELECT 1
                FROM profile_tags pt
                JOIN tags t ON t.id = pt.tag_id
                WHERE pt.profile_id = p.id
                  AND LOWER(COALESCE(t.name, '')) LIKE $1
            )
        ORDER BY p.updated_at DESC
        LIMIT $2
        ",
        vec![pattern.to_string().into(), limit_i64.into()],
    ))
    .all(db)
    .await
    .map_err(|e| loco_rs::Error::Message(format!("Profile search failed: {e}")))?;

    let profile_ids: Vec<uuid::Uuid> = profile_rows.iter().map(|row| row.id).collect();
    let profile_tag_names = load_profile_tag_names(db, &profile_ids).await?;

    Ok(profile_rows
        .into_iter()
        .map(|row| ProfileDocument {
            id: row.id.to_string(),
            name: row.name,
            bio: row.bio,
            age: row.age,
            program: row.program,
            profile_picture: row.profile_picture,
            tags: profile_tag_names.get(&row.id).cloned().unwrap_or_default(),
        })
        .collect())
}

async fn search_events_postgres(
    db: &DatabaseConnection,
    pattern: &str,
    events_limit_i64: i64,
    limit: usize,
    geo: Option<&GeoSearchParams>,
) -> loco_rs::Result<Vec<EventDocument>> {
    let mut event_rows = EventSearchRow::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r"
        WITH matched_events AS (
            SELECT e.id
            FROM events e
            WHERE
                LOWER(e.title) LIKE $1
                OR LOWER(COALESCE(e.description, '')) LIKE $1
                OR LOWER(COALESCE(e.location, '')) LIKE $1
                OR EXISTS (
                    SELECT 1
                    FROM profiles p
                    WHERE p.id = e.creator_id
                      AND LOWER(COALESCE(p.name, '')) LIKE $1
                )
            ORDER BY e.starts_at ASC
            LIMIT $2
        )
        SELECT
            e.id,
            e.title,
            e.description,
            e.location,
            e.starts_at::text AS starts_at,
            e.cover_image,
            COALESCE(p.name, 'Unknown') AS creator_name,
            e.latitude,
            e.longitude
        FROM matched_events m
        JOIN events e ON e.id = m.id
        LEFT JOIN profiles p ON p.id = e.creator_id
        ORDER BY e.starts_at ASC
        ",
        vec![pattern.to_string().into(), events_limit_i64.into()],
    ))
    .all(db)
    .await
    .map_err(|e| loco_rs::Error::Message(format!("Event search failed: {e}")))?;

    if let Some(geo_query) = geo {
        event_rows = filter_and_sort_events_by_geo(event_rows, geo_query);
    }
    if event_rows.len() > limit {
        event_rows.truncate(limit);
    }

    Ok(event_rows
        .into_iter()
        .map(|row| EventDocument {
            id: row.id.to_string(),
            title: row.title,
            description: row.description,
            location: row.location,
            starts_at: row.starts_at,
            cover_image: row.cover_image,
            creator_name: row.creator_name,
            geo: row
                .latitude
                .zip(row.longitude)
                .map(|(lat, lng)| GeoPoint { lat, lng }),
        })
        .collect())
}

async fn search_tags_postgres(
    db: &DatabaseConnection,
    pattern: &str,
    limit_i64: i64,
) -> loco_rs::Result<Vec<TagDocument>> {
    let rows = TagSearchRow::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r"
        SELECT
            t.id,
            t.name,
            t.scope,
            t.category,
            t.emoji
        FROM tags t
        WHERE LOWER(t.name) LIKE $1
        ORDER BY t.name ASC
        LIMIT $2
        ",
        vec![pattern.to_string().into(), limit_i64.into()],
    ))
    .all(db)
    .await
    .map_err(|e| loco_rs::Error::Message(format!("Tag search failed: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|tag| TagDocument {
            id: tag.id.to_string(),
            name: tag.name,
            scope: tag.scope,
            category: tag.category,
            emoji: tag.emoji,
        })
        .collect())
}

async fn search_degrees_postgres(
    db: &DatabaseConnection,
    pattern: &str,
    limit_i64: i64,
) -> loco_rs::Result<Vec<DegreeDocument>> {
    let rows = DegreeSearchRow::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r"
        SELECT
            d.id,
            d.name
        FROM degrees d
        WHERE LOWER(d.name) LIKE $1
        ORDER BY d.name ASC
        LIMIT $2
        ",
        vec![pattern.to_string().into(), limit_i64.into()],
    ))
    .all(db)
    .await
    .map_err(|e| loco_rs::Error::Message(format!("Degree search failed: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|degree| DegreeDocument {
            id: degree.id.to_string(),
            name: degree.name,
        })
        .collect())
}

async fn load_profile_tag_names(
    db: &DatabaseConnection,
    profile_ids: &[uuid::Uuid],
) -> loco_rs::Result<HashMap<uuid::Uuid, Vec<String>>> {
    if profile_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let links = profile_tags::Entity::find()
        .filter(profile_tags::Column::ProfileId.is_in(profile_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Message(format!("Profile tag links failed: {e}")))?;

    if links.is_empty() {
        return Ok(HashMap::new());
    }

    let tag_ids: HashSet<uuid::Uuid> = links.iter().map(|link| link.tag_id).collect();
    let tags = tags::Entity::find()
        .filter(tags::Column::Id.is_in(tag_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|e| loco_rs::Error::Message(format!("Tag fetch failed: {e}")))?;

    let tag_name_by_id: HashMap<uuid::Uuid, String> =
        tags.into_iter().map(|tag| (tag.id, tag.name)).collect();

    let mut by_profile: HashMap<uuid::Uuid, Vec<String>> = HashMap::new();
    for link in links {
        if let Some(tag_name) = tag_name_by_id.get(&link.tag_id) {
            by_profile
                .entry(link.profile_id)
                .or_default()
                .push(tag_name.clone());
        }
    }

    Ok(by_profile)
}

fn filter_and_sort_events_by_geo(
    candidates: Vec<EventSearchRow>,
    geo_query: &GeoSearchParams,
) -> Vec<EventSearchRow> {
    let mut with_distance: Vec<(f64, EventSearchRow)> = candidates
        .into_iter()
        .filter_map(|row| {
            row.latitude.zip(row.longitude).and_then(|(lat, lng)| {
                let meters = haversine_meters(geo_query.lat, geo_query.lng, lat, lng);
                if meters <= f64::from(geo_query.radius_m) {
                    Some((meters, row))
                } else {
                    None
                }
            })
        })
        .collect();

    with_distance.sort_by(|(a, _), (b, _)| a.total_cmp(b));
    with_distance.into_iter().map(|(_, row)| row).collect()
}

fn haversine_meters(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let earth_radius_m = 6_371_000.0_f64;
    let d_lat = (lat2 - lat1).to_radians();
    let d_lng = (lng2 - lng1).to_radians();
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();

    let half_d_lat_sin = (d_lat / 2.0).sin();
    let half_d_lng_sin_sq = (d_lng / 2.0).sin().powi(2);
    let cos_product = lat1_rad.cos() * lat2_rad.cos();
    let a = cos_product.mul_add(half_d_lng_sin_sq, half_d_lat_sin * half_d_lat_sin);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    earth_radius_m * c
}

#[derive(Debug, Clone, FromQueryResult)]
struct ProfileSearchRow {
    id: uuid::Uuid,
    name: String,
    bio: Option<String>,
    age: i16,
    program: Option<String>,
    profile_picture: Option<String>,
}

#[derive(Debug, Clone, FromQueryResult)]
struct EventSearchRow {
    id: uuid::Uuid,
    title: String,
    description: Option<String>,
    location: Option<String>,
    starts_at: String,
    cover_image: Option<String>,
    creator_name: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
}

#[derive(Debug, Clone, FromQueryResult)]
struct TagSearchRow {
    id: uuid::Uuid,
    name: String,
    scope: String,
    category: Option<String>,
    emoji: Option<String>,
}

#[derive(Debug, Clone, FromQueryResult)]
struct DegreeSearchRow {
    id: uuid::Uuid,
    name: String,
}

// --- MEILI_COMPAT_REMOVE: Client and compatibility sync ---

pub fn create_client() -> Result<Client, meilisearch_sdk::errors::Error> {
    let url = std::env::var("MEILI_URL").unwrap_or_else(|_| "http://localhost:7700".to_string());
    let key = match std::env::var("MEILI_MASTER_KEY") {
        Ok(k) if !k.trim().is_empty() => k,
        _ => {
            tracing::warn!("MEILI_MASTER_KEY not set; Meilisearch compatibility may fail");
            String::new()
        }
    };
    Client::new(url, Some(key))
}

pub fn index_profile_compat(doc: ProfileDocument) {
    if let Some(client) = maybe_meili_client() {
        index_profile(&client, doc);
    }
}

pub fn index_event_compat(doc: EventDocument) {
    if let Some(client) = maybe_meili_client() {
        index_event(&client, doc);
    }
}

pub fn index_tag_compat(doc: TagDocument) {
    if let Some(client) = maybe_meili_client() {
        index_tag(&client, doc);
    }
}

pub fn index_degree_compat(doc: DegreeDocument) {
    if let Some(client) = maybe_meili_client() {
        index_degree(&client, doc);
    }
}

pub fn delete_profile_compat(id: String) {
    if let Some(client) = maybe_meili_client() {
        delete_profile(&client, id);
    }
}

pub fn delete_event_compat(id: String) {
    if let Some(client) = maybe_meili_client() {
        delete_event(&client, id);
    }
}

fn maybe_meili_client() -> Option<Client> {
    if !meili_compat_enabled() {
        return None;
    }

    match create_client() {
        Ok(client) => Some(client),
        Err(error) => {
            tracing::warn!("Failed to create Meilisearch client for compatibility sync: {error}");
            None
        }
    }
}

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

fn index_profile(client: &Client, doc: ProfileDocument) {
    let index = client.index("profiles");
    tokio::spawn(async move {
        if let Err(e) = index.add_or_replace(&[doc], Some("id")).await {
            tracing::warn!("Failed to index profile in Meilisearch: {e}");
        }
    });
}

fn index_event(client: &Client, doc: EventDocument) {
    let index = client.index("events");
    tokio::spawn(async move {
        if let Err(e) = index.add_or_replace(&[doc], Some("id")).await {
            tracing::warn!("Failed to index event in Meilisearch: {e}");
        }
    });
}

fn index_tag(client: &Client, doc: TagDocument) {
    let index = client.index("tags");
    tokio::spawn(async move {
        if let Err(e) = index.add_or_replace(&[doc], Some("id")).await {
            tracing::warn!("Failed to index tag in Meilisearch: {e}");
        }
    });
}

fn index_degree(client: &Client, doc: DegreeDocument) {
    let index = client.index("degrees");
    tokio::spawn(async move {
        if let Err(e) = index.add_or_replace(&[doc], Some("id")).await {
            tracing::warn!("Failed to index degree in Meilisearch: {e}");
        }
    });
}

fn delete_profile(client: &Client, id: String) {
    let index = client.index("profiles");
    tokio::spawn(async move {
        if let Err(e) = index.delete_document(&id).await {
            tracing::warn!("Failed to delete profile from Meilisearch: {e}");
        }
    });
}

fn delete_event(client: &Client, id: String) {
    let index = client.index("events");
    tokio::spawn(async move {
        if let Err(e) = index.delete_document(&id).await {
            tracing::warn!("Failed to delete event from Meilisearch: {e}");
        }
    });
}

async fn search_all_meilisearch(
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
