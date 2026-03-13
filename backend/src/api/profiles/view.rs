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

/// Looks up the thumbhash for a single profile picture URL, if available.
///
/// Given an optional profile-picture URL, queries the thumbhash resolver and
/// returns the first thumbhash found for that URL.
///
/// # Examples
///
/// ```
/// // Using an async runtime:
/// // let thumb = tokio::runtime::Runtime::new().unwrap().block_on(async {
/// //     lookup_thumbhash(None).await
/// // });
/// // assert_eq!(thumb, None);
/// ```
async fn lookup_thumbhash(profile_picture: Option<&String>) -> Option<String> {
    let pic = profile_picture?;
    let map = resolve_thumbhashes(std::slice::from_ref(pic)).await;
    map.into_values().next()
}

/// Determine whether a profile's `program` field should be visible to the viewer.
///
/// If the viewer is the profile owner, the provided `program` is returned unchanged. For other viewers,
/// the owner's `UserSetting.privacy_show_program` is consulted: if the setting is present and `true`,
/// or if there is no setting record, the `program` is returned; if the setting is present and `false`,
/// or if a database access error occurs, `None` is returned.
///
/// # Returns
///
/// `Some(program)` if the program is permitted to be shown to the viewer, `None` otherwise.
///
/// # Examples
///
/// ```rust,no_run
/// # use futures::executor::block_on;
/// // In an async context:
/// // let visible = resolve_program(Some("Artist".to_string()), Some(42), 43).await;
/// // assert!(visible.is_none() || visible == Some("Artist".to_string()));
/// ```
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

/// Builds a ProfileResponse for a profile as seen by a specific viewer.
///
/// The returned response contains resolved image URLs, an optional thumbhash for the profile
/// picture, and the profile's `program` field filtered according to the viewer's privacy
/// settings (the owner always sees their own program).
///
/// # Examples
///
/// ```
/// # async fn example() {
/// // given `profile: &Profile`, `user_pid: &Uuid`, and `viewer_user_id: Option<i32>`
/// let resp = crate::api::profiles::view::profile_to_response(profile, user_pid, viewer_user_id).await;
/// assert_eq!(resp.id, profile.id.to_string());
/// # }
/// ```
pub(in crate::api) async fn profile_to_response(
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

/// Builds a complete FullProfileResponse for a profile, resolving images, thumbhash, tags, and program visibility.
///
/// Loads the profile's tags and returns a `FullProfileResponse` with resolved image URLs, optional thumbhash,
/// and the program field filtered according to the viewer's privacy settings. Returns an `AppError` if loading
/// the profile tags fails.
///
/// # Parameters
/// - `profile`: the source `Profile` model to convert.
/// - `user_pid`: the profile owner's UUID as a `Uuid`.
/// - `viewer_user_id`: optional viewer user id used to determine program visibility.
///
/// # Returns
/// `Ok(FullProfileResponse)` with all response fields populated on success; `Err(AppError)` if tag loading fails.
///
/// # Examples
///
/// ```
/// # async fn example_call() -> Result<(), crate::error::AppError> {
/// // assume `profile`, `user_pid`, and `viewer_user_id` are available in scope
/// // let profile: Profile = ...;
/// // let user_pid: Uuid = ...;
/// // let viewer_user_id: Option<i32> = Some(1);
/// let resp = full_profile_response(&profile, &user_pid, viewer_user_id).await?;
/// println!("{}", resp.id);
/// # Ok(())
/// # }
/// ```
pub(in crate::api) async fn full_profile_response(
    profile: &Profile,
    user_pid: &Uuid,
    viewer_user_id: Option<i32>,
) -> std::result::Result<FullProfileResponse, crate::error::AppError> {
    let profile_tags = load_profile_tags(profile.id).await?;

    let profile_picture = match &profile.profile_picture {
        Some(pic) => Some(resolve_image_url(pic).await),
        None => None,
    };

    let thumbhash = lookup_thumbhash(profile.profile_picture.as_ref()).await;

    let raw_images = decode_profile_images(profile);
    let images = resolve_image_urls(&raw_images).await;

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
