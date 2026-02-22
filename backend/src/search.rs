use crate::error::{AppError, AppResult};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, FromQueryResult, Statement};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
    time::{Duration, Instant},
};

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
pub struct SearchResults {
    pub profiles: Vec<ProfileDocument>,
    pub events: Vec<EventDocument>,
    pub tags: Vec<TagDocument>,
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
) -> AppResult<SearchResults> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(AppError::Message(
            "Search requires a PostgreSQL database".to_string(),
        ));
    }

    let normalized_query = query.trim().to_ascii_lowercase();

    if normalized_query.len() < 2 {
        return Ok(SearchResults {
            profiles: vec![],
            events: vec![],
            tags: vec![],
        });
    }

    let pattern = format!("%{normalized_query}%");
    let cache_key = build_cache_key(&normalized_query, limit, geo);

    if let Some(cached) = read_cached_results(&cache_key) {
        return Ok(cached);
    }

    let limit_i64 = i64::try_from(limit).unwrap_or(50);

    let profiles_fut = search_profiles_postgres(db, &normalized_query, &pattern, limit_i64);
    let events_fut = search_events_postgres(db, &normalized_query, &pattern, limit_i64, geo);
    let tags_fut = search_tags_postgres(db, &pattern, limit_i64);

    let (profiles, events, tags) = tokio::try_join!(profiles_fut, events_fut, tags_fut)?;

    let results = SearchResults {
        profiles,
        events,
        tags,
    };

    write_cached_results(cache_key, &results);

    Ok(results)
}

async fn search_profiles_postgres(
    db: &DatabaseConnection,
    query: &str,
    pattern: &str,
    limit_i64: i64,
) -> AppResult<Vec<ProfileDocument>> {
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
            p.search_vector @@ websearch_to_tsquery('simple', $1)
            OR LOWER(p.name) LIKE $2
            OR EXISTS (
                SELECT 1
                FROM profile_tags pt
                JOIN tags t ON t.id = pt.tag_id
                WHERE pt.profile_id = p.id
                  AND LOWER(t.name) LIKE $2
            )
        GROUP BY
            p.id, p.name, p.bio, p.age, p.program, p.profile_picture, p.updated_at, p.search_vector
        ORDER BY
            ts_rank_cd(p.search_vector, websearch_to_tsquery('simple', $1)) DESC,
            p.updated_at DESC
        LIMIT $3
        ",
        vec![
            query.to_string().into(),
            pattern.to_string().into(),
            limit_i64.into(),
        ],
    ))
    .all(db)
    .await
    .map_err(|e| AppError::Message(format!("Profile search failed: {e}")))?;

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
    query: &str,
    pattern: &str,
    limit_i64: i64,
    geo: Option<&GeoSearchParams>,
) -> AppResult<Vec<EventDocument>> {
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
                        e.search_vector @@ websearch_to_tsquery('simple', $1)
                        OR LOWER(e.title) LIKE $2
                        OR LOWER(COALESCE(p.name, '')) LIKE $2
                    )
                    AND e.latitude IS NOT NULL
                    AND e.longitude IS NOT NULL
                    AND earth_box(ll_to_earth($3, $4), $5) @> ll_to_earth(e.latitude, e.longitude)
                    AND earth_distance(ll_to_earth($3, $4), ll_to_earth(e.latitude, e.longitude)) <= $5
                ORDER BY
                    earth_distance(ll_to_earth($3, $4), ll_to_earth(e.latitude, e.longitude)) ASC,
                    ts_rank_cd(e.search_vector, websearch_to_tsquery('simple', $1)) DESC,
                    e.starts_at ASC
                LIMIT $6
                ",
                vec![
                    query.to_string().into(),
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
                    e.search_vector @@ websearch_to_tsquery('simple', $1)
                    OR LOWER(e.title) LIKE $2
                    OR LOWER(COALESCE(p.name, '')) LIKE $2
                ORDER BY
                    ts_rank_cd(e.search_vector, websearch_to_tsquery('simple', $1)) DESC,
                    e.starts_at ASC
                LIMIT $3
                ",
                vec![
                    query.to_string().into(),
                    pattern.to_string().into(),
                    limit_i64.into(),
                ],
            ))
            .all(db)
            .await
        }
    }
    .map_err(|e| AppError::Message(format!("Event search failed: {e}")))?;

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
) -> AppResult<Vec<TagDocument>> {
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
    .map_err(|e| AppError::Message(format!("Tag search failed: {e}")))?;

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

fn cache_map() -> &'static RwLock<HashMap<String, CacheEntry>> {
    SEARCH_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn search_cache_enabled() -> bool {
    // Default off: per-process caches become inconsistent under multi-instance deployments.
    std::env::var("SEARCH_CACHE_ENABLED").is_ok_and(|value| {
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

pub fn invalidate_search_cache() {
    if let Ok(mut map) = cache_map().write() {
        map.clear();
    }
}
