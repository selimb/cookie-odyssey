use crate::m20240508_221939_create_table_file::File;
use crate::m20240518_025335_create_table_journal_entry::JournalEntry;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Ugh. SQLite doesn't let you add foreign keys to an existing table.
        // Let's recreate it!

        // First, rename the existing table to a temporary name.
        let old_table = Alias::new("media_backup");
        manager
            .rename_table(
                Table::rename()
                    .table(JournalEntryMedia::Table, old_table.clone())
                    .to_owned(),
            )
            .await?;

        // Create the table.
        manager
            .create_table(
                Table::create()
                    .table(JournalEntryMedia::Table)
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
                        ColumnDef::new(JournalEntryMedia::Caption)
                            .string()
                            .not_null()
                            .default(Value::String(Some(Box::new("".to_string())))),
                    )
                    .col(
                        ColumnDef::new(JournalEntryMedia::MediaType)
                            .string()
                            .not_null()
                            .default("image")
                            .check(
                                Expr::col(JournalEntryMedia::MediaType).is_in(["image", "video"]),
                            ),
                    )
                    .col(
                        ColumnDef::new(JournalEntryMedia::Width)
                            .unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(JournalEntryMedia::Height)
                            .unsigned()
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
                            .to(File::Table, File::Id)
                            .name("FK_journal_entry_media_file"),
                    )
                    .col(
                        ColumnDef::new(JournalEntryMedia::ThumbnailWidth)
                            .unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(JournalEntryMedia::ThumbnailHeight)
                            .unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(JournalEntryMedia::ThumbnailFileId)
                            .integer()
                            .not_null(),
                    )
                    // NOTE: There's no way to name forein key constraints in SQLite...
                    .foreign_key(
                        ForeignKey::create()
                            .from(JournalEntryMedia::Table, JournalEntryMedia::ThumbnailFileId)
                            .to(File::Table, File::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // Copy data from old table to new table.
        // New width/height fields are temporarily set to 0, and thumbnail_file_id is set to file_id.
        // The actual values are set with a one-time script (see migration/data-migrations/journal-entry-media-width-height-thumbnail/index.ts).
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
                INSERT INTO journal_entry_media (
                    "id",
                    "journal_entry_id",
                    "order",
                    "caption",
                    "media_type",
                    "width",
                    "height",
                    "file_id",
                    "thumbnail_width",
                    "thumbnail_height",
                    "thumbnail_file_id"
                )
                SELECT
                    "id",
                    "journal_entry_id",
                    "order",
                    "caption",
                    "media_type",
                    0,
                    0,
                    "file_id",
                    0,
                    0,
                    "file_id"
                FROM media_backup
            "#,
        )
        .await?;

        manager
            .drop_table(Table::drop().table(old_table).to_owned())
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
    Caption,
    MediaType,
    Width,  // NEW
    Height, // NEW
    FileId,
    ThumbnailWidth,  // NEW
    ThumbnailHeight, // NEW
    ThumbnailFileId, // NEW
}
