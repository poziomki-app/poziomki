type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use diesel::QueryDsl;
use diesel_async::{AsyncConnection, RunQueryDsl};
use uuid::Uuid;

use crate::api::state::{
    AttendEventBody, AttendeeStatus, CreateEventBody, DataResponse, SuccessResponse,
    UpdateEventBody,
};
use crate::db::models::events::{Event, NewEvent};
use crate::db::models::profiles::Profile;
use crate::jobs::enqueue_chat_membership_sync;

use super::events_interactions_repo::{
    delete_event_interaction, delete_event_interaction_with_conn, upsert_event_interaction,
    upsert_event_interaction_with_conn, EVENT_INTERACTION_JOINED, EVENT_INTERACTION_SAVED,
};
use super::events_service::{
    forbidden, load_event, parse_create_dates, require_auth_profile, validate_max_attendees,
    validation_error,
};
use super::events_tags_repo::{sync_event_tags_with_conn, upsert_attendee_with_conn};
use super::events_tags_service::{
    maybe_sync_tags_with_conn, resolve_event_tag_ids, resolve_event_tag_ids_with_conn,
};
use super::events_view::{build_event_response, created_event_response};
use super::events_write_repo;
use super::events_write_service::event_update_inner;

struct ValidatedCreate {
    profile: Profile,
    title: String,
    starts_at: chrono::DateTime<Utc>,
    ends_at: Option<chrono::DateTime<Utc>>,
}

async fn event_create_validate(
    headers: &HeaderMap,
    payload: &CreateEventBody,
) -> std::result::Result<ValidatedCreate, Box<Response>> {
    let (profile, _user_pid) = require_auth_profile(headers).await?;
    let (title, starts_at, ends_at) = parse_create_dates(headers, payload)?;
    validate_max_attendees(payload.max_attendees)
        .map_err(|msg| Box::new(validation_error(headers, msg)))?;
    Ok(ValidatedCreate {
        profile,
        title,
        starts_at,
        ends_at,
    })
}

fn build_create_event(validated: &ValidatedCreate, payload: &CreateEventBody) -> (NewEvent, Uuid) {
    let now = Utc::now();
    let event_id = Uuid::new_v4();
    let model = NewEvent {
        id: event_id,
        title: validated.title.clone(),
        description: payload.description.clone(),
        cover_image: payload.cover_image.clone(),
        category: payload.category.clone(),
        location: payload.location.clone(),
        starts_at: validated.starts_at,
        ends_at: validated.ends_at,
        creator_id: validated.profile.id,
        conversation_id: None,
        latitude: payload.latitude,
        longitude: payload.longitude,
        max_attendees: payload.max_attendees,
        created_at: now,
        updated_at: now,
        requires_approval: payload.requires_approval.unwrap_or(false),
    };
    (model, event_id)
}

pub(in crate::api) async fn event_create(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<CreateEventBody>,
) -> Result<Response> {
    let validated = match event_create_validate(&headers, &payload).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    let (new_event, event_id) = build_create_event(&validated, &payload);
    let tag_names = payload.tags.clone();
    let validated_tag_ids = match payload.tag_ids.clone() {
        Some(ids) => match resolve_event_tag_ids(&headers, None, Some(ids)).await {
            Ok(ids) => Some(ids),
            Err(response) => return Ok(*response),
        },
        None => None,
    };

    let mut conn = crate::db::conn().await?;
    let inserted = conn
        .transaction(|conn| {
            let new_event = NewEvent {
                id: new_event.id,
                title: new_event.title.clone(),
                description: new_event.description.clone(),
                cover_image: new_event.cover_image.clone(),
                category: new_event.category.clone(),
                location: new_event.location.clone(),
                starts_at: new_event.starts_at,
                ends_at: new_event.ends_at,
                creator_id: new_event.creator_id,
                conversation_id: new_event.conversation_id.clone(),
                latitude: new_event.latitude,
                longitude: new_event.longitude,
                max_attendees: new_event.max_attendees,
                created_at: new_event.created_at,
                updated_at: new_event.updated_at,
                requires_approval: new_event.requires_approval,
            };
            let tag_names = tag_names.clone();
            let validated_tag_ids = validated_tag_ids.clone();
            let profile_id = validated.profile.id;
            Box::pin(async move {
                let inserted = events_write_repo::insert_event_with_conn(conn, &new_event).await?;
                let tag_ids =
                    resolve_event_tag_ids_with_conn(conn, tag_names, validated_tag_ids).await?;
                if !tag_ids.is_empty() {
                    sync_event_tags_with_conn(conn, event_id, &tag_ids).await?;
                }
                upsert_attendee_with_conn(conn, event_id, profile_id, ATTENDEE_GOING).await?;
                upsert_event_interaction_with_conn(
                    conn,
                    profile_id,
                    event_id,
                    EVENT_INTERACTION_JOINED,
                )
                .await?;
                Ok::<Event, crate::error::AppError>(inserted)
            })
        })
        .await?;

    let data = build_event_response(&inserted, &validated.profile.id).await?;
    Ok(created_event_response(data))
}

pub(in crate::api) async fn event_update(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateEventBody>,
) -> Result<Response> {
    let (changeset, event_uuid, profile, _current_event) =
        match event_update_inner(&headers, &id, &payload).await {
            Ok(data) => data,
            Err(response) => return Ok(*response),
        };

    let tag_names = payload.tags.clone();
    let validated_tag_ids = if payload.tag_ids.is_some() {
        match resolve_event_tag_ids(&headers, None, payload.tag_ids.clone()).await {
            Ok(ids) => Some(ids),
            Err(response) => return Ok(*response),
        }
    } else {
        None
    };

    let sets_approval_false = changeset.requires_approval == Some(false);

    let mut conn = crate::db::conn().await?;
    let mut attempts = 0;
    let (updated, auto_approved) = loop {
        attempts += 1;
        let result = conn
            .build_transaction()
            .serializable()
            .run(|conn| {
                let changeset = crate::db::models::events::EventChangeset {
                    title: changeset.title.clone(),
                    description: changeset.description.clone(),
                    cover_image: changeset.cover_image.clone(),
                    category: changeset.category.clone(),
                    location: changeset.location.clone(),
                    starts_at: changeset.starts_at,
                    ends_at: changeset.ends_at,
                    conversation_id: changeset.conversation_id.clone(),
                    latitude: changeset.latitude,
                    longitude: changeset.longitude,
                    max_attendees: changeset.max_attendees,
                    updated_at: changeset.updated_at,
                    requires_approval: changeset.requires_approval,
                };
                let tag_names = tag_names.clone();
                let validated_tag_ids = validated_tag_ids.clone();
                Box::pin(async move {
                    // Read requires_approval inside the txn so retries see fresh state
                    let was_requiring = sets_approval_false
                        && crate::db::schema::events::table
                            .find(event_uuid)
                            .select(crate::db::schema::events::requires_approval)
                            .first::<bool>(conn)
                            .await?;
                    let updated =
                        events_write_repo::update_event_with_conn(conn, event_uuid, &changeset)
                            .await?;
                    let auto_approved = if was_requiring {
                        events_write_repo::auto_approve_pending_with_conn(
                            conn,
                            event_uuid,
                            updated.max_attendees,
                        )
                        .await?
                    } else {
                        vec![]
                    };
                    maybe_sync_tags_with_conn(conn, event_uuid, tag_names, validated_tag_ids)
                        .await?;
                    Ok::<(Event, Vec<Uuid>), crate::error::AppError>((updated, auto_approved))
                })
            })
            .await;
        match result {
            Ok(val) => break val,
            Err(ref e)
                if attempts < events_write_repo::MAX_ATTEMPTS
                    && events_write_repo::is_serialization_failure_app(e) =>
            {
                tokio::time::sleep(std::time::Duration::from_millis(10u64 << attempts)).await;
            }
            Err(e) => return Err(e),
        }
    };

    for pid in &auto_approved {
        if let Err(error) = enqueue_chat_membership_sync(&updated.id, pid, false).await {
            tracing::warn!(
                %error,
                event_id = %updated.id,
                profile_id = %pid,
                "failed to enqueue chat membership sync for auto-approved attendee"
            );
        }
    }

    let data = build_event_response(&updated, &profile.id).await?;
    Ok(Json(DataResponse { data }).into_response())
}

async fn load_owned_event(
    headers: &HeaderMap,
    id: &str,
    message: &str,
) -> std::result::Result<(Event, Uuid), Box<Response>> {
    let (profile, _user_pid) = require_auth_profile(headers).await?;
    let (event, event_uuid) = load_event(headers, id).await?;
    if event.creator_id != profile.id {
        return Err(Box::new(forbidden(headers, message)));
    }
    Ok((event, event_uuid))
}

pub(in crate::api) async fn event_delete(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_event, event_uuid) =
        match load_owned_event(&headers, &id, "Only the creator can delete this event").await {
            Ok(data) => data,
            Err(response) => return Ok(*response),
        };

    events_write_repo::delete_event(event_uuid).await?;

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

async fn load_event_with_profile(
    headers: &HeaderMap,
    id: &str,
) -> std::result::Result<(Event, Uuid, Profile), Box<Response>> {
    let (profile, _user_pid) = require_auth_profile(headers).await?;
    let (event, event_uuid) = load_event(headers, id).await?;
    Ok((event, event_uuid, profile))
}

const ATTENDEE_GOING: &str = "going";
const ATTENDEE_INTERESTED: &str = "interested";
const ATTENDEE_INVITED: &str = "invited";

fn resolve_attend_status(payload: Option<Json<AttendEventBody>>) -> &'static str {
    match payload
        .and_then(|Json(body)| body.status)
        .unwrap_or(AttendeeStatus::Going)
    {
        // Clients cannot self-assign pending; the approval gate below handles downgrade
        AttendeeStatus::Going | AttendeeStatus::Pending => ATTENDEE_GOING,
        AttendeeStatus::Interested => ATTENDEE_INTERESTED,
        AttendeeStatus::Invited => ATTENDEE_INVITED,
    }
}

pub(in crate::api) async fn event_attend(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    payload: Option<Json<AttendEventBody>>,
) -> Result<Response> {
    let (event, event_uuid, profile) = match load_event_with_profile(&headers, &id).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    let status_str = resolve_attend_status(payload);

    // Approval gate only applies to "going" — "interested" is a soft signal that
    // intentionally bypasses approval so users can bookmark events without creator action.
    let requires_approval =
        event.requires_approval && status_str == ATTENDEE_GOING && event.creator_id != profile.id;

    let outcome = events_write_repo::check_capacity_and_upsert(
        event_uuid,
        profile.id,
        status_str,
        event.max_attendees,
        None,
        requires_approval,
    )
    .await?;
    let written_status = match outcome {
        events_write_repo::UpsertOutcome::Full => {
            return Ok(validation_error(&headers, "Event is full"));
        }
        events_write_repo::UpsertOutcome::Accepted(s) => s,
        events_write_repo::UpsertOutcome::StatusMismatch => {
            debug_assert!(false, "StatusMismatch returned with require_status = None");
            return Ok(validation_error(&headers, "Unexpected status mismatch"));
        }
    };

    if written_status == ATTENDEE_GOING {
        if let Err(error) = enqueue_chat_membership_sync(&event.id, &profile.id, false).await {
            tracing::warn!(
                %error,
                event_id = %event.id,
                profile_id = %profile.id,
                "failed to enqueue chat membership sync after attend"
            );
        }
    }

    let data = build_event_response(&event, &profile.id).await?;
    Ok(Json(DataResponse { data }).into_response())
}

pub(in crate::api) async fn event_save(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (event, event_uuid, profile) = match load_event_with_profile(&headers, &id).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    upsert_event_interaction(profile.id, event_uuid, EVENT_INTERACTION_SAVED).await?;

    let data = build_event_response(&event, &profile.id).await?;
    Ok(Json(DataResponse { data }).into_response())
}

pub(in crate::api) async fn event_approve_attendee(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path((id, profile_id_str)): Path<(String, String)>,
) -> Result<Response> {
    let (profile, _user_pid) = match require_auth_profile(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let (event, event_uuid) = match load_event(&headers, &id).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    if event.creator_id != profile.id {
        return Ok(forbidden(
            &headers,
            "Only the creator can approve attendees",
        ));
    }

    let target_profile_id = Uuid::parse_str(&profile_id_str)
        .map_err(|_| crate::error::AppError::Message("Invalid profile ID".to_string()))?;

    let outcome = events_write_repo::check_capacity_and_upsert(
        event_uuid,
        target_profile_id,
        "going",
        event.max_attendees,
        Some("pending"),
        false,
    )
    .await?;
    match outcome {
        events_write_repo::UpsertOutcome::Full => {
            return Ok(validation_error(&headers, "Event is full"));
        }
        events_write_repo::UpsertOutcome::StatusMismatch => {
            return Ok(super::events_service::validation_error(
                &headers,
                "Attendee is not pending approval",
            ));
        }
        events_write_repo::UpsertOutcome::Accepted(_) => {}
    }

    if let Err(error) = enqueue_chat_membership_sync(&event.id, &target_profile_id, false).await {
        tracing::warn!(
            %error,
            event_id = %event.id,
            profile_id = %target_profile_id,
            "failed to enqueue chat membership sync after approve"
        );
    }

    // Notify the approved user via ntfy
    tokio::spawn(async move {
        notify_event_approval(target_profile_id, &event.title, true).await;
    });

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

pub(in crate::api) async fn event_reject_attendee(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path((id, profile_id_str)): Path<(String, String)>,
) -> Result<Response> {
    let (profile, _user_pid) = match require_auth_profile(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let (event, event_uuid) = match load_event(&headers, &id).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    if event.creator_id != profile.id {
        return Ok(forbidden(&headers, "Only the creator can reject attendees"));
    }

    let target_profile_id = Uuid::parse_str(&profile_id_str)
        .map_err(|_| crate::error::AppError::Message("Invalid profile ID".to_string()))?;

    let deleted = events_write_repo::delete_pending_attendee(event_uuid, target_profile_id).await?;
    if !deleted {
        return Ok(super::events_service::validation_error(
            &headers,
            "Attendee is not pending approval",
        ));
    }

    // Notify the rejected user via ntfy
    tokio::spawn(async move {
        notify_event_approval(target_profile_id, &event.title, false).await;
    });

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

async fn load_event_for_leave(
    headers: &HeaderMap,
    id: &str,
) -> std::result::Result<(Event, Uuid, Profile), Box<Response>> {
    let (profile, _user_pid) = require_auth_profile(headers).await?;
    let (event, event_uuid) = load_event(headers, id).await?;
    if event.creator_id == profile.id {
        return Err(Box::new(forbidden(
            headers,
            "Event creator cannot leave the event",
        )));
    }
    Ok((event, event_uuid, profile))
}

pub(in crate::api) async fn event_leave(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (event, event_uuid, profile) = match load_event_for_leave(&headers, &id).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    let mut conn = crate::db::conn().await?;
    conn.transaction(|conn| {
        Box::pin(async move {
            events_write_repo::delete_event_attendee_with_conn(conn, event_uuid, profile.id)
                .await?;
            delete_event_interaction_with_conn(
                conn,
                profile.id,
                event_uuid,
                EVENT_INTERACTION_JOINED,
            )
            .await?;
            Ok::<(), crate::error::AppError>(())
        })
    })
    .await?;

    if let Err(error) = enqueue_chat_membership_sync(&event.id, &profile.id, true).await {
        tracing::warn!(
            %error,
            event_id = %event.id,
            profile_id = %profile.id,
            "failed to enqueue chat membership sync after leave"
        );
    }

    let data = build_event_response(&event, &profile.id).await?;
    Ok(Json(DataResponse { data }).into_response())
}

pub(in crate::api) async fn event_unsave(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (event, event_uuid, profile) = match load_event_with_profile(&headers, &id).await {
        Ok(data) => data,
        Err(response) => return Ok(*response),
    };

    delete_event_interaction(profile.id, event_uuid, EVENT_INTERACTION_SAVED).await?;

    let data = build_event_response(&event, &profile.id).await?;
    Ok(Json(DataResponse { data }).into_response())
}

async fn notify_event_approval(target_profile_id: Uuid, event_title: &str, approved: bool) {
    use diesel::ExpressionMethods;
    use diesel::QueryDsl;
    use diesel_async::RunQueryDsl;

    let user_id: i32 = {
        let Ok(mut conn) = crate::db::conn().await else {
            return;
        };
        match crate::db::schema::profiles::table
            .filter(crate::db::schema::profiles::id.eq(target_profile_id))
            .select(crate::db::schema::profiles::user_id)
            .first::<i32>(&mut conn)
            .await
        {
            Ok(uid) => uid,
            Err(_) => return,
        }
    };

    let body = if approved {
        format!("Twoje zgloszenie na \"{event_title}\" zostalo zatwierdzone!")
    } else {
        format!("Twoje zgloszenie na \"{event_title}\" zostalo odrzucone.")
    };

    crate::api::chat::push::notify_push(vec![user_id], Uuid::nil(), 0, &body).await;
}
