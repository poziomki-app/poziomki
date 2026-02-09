#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;
mod m20250209_000002_create_sessions;
mod m20250209_000003_create_tags_and_degrees;
mod m20250209_000004_create_profiles;
mod m20250209_000005_create_events_uploads_settings;

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
            // inject-above (do not remove this comment)
        ]
    }
}
