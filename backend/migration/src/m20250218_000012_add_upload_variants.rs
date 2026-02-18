use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Uploads::Table)
                .add_column(ColumnDef::new(Uploads::Thumbhash).binary().null())
                .add_column(
                    ColumnDef::new(Uploads::HasVariants)
                        .boolean()
                        .not_null()
                        .default(false),
                )
                .to_owned(),
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Uploads::Table)
                .drop_column(Uploads::Thumbhash)
                .drop_column(Uploads::HasVariants)
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Uploads {
    Table,
    Thumbhash,
    HasVariants,
}
