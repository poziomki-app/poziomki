#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;
mod m20250209_000002_create_sessions;
mod m20250209_000003_create_tags_and_degrees;
mod m20250209_000004_create_profiles;
mod m20250209_000005_create_events_uploads_settings;
mod m20250209_000006_seed_full_degrees;
mod m20250215_000007_replace_degrees_uw;
mod m20250216_000008_add_event_geo;
mod m20250216_000009_add_profile_personalization;
mod m20250217_000010_add_indexes;
mod m20250218_000011_create_otp_codes;
mod m20250218_000012_add_upload_variants;
mod m20250218_000013_add_search_trgm_indexes;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20250209_000002_create_sessions::Migration),
            Box::new(m20250209_000003_create_tags_and_degrees::Migration),
            Box::new(m20250209_000004_create_profiles::Migration),
            Box::new(m20250209_000005_create_events_uploads_settings::Migration),
            Box::new(m20250209_000006_seed_full_degrees::Migration),
            Box::new(m20250215_000007_replace_degrees_uw::Migration),
            Box::new(m20250216_000008_add_event_geo::Migration),
            Box::new(m20250216_000009_add_profile_personalization::Migration),
            Box::new(m20250217_000010_add_indexes::Migration),
            Box::new(m20250218_000011_create_otp_codes::Migration),
            Box::new(m20250218_000012_add_upload_variants::Migration),
            Box::new(m20250218_000013_add_search_trgm_indexes::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
