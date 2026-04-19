use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::db::models::event_attendees::EventAttendee;
use crate::db::models::event_interactions::EventInteraction;
use crate::db::models::events::{Event, EventChangeset, NewEvent};
use crate::db::schema::{event_attendees, event_interactions, events};

pub(in crate::api) const MAX_ATTEMPTS: usize = 3;

pub(in crate::api) const fn is_serialization_failure(err: &diesel::result::Error) -> bool {
    matches!(
        err,
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::SerializationFailure,
            _
        )
    )
}

pub(in crate::api) enum UpsertOutcome {
    /// The upsert succeeded. Contains the status that was actually written,
    /// which may differ from the requested status when the approval gate
    /// downgrades "going" → "pending".
    Accepted(String),
    Full,
    StatusMismatch,
}

/// Body of the capacity/approval check. Caller is responsible for running
/// this inside a serializable transaction with the viewer context set.
pub(in crate::api) async fn check_capacity_and_upsert_with_conn(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
    profile_id: Uuid,
    status: &str,
    max_attendees: Option<i32>,
    require_status: Option<&str>,
    requires_approval: bool,
) -> std::result::Result<UpsertOutcome, diesel::result::Error> {
    // Read current status inside the txn so retries see fresh state
    let current_status = event_attendees::table
        .filter(event_attendees::event_id.eq(event_id))
        .filter(event_attendees::profile_id.eq(profile_id))
        .select(event_attendees::status)
        .first::<String>(conn)
        .await
        .optional()?;

    if let Some(required) = require_status {
        if current_status.as_deref() != Some(required) {
            return Ok(UpsertOutcome::StatusMismatch);
        }
    }

    let already_going = current_status.as_deref() == Some("going");

    let effective_status = if requires_approval && status == "going" && !already_going {
        "pending".to_string()
    } else {
        status.to_string()
    };

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

    if effective_status == "going" {
        upsert_interaction_with_conn(conn, profile_id, event_id, "joined").await?;
    } else {
        delete_interaction_with_conn(conn, profile_id, event_id, "joined").await?;
    }

    Ok(UpsertOutcome::Accepted(effective_status))
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

/// Delete an event plus its reports and chat conversation. Caller must
/// supply a connection already inside a transaction.
pub(in crate::api) async fn delete_event_with_conn(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
) -> std::result::Result<(), crate::error::AppError> {
    // Route through the SD helper installed by Tier-B migration so
    // the API role doesn't need broad DELETE on conversations. The
    // helper auth-checks (event creator or BYPASSRLS), deletes
    // reports, then deletes the event — FK cascades handle the chat
    // fan-out (conversations → conversation_members → messages →
    // message_reactions).
    diesel::sql_query("SELECT app.delete_event_and_chat($1)")
        .bind::<diesel::sql_types::Uuid, _>(event_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// Atomically delete an attendee only if their status is "pending".
/// Returns `true` when the row was deleted, `false` when no pending row existed.
pub(in crate::api) async fn delete_pending_attendee_with_conn(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
    profile_id: Uuid,
) -> std::result::Result<bool, crate::error::AppError> {
    let deleted = diesel::delete(
        event_attendees::table
            .filter(event_attendees::event_id.eq(event_id))
            .filter(event_attendees::profile_id.eq(profile_id))
            .filter(event_attendees::status.eq("pending")),
    )
    .execute(conn)
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
