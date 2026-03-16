use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::api::{
    resolve_image_url, resolve_image_urls, resolve_thumbhashes,
    state::{FullProfileResponse, ProfileResponse},
};
use crate::db::models::profiles::Profile;
use crate::db::models::user_settings::UserSetting;
use crate::db::schema::user_settings;

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
    program: Option<String>,
    viewer_user_id: Option<i32>,
    profile_user_id: i32,
) -> Option<String> {
    if viewer_user_id == Some(profile_user_id) {
        return program;
    }
    let show = {
        let Ok(mut conn) = crate::db::conn().await else {
            return None;
        };
        user_settings::table
            .filter(user_settings::user_id.eq(profile_user_id))
            .first::<UserSetting>(&mut conn)
            .await
            .optional()
            .map(|opt| opt.is_none_or(|s| s.privacy_show_program))
            .unwrap_or(false)
    };
    if show {
        program
    } else {
        None
    }
}

pub(in crate::api) async fn profile_to_response(
    profile: &Profile,
    user_pid: &Uuid,
    viewer_user_id: Option<i32>,
    format: &str,
) -> ProfileResponse {
    let profile_picture = match &profile.profile_picture {
        Some(pic) => Some(resolve_image_url(pic, format).await),
        None => None,
    };

    let thumbhash = lookup_thumbhash(profile.profile_picture.as_ref()).await;

    let raw_images = decode_profile_images(profile);
    let images = resolve_image_urls(&raw_images, format).await;

    let program = resolve_program(profile.program.clone(), viewer_user_id, profile.user_id).await;

    ProfileResponse {
        id: profile.id.to_string(),
        user_id: user_pid.to_string(),
        name: profile.name.clone(),
        bio: profile.bio.clone(),
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
    profile: &Profile,
    user_pid: &Uuid,
    viewer_user_id: Option<i32>,
    format: &str,
) -> std::result::Result<FullProfileResponse, crate::error::AppError> {
    let profile_tags = load_profile_tags(profile.id).await?;

    let profile_picture = match &profile.profile_picture {
        Some(pic) => Some(resolve_image_url(pic, format).await),
        None => None,
    };

    let thumbhash = lookup_thumbhash(profile.profile_picture.as_ref()).await;

    let raw_images = decode_profile_images(profile);
    let images = resolve_image_urls(&raw_images, format).await;

    let program = resolve_program(profile.program.clone(), viewer_user_id, profile.user_id).await;

    Ok(FullProfileResponse {
        id: profile.id.to_string(),
        user_id: user_pid.to_string(),
        name: profile.name.clone(),
        bio: profile.bio.clone(),
        profile_picture,
        thumbhash,
        images,
        program,
        gradient_start: profile.gradient_start.clone(),
        gradient_end: profile.gradient_end.clone(),
        tags: profile_tags,
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    })
}
