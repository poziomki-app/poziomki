use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AuthRateLimits::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AuthRateLimits::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AuthRateLimits::RateKey).string().not_null())
                    .col(
                        ColumnDef::new(AuthRateLimits::WindowStart)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AuthRateLimits::Attempts)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(AuthRateLimits::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_auth_rate_limits_rate_key_unique")
                    .table(AuthRateLimits::Table)
                    .col(AuthRateLimits::RateKey)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_auth_rate_limits_updated_at")
                    .table(AuthRateLimits::Table)
                    .col(AuthRateLimits::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuthRateLimits::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AuthRateLimits {
    Table,
    Id,
    RateKey,
    WindowStart,
    Attempts,
    UpdatedAt,
}
