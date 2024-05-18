use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(JournalEntry::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(JournalEntry::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(JournalEntry::Title).string().not_null())
                    .col(ColumnDef::new(JournalEntry::Text).string().not_null())
                    .col(ColumnDef::new(JournalEntry::Address).string().null())
                    .col(ColumnDef::new(JournalEntry::Lat).float().null())
                    .col(ColumnDef::new(JournalEntry::Lng).float().null())
                    .col(
                        ColumnDef::new(JournalEntry::DateTime)
                            // Use a naive datetime here, since the time is implicitly
                            // in the timezone of the location, and everyone should
                            // see the same time regardless of where they are!
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(JournalEntry::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum JournalEntry {
    Table,
    Id,
    Title,
    Text,
    Address,
    Lat,
    Lng,
    DateTime,
}
