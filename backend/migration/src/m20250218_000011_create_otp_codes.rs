use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(OtpCodes::Table)
                .if_not_exists()
                .col(ColumnDef::new(OtpCodes::Id).uuid().not_null().primary_key())
                .col(
                    ColumnDef::new(OtpCodes::Email)
                        .string()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(OtpCodes::Code).string().not_null())
                .col(
                    ColumnDef::new(OtpCodes::Attempts)
                        .small_integer()
                        .not_null()
                        .default(0),
                )
                .col(
                    ColumnDef::new(OtpCodes::ExpiresAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(OtpCodes::LastSentAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(OtpCodes::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_otp_codes_expires_at")
                .table(OtpCodes::Table)
                .col(OtpCodes::ExpiresAt)
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(OtpCodes::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum OtpCodes {
    Table,
    Id,
    Email,
    Code,
    Attempts,
    ExpiresAt,
    LastSentAt,
    CreatedAt,
}
