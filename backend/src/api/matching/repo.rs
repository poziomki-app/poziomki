use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::api::state::{MatchingTagResponse, TagScope};
use crate::db::models::event_interactions::EventInteraction;
use crate::db::models::event_tags::EventTag;
use crate::db::models::events::Event;
use crate::db::models::profile_tags::ProfileTag;
use crate::db::models::profiles::Profile;
use crate::db::models::recommendation_feedback::RecommendationFeedback;
use crate::db::models::tags::Tag;
use crate::db::models::users::User;
use crate::db::schema::{
    event_interactions, event_tags, events, profile_tags, profiles, recommendation_feedback, tags,
    user_settings, users,
};

pub(super) struct MatchingRepository;

pub(super) struct MatchingProfileContext {
    pub(super) profile: Option<Profile>,
    pub(super) profile_tag_ids: HashSet<Uuid>,
}

pub(super) struct MatchingUserContext {
    pub(super) profile: Option<Profile>,
    pub(super) profile_tag_ids: HashSet<Uuid>,
    pub(super) saved_event_ids: HashSet<Uuid>,
    pub(super) joined_event_ids: HashSet<Uuid>,
    pub(super) more_event_ids: HashSet<Uuid>,
    pub(super) less_event_ids: HashSet<Uuid>,
}

impl MatchingRepository {
    async fn load_profile_tag_ids(
        &self,
        profile_id: Uuid,
        conn: &mut crate::db::DbConn,
    ) -> std::result::Result<HashSet<Uuid>, crate::error::AppError> {
        let tag_links = profile_tags::table
            .filter(profile_tags::profile_id.eq(profile_id))
            .load::<ProfileTag>(conn)
            .await?;
        Ok(tag_links.iter().map(|link| link.tag_id).collect())
    }

    async fn load_interaction_event_ids(
        &self,
        profile_id: Uuid,
        kind: &str,
        conn: &mut crate::db::DbConn,
    ) -> std::result::Result<HashSet<Uuid>, crate::error::AppError> {
        let interactions = event_interactions::table
            .filter(event_interactions::profile_id.eq(profile_id))
            .filter(event_interactions::kind.eq(kind))
            .load::<EventInteraction>(conn)
            .await?;
        Ok(interactions.into_iter().map(|row| row.event_id).collect())
    }

    async fn load_feedback_event_ids(
        &self,
        profile_id: Uuid,
        conn: &mut crate::db::DbConn,
    ) -> std::result::Result<(HashSet<Uuid>, HashSet<Uuid>), crate::error::AppError> {
        let rows = recommendation_feedback::table
            .filter(recommendation_feedback::profile_id.eq(profile_id))
            .load::<RecommendationFeedback>(conn)
            .await?;
        let mut more = HashSet::new();
        let mut less = HashSet::new();
        for row in rows {
            match row.feedback.as_str() {
                "more" => {
                    more.insert(row.event_id);
                }
                "less" => {
                    less.insert(row.event_id);
                }
                _ => {}
            }
        }
        Ok((more, less))
    }

    /// Lean context for profile-only recommendations (no event interactions).
    pub(super) async fn load_profile_context(
        &self,
        user_id: i32,
        conn: &mut crate::db::DbConn,
    ) -> std::result::Result<MatchingProfileContext, crate::error::AppError> {
        let profile = profiles::table
            .filter(profiles::user_id.eq(user_id))
            .first::<Profile>(conn)
            .await
            .optional()?;
        let profile_tag_ids = match &profile {
            Some(profile) => self.load_profile_tag_ids(profile.id, conn).await?,
            None => HashSet::new(),
        };
        Ok(MatchingProfileContext {
            profile,
            profile_tag_ids,
        })
    }

    pub(super) async fn load_user_context(
        &self,
        user_id: i32,
        conn: &mut crate::db::DbConn,
    ) -> std::result::Result<MatchingUserContext, crate::error::AppError> {
        let profile = profiles::table
            .filter(profiles::user_id.eq(user_id))
            .first::<Profile>(conn)
            .await
            .optional()?;
        let (profile_tag_ids, saved_event_ids, joined_event_ids, more_event_ids, less_event_ids) =
            match &profile {
                Some(profile) => {
                    let (more, less) = self.load_feedback_event_ids(profile.id, conn).await?;
                    (
                        self.load_profile_tag_ids(profile.id, conn).await?,
                        self.load_interaction_event_ids(profile.id, "saved", conn)
                            .await?,
                        self.load_interaction_event_ids(profile.id, "joined", conn)
                            .await?,
                        more,
                        less,
                    )
                }
                None => (
                    HashSet::new(),
                    HashSet::new(),
                    HashSet::new(),
                    HashSet::new(),
                    HashSet::new(),
                ),
            };
        Ok(MatchingUserContext {
            profile,
            profile_tag_ids,
            saved_event_ids,
            joined_event_ids,
            more_event_ids,
            less_event_ids,
        })
    }

    pub(super) async fn load_candidate_profiles(
        &self,
        user_id: i32,
        limit: i64,
        conn: &mut crate::db::DbConn,
    ) -> std::result::Result<Vec<Profile>, crate::error::AppError> {
        profiles::table
            .left_join(user_settings::table.on(user_settings::user_id.eq(profiles::user_id)))
            .filter(profiles::user_id.ne(user_id))
            .filter(
                user_settings::privacy_discoverable
                    .eq(true)
                    .or(user_settings::privacy_discoverable.is_null()),
            )
            .order(profiles::created_at.desc())
            .limit(limit)
            .select(profiles::all_columns)
            .load::<Profile>(conn)
            .await
            .map_err(Into::into)
    }

    pub(super) async fn load_future_events(
        &self,
        now: DateTime<Utc>,
        limit: i64,
        conn: &mut crate::db::DbConn,
    ) -> std::result::Result<Vec<Event>, crate::error::AppError> {
        events::table
            .filter(events::starts_at.ge(now))
            .order(events::starts_at.asc())
            .limit(limit)
            .load::<Event>(conn)
            .await
            .map_err(Into::into)
    }

    pub(super) async fn batch_load_profile_tags(
        &self,
        profile_ids: &[Uuid],
        conn: &mut crate::db::DbConn,
    ) -> std::result::Result<HashMap<Uuid, Vec<MatchingTagResponse>>, crate::error::AppError> {
        if profile_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let all_links = profile_tags::table
            .filter(profile_tags::profile_id.eq_any(profile_ids))
            .load::<ProfileTag>(conn)
            .await?;

        let all_tag_ids: HashSet<Uuid> = all_links.iter().map(|link| link.tag_id).collect();
        let tag_models = if all_tag_ids.is_empty() {
            vec![]
        } else {
            tags::table
                .filter(tags::id.eq_any(&all_tag_ids.into_iter().collect::<Vec<_>>()))
                .load::<Tag>(conn)
                .await?
        };

        let tag_by_id: HashMap<Uuid, &Tag> = tag_models.iter().map(|tag| (tag.id, tag)).collect();

        let mut result: HashMap<Uuid, Vec<MatchingTagResponse>> = HashMap::new();
        for link in &all_links {
            if let Some(tag) = tag_by_id.get(&link.tag_id) {
                result
                    .entry(link.profile_id)
                    .or_default()
                    .push(MatchingTagResponse {
                        id: tag.id.to_string(),
                        name: tag.name.clone(),
                        scope: scope_from_str(&tag.scope),
                        parent_id: tag.parent_id.map(|id| id.to_string()),
                    });
            }
        }
        Ok(result)
    }

    pub(super) async fn batch_load_profile_tag_ids(
        &self,
        profile_ids: &[Uuid],
        conn: &mut crate::db::DbConn,
    ) -> std::result::Result<HashMap<Uuid, HashSet<Uuid>>, crate::error::AppError> {
        if profile_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let all_links = profile_tags::table
            .filter(profile_tags::profile_id.eq_any(profile_ids))
            .load::<ProfileTag>(conn)
            .await?;

        let mut result: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
        for link in &all_links {
            result
                .entry(link.profile_id)
                .or_default()
                .insert(link.tag_id);
        }
        Ok(result)
    }

    pub(super) async fn batch_load_event_tag_ids(
        &self,
        event_ids: &[Uuid],
        conn: &mut crate::db::DbConn,
    ) -> std::result::Result<HashMap<Uuid, HashSet<Uuid>>, crate::error::AppError> {
        if event_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let all_links = event_tags::table
            .filter(event_tags::event_id.eq_any(event_ids))
            .load::<EventTag>(conn)
            .await?;

        let mut result: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
        for link in &all_links {
            result.entry(link.event_id).or_default().insert(link.tag_id);
        }
        Ok(result)
    }

    pub(super) async fn load_users_by_ids(
        &self,
        user_ids: &[i32],
        conn: &mut crate::db::DbConn,
    ) -> std::result::Result<Vec<User>, crate::error::AppError> {
        if user_ids.is_empty() {
            return Ok(vec![]);
        }

        users::table
            .filter(users::id.eq_any(user_ids))
            .load::<User>(conn)
            .await
            .map_err(Into::into)
    }

    pub(super) async fn load_tag_parent_map(
        &self,
        conn: &mut crate::db::DbConn,
    ) -> std::result::Result<HashMap<Uuid, Option<Uuid>>, crate::error::AppError> {
        Ok(tags::table
            .select((tags::id, tags::parent_id))
            .load::<(Uuid, Option<Uuid>)>(conn)
            .await?
            .into_iter()
            .collect())
    }
}

fn scope_from_str(s: &str) -> TagScope {
    match s {
        "activity" => TagScope::Activity,
        "event" => TagScope::Event,
        _ => TagScope::Interest,
    }
}
