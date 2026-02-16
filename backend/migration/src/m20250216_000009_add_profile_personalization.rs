use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Profiles::Table)
                .add_column(ColumnDef::new(Profiles::GradientStart).string().null())
                .add_column(ColumnDef::new(Profiles::GradientEnd).string().null())
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Profiles::Table)
                .drop_column(Profiles::GradientStart)
                .drop_column(Profiles::GradientEnd)
                .to_owned(),
        )
        .await
    }
}

#[derive(DeriveIden)]
enum Profiles {
    Table,
    GradientStart,
    GradientEnd,
}
