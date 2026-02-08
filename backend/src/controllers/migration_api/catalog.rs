use axum::{extract::Query, http::HeaderMap, response::IntoResponse, Json};
use loco_rs::prelude::*;
use uuid::Uuid;

use super::{
    error_response,
    state::{
        bounded_limit, lock_state, require_auth, to_tag_response, CreateTagBody, DataResponse,
        DegreeResponse, DegreesQuery, MigrationState, TagRecord, TagScope, TagsQuery,
    },
    ErrorSpec,
};

pub(super) async fn tags_search(Query(query): Query<TagsQuery>) -> Result<Response> {
    let search = query.search.unwrap_or_default().to_lowercase();
    let limit = bounded_limit(query.limit);
    let data = {
        let state = lock_state();
        state
            .tags
            .values()
            .filter(|tag| tag.scope == query.scope)
            .filter(|tag| search.is_empty() || tag.name.to_lowercase().contains(&search))
            .take(limit)
            .map(to_tag_response)
            .collect::<Vec<_>>()
    };

    Ok(Json(DataResponse { data }).into_response())
}

fn duplicate_tag_error(headers: &HeaderMap, name: &str, scope: TagScope) -> Response {
    let scope_label = match scope {
        TagScope::Interest => "interest",
        TagScope::Activity => "activity",
        TagScope::Event => "event",
    };
    error_response(
        axum::http::StatusCode::CONFLICT,
        headers,
        ErrorSpec {
            error: format!("Tag '{name}' already exists for scope '{scope_label}'"),
            code: "CONFLICT",
            details: None,
        },
    )
}

fn validate_tag_name(headers: &HeaderMap, name: &str) -> Option<Response> {
    if name.is_empty() || name.chars().count() > 100 {
        Some(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            ErrorSpec {
                error: "Tag name must be between 1 and 100 characters".to_string(),
                code: "VALIDATION_ERROR",
                details: None,
            },
        ))
    } else {
        None
    }
}

fn tag_exists(state: &MigrationState, payload: &CreateTagBody, name: &str) -> bool {
    state
        .tags
        .values()
        .any(|tag| tag.scope == payload.scope && tag.name.eq_ignore_ascii_case(name))
}

fn create_tag_response(
    state: &mut MigrationState,
    payload: &CreateTagBody,
    name: &str,
) -> Response {
    let tag = TagRecord {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        scope: payload.scope,
        category: payload.category.clone(),
        emoji: payload.emoji.clone(),
        onboarding_order: None,
    };
    state.tags.insert(tag.id.clone(), tag.clone());
    let data = to_tag_response(&tag);
    (axum::http::StatusCode::CREATED, Json(DataResponse { data })).into_response()
}

pub(super) async fn tags_create(
    headers: HeaderMap,
    Json(payload): Json<CreateTagBody>,
) -> Result<Response> {
    let mut state = lock_state();
    let response = match require_auth(&headers, &mut state) {
        Err(response) => *response,
        Ok((_session, _user)) => {
            let name = payload.name.trim();
            if let Some(error) = validate_tag_name(&headers, name) {
                error
            } else if tag_exists(&state, &payload, name) {
                duplicate_tag_error(&headers, name, payload.scope)
            } else {
                create_tag_response(&mut state, &payload, name)
            }
        }
    };
    drop(state);
    Ok(response)
}

pub(super) async fn degrees_search(Query(query): Query<DegreesQuery>) -> Result<Response> {
    let search = query.search.unwrap_or_default().to_lowercase();
    let limit = bounded_limit(query.limit);
    let data = {
        let state = lock_state();
        state
            .degrees
            .iter()
            .filter(|degree| search.is_empty() || degree.name.to_lowercase().contains(&search))
            .take(limit)
            .map(|degree| DegreeResponse {
                id: degree.id.clone(),
                name: degree.name.clone(),
            })
            .collect::<Vec<_>>()
    };

    Ok(Json(DataResponse { data }).into_response())
}
