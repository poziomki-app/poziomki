use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::db::models::event_attendees::EventAttendee;
use crate::db::models::event_interactions::EventInteraction;
use crate::db::models::events::{Event, EventChangeset, NewEvent};
use crate::db::schema::{event_attendees, event_interactions, events, reports};

pub(in crate::api) const MAX_ATTEMPTS: usize = 3;

const fn is_serialization_failure(err: &diesel::result::Error) -> bool {
    matches!(
        err,
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::SerializationFailure,
            _
        )
    )
}

pub(in crate::api) fn is_serialization_failure_app(err: &crate::error::AppError) -> bool {
    match err {
        crate::error::AppError::Any(boxed) => boxed
            .downcast_ref::<diesel::result::Error>()
            .is_some_and(is_serialization_failure),
        _ => false,
    }
}

pub(in crate::api) enum UpsertOutcome {
    /// The upsert succeeded. Contains the status that was actually written,
    /// which may differ from the requested status when the approval gate
    /// downgrades "going" → "pending".
    Accepted(String),
    Full,
    StatusMismatch,
}

/// Atomically check capacity, upsert the attendee, and track the interaction
/// inside a serializable transaction so that two concurrent requests cannot
/// exceed `max_attendees` and the interaction stays consistent.
///
/// When `require_status` is `Some`, the attendee's current status is verified
/// inside the transaction before proceeding, preventing TOCTOU races.
///
/// When `requires_approval` is true and the requested status is "going",
/// the approval gate is evaluated inside the transaction: if the attendee
/// is not already "going", the written status is downgraded to "pending".
/// The actual written status is returned in `UpsertOutcome::Accepted`.
pub(in crate::api) async fn check_capacity_and_upsert(
    event_id: Uuid,
    profile_id: Uuid,
    status: &str,
    max_attendees: Option<i32>,
    require_status: Option<&str>,
    requires_approval: bool,
) -> std::result::Result<UpsertOutcome, crate::error::AppError> {
    let status = status.to_string();
    let require_status = require_status.map(String::from);
    let mut conn = crate::db::conn().await?;

    let mut attempts = 0;
    loop {
        attempts += 1;
        let result = conn
            .build_transaction()
            .serializable()
            .run(|conn| {
                let status = status.clone();
                let require_status = require_status.clone();
                Box::pin(async move {
                    // Read current status inside the txn so retries see fresh state
                    let current_status = event_attendees::table
                        .filter(event_attendees::event_id.eq(event_id))
                        .filter(event_attendees::profile_id.eq(profile_id))
                        .select(event_attendees::status)
                        .first::<String>(conn)
                        .await
                        .optional()?;

                    // Precondition: verify attendee has required status
                    if let Some(required) = &require_status {
                        if current_status.as_deref() != Some(required.as_str()) {
                            return Ok::<UpsertOutcome, diesel::result::Error>(
                                UpsertOutcome::StatusMismatch,
                            );
                        }
                    }

                    let already_going = current_status.as_deref() == Some("going");

                    // Approval gate: downgrade "going" → "pending" unless already going
                    let effective_status =
                        if requires_approval && status == "going" && !already_going {
                            "pending".to_string()
                        } else {
                            status
                        };

                    // Capacity gate: only enforce when switching to "going"
                    if effective_status == "going" && !already_going {
                        if let Some(max) = max_attendees {
                            let current_going = event_attendees::table
                                .filter(event_attendees::event_id.eq(event_id))
                                .filter(event_attendees::status.eq("going"))
                                .count()
                                .get_result::<i64>(conn)
                                .await?;
                            if current_going >= i64::from(max) {
                                return Ok(UpsertOutcome::Full);
                            }
                        }
                    }

                    // Upsert attendee: delete-then-insert
                    diesel::delete(
                        event_attendees::table
                            .filter(event_attendees::event_id.eq(event_id))
                            .filter(event_attendees::profile_id.eq(profile_id)),
                    )
                    .execute(conn)
                    .await?;

                    let attendee = EventAttendee {
                        event_id,
                        profile_id,
                        status: effective_status.clone(),
                    };
                    diesel::insert_into(event_attendees::table)
                        .values(&attendee)
                        .execute(conn)
                        .await?;

                    // Track "joined" interaction atomically with the attendee change
                    if effective_status == "going" {
                        upsert_interaction_with_conn(conn, profile_id, event_id, "joined").await?;
                    } else {
                        delete_interaction_with_conn(conn, profile_id, event_id, "joined").await?;
                    }

                    Ok(UpsertOutcome::Accepted(effective_status))
                })
            })
            .await;
        match result {
            Ok(val) => return Ok(val),
            Err(ref e) if attempts < MAX_ATTEMPTS && is_serialization_failure(e) => {
                tokio::time::sleep(std::time::Duration::from_millis(10u64 << attempts)).await;
            }
            Err(e) => return Err(e.into()),
        }
    }
}

pub(in crate::api) async fn insert_event_with_conn(
    conn: &mut AsyncPgConnection,
    new_event: &NewEvent,
) -> std::result::Result<Event, crate::error::AppError> {
    let inserted = diesel::insert_into(events::table)
        .values(new_event)
        .get_result::<Event>(conn)
        .await?;
    Ok(inserted)
}

pub(in crate::api) async fn update_event_with_conn(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
    changeset: &EventChangeset,
) -> std::result::Result<Event, crate::error::AppError> {
    let updated = diesel::update(events::table.find(event_id))
        .set(changeset)
        .get_result::<Event>(conn)
        .await?;
    Ok(updated)
}

pub(in crate::api) async fn delete_event(
    event_id: Uuid,
) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    diesel::delete(
        reports::table
            .filter(reports::target_type.eq("event"))
            .filter(reports::target_id.eq(event_id)),
    )
    .execute(&mut conn)
    .await?;
    diesel::delete(events::table.find(event_id))
        .execute(&mut conn)
        .await?;
    Ok(())
}

/// Atomically delete an attendee only if their status is "pending".
/// Returns `true` when the row was deleted, `false` when no pending row existed.
pub(in crate::api) async fn delete_pending_attendee(
    event_id: Uuid,
    profile_id: Uuid,
) -> std::result::Result<bool, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let deleted = diesel::delete(
        event_attendees::table
            .filter(event_attendees::event_id.eq(event_id))
            .filter(event_attendees::profile_id.eq(profile_id))
            .filter(event_attendees::status.eq("pending")),
    )
    .execute(&mut conn)
    .await?;
    Ok(deleted > 0)
}

pub(in crate::api) async fn delete_event_attendee_with_conn(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
    profile_id: Uuid,
) -> std::result::Result<(), crate::error::AppError> {
    diesel::delete(
        event_attendees::table
            .filter(event_attendees::event_id.eq(event_id))
            .filter(event_attendees::profile_id.eq(profile_id)),
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Auto-approve pending attendees for an event, respecting `max_attendees`.
/// Must be called inside a serializable transaction to prevent capacity races.
/// Promotion order is deterministic (by UUID) but not FIFO — the table has no
/// `created_at` column, so arrival-time ordering is unavailable.
/// Returns the profile IDs that were promoted so the caller can enqueue
/// chat membership syncs.
pub(in crate::api) async fn auto_approve_pending_with_conn(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
    max_attendees: Option<i32>,
) -> std::result::Result<Vec<Uuid>, crate::error::AppError> {
    let pending_ids: Vec<Uuid> = event_attendees::table
        .filter(event_attendees::event_id.eq(event_id))
        .filter(event_attendees::status.eq("pending"))
        .select(event_attendees::profile_id)
        .order(event_attendees::profile_id.asc())
        .load::<Uuid>(conn)
        .await?;

    if pending_ids.is_empty() {
        return Ok(vec![]);
    }

    let to_approve: Vec<Uuid> = if let Some(max) = max_attendees {
        let current_going = event_attendees::table
            .filter(event_attendees::event_id.eq(event_id))
            .filter(event_attendees::status.eq("going"))
            .count()
            .get_result::<i64>(conn)
            .await?;
        let remaining = usize::try_from((i64::from(max) - current_going).max(0)).unwrap_or(0);
        pending_ids.into_iter().take(remaining).collect()
    } else {
        pending_ids
    };

    if to_approve.is_empty() {
        return Ok(vec![]);
    }

    diesel::update(
        event_attendees::table
            .filter(event_attendees::event_id.eq(event_id))
            .filter(event_attendees::profile_id.eq_any(&to_approve))
            .filter(event_attendees::status.eq("pending")),
    )
    .set(event_attendees::status.eq("going"))
    .execute(conn)
    .await?;

    let now = Utc::now();
    let interactions: Vec<EventInteraction> = to_approve
        .iter()
        .map(|&pid| EventInteraction {
            profile_id: pid,
            event_id,
            kind: "joined".to_string(),
            created_at: now,
            updated_at: now,
        })
        .collect();
    diesel::insert_into(event_interactions::table)
        .values(&interactions)
        .on_conflict((
            event_interactions::profile_id,
            event_interactions::event_id,
            event_interactions::kind,
        ))
        .do_update()
        .set(event_interactions::updated_at.eq(now))
        .execute(conn)
        .await?;

    Ok(to_approve)
}

async fn upsert_interaction_with_conn(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
    event_id: Uuid,
    kind: &str,
) -> std::result::Result<(), diesel::result::Error> {
    let now = Utc::now();
    diesel::insert_into(event_interactions::table)
        .values(EventInteraction {
            profile_id,
            event_id,
            kind: kind.to_string(),
            created_at: now,
            updated_at: now,
        })
        .on_conflict((
            event_interactions::profile_id,
            event_interactions::event_id,
            event_interactions::kind,
        ))
        .do_update()
        .set(event_interactions::updated_at.eq(now))
        .execute(conn)
        .await?;
    Ok(())
}

async fn delete_interaction_with_conn(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
    event_id: Uuid,
    kind: &str,
) -> std::result::Result<(), diesel::result::Error> {
    diesel::delete(
        event_interactions::table
            .filter(event_interactions::profile_id.eq(profile_id))
            .filter(event_interactions::event_id.eq(event_id))
            .filter(event_interactions::kind.eq(kind)),
    )
    .execute(conn)
    .await?;
    Ok(())
}
