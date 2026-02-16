use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_index(
            Index::create()
                .name("idx_profiles_user_id")
                .table(Profiles::Table)
                .col(Profiles::UserId)
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_profile_tags_profile_id")
                .table(ProfileTags::Table)
                .col(ProfileTags::ProfileId)
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_profile_tags_tag_id")
                .table(ProfileTags::Table)
                .col(ProfileTags::TagId)
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_event_attendees_event_id")
                .table(EventAttendees::Table)
                .col(EventAttendees::EventId)
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_event_attendees_profile_id")
                .table(EventAttendees::Table)
                .col(EventAttendees::ProfileId)
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_event_tags_event_id")
                .table(EventTags::Table)
                .col(EventTags::EventId)
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_event_tags_tag_id")
                .table(EventTags::Table)
                .col(EventTags::TagId)
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_uploads_owner_id")
                .table(Uploads::Table)
                .col(Uploads::OwnerId)
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_sessions_user_id")
                .table(Sessions::Table)
                .col(Sessions::UserId)
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_sessions_token")
                .table(Sessions::Table)
                .col(Sessions::Token)
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let indexes = [
            "idx_profiles_user_id",
            "idx_profile_tags_profile_id",
            "idx_profile_tags_tag_id",
            "idx_event_attendees_event_id",
            "idx_event_attendees_profile_id",
            "idx_event_tags_event_id",
            "idx_event_tags_tag_id",
            "idx_uploads_owner_id",
            "idx_sessions_user_id",
            "idx_sessions_token",
        ];
        for name in indexes {
            m.drop_index(Index::drop().name(name).to_owned()).await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Profiles {
    Table,
    UserId,
}

#[derive(DeriveIden)]
enum ProfileTags {
    Table,
    ProfileId,
    TagId,
}

#[derive(DeriveIden)]
enum EventAttendees {
    Table,
    EventId,
    ProfileId,
}

#[derive(DeriveIden)]
enum EventTags {
    Table,
    EventId,
    TagId,
}

#[derive(DeriveIden)]
enum Uploads {
    Table,
    OwnerId,
}

#[derive(DeriveIden)]
enum Sessions {
    Table,
    UserId,
    Token,
}
