use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240508_223223_create_table_journal::Journal;
use crate::m20240518_025335_create_table_journal_entry::JournalEntry;
use crate::m20240526_203520_create_table_journal_comment::JournalComment;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Journal::Table)
                    .add_column(boolean(JournalScrapbook::Scrapbook).default(false))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(JournalGetaway::Table)
                    .if_not_exists()
                    .col(pk_auto(JournalGetaway::Id))
                    .col(integer(JournalGetaway::JournalId))
                    // [getaway-name-nocase] Names compare ignoring case.
                    // There is no DDL method for a collation.
                    .col(string(JournalGetaway::Name).extra("COLLATE NOCASE"))
                    .foreign_key(
                        ForeignKey::create()
                            .from(JournalGetaway::Table, JournalGetaway::JournalId)
                            .to(Journal::Table, Journal::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // [getaway-name-index] Names are unique per Scrapbook.
        // It also serves the lookups by name, and compares with the collation
        // of `name`, see [getaway-name-nocase].
        manager
            .create_index(
                Index::create()
                    .name("idx_journal_getaway_journal_name")
                    .table(JournalGetaway::Table)
                    .col(JournalGetaway::JournalId)
                    .col(JournalGetaway::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // [entry-getaway-fk] A Getaway cannot be deleted while it has Entries.
        // SQLite can only add a foreign key through the column definition.
        manager
            .alter_table(
                Table::alter()
                    .table(JournalEntry::Table)
                    .add_column(
                        integer_null(JournalEntryGetaway::GetawayId)
                            .extra("REFERENCES journal_getaway (id)"),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_journal_entry_getaway_id")
                    .table(JournalEntry::Table)
                    .col(JournalEntryGetaway::GetawayId)
                    .to_owned(),
            )
            .await?;

        // [comment-scope-check] A Comment belongs to the Journal, a Day or a
        // Getaway, so at most one of `date` and `getaway_id` is set.
        manager
            .alter_table(
                Table::alter()
                    .table(JournalComment::Table)
                    .add_column(
                        integer_null(JournalCommentGetaway::GetawayId)
                            // [getaway-comment-cascade] Deleting a Getaway
                            // deletes its Comments.
                            .extra("REFERENCES journal_getaway (id) ON DELETE CASCADE")
                            .check(
                                Expr::col(JournalCommentGetaway::GetawayId)
                                    .is_null()
                                    .or(Expr::col(JournalComment::Date).is_null()),
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
                    .table(JournalComment::Table)
                    .drop_column(JournalCommentGetaway::GetawayId)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_journal_entry_getaway_id")
                    .table(JournalEntry::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(JournalEntry::Table)
                    .drop_column(JournalEntryGetaway::GetawayId)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(JournalGetaway::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Journal::Table)
                    .drop_column(JournalScrapbook::Scrapbook)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum JournalScrapbook {
    Scrapbook,
}

#[derive(DeriveIden)]
enum JournalGetaway {
    Table,
    Id,
    JournalId,
    Name,
}

#[derive(DeriveIden)]
enum JournalEntryGetaway {
    GetawayId,
}

#[derive(DeriveIden)]
enum JournalCommentGetaway {
    GetawayId,
}
