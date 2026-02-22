use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(Sessions::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Sessions::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Sessions::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(ColumnDef::new(Sessions::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(Sessions::UserId).integer().not_null())
                .col(
                    ColumnDef::new(Sessions::Token)
                        .string()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(Sessions::IpAddress).string().null())
                .col(ColumnDef::new(Sessions::UserAgent).string().null())
                .col(
                    ColumnDef::new(Sessions::ExpiresAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-sessions-user_id")
                        .from(Sessions::Table, Sessions::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Sessions {
    Table,
    CreatedAt,
    UpdatedAt,
    Id,
    UserId,
    Token,
    IpAddress,
    UserAgent,
    ExpiresAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
