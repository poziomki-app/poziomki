use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{ActiveValue, QueryFilter, QuerySelect, Select};
use uuid::Uuid;

use super::{
    error_response,
    state::{
        require_auth_db, CreateTagBody, DataResponse, DegreeResponse, DegreesQuery, TagResponse,
        TagScope, TagsQuery,
    },
    ErrorSpec,
};
use crate::models::_entities::{degrees, tags};

const PUBLIC_CACHE_MEDIUM: HeaderValue = HeaderValue::from_static("public, max-age=1800");

const fn scope_to_str(scope: TagScope) -> &'static str {
    match scope {
        TagScope::Interest => "interest",
        TagScope::Activity => "activity",
        TagScope::Event => "event",
    }
}

fn bounded_limit(limit: Option<u8>) -> u64 {
    u64::from(limit.unwrap_or(20).clamp(1, 100))
}

fn str_to_scope(s: &str) -> TagScope {
    match s {
        "activity" => TagScope::Activity,
        "event" => TagScope::Event,
        _ => TagScope::Interest,
    }
}

fn tag_model_to_response(tag: &tags::Model) -> TagResponse {
    TagResponse {
        id: tag.id.to_string(),
        name: tag.name.clone(),
        scope: str_to_scope(&tag.scope),
        category: tag.category.clone(),
        emoji: tag.emoji.clone(),
        onboarding_order: tag.onboarding_order.clone(),
    }
}

pub(super) async fn tags_search(
    State(ctx): State<AppContext>,
    Query(query): Query<TagsQuery>,
) -> Result<Response> {
    let search = query.search.unwrap_or_default().to_lowercase();
    let limit = bounded_limit(query.limit);

    let mut query_builder: Select<tags::Entity> = tags::Entity::find();

    if let Some(scope) = query.scope {
        query_builder = query_builder.filter(tags::Column::Scope.eq(scope_to_str(scope)));
    }

    if !search.is_empty() {
        query_builder = query_builder.filter(tags::Column::Name.contains(&search));
    }

    let all_tags = query_builder
        .limit(limit)
        .all(&ctx.db)
        .await
        .map_err(|e: sea_orm::DbErr| loco_rs::Error::Any(e.into()))?;

    let data = all_tags
        .iter()
        .map(tag_model_to_response)
        .collect::<Vec<_>>();

    let mut response = Json(DataResponse { data }).into_response();
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PUBLIC_CACHE_MEDIUM);
    Ok(response)
}

async fn validate_and_insert_tag(
    db: &DatabaseConnection,
    headers: &HeaderMap,
    payload: CreateTagBody,
) -> std::result::Result<tags::Model, Response> {
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

    let existing = tags::Entity::find()
        .filter(tags::Column::Scope.eq(scope_str))
        .filter(tags::Column::Name.eq(&name))
        .one(db)
        .await
        .map_err(|e: sea_orm::DbErr| {
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

    let now = chrono::Utc::now();
    let tag = tags::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        name: ActiveValue::Set(name),
        scope: ActiveValue::Set(scope_str.to_string()),
        category: ActiveValue::Set(payload.category),
        emoji: ActiveValue::Set(payload.emoji),
        onboarding_order: ActiveValue::Set(None),
        created_at: ActiveValue::Set(now.into()),
        updated_at: ActiveValue::Set(now.into()),
    };

    tag.insert(db).await.map_err(|e: sea_orm::DbErr| {
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
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<CreateTagBody>,
) -> Result<Response> {
    let (_session, _user) = match require_auth_db(&ctx.db, &headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    match validate_and_insert_tag(&ctx.db, &headers, payload).await {
        Ok(inserted) => {
            // MEILI_COMPAT_REMOVE
            crate::search::index_tag_compat(crate::search::TagDocument {
                id: inserted.id.to_string(),
                name: inserted.name.clone(),
                scope: inserted.scope.clone(),
                category: inserted.category.clone(),
                emoji: inserted.emoji.clone(),
            });

            let data = tag_model_to_response(&inserted);
            Ok((axum::http::StatusCode::CREATED, Json(DataResponse { data })).into_response())
        }
        Err(response) => Ok(response),
    }
}

pub(super) async fn degrees_search(
    State(ctx): State<AppContext>,
    Query(query): Query<DegreesQuery>,
) -> Result<Response> {
    let search = query.search.unwrap_or_default().to_lowercase();
    let limit = bounded_limit(query.limit);

    let mut query_builder: Select<degrees::Entity> = degrees::Entity::find();

    if !search.is_empty() {
        query_builder = query_builder.filter(degrees::Column::Name.contains(&search));
    }

    let all_degrees = query_builder
        .limit(limit)
        .all(&ctx.db)
        .await
        .map_err(|e: sea_orm::DbErr| loco_rs::Error::Any(e.into()))?;

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
