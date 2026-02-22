use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(JobOutbox::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(JobOutbox::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(JobOutbox::Topic).string().not_null())
                    .col(ColumnDef::new(JobOutbox::Payload).json_binary().not_null())
                    .col(
                        ColumnDef::new(JobOutbox::Attempts)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(JobOutbox::MaxAttempts)
                            .integer()
                            .not_null()
                            .default(10),
                    )
                    .col(
                        ColumnDef::new(JobOutbox::AvailableAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(JobOutbox::LockedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(JobOutbox::ProcessedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(JobOutbox::FailedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(JobOutbox::LastError).text())
                    .col(
                        ColumnDef::new(JobOutbox::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(JobOutbox::UpdatedAt)
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
                    .name("idx_job_outbox_topic")
                    .table(JobOutbox::Table)
                    .col(JobOutbox::Topic)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_job_outbox_available_at")
                    .table(JobOutbox::Table)
                    .col(JobOutbox::AvailableAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_job_outbox_pending_scan")
                    .table(JobOutbox::Table)
                    .col(JobOutbox::ProcessedAt)
                    .col(JobOutbox::FailedAt)
                    .col(JobOutbox::AvailableAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(JobOutbox::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum JobOutbox {
    Table,
    Id,
    Topic,
    Payload,
    Attempts,
    MaxAttempts,
    AvailableAt,
    LockedAt,
    ProcessedAt,
    FailedAt,
    LastError,
    CreatedAt,
    UpdatedAt,
}
