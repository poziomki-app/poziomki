use uuid::Uuid;

use crate::api::{
    resolve_image_url, resolve_image_urls,
    state::{FullProfileResponse, ProfileResponse},
};
use crate::db::models::profiles::Profile;

use super::profiles_tags::load_profile_tags;

fn decode_profile_images(profile: &Profile) -> Vec<String> {
    profile
        .images
        .as_ref()
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
        .unwrap_or_default()
}

pub(in crate::api) async fn profile_to_response(
    profile: &Profile,
    user_pid: &Uuid,
) -> ProfileResponse {
    let profile_picture = match &profile.profile_picture {
        Some(pic) => Some(resolve_image_url(pic).await),
        None => None,
    };

    let raw_images = decode_profile_images(profile);
    let images = resolve_image_urls(&raw_images).await;

    ProfileResponse {
        id: profile.id.to_string(),
        user_id: user_pid.to_string(),
        name: profile.name.clone(),
        bio: profile.bio.clone(),
        age: u8::try_from(profile.age).unwrap_or(0),
        profile_picture,
        images,
        program: profile.program.clone(),
        gradient_start: profile.gradient_start.clone(),
        gradient_end: profile.gradient_end.clone(),
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    }
}

pub(in crate::api) async fn full_profile_response(
    profile: &Profile,
    user_pid: &Uuid,
) -> std::result::Result<FullProfileResponse, crate::error::AppError> {
    let profile_tags = load_profile_tags(profile.id).await?;

    let profile_picture = match &profile.profile_picture {
        Some(pic) => Some(resolve_image_url(pic).await),
        None => None,
    };

    let raw_images = decode_profile_images(profile);
    let images = resolve_image_urls(&raw_images).await;

    Ok(FullProfileResponse {
        id: profile.id.to_string(),
        user_id: user_pid.to_string(),
        name: profile.name.clone(),
        bio: profile.bio.clone(),
        age: u8::try_from(profile.age).unwrap_or(0),
        profile_picture,
        images,
        program: profile.program.clone(),
        gradient_start: profile.gradient_start.clone(),
        gradient_end: profile.gradient_end.clone(),
        tags: profile_tags,
        created_at: profile.created_at.to_rfc3339(),
        updated_at: profile.updated_at.to_rfc3339(),
    })
}
