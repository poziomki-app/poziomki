use meilisearch_sdk::client::Client;
use serde::{Deserialize, Serialize};

// --- Document types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDocument {
    pub id: String,
    pub name: String,
    pub bio: Option<String>,
    pub age: i16,
    pub program: Option<String>,
    pub profile_picture: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDocument {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub starts_at: String,
    pub cover_image: Option<String>,
    pub creator_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagDocument {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub category: Option<String>,
    pub emoji: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegreeDocument {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub profiles: Vec<ProfileDocument>,
    pub events: Vec<EventDocument>,
    pub tags: Vec<TagDocument>,
    pub degrees: Vec<DegreeDocument>,
}

// --- Client ---

pub fn create_client() -> Result<Client, meilisearch_sdk::errors::Error> {
    let url = std::env::var("MEILI_URL").unwrap_or_else(|_| "http://localhost:7700".to_string());
    let key = std::env::var("MEILI_MASTER_KEY").unwrap_or_else(|_| "meili-dev-key".to_string());
    Client::new(url, Some(key))
}

// --- Index configuration ---

pub async fn configure_indexes(client: &Client) {
    configure_profiles_index(client).await;
    configure_events_index(client).await;
    configure_tags_index(client).await;
    configure_degrees_index(client).await;
}

async fn configure_profiles_index(client: &Client) {
    let index = client.index("profiles");
    let _ = index
        .set_searchable_attributes(["name", "bio", "program", "tags"])
        .await;
    let _ = index
        .set_filterable_attributes(["age", "program", "tags"])
        .await;
    let _ = index.set_sortable_attributes(["name"]).await;
}

async fn configure_events_index(client: &Client) {
    let index = client.index("events");
    let _ = index
        .set_searchable_attributes(["title", "description", "location", "creator_name"])
        .await;
    let _ = index
        .set_filterable_attributes(["starts_at", "location"])
        .await;
    let _ = index.set_sortable_attributes(["starts_at"]).await;
}

async fn configure_tags_index(client: &Client) {
    let index = client.index("tags");
    let _ = index.set_searchable_attributes(["name", "category"]).await;
    let _ = index.set_filterable_attributes(["scope"]).await;
    let _ = index.set_sortable_attributes(["name"]).await;
}

async fn configure_degrees_index(client: &Client) {
    let index = client.index("degrees");
    let _ = index.set_searchable_attributes(["name"]).await;
    let _ = index.set_sortable_attributes(["name"]).await;
}

// --- Sync helpers (fire-and-forget, log warnings on failure) ---

pub fn index_profile(client: &Client, doc: ProfileDocument) {
    let index = client.index("profiles");
    tokio::spawn(async move {
        if let Err(e) = index.add_or_replace(&[doc], Some("id")).await {
            tracing::warn!("Failed to index profile in Meilisearch: {e}");
        }
    });
}

pub fn index_event(client: &Client, doc: EventDocument) {
    let index = client.index("events");
    tokio::spawn(async move {
        if let Err(e) = index.add_or_replace(&[doc], Some("id")).await {
            tracing::warn!("Failed to index event in Meilisearch: {e}");
        }
    });
}

pub fn index_tag(client: &Client, doc: TagDocument) {
    let index = client.index("tags");
    tokio::spawn(async move {
        if let Err(e) = index.add_or_replace(&[doc], Some("id")).await {
            tracing::warn!("Failed to index tag in Meilisearch: {e}");
        }
    });
}

pub fn index_degree(client: &Client, doc: DegreeDocument) {
    let index = client.index("degrees");
    tokio::spawn(async move {
        if let Err(e) = index.add_or_replace(&[doc], Some("id")).await {
            tracing::warn!("Failed to index degree in Meilisearch: {e}");
        }
    });
}

pub fn delete_profile(client: &Client, id: String) {
    let index = client.index("profiles");
    tokio::spawn(async move {
        if let Err(e) = index.delete_document(&id).await {
            tracing::warn!("Failed to delete profile from Meilisearch: {e}");
        }
    });
}

pub fn delete_event(client: &Client, id: String) {
    let index = client.index("events");
    tokio::spawn(async move {
        if let Err(e) = index.delete_document(&id).await {
            tracing::warn!("Failed to delete event from Meilisearch: {e}");
        }
    });
}

// --- Multi-search ---

pub async fn search_all(
    client: &Client,
    query: &str,
    limit: usize,
) -> Result<SearchResults, meilisearch_sdk::errors::Error> {
    let profiles_idx = client.index("profiles");
    let events_idx = client.index("events");
    let tags_idx = client.index("tags");
    let degrees_idx = client.index("degrees");

    let mut pq = profiles_idx.search();
    pq.with_query(query).with_limit(limit);
    let mut eq = events_idx.search();
    eq.with_query(query).with_limit(limit);
    let mut tq = tags_idx.search();
    tq.with_query(query).with_limit(limit);
    let mut dq = degrees_idx.search();
    dq.with_query(query).with_limit(limit);

    let (profiles_res, events_res, tags_res, degrees_res) = tokio::join!(
        pq.execute::<ProfileDocument>(),
        eq.execute::<EventDocument>(),
        tq.execute::<TagDocument>(),
        dq.execute::<DegreeDocument>(),
    );

    Ok(SearchResults {
        profiles: profiles_res
            .map(|r| r.hits.into_iter().map(|h| h.result).collect())
            .unwrap_or_default(),
        events: events_res
            .map(|r| r.hits.into_iter().map(|h| h.result).collect())
            .unwrap_or_default(),
        tags: tags_res
            .map(|r| r.hits.into_iter().map(|h| h.result).collect())
            .unwrap_or_default(),
        degrees: degrees_res
            .map(|r| r.hits.into_iter().map(|h| h.result).collect())
            .unwrap_or_default(),
    })
}
