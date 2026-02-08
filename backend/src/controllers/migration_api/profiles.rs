use axum::{extract::Path, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use loco_rs::prelude::*;
use uuid::Uuid;

use super::{
    error_response,
    state::{
        lock_state, normalized_tag_ids, require_auth, to_full_profile_response,
        to_profile_response, validate_profile_age, validate_profile_name, CreateProfileBody,
        DataResponse, MigrationState, ProfileRecord, SuccessResponse, UpdateProfileBody,
        UserRecord,
    },
    ErrorSpec,
};

type HandlerError = Box<Response>;

fn not_found_profile(headers: &HeaderMap, id: &str) -> Response {
    error_response(
        axum::http::StatusCode::NOT_FOUND,
        headers,
        ErrorSpec {
            error: format!("Profile '{id}' not found"),
            code: "NOT_FOUND",
            details: None,
        },
    )
}

fn validation_error(headers: &HeaderMap, message: &str) -> Response {
    error_response(
        axum::http::StatusCode::BAD_REQUEST,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code: "VALIDATION_ERROR",
            details: None,
        },
    )
}

fn forbidden_error(headers: &HeaderMap) -> Response {
    error_response(
        axum::http::StatusCode::FORBIDDEN,
        headers,
        ErrorSpec {
            error: "You can only access your own profile".to_string(),
            code: "FORBIDDEN",
            details: None,
        },
    )
}

fn apply_name_update(
    headers: &HeaderMap,
    profile: &mut ProfileRecord,
    name: Option<String>,
) -> std::result::Result<(), HandlerError> {
    if let Some(next_name) = name {
        if let Err(msg) = validate_profile_name(&next_name) {
            return Err(Box::new(validation_error(headers, msg)));
        }
        profile.name = next_name.trim().to_string();
    }
    Ok(())
}

fn apply_age_update(
    headers: &HeaderMap,
    profile: &mut ProfileRecord,
    age: Option<u8>,
) -> std::result::Result<(), HandlerError> {
    if let Some(next_age) = age {
        if let Err(msg) = validate_profile_age(next_age) {
            return Err(Box::new(validation_error(headers, msg)));
        }
        profile.age = next_age;
    }
    Ok(())
}

fn validate_create_payload(headers: &HeaderMap, payload: &CreateProfileBody) -> Option<Response> {
    if let Err(msg) = validate_profile_name(&payload.name) {
        Some(validation_error(headers, msg))
    } else if let Err(msg) = validate_profile_age(payload.age) {
        Some(validation_error(headers, msg))
    } else {
        None
    }
}

fn resolve_user(
    headers: &HeaderMap,
    state: &mut MigrationState,
) -> std::result::Result<UserRecord, HandlerError> {
    require_auth(headers, state).map(|(_session, user)| user)
}

fn load_owned_profile(
    headers: &HeaderMap,
    state: &MigrationState,
    profile_id: &str,
    user_id: &str,
) -> std::result::Result<ProfileRecord, HandlerError> {
    state
        .profiles
        .get(profile_id)
        .cloned()
        .ok_or_else(|| Box::new(not_found_profile(headers, profile_id)))
        .and_then(|profile| {
            if profile.user_id == user_id {
                Ok(profile)
            } else {
                Err(Box::new(forbidden_error(headers)))
            }
        })
}

fn apply_update_payload(
    headers: &HeaderMap,
    state: &MigrationState,
    profile: &mut ProfileRecord,
    payload: UpdateProfileBody,
) -> std::result::Result<(), HandlerError> {
    let UpdateProfileBody {
        name,
        age,
        bio,
        program,
        profile_picture,
        images,
        tags,
        tag_ids,
    } = payload;

    apply_name_update(headers, profile, name)?;
    apply_age_update(headers, profile, age)?;
    apply_optional_scalar_updates(
        profile,
        OptionalScalarUpdates {
            bio,
            program,
            profile_picture,
            images,
        },
    );
    if tags.is_some() || tag_ids.is_some() {
        profile.tag_ids = normalized_tag_ids(state, tags, tag_ids);
    }
    profile.updated_at = Utc::now();
    Ok(())
}

struct OptionalScalarUpdates {
    bio: Option<String>,
    program: Option<String>,
    profile_picture: Option<String>,
    images: Option<Vec<String>>,
}

fn apply_optional_scalar_updates(profile: &mut ProfileRecord, updates: OptionalScalarUpdates) {
    if let Some(value) = updates.bio {
        profile.bio = Some(value);
    }
    if let Some(value) = updates.program {
        profile.program = Some(value);
    }
    if let Some(value) = updates.profile_picture {
        profile.profile_picture = Some(value);
    }
    if let Some(value) = updates.images {
        profile.images = value;
    }
}

pub(super) async fn profile_me(headers: HeaderMap) -> Result<Response> {
    let response = {
        let mut state = lock_state();
        let (_session, user) = match require_auth(&headers, &mut state) {
            Ok(auth) => auth,
            Err(response) => return Ok(*response),
        };
        state
            .profiles_by_user
            .get(&user.id)
            .and_then(|id| state.profiles.get(id))
            .map(|profile| to_full_profile_response(&state, profile))
    };

    Ok(Json(DataResponse { data: response }).into_response())
}

pub(super) async fn profile_get(headers: HeaderMap, Path(id): Path<String>) -> Result<Response> {
    let mut state = lock_state();
    let _auth = match require_auth(&headers, &mut state) {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let Some(profile) = state.profiles.get(&id) else {
        return Ok(not_found_profile(&headers, &id));
    };
    let data = to_profile_response(profile);
    drop(state);

    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn profile_get_full(
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let mut state = lock_state();
    let _auth = match require_auth(&headers, &mut state) {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let Some(profile) = state.profiles.get(&id) else {
        return Ok(not_found_profile(&headers, &id));
    };
    let data = to_full_profile_response(&state, profile);
    drop(state);

    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn profile_create(
    headers: HeaderMap,
    Json(payload): Json<CreateProfileBody>,
) -> Result<Response> {
    let mut state = lock_state();
    let response = match resolve_user(&headers, &mut state) {
        Err(response) => *response,
        Ok(user) => {
            if let Some(validation_response) = validate_create_payload(&headers, &payload) {
                validation_response
            } else if state.profiles_by_user.contains_key(&user.id) {
                error_response(
                    axum::http::StatusCode::CONFLICT,
                    &headers,
                    ErrorSpec {
                        error: "Profile already exists".to_string(),
                        code: "CONFLICT",
                        details: None,
                    },
                )
            } else {
                let now = Utc::now();
                let profile = ProfileRecord {
                    id: Uuid::new_v4().to_string(),
                    user_id: user.id,
                    name: payload.name.trim().to_string(),
                    bio: payload.bio,
                    age: payload.age,
                    profile_picture: payload.profile_picture,
                    images: payload.images.unwrap_or_default(),
                    program: payload.program,
                    tag_ids: normalized_tag_ids(&state, payload.tags, payload.tag_ids),
                    created_at: now,
                    updated_at: now,
                };

                let profile_id = profile.id.clone();
                state
                    .profiles_by_user
                    .insert(profile.user_id.clone(), profile_id.clone());
                state.profiles.insert(profile_id, profile.clone());
                let data = to_full_profile_response(&state, &profile);
                drop(state);

                (axum::http::StatusCode::CREATED, Json(DataResponse { data })).into_response()
            }
        }
    };
    Ok(response)
}

pub(super) async fn profile_update(
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateProfileBody>,
) -> Result<Response> {
    let mut state = lock_state();
    let response = (|| -> std::result::Result<Response, HandlerError> {
        let user = resolve_user(&headers, &mut state)?;
        let mut profile = load_owned_profile(&headers, &state, &id, &user.id)?;
        apply_update_payload(&headers, &state, &mut profile, payload)?;
        state.profiles.insert(id, profile.clone());
        let data = to_full_profile_response(&state, &profile);
        Ok(Json(DataResponse { data }).into_response())
    })()
    .unwrap_or_else(|response| *response);
    drop(state);
    Ok(response)
}

pub(super) async fn profile_delete(headers: HeaderMap, Path(id): Path<String>) -> Result<Response> {
    let mut state = lock_state();
    let response = match resolve_user(&headers, &mut state)
        .and_then(|user| load_owned_profile(&headers, &state, &id, &user.id).map(|_profile| user))
    {
        Ok(user) => {
            state.profiles.remove(&id);
            state.profiles_by_user.remove(&user.id);
            drop(state);
            Json(SuccessResponse { success: true }).into_response()
        }
        Err(response) => *response,
    };
    Ok(response)
}
