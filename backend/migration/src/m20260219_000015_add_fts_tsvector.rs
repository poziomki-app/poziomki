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

        // Add generated tsvector columns and GIN indexes for full-text search.
        // Uses 'simple' dictionary (no stemming) — works for Polish + English.
        m.get_connection()
            .execute_unprepared(
                r"
                -- Profiles: weighted vector (name > program > bio)
                ALTER TABLE profiles ADD COLUMN search_vector tsvector
                  GENERATED ALWAYS AS (
                    setweight(to_tsvector('simple', COALESCE(name, '')), 'A') ||
                    setweight(to_tsvector('simple', COALESCE(program, '')), 'B') ||
                    setweight(to_tsvector('simple', COALESCE(bio, '')), 'C')
                  ) STORED;
                CREATE INDEX idx_profiles_fts ON profiles USING gin(search_vector);

                -- Events: weighted vector (title > location > description)
                ALTER TABLE events ADD COLUMN search_vector tsvector
                  GENERATED ALWAYS AS (
                    setweight(to_tsvector('simple', COALESCE(title, '')), 'A') ||
                    setweight(to_tsvector('simple', COALESCE(location, '')), 'B') ||
                    setweight(to_tsvector('simple', COALESCE(description, '')), 'C')
                  ) STORED;
                CREATE INDEX idx_events_fts ON events USING gin(search_vector);

                -- Drop trigram indexes now covered by tsvector
                DROP INDEX IF EXISTS idx_profiles_bio_trgm;
                DROP INDEX IF EXISTS idx_profiles_program_trgm;
                DROP INDEX IF EXISTS idx_events_description_trgm;
                DROP INDEX IF EXISTS idx_events_location_trgm;
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
                -- Restore trigram indexes
                CREATE INDEX IF NOT EXISTS idx_profiles_bio_trgm
                    ON profiles USING gin (LOWER(COALESCE(bio, '')) gin_trgm_ops);
                CREATE INDEX IF NOT EXISTS idx_profiles_program_trgm
                    ON profiles USING gin (LOWER(COALESCE(program, '')) gin_trgm_ops);
                CREATE INDEX IF NOT EXISTS idx_events_description_trgm
                    ON events USING gin (LOWER(COALESCE(description, '')) gin_trgm_ops);
                CREATE INDEX IF NOT EXISTS idx_events_location_trgm
                    ON events USING gin (LOWER(COALESCE(location, '')) gin_trgm_ops);

                -- Drop FTS indexes and columns
                DROP INDEX IF EXISTS idx_events_fts;
                DROP INDEX IF EXISTS idx_profiles_fts;
                ALTER TABLE events DROP COLUMN IF EXISTS search_vector;
                ALTER TABLE profiles DROP COLUMN IF EXISTS search_vector;
                ",
            )
            .await?;

        Ok(())
    }
}
