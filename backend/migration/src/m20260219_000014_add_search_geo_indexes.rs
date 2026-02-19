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
            .execute_unprepared("CREATE EXTENSION IF NOT EXISTS cube")
            .await?;
        m.get_connection()
            .execute_unprepared("CREATE EXTENSION IF NOT EXISTS earthdistance")
            .await?;

        m.get_connection()
            .execute_unprepared(
                r"
                CREATE INDEX IF NOT EXISTS idx_events_geo_earth
                    ON events
                    USING gist (ll_to_earth(latitude, longitude))
                    WHERE latitude IS NOT NULL AND longitude IS NOT NULL;
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
                DROP INDEX IF EXISTS idx_events_geo_earth;
                ",
            )
            .await?;

        Ok(())
    }
}
