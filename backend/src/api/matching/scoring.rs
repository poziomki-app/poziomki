use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::db::models::events::Event;
use crate::db::models::profiles::Profile;

/// Haversine distance between two (lat, lng) points in kilometres.
#[allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]
pub(super) fn haversine_km(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const R: f64 = 6_371.0; // Earth radius in km
    let d_lat = (lat2 - lat1).to_radians();
    let d_lng = (lng2 - lng1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lng / 2.0).sin().powi(2);
    2.0 * R * a.sqrt().asin()
}

/// Returns a 0.0-1.0 score: 1.0 when distance is 0, 0.0 at or beyond `max_km`.
pub(super) fn proximity_score(distance_km: f64, max_km: f64) -> f64 {
    if max_km <= 0.0 {
        return 0.0;
    }
    (1.0 - distance_km / max_km).clamp(0.0, 1.0)
}

#[allow(clippy::cast_precision_loss)]
pub(super) fn score_profile(
    my_tag_ids: &HashSet<Uuid>,
    candidate_tags: &HashSet<Uuid>,
    my_program: Option<&str>,
    candidate: &Profile,
    candidate_show_program: bool,
) -> f64 {
    let shared = my_tag_ids.intersection(candidate_tags).count();
    let mut score = if my_tag_ids.is_empty() {
        0.0
    } else {
        (shared as f64 / my_tag_ids.len() as f64) * 100.0
    };
    if candidate_show_program {
        let same_program =
            my_program.is_some_and(|prog| candidate.program.as_deref() == Some(prog));
        if same_program {
            score += 5.0;
        }
    }
    score
}

/// Sort by score DESC, break ties by `created_at` DESC, return top `limit`.
pub(super) fn rank_and_take<'a>(
    scored: &mut [(f64, &'a Profile)],
    limit: usize,
) -> Vec<(f64, &'a Profile)> {
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.1.created_at.cmp(&a.1.created_at))
    });
    scored.iter().copied().take(limit).collect()
}

/// Sort events by score DESC, break ties by `starts_at` ASC, return top `limit`.
pub(super) fn rank_events_and_take<'a>(
    scored: &mut [(f64, &'a Event)],
    limit: usize,
) -> Vec<(f64, &'a Event)> {
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.starts_at.cmp(&b.1.starts_at))
    });
    scored.iter().copied().take(limit).collect()
}

pub(super) fn build_affinity_map<I>(
    source_tags: I,
    tag_parent_map: &HashMap<Uuid, Option<Uuid>>,
) -> HashMap<Uuid, f64>
where
    I: IntoIterator<Item = (Uuid, f64)>,
{
    let mut affinity = HashMap::new();

    for (tag_id, source_weight) in source_tags {
        let mut current = Some(tag_id);
        let mut depth = 0usize;
        let mut seen = HashSet::new();

        while let Some(node_id) = current {
            if !seen.insert(node_id) {
                break;
            }

            let ancestor_weight = match depth {
                0 => 1.0,
                1 => 0.7,
                _ => 0.4,
            };

            let entry = affinity.entry(node_id).or_insert(0.0);
            *entry = source_weight.mul_add(ancestor_weight, *entry);

            current = tag_parent_map.get(&node_id).copied().flatten();
            depth += 1;
        }
    }

    affinity
}

fn best_affinity_for_event_tag(
    tag_id: Uuid,
    affinity_map: &HashMap<Uuid, f64>,
    tag_parent_map: &HashMap<Uuid, Option<Uuid>>,
) -> f64 {
    let mut best = affinity_map.get(&tag_id).copied().unwrap_or(0.0);
    let mut current = tag_parent_map.get(&tag_id).copied().flatten();
    let mut seen = HashSet::from([tag_id]);

    while let Some(node_id) = current {
        if !seen.insert(node_id) {
            break;
        }
        best = best.max(affinity_map.get(&node_id).copied().unwrap_or(0.0));
        current = tag_parent_map.get(&node_id).copied().flatten();
    }

    best.clamp(0.0, 1.0)
}

fn affinity_score(
    affinity_map: &HashMap<Uuid, f64>,
    event_tag_ids: &HashSet<Uuid>,
    tag_parent_map: &HashMap<Uuid, Option<Uuid>>,
) -> f64 {
    if affinity_map.is_empty() || event_tag_ids.is_empty() {
        return 0.0;
    }

    #[allow(clippy::cast_precision_loss)]
    let total = event_tag_ids
        .iter()
        .map(|tag_id| best_affinity_for_event_tag(*tag_id, affinity_map, tag_parent_map))
        .sum::<f64>();

    #[allow(clippy::cast_precision_loss)]
    let average = total / event_tag_ids.len() as f64;
    average * 100.0
}

#[allow(clippy::cast_precision_loss)]
pub(super) fn score_event(
    profile_affinity: &HashMap<Uuid, f64>,
    history_affinity: &HashMap<Uuid, f64>,
    interest_categories: &HashSet<String>,
    event_tag_ids: &HashSet<Uuid>,
    event: &Event,
    user_geo: Option<(f64, f64, f64)>,
    tag_parent_map: &HashMap<Uuid, Option<Uuid>>,
) -> f64 {
    let content_score = affinity_score(profile_affinity, event_tag_ids, tag_parent_map);
    let history_score = affinity_score(history_affinity, event_tag_ids, tag_parent_map);
    let category_bonus = event
        .category
        .as_ref()
        .filter(|category| interest_categories.contains(category.as_str()))
        .map_or(0.0, |_| 12.0);

    if let Some((ulat, ulng, max_km)) = user_geo {
        let geo_bonus = match (event.latitude, event.longitude) {
            (Some(elat), Some(elng)) => {
                proximity_score(haversine_km(ulat, ulng, elat, elng), max_km) * 15.0
            }
            _ => 0.0,
        };
        content_score.mul_add(0.65, history_score * 0.20) + geo_bonus + category_bonus
    } else {
        content_score.mul_add(0.65, history_score * 0.20) + category_bonus
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::unwrap_used, clippy::suboptimal_flops)]
mod tests {
    use super::*;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn recall_based_profile_scoring() {
        let my_tags: HashSet<Uuid> = [id(1), id(2), id(3), id(4), id(5)].into();
        let candidate_3_match: HashSet<Uuid> = [id(1), id(2), id(3), id(6), id(7)].into();
        let candidate_1_match: HashSet<Uuid> = [id(1), id(6)].into();
        let candidate_5_match: HashSet<Uuid> = [id(1), id(2), id(3), id(4), id(5), id(6)].into();

        let make_profile = || Profile {
            id: id(99),
            user_id: 1,
            name: "t".to_string(),
            bio: None,
            profile_picture: None,
            images: None,
            program: None,
            gradient_start: None,
            gradient_end: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let p = make_profile();
        let s3 = score_profile(&my_tags, &candidate_3_match, None, &p, false);
        let s1 = score_profile(&my_tags, &candidate_1_match, None, &p, false);
        let s5 = score_profile(&my_tags, &candidate_5_match, None, &p, false);

        assert!((s3 - 60.0).abs() < f64::EPSILON, "3/5 = 60, got {s3}");
        assert!((s1 - 20.0).abs() < f64::EPSILON, "1/5 = 20, got {s1}");
        assert!((s5 - 100.0).abs() < f64::EPSILON, "5/5 = 100, got {s5}");
        assert!(s5 > s3 && s3 > s1, "more matches = higher score");
    }

    #[test]
    fn program_bonus_does_not_flip_one_tag_difference() {
        let my_tags: HashSet<Uuid> = [id(1), id(2), id(3), id(4), id(5)].into();
        let two_match: HashSet<Uuid> = [id(1), id(2), id(6)].into();
        let one_match: HashSet<Uuid> = [id(1), id(6), id(7)].into();

        let no_prog = Profile {
            id: id(99),
            user_id: 1,
            name: "t".to_string(),
            bio: None,
            profile_picture: None,
            images: None,
            program: None,
            gradient_start: None,
            gradient_end: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let with_prog = Profile {
            program: Some("CS".to_string()),
            ..no_prog.clone()
        };

        let a = score_profile(&my_tags, &two_match, None, &no_prog, false);
        let b = score_profile(&my_tags, &one_match, Some("CS"), &with_prog, true);
        assert!(a > b, "2 match ({a}) should beat 1 match + program ({b})");
    }

    #[test]
    fn haversine_krakow_to_warsaw() {
        let dist = haversine_km(50.06, 19.94, 52.23, 21.01);
        assert!(dist > 240.0 && dist < 260.0, "got {dist}");
    }

    #[test]
    fn proximity_at_zero() {
        assert!((proximity_score(0.0, 10.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn proximity_at_max() {
        assert!((proximity_score(10.0, 10.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn proximity_beyond_max() {
        assert!((proximity_score(15.0, 10.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn event_combined_score_with_geo_max() {
        let tag_score: f64 = 100.0;
        let history_score: f64 = 50.0;
        let geo_bonus = proximity_score(0.0, 20.0) * 15.0;
        let combined = tag_score * 0.65 + history_score * 0.20 + geo_bonus;
        assert!((combined - 90.0).abs() < f64::EPSILON);
    }

    #[test]
    fn affinity_propagates_to_parent_chain() {
        let tag_parent_map =
            HashMap::from([(id(1), None), (id(2), Some(id(1))), (id(3), Some(id(2)))]);

        let affinity = build_affinity_map([(id(3), 1.0)], &tag_parent_map);
        assert_eq!(affinity.get(&id(3)).copied(), Some(1.0));
        assert_eq!(affinity.get(&id(2)).copied(), Some(0.7));
        assert_eq!(affinity.get(&id(1)).copied(), Some(0.4));
    }

    #[test]
    fn event_score_matches_parent_interest() {
        let tag_parent_map = HashMap::from([(id(1), None), (id(2), Some(id(1)))]);
        let profile_affinity = build_affinity_map([(id(1), 1.0)], &tag_parent_map);
        let history_affinity = HashMap::new();
        let event_tags: HashSet<Uuid> = [id(2)].into();
        let event = Event {
            id: id(10),
            title: "Test".to_string(),
            description: None,
            cover_image: None,
            category: None,
            location: None,
            starts_at: chrono::Utc::now(),
            ends_at: None,
            creator_id: id(20),
            conversation_id: None,
            latitude: None,
            longitude: None,
            max_attendees: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            requires_approval: false,
            recurrence_rule: None,
            visibility: "public".to_string(),
        };

        let score = score_event(
            &profile_affinity,
            &history_affinity,
            &HashSet::new(),
            &event_tags,
            &event,
            None,
            &tag_parent_map,
        );
        assert!(score > 25.0, "got {score}");
    }

    #[test]
    fn sort_with_nan_does_not_panic() {
        let mut scored = [(f64::NAN, "a"), (50.0, "b"), (f64::NAN, "c")];
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        assert_eq!(scored.len(), 3);
    }
}
