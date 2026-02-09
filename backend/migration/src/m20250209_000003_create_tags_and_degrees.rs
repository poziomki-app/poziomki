use sea_orm_migration::prelude::*;
use uuid::Uuid;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UUID_NAMESPACE: Uuid = Uuid::from_bytes([
    0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30,
    0xc8,
]);

fn deterministic_uuid(name: &str) -> Uuid {
    Uuid::new_v5(&UUID_NAMESPACE, name.as_bytes())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // Create tags table
        m.create_table(
            Table::create()
                .table(Tags::Table)
                .if_not_exists()
                .col(ColumnDef::new(Tags::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(Tags::Name).string().not_null())
                .col(ColumnDef::new(Tags::Scope).string().not_null())
                .col(ColumnDef::new(Tags::Category).string().null())
                .col(ColumnDef::new(Tags::Emoji).string().null())
                .col(ColumnDef::new(Tags::OnboardingOrder).string().null())
                .col(
                    ColumnDef::new(Tags::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Tags::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await?;

        // Create degrees table
        m.create_table(
            Table::create()
                .table(Degrees::Table)
                .if_not_exists()
                .col(ColumnDef::new(Degrees::Id).uuid().not_null().primary_key())
                .col(
                    ColumnDef::new(Degrees::Name)
                        .string()
                        .not_null()
                        .unique_key(),
                )
                .col(
                    ColumnDef::new(Degrees::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Degrees::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await?;

        // Seed tags
        let tags: &[(&str, &str, &str, &str)] = &[
            ("Muzyka", "interest", "hobby", "1"),
            ("Sport", "interest", "hobby", "2"),
            ("Podróże", "interest", "hobby", "3"),
            ("Fotografia", "interest", "hobby", "4"),
            ("Gry", "interest", "hobby", "5"),
            ("Gotowanie", "interest", "hobby", "6"),
            ("Czytanie", "interest", "hobby", "7"),
            ("Sztuka", "interest", "hobby", "8"),
            ("Film", "interest", "hobby", "9"),
            ("Taniec", "interest", "hobby", "10"),
            ("Fitness", "interest", "styl życia", "11"),
            ("Joga", "interest", "styl życia", "12"),
            ("Góry", "interest", "styl życia", "13"),
            ("Rower", "interest", "styl życia", "14"),
            ("Bieganie", "interest", "styl życia", "15"),
            ("Programowanie", "interest", "tech", "16"),
            ("AI i ML", "interest", "tech", "17"),
            ("Startupy", "interest", "tech", "18"),
            ("Design", "interest", "tech", "19"),
            ("Nauka", "interest", "akademickie", "20"),
            ("Filozofia", "interest", "akademickie", "21"),
            ("Języki obce", "interest", "akademickie", "22"),
            ("Wolontariat", "interest", "społeczne", "23"),
            ("Gry planszowe", "interest", "społeczne", "24"),
            ("Grupa naukowa", "activity", "akademickie", "1"),
            ("Kawa i rozmowa", "activity", "społeczne", "2"),
            ("Partner treningowy", "activity", "fitness", "3"),
            ("Wspólny projekt", "activity", "tech", "4"),
            ("Wymiana językowa", "activity", "akademickie", "5"),
        ];

        let db = m.get_connection();
        for (name, scope, category, order) in tags {
            let id = deterministic_uuid(&format!("{scope}:{name}"));
            db.execute_unprepared(&format!(
                "INSERT INTO tags (id, name, scope, category, onboarding_order) VALUES ('{id}', '{name}', '{scope}', '{category}', '{order}')"
            ))
            .await?;
        }

        // Seed degrees
        let degrees = ["Computer Science", "Data Science", "Psychology"];
        for name in &degrees {
            let id = deterministic_uuid(&format!("degree:{name}"));
            db.execute_unprepared(&format!(
                "INSERT INTO degrees (id, name) VALUES ('{id}', '{name}')"
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(Degrees::Table).to_owned())
            .await?;
        m.drop_table(Table::drop().table(Tags::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Tags {
    Table,
    Id,
    Name,
    Scope,
    Category,
    Emoji,
    OnboardingOrder,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Degrees {
    Table,
    Id,
    Name,
    CreatedAt,
    UpdatedAt,
}
