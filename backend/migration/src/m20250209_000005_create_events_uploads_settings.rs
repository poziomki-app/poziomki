use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // --- events ---
        m.create_table(
            Table::create()
                .table(Events::Table)
                .if_not_exists()
                .col(ColumnDef::new(Events::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(Events::Title).string().not_null())
                .col(ColumnDef::new(Events::Description).text().null())
                .col(ColumnDef::new(Events::CoverImage).string().null())
                .col(ColumnDef::new(Events::Location).string().null())
                .col(
                    ColumnDef::new(Events::StartsAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Events::EndsAt)
                        .timestamp_with_time_zone()
                        .null(),
                )
                .col(ColumnDef::new(Events::CreatorId).uuid().not_null())
                .col(ColumnDef::new(Events::ConversationId).string().null())
                .col(
                    ColumnDef::new(Events::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Events::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await?;

        m.create_foreign_key(
            ForeignKey::create()
                .name("fk_events_creator_id")
                .from(Events::Table, Events::CreatorId)
                .to(Profiles::Table, Profiles::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;

        // --- event_tags ---
        m.create_table(
            Table::create()
                .table(EventTags::Table)
                .if_not_exists()
                .col(ColumnDef::new(EventTags::EventId).uuid().not_null())
                .col(ColumnDef::new(EventTags::TagId).uuid().not_null())
                .primary_key(
                    Index::create()
                        .col(EventTags::EventId)
                        .col(EventTags::TagId),
                )
                .to_owned(),
        )
        .await?;

        m.create_foreign_key(
            ForeignKey::create()
                .name("fk_event_tags_event_id")
                .from(EventTags::Table, EventTags::EventId)
                .to(Events::Table, Events::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;

        m.create_foreign_key(
            ForeignKey::create()
                .name("fk_event_tags_tag_id")
                .from(EventTags::Table, EventTags::TagId)
                .to(Tags::Table, Tags::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;

        // --- event_attendees ---
        m.create_table(
            Table::create()
                .table(EventAttendees::Table)
                .if_not_exists()
                .col(ColumnDef::new(EventAttendees::EventId).uuid().not_null())
                .col(ColumnDef::new(EventAttendees::ProfileId).uuid().not_null())
                .col(
                    ColumnDef::new(EventAttendees::Status)
                        .string()
                        .not_null()
                        .default("going"),
                )
                .primary_key(
                    Index::create()
                        .col(EventAttendees::EventId)
                        .col(EventAttendees::ProfileId),
                )
                .to_owned(),
        )
        .await?;

        m.create_foreign_key(
            ForeignKey::create()
                .name("fk_event_attendees_event_id")
                .from(EventAttendees::Table, EventAttendees::EventId)
                .to(Events::Table, Events::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;

        m.create_foreign_key(
            ForeignKey::create()
                .name("fk_event_attendees_profile_id")
                .from(EventAttendees::Table, EventAttendees::ProfileId)
                .to(Profiles::Table, Profiles::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;

        // --- uploads ---
        m.create_table(
            Table::create()
                .table(Uploads::Table)
                .if_not_exists()
                .col(ColumnDef::new(Uploads::Id).uuid().not_null().primary_key())
                .col(
                    ColumnDef::new(Uploads::Filename)
                        .string()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(Uploads::OwnerId).uuid().null())
                .col(ColumnDef::new(Uploads::Context).string().not_null())
                .col(ColumnDef::new(Uploads::ContextId).string().null())
                .col(ColumnDef::new(Uploads::MimeType).string().not_null())
                .col(
                    ColumnDef::new(Uploads::Deleted)
                        .boolean()
                        .not_null()
                        .default(false),
                )
                .col(
                    ColumnDef::new(Uploads::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Uploads::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await?;

        // --- user_settings ---
        m.create_table(
            Table::create()
                .table(UserSettings::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(UserSettings::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(UserSettings::UserId).integer().not_null())
                .col(
                    ColumnDef::new(UserSettings::Theme)
                        .string()
                        .not_null()
                        .default("system"),
                )
                .col(
                    ColumnDef::new(UserSettings::Language)
                        .string()
                        .not_null()
                        .default("system"),
                )
                .col(
                    ColumnDef::new(UserSettings::NotificationsEnabled)
                        .boolean()
                        .not_null()
                        .default(true),
                )
                .col(
                    ColumnDef::new(UserSettings::PrivacyShowAge)
                        .boolean()
                        .not_null()
                        .default(true),
                )
                .col(
                    ColumnDef::new(UserSettings::PrivacyShowProgram)
                        .boolean()
                        .not_null()
                        .default(true),
                )
                .col(
                    ColumnDef::new(UserSettings::PrivacyDiscoverable)
                        .boolean()
                        .not_null()
                        .default(true),
                )
                .col(
                    ColumnDef::new(UserSettings::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(UserSettings::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await?;

        m.create_foreign_key(
            ForeignKey::create()
                .name("fk_user_settings_user_id")
                .from(UserSettings::Table, UserSettings::UserId)
                .to(Users::Table, Users::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_user_settings_user_id_unique")
                .table(UserSettings::Table)
                .col(UserSettings::UserId)
                .unique()
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(UserSettings::Table).to_owned())
            .await?;
        m.drop_table(Table::drop().table(Uploads::Table).to_owned())
            .await?;
        m.drop_table(Table::drop().table(EventAttendees::Table).to_owned())
            .await?;
        m.drop_table(Table::drop().table(EventTags::Table).to_owned())
            .await?;
        m.drop_table(Table::drop().table(Events::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Events {
    Table,
    Id,
    Title,
    Description,
    CoverImage,
    Location,
    StartsAt,
    EndsAt,
    CreatorId,
    ConversationId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum EventTags {
    Table,
    EventId,
    TagId,
}

#[derive(DeriveIden)]
enum EventAttendees {
    Table,
    EventId,
    ProfileId,
    Status,
}

#[derive(DeriveIden)]
enum Uploads {
    Table,
    Id,
    Filename,
    OwnerId,
    Context,
    ContextId,
    MimeType,
    Deleted,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UserSettings {
    Table,
    Id,
    UserId,
    Theme,
    Language,
    NotificationsEnabled,
    PrivacyShowAge,
    PrivacyShowProgram,
    PrivacyDiscoverable,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Profiles {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Tags {
    Table,
    Id,
}
