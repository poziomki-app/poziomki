use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MatrixDmRooms::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MatrixDmRooms::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MatrixDmRooms::UserLowPid).uuid().not_null())
                    .col(ColumnDef::new(MatrixDmRooms::UserHighPid).uuid().not_null())
                    .col(ColumnDef::new(MatrixDmRooms::RoomId).string().not_null())
                    .col(
                        ColumnDef::new(MatrixDmRooms::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MatrixDmRooms::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_matrix_dm_rooms_pair_unique")
                    .table(MatrixDmRooms::Table)
                    .col(MatrixDmRooms::UserLowPid)
                    .col(MatrixDmRooms::UserHighPid)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MatrixDmRooms::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MatrixDmRooms {
    Table,
    Id,
    UserLowPid,
    UserHighPid,
    RoomId,
    CreatedAt,
    UpdatedAt,
}
