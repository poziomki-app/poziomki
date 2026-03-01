use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::events_view_images::resolve_image_map;
use super::events_view_repo::{load_attendee_rows, AttendeeRow};
use crate::api::state::AttendeeFullInfo;
use crate::db::models::users::User;
use crate::db::schema::users;

async fn load_users_for_profiles(
    rows: &[AttendeeRow],
    conn: &mut crate::db::DbConn,
) -> std::result::Result<Vec<User>, crate::error::AppError> {
    let user_ids: Vec<i32> = rows.iter().map(|r| r.profile.user_id).collect();
    if user_ids.is_empty() {
        return Ok(vec![]);
    }
    Ok(users::table
        .filter(users::id.eq_any(&user_ids))
        .load::<User>(conn)
        .await?)
}

fn build_attendee_info(
    row: &AttendeeRow,
    user_models: &[User],
    url_map: &HashMap<String, String>,
) -> AttendeeFullInfo {
    let user_pid = user_models
        .iter()
        .find(|u| u.id == row.profile.user_id)
        .map_or(uuid::Uuid::nil(), |u| u.pid);
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
    }
}

pub(in crate::api) async fn attendee_info(
    event_id: Uuid,
) -> std::result::Result<Vec<AttendeeFullInfo>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let rows = load_attendee_rows(event_id, &mut conn).await?;
    let user_models = load_users_for_profiles(&rows, &mut conn).await?;

    let filenames = rows
        .iter()
        .filter_map(|r| r.profile.profile_picture.clone())
        .collect();
    let url_map = resolve_image_map(filenames).await;

    Ok(rows
        .iter()
        .map(|row| build_attendee_info(row, &user_models, &url_map))
        .collect())
}
