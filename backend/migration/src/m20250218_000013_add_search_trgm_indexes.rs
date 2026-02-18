use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseBackend;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        if m.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }

        m.get_connection()
            .execute_unprepared("CREATE EXTENSION IF NOT EXISTS pg_trgm")
            .await?;

        m.get_connection()
            .execute_unprepared(
                r"
                CREATE INDEX IF NOT EXISTS idx_profiles_name_trgm
                    ON profiles USING gin (LOWER(name) gin_trgm_ops);
                CREATE INDEX IF NOT EXISTS idx_profiles_bio_trgm
                    ON profiles USING gin (LOWER(COALESCE(bio, '')) gin_trgm_ops);
                CREATE INDEX IF NOT EXISTS idx_profiles_program_trgm
                    ON profiles USING gin (LOWER(COALESCE(program, '')) gin_trgm_ops);
                CREATE INDEX IF NOT EXISTS idx_profiles_updated_at
                    ON profiles (updated_at DESC);
                CREATE INDEX IF NOT EXISTS idx_tags_name_trgm
                    ON tags USING gin (LOWER(name) gin_trgm_ops);
                CREATE INDEX IF NOT EXISTS idx_degrees_name_trgm
                    ON degrees USING gin (LOWER(name) gin_trgm_ops);
                CREATE INDEX IF NOT EXISTS idx_events_title_trgm
                    ON events USING gin (LOWER(title) gin_trgm_ops);
                CREATE INDEX IF NOT EXISTS idx_events_description_trgm
                    ON events USING gin (LOWER(COALESCE(description, '')) gin_trgm_ops);
                CREATE INDEX IF NOT EXISTS idx_events_location_trgm
                    ON events USING gin (LOWER(COALESCE(location, '')) gin_trgm_ops);
                CREATE INDEX IF NOT EXISTS idx_events_starts_at
                    ON events (starts_at);
                CREATE INDEX IF NOT EXISTS idx_events_creator_id
                    ON events (creator_id);
                ",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        if m.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }

        m.get_connection()
            .execute_unprepared(
                r"
                DROP INDEX IF EXISTS idx_events_creator_id;
                DROP INDEX IF EXISTS idx_events_starts_at;
                DROP INDEX IF EXISTS idx_events_location_trgm;
                DROP INDEX IF EXISTS idx_events_description_trgm;
                DROP INDEX IF EXISTS idx_events_title_trgm;
                DROP INDEX IF EXISTS idx_degrees_name_trgm;
                DROP INDEX IF EXISTS idx_tags_name_trgm;
                DROP INDEX IF EXISTS idx_profiles_updated_at;
                DROP INDEX IF EXISTS idx_profiles_program_trgm;
                DROP INDEX IF EXISTS idx_profiles_bio_trgm;
                DROP INDEX IF EXISTS idx_profiles_name_trgm;
                ",
            )
            .await?;

        Ok(())
    }
}
