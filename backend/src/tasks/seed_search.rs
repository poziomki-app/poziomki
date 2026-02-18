use async_trait::async_trait;
use loco_rs::{
    app::AppContext,
    task::{Task, TaskInfo, Vars},
};
use sea_orm::EntityTrait;

use crate::models::_entities::{degrees, events, profile_tags, profiles, tags};
use crate::search;

pub struct SeedSearch;

#[async_trait]
impl Task for SeedSearch {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "seed_search".to_string(),
            detail: "Seed Meilisearch indexes with all profiles, events, tags, and degrees"
                .to_string(),
        }
    }

    async fn run(&self, ctx: &AppContext, _vars: &Vars) -> loco_rs::Result<()> {
        if !search::meili_compat_enabled() {
            tracing::info!(
                "SEARCH_MEILI_COMPAT is disabled and SEARCH_BACKEND is not meilisearch; skipping seed_search"
            );
            return Ok(());
        }

        let client = search::create_client().map_err(|e| {
            loco_rs::Error::Message(format!("Failed to create Meilisearch client: {e}"))
        })?;

        tracing::info!("Configuring Meilisearch indexes...");
        search::configure_indexes(&client).await;

        // --- Profiles ---
        let all_profiles = profiles::Entity::find()
            .all(&ctx.db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))?;

        let all_profile_tag_links = profile_tags::Entity::find()
            .all(&ctx.db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))?;

        let all_tags = tags::Entity::find()
            .all(&ctx.db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))?;

        let profile_docs: Vec<search::ProfileDocument> = all_profiles
            .iter()
            .map(|p| {
                let tag_ids: Vec<uuid::Uuid> = all_profile_tag_links
                    .iter()
                    .filter(|link| link.profile_id == p.id)
                    .map(|link| link.tag_id)
                    .collect();
                let tag_names: Vec<String> = all_tags
                    .iter()
                    .filter(|t| tag_ids.contains(&t.id))
                    .map(|t| t.name.clone())
                    .collect();

                search::ProfileDocument {
                    id: p.id.to_string(),
                    name: p.name.clone(),
                    bio: p.bio.clone(),
                    age: p.age,
                    program: p.program.clone(),
                    profile_picture: p.profile_picture.clone(),
                    tags: tag_names,
                }
            })
            .collect();

        if !profile_docs.is_empty() {
            let idx = client.index("profiles");
            idx.add_or_replace(&profile_docs, Some("id"))
                .await
                .map_err(|e| loco_rs::Error::Message(format!("Failed to index profiles: {e}")))?;
        }
        tracing::info!("Indexed {} profiles", profile_docs.len());

        // --- Events ---
        let all_events = events::Entity::find()
            .all(&ctx.db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))?;

        let event_docs: Vec<search::EventDocument> = all_events
            .iter()
            .map(|e| {
                let creator_name = all_profiles
                    .iter()
                    .find(|p| p.id == e.creator_id)
                    .map_or_else(|| "Unknown".to_string(), |p| p.name.clone());

                search::EventDocument {
                    id: e.id.to_string(),
                    title: e.title.clone(),
                    description: e.description.clone(),
                    location: e.location.clone(),
                    starts_at: e.starts_at.to_rfc3339(),
                    cover_image: e.cover_image.clone(),
                    creator_name,
                    geo: e
                        .latitude
                        .zip(e.longitude)
                        .map(|(lat, lng)| search::GeoPoint { lat, lng }),
                }
            })
            .collect();

        if !event_docs.is_empty() {
            let idx = client.index("events");
            idx.add_or_replace(&event_docs, Some("id"))
                .await
                .map_err(|e| loco_rs::Error::Message(format!("Failed to index events: {e}")))?;
        }
        tracing::info!("Indexed {} events", event_docs.len());

        // --- Tags ---
        let tag_docs: Vec<search::TagDocument> = all_tags
            .iter()
            .map(|t| search::TagDocument {
                id: t.id.to_string(),
                name: t.name.clone(),
                scope: t.scope.clone(),
                category: t.category.clone(),
                emoji: t.emoji.clone(),
            })
            .collect();

        if !tag_docs.is_empty() {
            let idx = client.index("tags");
            idx.add_or_replace(&tag_docs, Some("id"))
                .await
                .map_err(|e| loco_rs::Error::Message(format!("Failed to index tags: {e}")))?;
        }
        tracing::info!("Indexed {} tags", tag_docs.len());

        // --- Degrees ---
        let all_degrees = degrees::Entity::find()
            .all(&ctx.db)
            .await
            .map_err(|e| loco_rs::Error::Any(e.into()))?;

        let degree_docs: Vec<search::DegreeDocument> = all_degrees
            .iter()
            .map(|d| search::DegreeDocument {
                id: d.id.to_string(),
                name: d.name.clone(),
            })
            .collect();

        if !degree_docs.is_empty() {
            let idx = client.index("degrees");
            idx.add_or_replace(&degree_docs, Some("id"))
                .await
                .map_err(|e| loco_rs::Error::Message(format!("Failed to index degrees: {e}")))?;
        }
        tracing::info!("Indexed {} degrees", degree_docs.len());

        tracing::info!("Search seed complete!");
        Ok(())
    }
}
