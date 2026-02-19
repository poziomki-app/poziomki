use meilisearch_sdk::client::Client;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, FromQueryResult, Statement};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
    time::{Duration, Instant},
};

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

#[derive(Clone)]
struct CacheEntry {
    stored_at: Instant,
    results: SearchResults,
}

const DEFAULT_SEARCH_CACHE_TTL_MS: u64 = 15_000;
const DEFAULT_SEARCH_CACHE_MAX_ENTRIES: usize = 512;
static SEARCH_CACHE: OnceLock<RwLock<HashMap<String, CacheEntry>>> = OnceLock::new();

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

    let normalized_query = query.trim().to_ascii_lowercase();
    let pattern = format!("%{normalized_query}%");
    let cache_key = build_cache_key(&normalized_query, limit, geo);

    if let Some(cached) = read_cached_results(&cache_key) {
        return Ok(cached);
    }

    let limit_i64 = i64::try_from(limit).unwrap_or(50);

    let profiles_fut = search_profiles_postgres(db, &pattern, limit_i64);
    let events_fut = search_events_postgres(db, &pattern, limit_i64, geo);
    let tags_fut = search_tags_postgres(db, &pattern, limit_i64);
    let degrees_fut = search_degrees_postgres(db, &pattern, limit_i64);

    let (profiles, events, tags, degrees) =
        tokio::try_join!(profiles_fut, events_fut, tags_fut, degrees_fut)?;

    let results = SearchResults {
        profiles,
        events,
        tags,
        degrees,
    };

    write_cached_results(cache_key, &results);

    Ok(results)
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
            COALESCE(
                ARRAY_REMOVE(ARRAY_AGG(DISTINCT t_agg.name), NULL),
                ARRAY[]::text[]
            ) AS tags
        FROM profiles p
        LEFT JOIN profile_tags pt_agg ON pt_agg.profile_id = p.id
        LEFT JOIN tags t_agg ON t_agg.id = pt_agg.tag_id
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
        GROUP BY
            p.id, p.name, p.bio, p.age, p.program, p.profile_picture, p.updated_at
        ORDER BY p.updated_at DESC
        LIMIT $2
        ",
        vec![pattern.to_string().into(), limit_i64.into()],
    ))
    .all(db)
    .await
    .map_err(|e| loco_rs::Error::Message(format!("Profile search failed: {e}")))?;

    Ok(profile_rows
        .into_iter()
        .map(|row| ProfileDocument {
            id: row.id.to_string(),
            name: row.name,
            bio: row.bio,
            age: row.age,
            program: row.program,
            profile_picture: row.profile_picture,
            tags: row.tags,
        })
        .collect())
}

async fn search_events_postgres(
    db: &DatabaseConnection,
    pattern: &str,
    limit_i64: i64,
    geo: Option<&GeoSearchParams>,
) -> loco_rs::Result<Vec<EventDocument>> {
    let event_rows = match geo {
        Some(geo_query) => {
            EventSearchRow::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r"
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
                FROM events e
                LEFT JOIN profiles p ON p.id = e.creator_id
                WHERE
                    (
                        LOWER(e.title) LIKE $1
                        OR LOWER(COALESCE(e.description, '')) LIKE $1
                        OR LOWER(COALESCE(e.location, '')) LIKE $1
                        OR LOWER(COALESCE(p.name, '')) LIKE $1
                    )
                    AND e.latitude IS NOT NULL
                    AND e.longitude IS NOT NULL
                    AND earth_box(ll_to_earth($2, $3), $4) @> ll_to_earth(e.latitude, e.longitude)
                    AND earth_distance(ll_to_earth($2, $3), ll_to_earth(e.latitude, e.longitude)) <= $4
                ORDER BY
                    earth_distance(ll_to_earth($2, $3), ll_to_earth(e.latitude, e.longitude)) ASC,
                    e.starts_at ASC
                LIMIT $5
                ",
                vec![
                    pattern.to_string().into(),
                    geo_query.lat.into(),
                    geo_query.lng.into(),
                    i64::from(geo_query.radius_m).into(),
                    limit_i64.into(),
                ],
            ))
            .all(db)
            .await
        }
        None => {
            EventSearchRow::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r"
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
                FROM events e
                LEFT JOIN profiles p ON p.id = e.creator_id
                WHERE
                    LOWER(e.title) LIKE $1
                    OR LOWER(COALESCE(e.description, '')) LIKE $1
                    OR LOWER(COALESCE(e.location, '')) LIKE $1
                    OR LOWER(COALESCE(p.name, '')) LIKE $1
                ORDER BY e.starts_at ASC
                LIMIT $2
                ",
                vec![pattern.to_string().into(), limit_i64.into()],
            ))
            .all(db)
            .await
        }
    }
    .map_err(|e| loco_rs::Error::Message(format!("Event search failed: {e}")))?;

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

#[derive(Debug, Clone, FromQueryResult)]
struct ProfileSearchRow {
    id: uuid::Uuid,
    name: String,
    bio: Option<String>,
    age: i16,
    program: Option<String>,
    profile_picture: Option<String>,
    tags: Vec<String>,
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

fn cache_map() -> &'static RwLock<HashMap<String, CacheEntry>> {
    SEARCH_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn search_cache_enabled() -> bool {
    std::env::var("SEARCH_CACHE_ENABLED").map_or(true, |value| {
        let normalized = value.trim().to_ascii_lowercase();
        normalized == "1" || normalized == "true" || normalized == "yes"
    })
}

fn search_cache_ttl() -> Duration {
    let ttl_ms = std::env::var("SEARCH_CACHE_TTL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SEARCH_CACHE_TTL_MS);
    Duration::from_millis(ttl_ms)
}

fn search_cache_max_entries() -> usize {
    std::env::var("SEARCH_CACHE_MAX_ENTRIES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_SEARCH_CACHE_MAX_ENTRIES)
}

fn build_cache_key(query: &str, limit: usize, geo: Option<&GeoSearchParams>) -> String {
    let geo_part = geo.map_or_else(
        || "none".to_string(),
        |g| format!("{:.4}:{:.4}:{}", g.lat, g.lng, g.radius_m),
    );
    format!("{query}|{limit}|{geo_part}")
}

fn read_cached_results(key: &str) -> Option<SearchResults> {
    if !search_cache_enabled() {
        return None;
    }

    let ttl = search_cache_ttl();
    let entry = cache_map().read().ok()?.get(key)?.clone();
    if entry.stored_at.elapsed() > ttl {
        return None;
    }
    Some(entry.results)
}

fn write_cached_results(key: String, results: &SearchResults) {
    if !search_cache_enabled() {
        return;
    }

    let ttl = search_cache_ttl();
    let max_entries = search_cache_max_entries();
    let now = Instant::now();
    let Ok(mut map) = cache_map().write() else {
        return;
    };

    map.retain(|_, entry| now.duration_since(entry.stored_at) <= ttl);
    map.insert(
        key,
        CacheEntry {
            stored_at: now,
            results: results.clone(),
        },
    );

    if map.len() > max_entries {
        let mut oldest_key: Option<String> = None;
        let mut oldest_instant = now;
        for (candidate_key, entry) in map.iter() {
            if entry.stored_at <= oldest_instant {
                oldest_instant = entry.stored_at;
                oldest_key = Some(candidate_key.clone());
            }
        }
        if let Some(k) = oldest_key {
            map.remove(&k);
        }
    }
}

pub fn invalidate_postgres_cache() {
    if let Ok(mut map) = cache_map().write() {
        map.clear();
    }
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
    invalidate_postgres_cache();
    if let Some(client) = maybe_meili_client() {
        index_profile(&client, doc);
    }
}

pub fn index_event_compat(doc: EventDocument) {
    invalidate_postgres_cache();
    if let Some(client) = maybe_meili_client() {
        index_event(&client, doc);
    }
}

pub fn index_tag_compat(doc: TagDocument) {
    invalidate_postgres_cache();
    if let Some(client) = maybe_meili_client() {
        index_tag(&client, doc);
    }
}

pub fn index_degree_compat(doc: DegreeDocument) {
    invalidate_postgres_cache();
    if let Some(client) = maybe_meili_client() {
        index_degree(&client, doc);
    }
}

pub fn delete_profile_compat(id: String) {
    invalidate_postgres_cache();
    if let Some(client) = maybe_meili_client() {
        delete_profile(&client, id);
    }
}

pub fn delete_event_compat(id: String) {
    invalidate_postgres_cache();
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
