use diesel::deserialize::QueryableByName;
use diesel::sql_types::{Array, BigInt, Float8, Nullable, Text, Uuid as DieselUuid};
use diesel_async::RunQueryDsl;
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
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
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

pub async fn search_all(
    query: &str,
    limit: usize,
    geo: Option<&GeoSearchParams>,
    viewer_user_id: i32,
) -> crate::error::AppResult<SearchResults> {
    let normalized_query = query.trim().to_ascii_lowercase();

    if normalized_query.len() < 2 {
        return Ok(SearchResults {
            profiles: vec![],
            events: vec![],
            tags: vec![],
        });
    }

    let pattern = format!("%{normalized_query}%");
    let limit_i64 = i64::try_from(limit).unwrap_or(50);

    let profiles_fut = search_profiles(&normalized_query, &pattern, limit_i64, viewer_user_id);
    let events_fut = search_events(&normalized_query, &pattern, limit_i64, geo);
    let tags_fut = search_tags(&normalized_query, &pattern, limit_i64);

    let (profiles, events, tags) = tokio::try_join!(profiles_fut, events_fut, tags_fut)?;

    Ok(SearchResults {
        profiles,
        events,
        tags,
    })
}

/// Searches profiles that match the given text query and returns their profile documents.
///
/// The search respects each profile's discoverability and program-visibility settings with
/// respect to `viewer_user_id`. Results are ranked by text-search relevance (using the
/// profile's public search vector and, when permitted, the program field) and then by
/// most recently updated. Returned documents include id (as a string), name, bio,
/// program (when visible), profile picture, and aggregated tag names.
///
/// # Arguments
///
/// * `query` - The raw search query used for full-text matching.
/// * `pattern` - A prepared LIKE pattern (typically `LOWER(query)` wrapped with `%`) used for name and tag matching.
/// * `limit_i64` - Maximum number of results to return.
/// * `viewer_user_id` - ID of the user performing the search; used to apply privacy rules.
///
/// # Returns
///
/// A `Vec<ProfileDocument>` containing matching profiles.
///
/// # Examples
///
/// ```no_run
/// # async fn run_example() -> Result<(), Box<dyn std::error::Error>> {
/// let results = crate::search_profiles("alice", "%alice%", 10, 42).await?;
/// // `results` is a Vec<ProfileDocument>
/// # Ok(()) }
/// ```
async fn search_profiles(
    query: &str,
    pattern: &str,
    limit_i64: i64,
    viewer_user_id: i32,
) -> crate::error::AppResult<Vec<ProfileDocument>> {
    let mut conn = crate::db::conn().await?;

    let profile_rows = diesel::sql_query(
        r"
        SELECT
            p.id,
            p.name,
            p.bio,
            CASE
                WHEN p.user_id = $4 THEN p.program
                WHEN COALESCE(us.privacy_show_program, true) THEN p.program
                ELSE NULL
            END AS program,
            p.profile_picture,
            COALESCE(
                ARRAY_REMOVE(ARRAY_AGG(DISTINCT t_agg.name), NULL),
                ARRAY[]::text[]
            ) AS tags
        FROM profiles p
        LEFT JOIN user_settings us ON us.user_id = p.user_id
        LEFT JOIN profile_tags pt_agg ON pt_agg.profile_id = p.id
        LEFT JOIN tags t_agg ON t_agg.id = pt_agg.tag_id
        WHERE
            (COALESCE(us.privacy_discoverable, true) = true OR p.user_id = $4)
            AND (
                p.public_search_vector @@ websearch_to_tsquery('simple', $1)
                OR (
                    (COALESCE(us.privacy_show_program, true) = true OR p.user_id = $4)
                    AND to_tsvector('simple', COALESCE(p.program, '')) @@ websearch_to_tsquery('simple', $1)
                )
                OR LOWER(p.name) LIKE $2
                OR EXISTS (
                    SELECT 1
                    FROM profile_tags pt
                    JOIN tags t ON t.id = pt.tag_id
                    WHERE pt.profile_id = p.id
                      AND LOWER(t.name) LIKE $2
                )
            )
        GROUP BY
            p.id, p.name, p.bio, p.program, p.profile_picture, p.updated_at, p.public_search_vector,
            us.privacy_show_program, us.privacy_discoverable
        ORDER BY
            GREATEST(
                ts_rank_cd(p.public_search_vector, websearch_to_tsquery('simple', $1)),
                CASE
                    WHEN (COALESCE(us.privacy_show_program, true) = true OR p.user_id = $4)
                    THEN ts_rank_cd(to_tsvector('simple', COALESCE(p.program, '')), websearch_to_tsquery('simple', $1))
                    ELSE 0
                END
            ) DESC,
            p.updated_at DESC
        LIMIT $3
        ",
    )
    .bind::<Text, _>(query)
    .bind::<Text, _>(pattern)
    .bind::<BigInt, _>(limit_i64)
    .bind::<diesel::sql_types::Integer, _>(viewer_user_id)
    .load::<ProfileSearchRow>(&mut conn)
    .await?;

    Ok(profile_rows
        .into_iter()
        .map(|row| ProfileDocument {
            id: row.id.to_string(),
            name: row.name,
            bio: row.bio,
            program: row.program,
            profile_picture: row.profile_picture,
            tags: row.tags,
        })
        .collect())
}

async fn search_events(
    query: &str,
    pattern: &str,
    limit_i64: i64,
    geo: Option<&GeoSearchParams>,
) -> crate::error::AppResult<Vec<EventDocument>> {
    let mut conn = crate::db::conn().await?;

    let (geo_filter, order_by, has_geo) = match geo {
        Some(_) => (
            "AND e.latitude IS NOT NULL AND e.longitude IS NOT NULL \
             AND earth_box(ll_to_earth($3, $4), $5) @> ll_to_earth(e.latitude, e.longitude) \
             AND earth_distance(ll_to_earth($3, $4), ll_to_earth(e.latitude, e.longitude)) <= $5",
            "earth_distance(ll_to_earth($3, $4), ll_to_earth(e.latitude, e.longitude)) ASC, \
             ts_rank_cd(e.search_vector, websearch_to_tsquery('simple', $1)) DESC, \
             e.starts_at ASC",
            true,
        ),
        None => (
            "",
            "ts_rank_cd(e.search_vector, websearch_to_tsquery('simple', $1)) DESC, \
             e.starts_at ASC",
            false,
        ),
    };

    let limit_param = if has_geo { "$6" } else { "$3" };

    let sql = format!(
        "SELECT e.id, e.title, e.description, e.location, \
                e.starts_at::text AS starts_at, e.cover_image, \
                COALESCE(p.name, 'Unknown') AS creator_name, \
                e.latitude, e.longitude \
         FROM events e \
         LEFT JOIN profiles p ON p.id = e.creator_id \
         WHERE (e.search_vector @@ websearch_to_tsquery('simple', $1) \
                OR LOWER(e.title) LIKE $2 \
                OR LOWER(COALESCE(p.name, '')) LIKE $2) \
         {geo_filter} \
         ORDER BY {order_by} \
         LIMIT {limit_param}"
    );

    let event_rows = if let Some(g) = geo {
        diesel::sql_query(&sql)
            .bind::<Text, _>(query)
            .bind::<Text, _>(pattern)
            .bind::<Float8, _>(g.lat)
            .bind::<Float8, _>(g.lng)
            .bind::<BigInt, _>(i64::from(g.radius_m))
            .bind::<BigInt, _>(limit_i64)
            .load::<EventSearchRow>(&mut conn)
            .await?
    } else {
        diesel::sql_query(&sql)
            .bind::<Text, _>(query)
            .bind::<Text, _>(pattern)
            .bind::<BigInt, _>(limit_i64)
            .load::<EventSearchRow>(&mut conn)
            .await?
    };

    Ok(event_rows.into_iter().map(event_row_to_doc).collect())
}

fn event_row_to_doc(row: EventSearchRow) -> EventDocument {
    EventDocument {
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
    }
}

async fn search_tags(
    query: &str,
    pattern: &str,
    limit_i64: i64,
) -> crate::error::AppResult<Vec<TagDocument>> {
    let mut conn = crate::db::conn().await?;

    let rows = diesel::sql_query(
        r"
        SELECT
            t.id,
            t.name,
            t.scope,
            t.category,
            t.emoji,
            t.parent_id
        FROM tags t
        WHERE
            t.search_vector @@ websearch_to_tsquery('simple', $1)
            OR LOWER(t.name) LIKE $2
        ORDER BY
            ts_rank_cd(t.search_vector, websearch_to_tsquery('simple', $1)) DESC,
            t.name ASC
        LIMIT $3
        ",
    )
    .bind::<Text, _>(query)
    .bind::<Text, _>(pattern)
    .bind::<BigInt, _>(limit_i64)
    .load::<TagSearchRow>(&mut conn)
    .await?;

    Ok(rows
        .into_iter()
        .map(|tag| TagDocument {
            id: tag.id.to_string(),
            name: tag.name,
            scope: tag.scope,
            category: tag.category,
            emoji: tag.emoji,
            parent_id: tag.parent_id.map(|id| id.to_string()),
        })
        .collect())
}

// --- Message room search ---

#[derive(Debug, Clone, QueryableByName)]
struct RoomIdRow {
    #[diesel(sql_type = Text)]
    room_id: String,
}

pub async fn search_message_rooms(
    query: &str,
    user_pid: &uuid::Uuid,
    limit: usize,
) -> crate::error::AppResult<Vec<String>> {
    let normalized_query = query.trim().to_ascii_lowercase();

    if normalized_query.len() < 2 {
        return Ok(vec![]);
    }

    let pattern = format!("%{normalized_query}%");
    let limit_i64 = i64::try_from(limit).unwrap_or(20);

    let dm_fut = search_dm_room_ids(&normalized_query, &pattern, user_pid, limit_i64);
    let event_fut = search_event_room_ids(&normalized_query, &pattern, limit_i64);

    let (dm_rooms, event_rooms) = tokio::try_join!(dm_fut, event_fut)?;

    let mut room_ids = dm_rooms;
    room_ids.extend(event_rooms);
    Ok(room_ids)
}

/// Finds direct-message room IDs for profiles that match the provided search query, excluding the given user.
///
/// Matches profiles by their public search vector, by program text when the profile permits showing program,
/// or by a case-insensitive name LIKE pattern. Results are ordered by relevance (respecting program privacy)
/// then by profile update time, and limited to `limit_i64`.
///
/// Parameters:
/// - `query`: full-text search query passed to the text-search predicates.
/// - `pattern`: SQL LIKE pattern (typically lowercase) used for name matching.
/// - `user_pid`: the profile UUID of the querying user; rooms involving this profile are paired with matching profiles and the profile itself is excluded.
/// - `limit_i64`: maximum number of room IDs to return.
///
/// # Examples
///
/// ```
/// use uuid::Uuid;
/// // run the async function on the current thread for demonstration
/// let rooms = futures::executor::block_on(crate::search_dm_room_ids(
///     "alice",
///     "%alice%",
///     &Uuid::nil(),
///     10,
/// )).unwrap();
/// assert!(rooms.len() <= 10);
/// ```
async fn search_dm_room_ids(
    query: &str,
    pattern: &str,
    user_pid: &uuid::Uuid,
    limit_i64: i64,
) -> crate::error::AppResult<Vec<String>> {
    let mut conn = crate::db::conn().await?;

    let rows = diesel::sql_query(
        r"
        SELECT dmr.room_id
        FROM profiles p
        JOIN users u ON u.id = p.user_id
        LEFT JOIN user_settings us ON us.user_id = p.user_id
        JOIN matrix_dm_rooms dmr
          ON (dmr.user_low_pid = p.id AND dmr.user_high_pid = $3)
          OR (dmr.user_high_pid = p.id AND dmr.user_low_pid = $3)
        WHERE
            p.id != $3
            AND (
                p.public_search_vector @@ websearch_to_tsquery('simple', $1)
                OR (
                    COALESCE(us.privacy_show_program, true) = true
                    AND to_tsvector('simple', COALESCE(p.program, '')) @@ websearch_to_tsquery('simple', $1)
                )
                OR LOWER(p.name) LIKE $2
            )
        ORDER BY
            GREATEST(
                ts_rank_cd(p.public_search_vector, websearch_to_tsquery('simple', $1)),
                CASE
                    WHEN COALESCE(us.privacy_show_program, true) = true
                    THEN ts_rank_cd(to_tsvector('simple', COALESCE(p.program, '')), websearch_to_tsquery('simple', $1))
                    ELSE 0
                END
            ) DESC,
            p.updated_at DESC
        LIMIT $4
        ",
    )
    .bind::<Text, _>(query)
    .bind::<Text, _>(pattern)
    .bind::<DieselUuid, _>(user_pid)
    .bind::<BigInt, _>(limit_i64)
    .load::<RoomIdRow>(&mut conn)
    .await?;

    Ok(rows.into_iter().map(|r| r.room_id).collect())
}

async fn search_event_room_ids(
    query: &str,
    pattern: &str,
    limit_i64: i64,
) -> crate::error::AppResult<Vec<String>> {
    let mut conn = crate::db::conn().await?;

    let rows = diesel::sql_query(
        r"
        SELECT e.conversation_id AS room_id
        FROM events e
        LEFT JOIN profiles p ON p.id = e.creator_id
        WHERE
            e.conversation_id IS NOT NULL
            AND (
                e.search_vector @@ websearch_to_tsquery('simple', $1)
                OR LOWER(e.title) LIKE $2
                OR LOWER(COALESCE(p.name, '')) LIKE $2
            )
        ORDER BY
            ts_rank_cd(e.search_vector, websearch_to_tsquery('simple', $1)) DESC,
            e.starts_at DESC
        LIMIT $3
        ",
    )
    .bind::<Text, _>(query)
    .bind::<Text, _>(pattern)
    .bind::<BigInt, _>(limit_i64)
    .load::<RoomIdRow>(&mut conn)
    .await?;

    Ok(rows.into_iter().map(|r| r.room_id).collect())
}

#[derive(Debug, Clone, QueryableByName)]
struct ProfileSearchRow {
    #[diesel(sql_type = DieselUuid)]
    id: uuid::Uuid,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Nullable<Text>)]
    bio: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    program: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    profile_picture: Option<String>,
    #[diesel(sql_type = Array<Text>)]
    tags: Vec<String>,
}

#[derive(Debug, Clone, QueryableByName)]
struct EventSearchRow {
    #[diesel(sql_type = DieselUuid)]
    id: uuid::Uuid,
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Nullable<Text>)]
    description: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    location: Option<String>,
    #[diesel(sql_type = Text)]
    starts_at: String,
    #[diesel(sql_type = Nullable<Text>)]
    cover_image: Option<String>,
    #[diesel(sql_type = Text)]
    creator_name: String,
    #[diesel(sql_type = Nullable<Float8>)]
    latitude: Option<f64>,
    #[diesel(sql_type = Nullable<Float8>)]
    longitude: Option<f64>,
}

#[derive(Debug, Clone, QueryableByName)]
pub struct TagSearchRow {
    #[diesel(sql_type = DieselUuid)]
    pub id: uuid::Uuid,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Text)]
    pub scope: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub category: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub emoji: Option<String>,
    #[diesel(sql_type = Nullable<DieselUuid>)]
    pub parent_id: Option<uuid::Uuid>,
}
