use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Journal::Table)
                    .col(
                        ColumnDef::new(Journal::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Journal::Name).string().not_null())
                    .col(ColumnDef::new(Journal::Slug).string().not_null())
                    .col(ColumnDef::new(Journal::StartDate).date().not_null())
                    .col(ColumnDef::new(Journal::EndDate).date().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Journal::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Journal {
    Table,
    Id,
    Name,
    Slug,
    StartDate,
    EndDate,
}
