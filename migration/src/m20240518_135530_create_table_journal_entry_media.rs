use sea_orm_migration::prelude::*;

use crate::m20240508_221939_create_table_file::File;
use crate::m20240518_025335_create_table_journal_entry::JournalEntry;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(JournalEntryMedia::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(JournalEntryMedia::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(JournalEntryMedia::JournalEntryId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(JournalEntryMedia::Table, JournalEntryMedia::JournalEntryId)
                            .to(JournalEntry::Table, JournalEntry::Id),
                    )
                    .col(
                        ColumnDef::new(JournalEntryMedia::Order)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(JournalEntryMedia::FileId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(JournalEntryMedia::Table, JournalEntryMedia::FileId)
                            .to(File::Table, File::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(JournalEntryMedia::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum JournalEntryMedia {
    Table,
    Id,
    JournalEntryId,
    Order,
    FileId,
}
