type Result<T> = crate::error::AppResult<T>;

use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::common::{error_response, ErrorSpec};
use crate::api::state::DataResponse;
use crate::app::AppContext;
use crate::db;
use crate::db::models::event_place_polls::{
    EventPlaceOption, EventPlaceVote, NewEventPlaceOption, NewEventPlacePoll,
};
use crate::db::schema::{event_place_options, event_place_polls, event_place_votes};

use super::events_service::{
    forbidden, load_event_by_id, load_profile_for_user, not_found_event, profile_not_found,
    validation_error,
};

const MIN_OPTIONS: usize = 2;
const MAX_OPTIONS: usize = 5;
const MAX_LABEL_LEN: usize = 120;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct CreatePlacePollBody {
    pub(in crate::api) options: Vec<CreatePlacePollOption>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct CreatePlacePollOption {
    pub(in crate::api) label: String,
    #[serde(default)]
    pub(in crate::api) latitude: Option<f64>,
    #[serde(default)]
    pub(in crate::api) longitude: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct VotePlacePollBody {
    pub(in crate::api) option_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct PlacePollResponse {
    pub(in crate::api) id: String,
    pub(in crate::api) options: Vec<PlacePollOptionResponse>,
    pub(in crate::api) my_vote: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct PlacePollOptionResponse {
    pub(in crate::api) id: String,
    pub(in crate::api) label: String,
    pub(in crate::api) latitude: Option<f64>,
    pub(in crate::api) longitude: Option<f64>,
    pub(in crate::api) vote_count: i64,
}

fn into_diesel(e: crate::error::AppError) -> diesel::result::Error {
    match e {
        crate::error::AppError::Message(_) | crate::error::AppError::Validation(_) => {
            diesel::result::Error::QueryBuilderError(Box::new(e))
        }
        crate::error::AppError::Any(_) => diesel::result::Error::RollbackTransaction,
    }
}

enum CreateOutcome {
    NoProfile,
    NotFound,
    NotCreator,
    AlreadyExists,
    Created(PlacePollResponse),
}

enum GetOutcome {
    NoProfile,
    NotFound,
    NoPoll,
    Loaded(PlacePollResponse),
}

enum VoteOutcome {
    NoProfile,
    NotFound,
    NoPoll,
    InvalidOption,
    Voted(PlacePollResponse),
}

async fn build_response(
    conn: &mut AsyncPgConnection,
    poll_id: Uuid,
    profile_id: Uuid,
) -> std::result::Result<PlacePollResponse, crate::error::AppError> {
    let options: Vec<EventPlaceOption> = event_place_options::table
        .filter(event_place_options::poll_id.eq(poll_id))
        .order(event_place_options::created_at.asc())
        .load(conn)
        .await?;

    let counts: Vec<(Uuid, i64)> = event_place_votes::table
        .filter(event_place_votes::poll_id.eq(poll_id))
        .group_by(event_place_votes::option_id)
        .select((event_place_votes::option_id, diesel::dsl::count_star()))
        .load(conn)
        .await?;

    let my_vote: Option<Uuid> = event_place_votes::table
        .filter(event_place_votes::poll_id.eq(poll_id))
        .filter(event_place_votes::profile_id.eq(profile_id))
        .select(event_place_votes::option_id)
        .first(conn)
        .await
        .optional()?;

    Ok(PlacePollResponse {
        id: poll_id.to_string(),
        options: options
            .into_iter()
            .map(|opt| {
                let count = counts
                    .iter()
                    .find(|(id, _)| *id == opt.id)
                    .map_or(0, |(_, c)| *c);
                PlacePollOptionResponse {
                    id: opt.id.to_string(),
                    label: opt.label,
                    latitude: opt.latitude,
                    longitude: opt.longitude,
                    vote_count: count,
                }
            })
            .collect(),
        my_vote: my_vote.map(|id| id.to_string()),
    })
}

pub(in crate::api) async fn place_poll_create(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<CreatePlacePollBody>,
) -> Result<Response> {
    let (_session, user) = match crate::api::state::require_auth_db(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let event_uuid = match crate::api::parse_uuid_response(&id, "event", &headers) {
        Ok(uuid) => uuid,
        Err(response) => return Ok(*response),
    };

    if body.options.len() < MIN_OPTIONS || body.options.len() > MAX_OPTIONS {
        return Ok(validation_error(
            &headers,
            "Głosowanie musi mieć od 2 do 5 propozycji",
        ));
    }
    let mut prepared: Vec<(String, Option<f64>, Option<f64>)> =
        Vec::with_capacity(body.options.len());
    for opt in &body.options {
        let label = opt.label.trim().to_string();
        if label.is_empty() || label.chars().count() > MAX_LABEL_LEN {
            return Ok(validation_error(&headers, "Nieprawidłowa nazwa propozycji"));
        }
        prepared.push((label, opt.latitude, opt.longitude));
    }

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let Some(profile) = load_profile_for_user(conn, user_id)
                .await
                .map_err(into_diesel)?
            else {
                return Ok::<CreateOutcome, diesel::result::Error>(CreateOutcome::NoProfile);
            };
            let Some(event) = load_event_by_id(conn, event_uuid)
                .await
                .map_err(into_diesel)?
            else {
                return Ok(CreateOutcome::NotFound);
            };
            if event.creator_id != profile.id {
                return Ok(CreateOutcome::NotCreator);
            }
            let exists = event_place_polls::table
                .filter(event_place_polls::event_id.eq(event_uuid))
                .select(event_place_polls::id)
                .first::<Uuid>(conn)
                .await
                .optional()
                .map_err(|e| into_diesel(crate::error::AppError::from(e)))?;
            if exists.is_some() {
                return Ok(CreateOutcome::AlreadyExists);
            }

            let poll_id = Uuid::new_v4();
            diesel::insert_into(event_place_polls::table)
                .values(NewEventPlacePoll {
                    id: poll_id,
                    event_id: event_uuid,
                })
                .execute(conn)
                .await
                .map_err(|e| into_diesel(crate::error::AppError::from(e)))?;

            let new_options: Vec<NewEventPlaceOption> = prepared
                .iter()
                .map(|(label, lat, lng)| NewEventPlaceOption {
                    id: Uuid::new_v4(),
                    poll_id,
                    label: label.clone(),
                    latitude: *lat,
                    longitude: *lng,
                })
                .collect();
            diesel::insert_into(event_place_options::table)
                .values(&new_options)
                .execute(conn)
                .await
                .map_err(|e| into_diesel(crate::error::AppError::from(e)))?;

            let response = build_response(conn, poll_id, profile.id)
                .await
                .map_err(into_diesel)?;
            Ok(CreateOutcome::Created(response))
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    match outcome {
        CreateOutcome::NoProfile => Ok(profile_not_found(&headers)),
        CreateOutcome::NotFound => Ok(not_found_event(&headers, &id)),
        CreateOutcome::NotCreator => Ok(forbidden(
            &headers,
            "Tylko organizator może utworzyć głosowanie",
        )),
        CreateOutcome::AlreadyExists => Ok(error_response(
            axum::http::StatusCode::CONFLICT,
            &headers,
            ErrorSpec {
                error: "Głosowanie już istnieje".to_string(),
                code: "ALREADY_EXISTS",
                details: None,
            },
        )),
        CreateOutcome::Created(response) => {
            Ok(Json(DataResponse { data: response }).into_response())
        }
    }
}

pub(in crate::api) async fn place_poll_get(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, user) = match crate::api::state::require_auth_db(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let event_uuid = match crate::api::parse_uuid_response(&id, "event", &headers) {
        Ok(uuid) => uuid,
        Err(response) => return Ok(*response),
    };

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let Some(profile) = load_profile_for_user(conn, user_id)
                .await
                .map_err(into_diesel)?
            else {
                return Ok::<GetOutcome, diesel::result::Error>(GetOutcome::NoProfile);
            };
            if load_event_by_id(conn, event_uuid)
                .await
                .map_err(into_diesel)?
                .is_none()
            {
                return Ok(GetOutcome::NotFound);
            }
            let poll_id = event_place_polls::table
                .filter(event_place_polls::event_id.eq(event_uuid))
                .select(event_place_polls::id)
                .first::<Uuid>(conn)
                .await
                .optional()
                .map_err(|e| into_diesel(crate::error::AppError::from(e)))?;
            let Some(poll_id) = poll_id else {
                return Ok(GetOutcome::NoPoll);
            };
            let response = build_response(conn, poll_id, profile.id)
                .await
                .map_err(into_diesel)?;
            Ok(GetOutcome::Loaded(response))
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    match outcome {
        GetOutcome::NoProfile => Ok(profile_not_found(&headers)),
        GetOutcome::NotFound => Ok(not_found_event(&headers, &id)),
        GetOutcome::NoPoll => Ok(error_response(
            axum::http::StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "Brak głosowania dla tego wydarzenia".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        )),
        GetOutcome::Loaded(response) => Ok(Json(DataResponse { data: response }).into_response()),
    }
}

pub(in crate::api) async fn place_poll_vote(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<VotePlacePollBody>,
) -> Result<Response> {
    let (_session, user) = match crate::api::state::require_auth_db(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let event_uuid = match crate::api::parse_uuid_response(&id, "event", &headers) {
        Ok(uuid) => uuid,
        Err(response) => return Ok(*response),
    };
    let Ok(option_uuid) = Uuid::parse_str(&body.option_id) else {
        return Ok(validation_error(
            &headers,
            "Nieprawidłowy identyfikator opcji",
        ));
    };

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let Some(profile) = load_profile_for_user(conn, user_id)
                .await
                .map_err(into_diesel)?
            else {
                return Ok::<VoteOutcome, diesel::result::Error>(VoteOutcome::NoProfile);
            };
            if load_event_by_id(conn, event_uuid)
                .await
                .map_err(into_diesel)?
                .is_none()
            {
                return Ok(VoteOutcome::NotFound);
            }
            let poll_id = event_place_polls::table
                .filter(event_place_polls::event_id.eq(event_uuid))
                .select(event_place_polls::id)
                .first::<Uuid>(conn)
                .await
                .optional()
                .map_err(|e| into_diesel(crate::error::AppError::from(e)))?;
            let Some(poll_id) = poll_id else {
                return Ok(VoteOutcome::NoPoll);
            };

            let option_belongs = event_place_options::table
                .filter(event_place_options::id.eq(option_uuid))
                .filter(event_place_options::poll_id.eq(poll_id))
                .select(event_place_options::id)
                .first::<Uuid>(conn)
                .await
                .optional()
                .map_err(|e| into_diesel(crate::error::AppError::from(e)))?;
            if option_belongs.is_none() {
                return Ok(VoteOutcome::InvalidOption);
            }

            let vote = EventPlaceVote {
                poll_id,
                profile_id: profile.id,
                option_id: option_uuid,
                voted_at: chrono::Utc::now(),
            };
            diesel::insert_into(event_place_votes::table)
                .values(&vote)
                .on_conflict((event_place_votes::poll_id, event_place_votes::profile_id))
                .do_update()
                .set((
                    event_place_votes::option_id.eq(option_uuid),
                    event_place_votes::voted_at.eq(chrono::Utc::now()),
                ))
                .execute(conn)
                .await
                .map_err(|e| into_diesel(crate::error::AppError::from(e)))?;

            let response = build_response(conn, poll_id, profile.id)
                .await
                .map_err(into_diesel)?;
            Ok(VoteOutcome::Voted(response))
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    match outcome {
        VoteOutcome::NoProfile => Ok(profile_not_found(&headers)),
        VoteOutcome::NotFound => Ok(not_found_event(&headers, &id)),
        VoteOutcome::NoPoll => Ok(error_response(
            axum::http::StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "Brak głosowania dla tego wydarzenia".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        )),
        VoteOutcome::InvalidOption => Ok(validation_error(
            &headers,
            "Wybrana propozycja nie należy do tego głosowania",
        )),
        VoteOutcome::Voted(response) => Ok(Json(DataResponse { data: response }).into_response()),
    }
}
