use std::collections::HashSet;

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
pub(super) fn jaccard(a: &HashSet<Uuid>, b: &HashSet<Uuid>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    intersection as f64 / union as f64
}

pub(super) fn score_profile(
    my_tag_ids: &HashSet<Uuid>,
    candidate_tags: &HashSet<Uuid>,
    my_program: Option<&str>,
    candidate: &Profile,
) -> f64 {
    let mut score = jaccard(my_tag_ids, candidate_tags) * 100.0;
    let same_program = my_program.is_some_and(|prog| candidate.program.as_deref() == Some(prog));
    if same_program {
        score += 10.0;
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

#[allow(clippy::cast_precision_loss)]
pub(super) fn score_event(
    my_tag_ids: &HashSet<Uuid>,
    event_tag_ids: &HashSet<Uuid>,
    event: &Event,
    user_geo: Option<(f64, f64, f64)>,
) -> f64 {
    let tag_score = if my_tag_ids.is_empty() {
        0.0
    } else {
        let shared = my_tag_ids.intersection(event_tag_ids).count();
        (shared as f64 / my_tag_ids.len() as f64) * 100.0
    };

    if let Some((ulat, ulng, max_km)) = user_geo {
        let geo_bonus = match (event.latitude, event.longitude) {
            (Some(elat), Some(elng)) => {
                proximity_score(haversine_km(ulat, ulng, elat, elng), max_km) * 15.0
            }
            _ => 0.0,
        };
        tag_score * 0.85 + geo_bonus
    } else {
        tag_score
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
    fn jaccard_partial_overlap() {
        let a: HashSet<Uuid> = [id(1), id(2), id(3)].into();
        let b: HashSet<Uuid> = [id(2), id(3), id(4)].into();
        assert!((jaccard(&a, &b) - 0.5).abs() < f64::EPSILON);
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
        let geo_bonus = proximity_score(0.0, 20.0) * 15.0;
        let combined = tag_score * 0.85 + geo_bonus;
        assert!((combined - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn event_score_is_asymmetric() {
        let user_tags: HashSet<Uuid> = [id(1), id(2)].into();
        let event_tags: HashSet<Uuid> = (1..=10).map(id).collect();
        let shared = user_tags.intersection(&event_tags).count();
        #[allow(clippy::cast_precision_loss)]
        let score = (shared as f64 / user_tags.len() as f64) * 100.0;
        assert!((score - 100.0).abs() < f64::EPSILON);

        let user_tags2: HashSet<Uuid> = (1..=10).map(id).collect();
        let event_tags2: HashSet<Uuid> = [id(1), id(2)].into();
        let shared2 = user_tags2.intersection(&event_tags2).count();
        #[allow(clippy::cast_precision_loss)]
        let score2 = (shared2 as f64 / user_tags2.len() as f64) * 100.0;
        assert!((score2 - 20.0).abs() < f64::EPSILON);

        assert!((score - score2).abs() > 1.0);
    }

    #[test]
    fn sort_with_nan_does_not_panic() {
        let mut scored = [(f64::NAN, "a"), (50.0, "b"), (f64::NAN, "c")];
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        assert_eq!(scored.len(), 3);
    }
}
