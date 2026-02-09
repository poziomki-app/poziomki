use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "sessions",
            &[
                ("id", ColType::PkUuid),
                ("token", ColType::StringUniq),
                ("ip_address", ColType::StringNull),
                ("user_agent", ColType::StringNull),
                ("expires_at", ColType::TimestampWithTimeZone),
            ],
            &[("users", "")],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "sessions").await?;
        Ok(())
    }
}
