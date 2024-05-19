use sea_orm_migration::prelude::*;

use super::m20240508_223223_create_table_journal::Journal;

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
                    .col(ColumnDef::new(JournalEntry::JournalId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(JournalEntry::Table, JournalEntry::JournalId)
                            .to(Journal::Table, Journal::Id),
                    )
                    .col(ColumnDef::new(JournalEntry::Date).date().not_null())
                    .col(
                        // Use a naive time here, since the time is implicitly
                        // in the timezone of the location, and everyone should
                        // see the same time regardless of where they are!
                        ColumnDef::new(JournalEntry::Time).time().not_null(),
                    )
                    .col(ColumnDef::new(JournalEntry::Title).string().not_null())
                    .col(
                        ColumnDef::new(JournalEntry::Text)
                            .string()
                            .not_null()
                            .default(Value::String(Some(Box::new("".to_string())))),
                    )
                    .col(
                        ColumnDef::new(JournalEntry::Draft)
                            .boolean()
                            .not_null()
                            .default(Value::Bool(Some(true))),
                    )
                    .col(
                        ColumnDef::new(JournalEntry::Address)
                            .string()
                            .not_null()
                            .default(Value::String(Some(Box::new("".to_string())))),
                    )
                    .col(ColumnDef::new(JournalEntry::Lat).float().null())
                    .col(ColumnDef::new(JournalEntry::Lng).float().null())
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
pub enum JournalEntry {
    Table,
    Id,
    JournalId,
    Date,
    Time,
    Title,
    Text,
    Draft,
    Address,
    Lat,
    Lng,
}
