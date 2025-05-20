use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(JournalEntryMedia::Table)
                    .add_column(
                        ColumnDef::new(JournalEntryMedia::MediaType)
                            .string()
                            .not_null()
                            .default("image")
                            .check(
                                Expr::col(JournalEntryMedia::MediaType).is_in(["image", "video"]),
                            ),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(JournalEntryMedia::Table)
                    .drop_column(JournalEntryMedia::MediaType)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum JournalEntryMedia {
    Table,
    MediaType,
}
