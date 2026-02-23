use diesel::deserialize::QueryableByName;
use diesel::sql_types::{Array, BigInt, Float8, Nullable, SmallInt, Text, Uuid as DieselUuid};
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

pub async fn search_all(
    query: &str,
    limit: usize,
    geo: Option<&GeoSearchParams>,
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

    let profiles_fut = search_profiles_postgres(&normalized_query, &pattern, limit_i64);
    let events_fut = search_events_postgres(&normalized_query, &pattern, limit_i64, geo);
    let tags_fut = search_tags_postgres(&pattern, limit_i64);

    let (profiles, events, tags) = tokio::try_join!(profiles_fut, events_fut, tags_fut)?;

    Ok(SearchResults {
        profiles,
        events,
        tags,
    })
}

async fn search_profiles_postgres(
    query: &str,
    pattern: &str,
    limit_i64: i64,
) -> crate::error::AppResult<Vec<ProfileDocument>> {
    let mut conn = crate::db::conn().await?;

    let profile_rows = diesel::sql_query(
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
    )
    .bind::<Text, _>(query)
    .bind::<Text, _>(pattern)
    .bind::<BigInt, _>(limit_i64)
    .load::<ProfileSearchRow>(&mut conn)
    .await?;

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

async fn search_tags_postgres(
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
            t.emoji
        FROM tags t
        WHERE LOWER(t.name) LIKE $1
        ORDER BY t.name ASC
        LIMIT $2
        ",
    )
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
        })
        .collect())
}

#[derive(Debug, Clone, QueryableByName)]
struct ProfileSearchRow {
    #[diesel(sql_type = DieselUuid)]
    id: uuid::Uuid,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Nullable<Text>)]
    bio: Option<String>,
    #[diesel(sql_type = SmallInt)]
    age: i16,
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
struct TagSearchRow {
    #[diesel(sql_type = DieselUuid)]
    id: uuid::Uuid,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    scope: String,
    #[diesel(sql_type = Nullable<Text>)]
    category: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    emoji: Option<String>,
}
