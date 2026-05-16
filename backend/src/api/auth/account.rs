#[path = "export_queries.rs"]
mod auth_export_queries;

use std::io::Write;

use axum::http::header;
use axum::response::Response;
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;

use crate::api::{auth_or_respond, error_response, ErrorSpec};

use super::super::state::{
    invalidate_auth_cache_for_user_id, ChangePasswordBody, DataResponse, DeleteAccountBody,
    SuccessResponse,
};
type Result<T> = crate::error::AppResult<T>;

/// Password re-verification failed on a session-authenticated endpoint
/// (change password, delete account). Returns 403 rather than 401 so the
/// mobile client's generic 401 handler doesn't clear the session — the
/// user is still authenticated; they just typed the wrong password.
fn invalid_password_error(headers: &HeaderMap) -> Response {
    error_response(
        axum::http::StatusCode::FORBIDDEN,
        headers,
        ErrorSpec {
            error: "Invalid password".to_string(),
            code: "INVALID_PASSWORD",
            details: None,
        },
    )
}

use crate::app::AppContext;
use crate::db::models::profiles::Profile;
use crate::db::models::user_audit_log::NewUserAuditLog;
use crate::db::schema::{
    conversations, event_attendees, events, profile_tags, profiles, sessions, uploads,
    user_audit_log, user_settings, users,
};
use crate::db::{self, DbViewer};

/// Profile-scoped data for the export endpoint. Collected in one shot so the
/// export handler can stay linear instead of branching on `profile_id` at
/// every query.
struct ProfileExport {
    tags: Vec<serde_json::Value>,
    created: Vec<serde_json::Value>,
    attended: Vec<serde_json::Value>,
    interactions: Vec<serde_json::Value>,
    uploads_list: Vec<serde_json::Value>,
    filenames: Vec<String>,
}

impl ProfileExport {
    const fn empty() -> Self {
        Self {
            tags: Vec::new(),
            created: Vec::new(),
            attended: Vec::new(),
            interactions: Vec::new(),
            uploads_list: Vec::new(),
            filenames: Vec::new(),
        }
    }
}

/// Load every profile-scoped collection that goes into the export payload.
/// Returns empty vectors when the caller has no profile — lets the handler
/// treat both cases uniformly.
async fn load_profile_scoped_export(
    conn: &mut diesel_async::AsyncPgConnection,
    profile_id: Option<uuid::Uuid>,
) -> std::result::Result<ProfileExport, diesel::result::Error> {
    let Some(pid) = profile_id else {
        return Ok(ProfileExport::empty());
    };
    let rollback = |_| diesel::result::Error::RollbackTransaction;
    let tags = auth_export_queries::load_user_tags(conn, pid)
        .await
        .map_err(rollback)?;
    let created = auth_export_queries::load_created_events(conn, pid)
        .await
        .map_err(rollback)?;
    let attended = auth_export_queries::load_attended_events(conn, pid)
        .await
        .map_err(rollback)?;
    let interactions = auth_export_queries::load_event_interactions(conn, pid)
        .await
        .map_err(rollback)?;
    let uploads_list = auth_export_queries::load_user_uploads(conn, pid)
        .await
        .map_err(rollback)?;
    let filenames = auth_export_queries::load_upload_filenames(conn, pid)
        .await
        .map_err(rollback)?;
    Ok(ProfileExport {
        tags,
        created,
        attended,
        interactions,
        uploads_list,
        filenames,
    })
}

/// Record a GDPR-relevant action on an account (deletion, export, password
/// change). Best-effort: logs a warning on failure but never blocks the
/// originating operation. Runs under the caller's viewer context so the
/// insert is attributable once RLS policies are enabled.
async fn write_audit(viewer: DbViewer, user_pid: uuid::Uuid, action: &'static str) {
    let entry = NewUserAuditLog {
        id: uuid::Uuid::new_v4(),
        user_pid,
        action: action.to_string(),
        created_at: Utc::now(),
    };
    let result = db::with_viewer_tx(viewer, |conn| {
        async move {
            diesel::insert_into(user_audit_log::table)
                .values(&entry)
                .execute(conn)
                .await?;
            Ok::<(), diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await;
    if let Err(e) = result {
        tracing::warn!(action, error = %e, "audit log insert failed");
    }
}

async fn delete_user_data(viewer: DbViewer) -> std::result::Result<(), crate::error::AppError> {
    let user_id = viewer.user_id;

    // Collect upload filenames before the transaction so the outer-scope S3
    // cleanup can delete them even though the rows get wiped.
    let upload_filenames = db::with_viewer_tx(viewer, |conn| {
        async move {
            let profile_id: Option<uuid::Uuid> = profiles::table
                .filter(profiles::user_id.eq(user_id))
                .select(profiles::id)
                .first(conn)
                .await
                .optional()?;
            let Some(pid) = profile_id else {
                return Ok::<Vec<String>, diesel::result::Error>(Vec::new());
            };
            let files: Vec<String> = uploads::table
                .filter(uploads::owner_id.eq(pid))
                .select(uploads::filename)
                .load(conn)
                .await?;
            Ok(files)
        }
        .scope_boxed()
    })
    .await?;

    db::with_viewer_tx(viewer, |conn| {
        async move {
            let profile_id: Option<uuid::Uuid> = profiles::table
                .filter(profiles::user_id.eq(user_id))
                .select(profiles::id)
                .first(conn)
                .await
                .optional()?;

            if let Some(pid) = profile_id {
                // Collect upload IDs before removing the FK reference
                let upload_ids: Vec<uuid::Uuid> = uploads::table
                    .filter(uploads::owner_id.eq(pid))
                    .select(uploads::id)
                    .load(conn)
                    .await?;

                // Break uploads → profiles FK (owner_id is nullable)
                diesel::update(uploads::table.filter(uploads::owner_id.eq(pid)))
                    .set(uploads::owner_id.eq(None::<uuid::Uuid>))
                    .execute(conn)
                    .await?;

                // Remove user from events they attend (no CASCADE on profile_id)
                diesel::delete(event_attendees::table.filter(event_attendees::profile_id.eq(pid)))
                    .execute(conn)
                    .await?;

                // Remove profile tags (no CASCADE)
                diesel::delete(profile_tags::table.filter(profile_tags::profile_id.eq(pid)))
                    .execute(conn)
                    .await?;

                // Delete DM conversations (cascades → conv_members, messages, reactions)
                diesel::delete(
                    conversations::table
                        .filter(conversations::kind.eq("dm"))
                        .filter(
                            conversations::user_low_id
                                .eq(user_id)
                                .or(conversations::user_high_id.eq(user_id)),
                        ),
                )
                .execute(conn)
                .await?;

                // Delete user's events (cascades → attendees, tags, interactions,
                // feedback, event conversations + their messages)
                diesel::delete(events::table.filter(events::creator_id.eq(pid)))
                    .execute(conn)
                    .await?;

                // Delete user settings (no CASCADE)
                diesel::delete(user_settings::table.filter(user_settings::user_id.eq(user_id)))
                    .execute(conn)
                    .await?;

                // Delete profile (cascades → event_interactions, reports)
                diesel::delete(profiles::table.filter(profiles::user_id.eq(user_id)))
                    .execute(conn)
                    .await?;

                // Delete sessions
                diesel::delete(sessions::table.filter(sessions::user_id.eq(user_id)))
                    .execute(conn)
                    .await?;

                // Delete user (cascades → conv_members, messages, reactions, push_subscriptions)
                diesel::delete(users::table.find(user_id))
                    .execute(conn)
                    .await?;

                // Clean up orphaned upload rows (safe: all message FK refs are gone)
                if !upload_ids.is_empty() {
                    diesel::delete(uploads::table.filter(uploads::id.eq_any(&upload_ids)))
                        .execute(conn)
                        .await?;
                }
            } else {
                // No profile — clean up user-level data only
                diesel::delete(
                    conversations::table
                        .filter(conversations::kind.eq("dm"))
                        .filter(
                            conversations::user_low_id
                                .eq(user_id)
                                .or(conversations::user_high_id.eq(user_id)),
                        ),
                )
                .execute(conn)
                .await?;

                diesel::delete(user_settings::table.filter(user_settings::user_id.eq(user_id)))
                    .execute(conn)
                    .await?;

                diesel::delete(sessions::table.filter(sessions::user_id.eq(user_id)))
                    .execute(conn)
                    .await?;

                diesel::delete(users::table.find(user_id))
                    .execute(conn)
                    .await?;
            }

            Ok::<(), diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await?;

    // Best-effort S3 cleanup (outside transaction — external side-effect)
    for filename in &upload_filenames {
        crate::api::uploads::delete_upload_objects(filename).await;
    }

    Ok(())
}

pub(in crate::api) async fn delete_account(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<DeleteAccountBody>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);

    if payload.password.is_empty()
        || !crate::security::verify_password(&payload.password, &user.password)
    {
        return Ok(invalid_password_error(&headers));
    }

    let viewer = DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };

    // Record deletion request before wiping the user record so the audit
    // entry survives the cascade.
    write_audit(viewer, user.pid, "account_delete").await;
    delete_user_data(viewer).await?;
    invalidate_auth_cache_for_user_id(user.id).await;

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

pub(in crate::api) async fn change_password(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<ChangePasswordBody>,
) -> Result<Response> {
    let (session, user) = auth_or_respond!(headers);

    if payload.current_password.is_empty()
        || !crate::security::verify_password(&payload.current_password, &user.password)
    {
        return Ok(invalid_password_error(&headers));
    }

    if !(8..=128).contains(&payload.new_password.len()) {
        return Ok(error_response(
            axum::http::StatusCode::BAD_REQUEST,
            &headers,
            crate::api::ErrorSpec {
                error: "Password must be between 8 and 128 characters".to_string(),
                code: "VALIDATION_ERROR",
                details: None,
            },
        ));
    }

    let new_hash = crate::security::hash_password(&payload.new_password)?;
    let viewer = DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };

    db::with_viewer_tx(viewer, |conn| {
        let new_hash = new_hash.clone();
        async move {
            diesel::update(users::table.find(user.id))
                .set((
                    users::password.eq(new_hash),
                    users::updated_at.eq(Utc::now()),
                ))
                .execute(conn)
                .await?;
            // Keep the caller's session active so changing their password
            // doesn't kick them out of the device they just confirmed it on.
            // Every OTHER session for the user is invalidated.
            diesel::delete(
                sessions::table
                    .filter(sessions::user_id.eq(user.id))
                    .filter(sessions::id.ne(session.id)),
            )
            .execute(conn)
            .await?;
            Ok::<(), diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await?;

    invalidate_auth_cache_for_user_id(user.id).await;
    write_audit(viewer, user.pid, "password_change").await;

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

pub(in crate::api) async fn export_data(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);
    let viewer = DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;
    let user_pid_str = user.pid.to_string();

    // Run every export read inside one viewer-scoped transaction so future
    // RLS policies scope every SELECT to this user's own rows.
    let (
        profile,
        tags,
        created_events,
        attended_events,
        event_interactions,
        user_uploads,
        user_sessions,
        settings,
        upload_filenames,
    ) = db::with_viewer_tx(viewer, |conn| {
        async move {
            let profile = profiles::table
                .filter(profiles::user_id.eq(user_id))
                .first::<Profile>(conn)
                .await
                .optional()?;
            let profile_id = profile.as_ref().map(|p| p.id);

            let ProfileExport {
                tags,
                created,
                attended,
                interactions,
                uploads_list,
                filenames,
            } = load_profile_scoped_export(conn, profile_id).await?;

            let us = auth_export_queries::load_user_sessions(conn, user_id)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            let set = auth_export_queries::load_user_settings(conn, user_id)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            Ok::<_, diesel::result::Error>((
                profile,
                tags,
                created,
                attended,
                interactions,
                uploads_list,
                us,
                set,
                filenames,
            ))
        }
        .scope_boxed()
    })
    .await?;

    let profile_view: Option<serde_json::Value> = profile.map(|p| {
        serde_json::json!({
            "id": p.id.to_string(),
            "userId": user_pid_str,
            "name": p.name,
            "bio": p.bio,
            "profilePicture": p.profile_picture,
            "images": p.images,
            "program": p.program,
            "createdAt": p.created_at.to_rfc3339(),
            "updatedAt": p.updated_at.to_rfc3339(),
        })
    });

    let export = serde_json::json!({
        "user": {
            "id": user.pid.to_string(),
            "email": user.email,
            "name": user.name,
            "emailVerified": user.email_verified_at.is_some(),
            "createdAt": user.created_at.to_rfc3339(),
        },
        "profile": profile_view,
        "tags": tags,
        "events": created_events,
        "eventsAttended": attended_events,
        "eventInteractions": event_interactions,
        "uploads": user_uploads,
        "sessions": user_sessions,
        "settings": settings,
        "exportedAt": Utc::now().to_rfc3339(),
    });

    let mut image_files: Vec<(String, Vec<u8>)> = Vec::new();
    for filename in &upload_filenames {
        if let Ok(bytes) = crate::api::uploads::read_upload_bytes(filename).await {
            image_files.push((filename.clone(), bytes));
        }
    }

    // Build ZIP archive
    let json_bytes = serde_json::to_vec_pretty(&export)?;
    let cursor = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("data.json", options)
        .map_err(|e| crate::error::AppError::message(format!("zip error: {e}")))?;
    zip.write_all(&json_bytes)
        .map_err(|e| crate::error::AppError::message(format!("zip write error: {e}")))?;

    for (filename, bytes) in &image_files {
        if zip
            .start_file(format!("images/{filename}"), options)
            .is_ok()
        {
            let _ = zip.write_all(bytes);
        }
    }

    let cursor = zip
        .finish()
        .map_err(|e| crate::error::AppError::message(format!("zip finish error: {e}")))?;
    let zip_bytes = cursor.into_inner();

    write_audit(viewer, user.pid, "data_export").await;

    Ok((
        [
            (header::CONTENT_TYPE, "application/zip"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"poziomki-export.zip\"",
            ),
        ],
        zip_bytes,
    )
        .into_response())
}
