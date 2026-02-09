use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // Create profiles table
        m.create_table(
            Table::create()
                .table(Profiles::Table)
                .if_not_exists()
                .col(ColumnDef::new(Profiles::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(Profiles::UserId).integer().not_null())
                .col(ColumnDef::new(Profiles::Name).string().not_null())
                .col(ColumnDef::new(Profiles::Bio).text().null())
                .col(ColumnDef::new(Profiles::Age).small_integer().not_null())
                .col(ColumnDef::new(Profiles::ProfilePicture).string().null())
                .col(ColumnDef::new(Profiles::Images).json_binary().null())
                .col(ColumnDef::new(Profiles::Program).string().null())
                .col(
                    ColumnDef::new(Profiles::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Profiles::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await?;

        // FK: profiles.user_id → users.id
        m.create_foreign_key(
            ForeignKey::create()
                .name("fk_profiles_user_id")
                .from(Profiles::Table, Profiles::UserId)
                .to(Users::Table, Users::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;

        // Unique index on user_id (1:1 with users)
        m.create_index(
            Index::create()
                .name("idx_profiles_user_id_unique")
                .table(Profiles::Table)
                .col(Profiles::UserId)
                .unique()
                .to_owned(),
        )
        .await?;

        // Create profile_tags junction table
        m.create_table(
            Table::create()
                .table(ProfileTags::Table)
                .if_not_exists()
                .col(ColumnDef::new(ProfileTags::ProfileId).uuid().not_null())
                .col(ColumnDef::new(ProfileTags::TagId).uuid().not_null())
                .primary_key(
                    Index::create()
                        .col(ProfileTags::ProfileId)
                        .col(ProfileTags::TagId),
                )
                .to_owned(),
        )
        .await?;

        // FKs for profile_tags
        m.create_foreign_key(
            ForeignKey::create()
                .name("fk_profile_tags_profile_id")
                .from(ProfileTags::Table, ProfileTags::ProfileId)
                .to(Profiles::Table, Profiles::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;

        m.create_foreign_key(
            ForeignKey::create()
                .name("fk_profile_tags_tag_id")
                .from(ProfileTags::Table, ProfileTags::TagId)
                .to(Tags::Table, Tags::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(ProfileTags::Table).to_owned())
            .await?;
        m.drop_table(Table::drop().table(Profiles::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Profiles {
    Table,
    Id,
    UserId,
    Name,
    Bio,
    Age,
    ProfilePicture,
    Images,
    Program,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ProfileTags {
    Table,
    ProfileId,
    TagId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Tags {
    Table,
    Id,
}
