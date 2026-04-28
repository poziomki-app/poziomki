use diesel::deserialize::QueryableByName;
use diesel::sql_types::{Array, BigInt, Float8, Integer, Nullable, Text, Uuid as DieselUuid};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
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
    viewer: crate::db::DbViewer,
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
    let geo_owned = geo.map(|g| GeoSearchParams {
        lat: g.lat,
        lng: g.lng,
        radius_m: g.radius_m,
    });

    crate::db::with_viewer_tx(viewer, move |conn| {
        async move {
            let profiles =
                search_profiles(conn, &normalized_query, &pattern, limit_i64, viewer.user_id)
                    .await?;
            let events = search_events(
                conn,
                &normalized_query,
                &pattern,
                limit_i64,
                geo_owned.as_ref(),
            )
            .await?;
            let tags = search_tags(conn, &normalized_query, &pattern, limit_i64).await?;

            Ok(SearchResults {
                profiles,
                events,
                tags,
            })
        }
        .scope_boxed()
    })
    .await
    .map_err(Into::into)
}

async fn search_profiles(
    conn: &mut AsyncPgConnection,
    query: &str,
    pattern: &str,
    limit_i64: i64,
    viewer_user_id: i32,
) -> Result<Vec<ProfileDocument>, diesel::result::Error> {
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
            AND NOT EXISTS (
                SELECT 1 FROM profile_blocks pb
                JOIN profiles vp ON vp.user_id = $4
                WHERE (pb.blocker_id = vp.id AND pb.blocked_id = p.id)
                   OR (pb.blocked_id = vp.id AND pb.blocker_id = p.id)
            )
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
    .load::<ProfileSearchRow>(conn)
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
    conn: &mut AsyncPgConnection,
    query: &str,
    pattern: &str,
    limit_i64: i64,
    geo: Option<&GeoSearchParams>,
) -> Result<Vec<EventDocument>, diesel::result::Error> {
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
         AND COALESCE(e.ends_at, e.starts_at) >= NOW() \
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
            .load::<EventSearchRow>(conn)
            .await?
    } else {
        diesel::sql_query(&sql)
            .bind::<Text, _>(query)
            .bind::<Text, _>(pattern)
            .bind::<BigInt, _>(limit_i64)
            .load::<EventSearchRow>(conn)
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
    conn: &mut AsyncPgConnection,
    query: &str,
    pattern: &str,
    limit_i64: i64,
) -> Result<Vec<TagDocument>, diesel::result::Error> {
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
    .load::<TagSearchRow>(conn)
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

#[allow(clippy::similar_names)]
pub async fn search_message_rooms(
    query: &str,
    user_pid: &uuid::Uuid,
    limit: usize,
) -> crate::error::AppResult<Vec<String>> {
    use diesel_async::scoped_futures::ScopedFutureExt;

    let normalized_query = query.trim().to_ascii_lowercase();

    if normalized_query.len() < 2 {
        return Ok(vec![]);
    }

    let pattern = format!("%{normalized_query}%");
    let limit_i64 = i64::try_from(limit).unwrap_or(20);

    // Resolve caller's internal user id up front — the DM + event
    // branches both need it to constrain results to rooms the viewer
    // is actually a member of.
    //
    // Use the SECURITY DEFINER helper instead of a plain SELECT FROM
    // users: under RLS, `users` is own-row-only with no viewer GUC set
    // here, so a direct query returns nothing and search degrades to
    // empty for everyone.
    let mut conn = crate::db::conn().await?;
    let user_id: Option<i32> = crate::db::user_id_for_pid(&mut conn, *user_pid).await?;
    drop(conn);
    let Some(user_id) = user_id else {
        return Ok(Vec::new());
    };

    // Route through a viewer tx so RLS on conversations /
    // conversation_members / events applies. The membership filter on
    // events below is a defence in depth on top of that — without it,
    // an event with its conversation_id still populated would have
    // leaked through this search even though the caller never joined
    // the event's chat.
    let viewer = crate::db::DbViewer {
        user_id,
        is_review_stub: false,
    };
    let normalized = normalized_query.clone();
    let pattern_tx = pattern.clone();
    let rows = crate::db::with_viewer_tx(viewer, move |conn| {
        async move {
            let dm = search_dm_room_ids(conn, &normalized, &pattern_tx, user_id, limit_i64).await?;
            let ev =
                search_event_room_ids(conn, &normalized, &pattern_tx, user_id, limit_i64).await?;
            Ok::<_, diesel::result::Error>((dm, ev))
        }
        .scope_boxed()
    })
    .await
    .map_err(|e| crate::error::AppError::Any(e.into()))?;

    let (dm_rooms, event_rooms) = rows;
    let mut room_ids = dm_rooms;
    room_ids.extend(event_rooms);
    Ok(room_ids)
}

#[allow(clippy::similar_names)]
async fn search_dm_room_ids(
    conn: &mut diesel_async::AsyncPgConnection,
    query: &str,
    pattern: &str,
    user_id: i32,
    limit_i64: i64,
) -> Result<Vec<String>, diesel::result::Error> {
    let rows = diesel::sql_query(
        r"
        SELECT c.id::text AS room_id
        FROM conversations c
        JOIN conversation_members cm ON cm.conversation_id = c.id AND cm.user_id = $3
        WHERE c.kind = 'dm'
          AND EXISTS (
            -- Drop the JOIN to users: the users RLS policy is own-row
            -- only so joining other-bucket users returns zero rows and
            -- DM search degrades to empty. profiles is bucket-readable
            -- via RLS so we can hop straight from conversation_members
            -- to profiles via the shared user_id.
            SELECT 1 FROM conversation_members cm2
            JOIN profiles p ON p.user_id = cm2.user_id
            LEFT JOIN user_settings us ON us.user_id = cm2.user_id
            WHERE cm2.conversation_id = c.id
              AND cm2.user_id != $3
              AND (
                p.public_search_vector @@ websearch_to_tsquery('simple', $1)
                OR (
                    COALESCE(us.privacy_show_program, true) = true
                    AND to_tsvector('simple', COALESCE(p.program, '')) @@ websearch_to_tsquery('simple', $1)
                )
                OR LOWER(p.name) LIKE $2
              )
          )
        ORDER BY c.id
        LIMIT $4
        ",
    )
    .bind::<Text, _>(query)
    .bind::<Text, _>(pattern)
    .bind::<Integer, _>(user_id)
    .bind::<BigInt, _>(limit_i64)
    .load::<RoomIdRow>(conn)
    .await?;

    Ok(rows.into_iter().map(|r| r.room_id).collect())
}

async fn search_event_room_ids(
    conn: &mut diesel_async::AsyncPgConnection,
    query: &str,
    pattern: &str,
    user_id: i32,
    limit_i64: i64,
) -> Result<Vec<String>, diesel::result::Error> {
    let rows = diesel::sql_query(
        r"
        SELECT e.conversation_id AS room_id
        FROM events e
        JOIN conversation_members cm
          ON cm.conversation_id = e.conversation_id AND cm.user_id = $3
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
        LIMIT $4
        ",
    )
    .bind::<Text, _>(query)
    .bind::<Text, _>(pattern)
    .bind::<Integer, _>(user_id)
    .bind::<BigInt, _>(limit_i64)
    .load::<RoomIdRow>(conn)
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
