#[path = "m20250209_000005_helpers.rs"]
mod helpers;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        helpers::create_events_schema(m).await?;
        helpers::create_event_tags_schema(m).await?;
        helpers::create_event_attendees_schema(m).await?;
        helpers::create_uploads_table(m).await?;
        helpers::create_user_settings_schema(m).await?;
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
