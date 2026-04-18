use std::collections::{HashMap, HashSet};

use crate::api::state::EventResponse;

/// Resolve a set of raw image filenames to signed URLs in parallel, returning a lookup map.
pub async fn resolve_image_map(raw_values: HashSet<String>) -> HashMap<String, String> {
    let resolve = crate::api::resolve_image_url;
    let futs: Vec<_> = raw_values
        .into_iter()
        .map(|raw| async move {
            let resolved = resolve(&raw).await;
            (raw, resolved)
        })
        .collect();
    futures_util::future::join_all(futs)
        .await
        .into_iter()
        .collect()
}

fn collect_image_filenames(responses: &[EventResponse]) -> HashSet<String> {
    let mut filenames = HashSet::new();
    for r in responses {
        filenames.extend(r.cover_image.iter().cloned());
        filenames.extend(r.creator.profile_picture.iter().cloned());
        filenames.extend(
            r.attendees_preview
                .iter()
                .filter_map(|a| a.profile_picture.clone()),
        );
    }
    filenames
}

fn replace_resolved_image(value: &mut Option<String>, url_map: &HashMap<String, String>) {
    if let Some(resolved) = value
        .as_ref()
        .and_then(|raw| url_map.get(raw.as_str()))
        .cloned()
    {
        *value = Some(resolved);
    }
}

/// Resolve all image URLs (cover, creator, attendee previews) in event responses.
pub async fn resolve_event_images(responses: &mut [EventResponse]) {
    let filenames = collect_image_filenames(responses);
    if filenames.is_empty() {
        return;
    }
    let url_map = resolve_image_map(filenames).await;

    for response in responses.iter_mut() {
        replace_resolved_image(&mut response.cover_image, &url_map);
        replace_resolved_image(&mut response.creator.profile_picture, &url_map);
        for preview in &mut response.attendees_preview {
            replace_resolved_image(&mut preview.profile_picture, &url_map);
        }
    }
}
