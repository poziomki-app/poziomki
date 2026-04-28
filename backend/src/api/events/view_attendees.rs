use std::collections::{HashMap, HashSet};

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use super::events_view_images::resolve_image_map;
use super::events_view_repo::{load_attendee_rows, AttendeeRow};
use crate::api::state::AttendeeFullInfo;
use crate::db;
use crate::db::schema::{profile_blocks, profiles};

/// Resolve `profile.user_id -> user.pid` for a set of attendee profiles via
/// the narrow `app.user_pids_for_ids` SECURITY DEFINER helper. The API role
/// does not hold broad SELECT on `users`; this helper returns only the
/// `(id, pid)` tuples needed to render attendee identifiers.
async fn load_user_pids(
    conn: &mut AsyncPgConnection,
    rows: &[AttendeeRow],
) -> std::result::Result<HashMap<i32, Uuid>, crate::error::AppError> {
    let user_ids: Vec<i32> = rows.iter().map(|r| r.profile.user_id).collect();
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let pairs = db::user_pids_for_ids(conn, &user_ids).await?;
    Ok(pairs
        .into_iter()
        .map(|row| (row.user_id, row.pid))
        .collect())
}

fn build_attendee_info(
    row: &AttendeeRow,
    user_pids: &HashMap<i32, Uuid>,
    url_map: &HashMap<String, String>,
    creator_id: Uuid,
) -> AttendeeFullInfo {
    let user_pid = user_pids
        .get(&row.profile.user_id)
        .copied()
        .unwrap_or_else(Uuid::nil);
    let profile_picture = row
        .profile
        .profile_picture
        .as_ref()
        .and_then(|raw| url_map.get(raw.as_str()))
        .cloned();
    AttendeeFullInfo {
        id: row.profile.id.to_string(),
        user_id: user_pid.to_string(),
        name: row.profile.name.clone(),
        profile_picture,
        status: row.status,
        is_creator: row.profile.id == creator_id,
    }
}

/// Resolve the set of profile ids that are on either side of a block with the
/// viewer. Mirrors `api::matching::repo::load_candidate_profiles` so that
/// attendee listings hide blocked users in both directions.
async fn load_blocked_profile_ids(
    conn: &mut AsyncPgConnection,
    viewer_user_id: i32,
) -> std::result::Result<HashSet<Uuid>, crate::error::AppError> {
    let viewer_profile_ids: Vec<Uuid> = profiles::table
        .filter(profiles::user_id.eq(viewer_user_id))
        .select(profiles::id)
        .load(conn)
        .await?;

    if viewer_profile_ids.is_empty() {
        return Ok(HashSet::new());
    }

    let rows: Vec<(Uuid, Uuid)> = profile_blocks::table
        .filter(
            profile_blocks::blocker_id
                .eq_any(&viewer_profile_ids)
                .or(profile_blocks::blocked_id.eq_any(&viewer_profile_ids)),
        )
        .select((profile_blocks::blocker_id, profile_blocks::blocked_id))
        .load(conn)
        .await?;

    Ok(rows
        .into_iter()
        .flat_map(|(a, b)| {
            let mut out = Vec::with_capacity(2);
            if !viewer_profile_ids.contains(&a) {
                out.push(a);
            }
            if !viewer_profile_ids.contains(&b) {
                out.push(b);
            }
            out
        })
        .collect())
}

/// Collect attendee rows + the viewer-facing pid/name/picture tuples required
/// to render them. Must run inside an existing viewer-scoped transaction.
///
/// Filters out attendees on either side of a block with `viewer_user_id`. The
/// raw `event_attendees` table has no block awareness, and RLS on profiles is
/// bucket-only, so without this filter blocked users leak through the
/// attendee list (matching/search/chat already filter — this closes the gap).
pub(in crate::api) async fn attendee_info(
    conn: &mut AsyncPgConnection,
    event_id: Uuid,
    creator_id: Uuid,
    viewer_user_id: i32,
) -> std::result::Result<Vec<AttendeeFullInfo>, crate::error::AppError> {
    let mut rows = load_attendee_rows(conn, event_id).await?;
    let blocked = load_blocked_profile_ids(conn, viewer_user_id).await?;
    if !blocked.is_empty() {
        rows.retain(|row| !blocked.contains(&row.profile.id));
    }
    let user_pids = load_user_pids(conn, &rows).await?;

    let filenames = rows
        .iter()
        .filter_map(|r| r.profile.profile_picture.clone())
        .collect();
    let url_map = resolve_image_map(filenames).await;

    let mut list: Vec<AttendeeFullInfo> = rows
        .iter()
        .map(|row| build_attendee_info(row, &user_pids, &url_map, creator_id))
        .collect();
    list.sort_by(|a, b| {
        b.is_creator
            .cmp(&a.is_creator)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(list)
}
