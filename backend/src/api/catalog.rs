type Result<T> = crate::error::AppResult<T>;

use crate::api::auth_or_respond;
use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Double, Nullable, Text};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::{
    error_response,
    state::{
        CreateTagBody, DataResponse, DegreeResponse, DegreesQuery, TagResponse, TagScope,
        TagSuggestionResponse, TagSuggestionsBody, TagsQuery,
    },
    ErrorSpec,
};
use crate::db::models::degrees::Degree;
use crate::db::models::tags::{NewTag, Tag};
use crate::db::schema::{degrees, tags};

const PUBLIC_CACHE_MEDIUM: HeaderValue = HeaderValue::from_static("public, max-age=1800");

const fn scope_to_str(scope: TagScope) -> &'static str {
    match scope {
        TagScope::Interest => "interest",
        TagScope::Activity => "activity",
        TagScope::Event => "event",
    }
}

fn bounded_limit(limit: Option<u8>) -> i64 {
    i64::from(limit.unwrap_or(20).clamp(1, 100))
}

fn str_to_scope(s: &str) -> TagScope {
    match s {
        "activity" => TagScope::Activity,
        "event" => TagScope::Event,
        _ => TagScope::Interest,
    }
}

fn tag_model_to_response(tag: &Tag) -> TagResponse {
    TagResponse {
        id: tag.id.to_string(),
        name: tag.name.clone(),
        scope: str_to_scope(&tag.scope),
        category: tag.category.clone(),
        emoji: tag.emoji.clone(),
        parent_id: tag.parent_id.map(|id| id.to_string()),
        onboarding_order: tag.onboarding_order.clone(),
    }
}

pub(super) async fn tags_search(
    State(_ctx): State<AppContext>,
    Query(query): Query<TagsQuery>,
) -> Result<Response> {
    let mut search = query.search.unwrap_or_default().to_lowercase();
    search.truncate(search.floor_char_boundary(200));
    let limit = bounded_limit(query.limit);

    let mut conn = crate::db::conn().await?;

    let data = if search.is_empty() {
        // No search term: use Diesel query builder
        let mut query_builder = tags::table.into_boxed();
        if let Some(scope) = query.scope {
            query_builder = query_builder.filter(tags::scope.eq(scope_to_str(scope)));
        }
        let all_tags = query_builder.limit(limit).load::<Tag>(&mut conn).await?;
        all_tags
            .iter()
            .map(tag_model_to_response)
            .collect::<Vec<_>>()
    } else {
        // Use tsvector + ILIKE fallback for ranked search
        let pattern = format!("%{search}%");
        let scope_filter = query.scope.map(scope_to_str);

        let rows = diesel::sql_query(
            r"
            SELECT t.id, t.name, t.scope, t.category, t.emoji, t.parent_id
            FROM tags t
            WHERE
                ($3 IS NULL OR t.scope = $3)
                AND (
                    t.search_vector @@ websearch_to_tsquery('simple', $1)
                    OR LOWER(t.name) LIKE $2
                )
            ORDER BY
                ts_rank_cd(t.search_vector, websearch_to_tsquery('simple', $1)) DESC,
                t.name ASC
            LIMIT $4
            ",
        )
        .bind::<Text, _>(&search)
        .bind::<Text, _>(&pattern)
        .bind::<Nullable<Text>, _>(scope_filter)
        .bind::<BigInt, _>(limit)
        .load::<crate::search::TagSearchRow>(&mut conn)
        .await?;

        rows.into_iter()
            .map(|row| TagResponse {
                id: row.id.to_string(),
                name: row.name,
                scope: str_to_scope(&row.scope),
                category: row.category,
                emoji: row.emoji,
                parent_id: row.parent_id.map(|id| id.to_string()),
                onboarding_order: None,
            })
            .collect::<Vec<_>>()
    };

    let mut response = Json(DataResponse { data }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PUBLIC_CACHE_MEDIUM);
    Ok(response)
}

async fn validate_and_insert_tag(
    headers: &HeaderMap,
    payload: CreateTagBody,
) -> std::result::Result<Tag, Response> {
    let name = payload.name.trim().to_string();

    if name.is_empty() || name.chars().count() > 100 {
        return Err(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            ErrorSpec {
                error: "Tag name must be between 1 and 100 characters".to_string(),
                code: "VALIDATION_ERROR",
                details: None,
            },
        ));
    }

    let scope_str = scope_to_str(payload.scope);
    let parent_id = if let Some(raw_parent_id) = payload.parent_id.as_deref() {
        Some(
            crate::api::parse_uuid_response(raw_parent_id, "parent tag", headers)
                .map_err(|response| *response)?,
        )
    } else {
        None
    };

    let mut conn = crate::db::conn().await.map_err(|e| {
        tracing::error!(error = %e, "database error checking existing tag");
        error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            headers,
            ErrorSpec {
                error: "Internal server error".to_string(),
                code: "INTERNAL_ERROR",
                details: None,
            },
        )
    })?;

    let existing = tags::table
        .filter(tags::scope.eq(scope_str))
        .filter(tags::name.eq(&name))
        .first::<Tag>(&mut conn)
        .await
        .optional()
        .map_err(|e| {
            tracing::error!(error = %e, "database error checking existing tag");
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                ErrorSpec {
                    error: "Internal server error".to_string(),
                    code: "INTERNAL_ERROR",
                    details: None,
                },
            )
        })?;

    if existing.is_some() {
        return Err(error_response(
            axum::http::StatusCode::CONFLICT,
            headers,
            ErrorSpec {
                error: format!("Tag '{name}' already exists for scope '{scope_str}'"),
                code: "CONFLICT",
                details: None,
            },
        ));
    }

    if let Some(parent_uuid) = parent_id {
        let parent_exists = tags::table
            .filter(tags::id.eq(parent_uuid))
            .select(tags::id)
            .first::<Uuid>(&mut conn)
            .await
            .optional()
            .map_err(|e| {
                tracing::error!(error = %e, "database error checking tag parent");
                error_response(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    headers,
                    ErrorSpec {
                        error: "Internal server error".to_string(),
                        code: "INTERNAL_ERROR",
                        details: None,
                    },
                )
            })?;
        if parent_exists.is_none() {
            return Err(error_response(
                axum::http::StatusCode::BAD_REQUEST,
                headers,
                ErrorSpec {
                    error: "Parent tag not found".to_string(),
                    code: "BAD_REQUEST",
                    details: None,
                },
            ));
        }
    }

    let now = chrono::Utc::now();
    let new_tag = NewTag {
        id: Uuid::new_v4(),
        name,
        scope: scope_str.to_string(),
        category: payload.category,
        emoji: payload.emoji,
        parent_id,
        onboarding_order: None,
        created_at: now,
        updated_at: now,
    };

    diesel::insert_into(tags::table)
        .values(&new_tag)
        .get_result::<Tag>(&mut conn)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "database error inserting tag");
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                ErrorSpec {
                    error: "Internal server error".to_string(),
                    code: "INTERNAL_ERROR",
                    details: None,
                },
            )
        })
}

pub(super) async fn tags_create(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<CreateTagBody>,
) -> Result<Response> {
    let (_session, _user) = auth_or_respond!(headers);

    match validate_and_insert_tag(&headers, payload).await {
        Ok(inserted) => {
            let data = tag_model_to_response(&inserted);
            Ok((axum::http::StatusCode::CREATED, Json(DataResponse { data })).into_response())
        }
        Err(response) => Ok(response),
    }
}

pub(super) async fn tags_suggestions(
    State(_ctx): State<AppContext>,
    Json(payload): Json<TagSuggestionsBody>,
) -> Result<Response> {
    let scope = scope_to_str(payload.scope);
    let mut search = payload.title.trim().to_lowercase();
    if let Some(description) = payload.description {
        let trimmed = description.trim();
        if !trimmed.is_empty() {
            if !search.is_empty() {
                search.push(' ');
            }
            search.push_str(trimmed);
        }
    }
    search.truncate(search.floor_char_boundary(200));

    if search.len() < 3 {
        return Ok(Json(DataResponse {
            data: Vec::<TagSuggestionResponse>::new(),
        })
        .into_response());
    }

    let pattern = format!("%{search}%");
    let mut conn = crate::db::conn().await?;
    let rows = diesel::sql_query(
        r"
        SELECT
            t.id,
            t.name,
            t.scope,
            t.category,
            t.emoji,
            t.parent_id,
            ts_rank_cd(t.search_vector, websearch_to_tsquery('simple', $1)) AS score
        FROM tags t
        WHERE
            t.scope = $2
            AND (
                t.search_vector @@ websearch_to_tsquery('simple', $1)
                OR LOWER(t.name) LIKE $3
            )
        ORDER BY score DESC, t.name ASC
        LIMIT 5
        ",
    )
    .bind::<Text, _>(&search)
    .bind::<Text, _>(scope)
    .bind::<Text, _>(&pattern)
    .load::<TagSuggestionRow>(&mut conn)
    .await?;

    let data = rows
        .into_iter()
        .map(|row| TagSuggestionResponse {
            tag: TagResponse {
                id: row.id.to_string(),
                name: row.name,
                scope: str_to_scope(&row.scope),
                category: row.category,
                emoji: row.emoji,
                parent_id: row.parent_id.map(|id| id.to_string()),
                onboarding_order: None,
            },
            score: row.score,
        })
        .collect::<Vec<_>>();

    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn degrees_search(
    State(_ctx): State<AppContext>,
    Query(query): Query<DegreesQuery>,
) -> Result<Response> {
    let mut search = query.search.unwrap_or_default().to_lowercase();
    search.truncate(search.floor_char_boundary(200));
    let limit = bounded_limit(query.limit);

    let mut conn = crate::db::conn().await?;

    let mut query_builder = degrees::table.into_boxed();

    if !search.is_empty() {
        let pattern = format!("%{search}%");
        query_builder = query_builder.filter(degrees::name.ilike(pattern));
    }

    let all_degrees = query_builder.limit(limit).load::<Degree>(&mut conn).await?;

    let data = all_degrees
        .iter()
        .map(|d| DegreeResponse {
            id: d.id.to_string(),
            name: d.name.clone(),
        })
        .collect::<Vec<_>>();

    let mut response = Json(DataResponse { data }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PUBLIC_CACHE_MEDIUM);
    Ok(response)
}

#[derive(QueryableByName)]
struct TagSuggestionRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    scope: String,
    #[diesel(sql_type = Nullable<Text>)]
    category: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    emoji: Option<String>,
    #[diesel(sql_type = Nullable<diesel::sql_types::Uuid>)]
    parent_id: Option<Uuid>,
    #[diesel(sql_type = Double)]
    score: f64,
}
