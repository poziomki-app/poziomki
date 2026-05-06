use diesel_async::AsyncPgConnection;
use uuid::Uuid;

use crate::api::{
    resolve_bio_image_urls, resolve_image_url, resolve_image_urls, resolve_thumbhashes,
    state::{FullProfileResponse, ProfileResponse},
};
use crate::db;
use crate::db::models::profiles::Profile;

use super::profiles_tags_repo::load_profile_tags;

fn decode_profile_images(profile: &Profile) -> Vec<String> {
    profile
        .images
        .as_ref()
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
        .unwrap_or_default()
}

async fn lookup_thumbhash(profile_picture: Option<&String>) -> Option<String> {
    let pic = profile_picture?;
    let map = resolve_thumbhashes(std::slice::from_ref(pic)).await;
    map.into_values().next()
}

async fn resolve_program(
    conn: &mut AsyncPgConnection,
    program: Option<String>,
    viewer_user_id: Option<i32>,
    profile_user_id: i32,
) -> Option<String> {
    if viewer_user_id == Some(profile_user_id) {
        return program;
    }
    // Narrow projection — only returns privacy_show_program, not the full
    // user_settings row. Lets Tier-A policy on user_settings stay "own row
    // only" without hiding public profile fields for every other viewer.
    let show = db::profile_program_visibility(conn, profile_user_id)
        .await
        .unwrap_or(false);
    if show {
        program
    } else {
        None
    }
}

pub(in crate::api) async fn profile_to_response(
    conn: &mut AsyncPgConnection,
    profile: &Profile,
    user_pid: &Uuid,
    viewer_user_id: Option<i32>,
) -> ProfileResponse {
    let profile_picture = match &profile.profile_picture {
        Some(pic) => Some(resolve_image_url(pic).await),
        None => None,
    };

    let thumbhash = lookup_thumbhash(profile.profile_picture.as_ref()).await;

    let raw_images = decode_profile_images(profile);
    let images = resolve_image_urls(&raw_images).await;

    let program = resolve_program(
        conn,
        profile.program.clone(),
        viewer_user_id,
        profile.user_id,
    )
    .await;

    let bio = match &profile.bio {
        Some(b) => Some(resolve_bio_image_urls(b).await),
        None => None,
    };

    ProfileResponse {
        id: profile.id.to_string(),
        user_id: user_pid.to_string(),
        name: profile.name.clone(),
        bio,
        profile_picture,
        thumbhash,
        images,
        program,
        gradient_start: profile.gradient_start.clone(),
        gradient_end: profile.gradient_end.clone(),
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    }
}

pub(in crate::api) async fn full_profile_response(
    conn: &mut AsyncPgConnection,
    profile: &Profile,
    user_pid: &Uuid,
    viewer_user_id: Option<i32>,
) -> std::result::Result<FullProfileResponse, crate::error::AppError> {
    let profile_tags = load_profile_tags(conn, profile.id).await?;

    let profile_picture = match &profile.profile_picture {
        Some(pic) => Some(resolve_image_url(pic).await),
        None => None,
    };

    let thumbhash = lookup_thumbhash(profile.profile_picture.as_ref()).await;

    let raw_images = decode_profile_images(profile);
    let images = resolve_image_urls(&raw_images).await;

    let program = resolve_program(
        conn,
        profile.program.clone(),
        viewer_user_id,
        profile.user_id,
    )
    .await;

    let bio = match &profile.bio {
        Some(b) => Some(resolve_bio_image_urls(b).await),
        None => None,
    };

    Ok(FullProfileResponse {
        id: profile.id.to_string(),
        user_id: user_pid.to_string(),
        name: profile.name.clone(),
        bio,
        profile_picture,
        thumbhash,
        images,
        program,
        gradient_start: profile.gradient_start.clone(),
        gradient_end: profile.gradient_end.clone(),
        tags: profile_tags,
        is_bookmarked: false,
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    })
}
