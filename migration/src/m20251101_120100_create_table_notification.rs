use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240512_173332_create_table_user::User;
use crate::m20240518_025335_create_table_journal_entry::JournalEntry;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Notification::Table)
                    .if_not_exists()
                    .col(pk_auto(Notification::Id))
                    .col(
                        string(Notification::Kind)
                            .check(Expr::col(Notification::Kind).is_in(["entry_published"])),
                    )
                    .col(integer(Notification::JournalEntryId))
                    .col(timestamp(Notification::CreatedAt))
                    .col(timestamp_null(Notification::FlushedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Notification::Table, Notification::JournalEntryId)
                            .to(JournalEntry::Table, JournalEntry::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // Pending Notifications: for the Admin badge and the Flush.
        manager
            .create_index(
                Index::create()
                    .name("idx_notification_pending")
                    .table(Notification::Table)
                    .col(Notification::Id)
                    .and_where(Expr::col(Notification::FlushedAt).is_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(NotificationRecipient::Table)
                    .if_not_exists()
                    .col(pk_auto(NotificationRecipient::Id))
                    .col(integer(NotificationRecipient::NotificationId))
                    .col(integer(NotificationRecipient::UserId))
                    .col(timestamp_null(NotificationRecipient::ReadAt))
                    .col(
                        string(NotificationRecipient::EmailStatus).check(
                            Expr::col(NotificationRecipient::EmailStatus)
                                .is_in(["unsent", "sent", "skipped"]),
                        ),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                NotificationRecipient::Table,
                                NotificationRecipient::NotificationId,
                            )
                            .to(Notification::Table, Notification::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(NotificationRecipient::Table, NotificationRecipient::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notification_recipient_notification_user")
                    .table(NotificationRecipient::Table)
                    .col(NotificationRecipient::NotificationId)
                    .col(NotificationRecipient::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Listing and keyset pagination.
        manager
            .create_index(
                Index::create()
                    .name("idx_notification_recipient_user_id")
                    .table(NotificationRecipient::Table)
                    .col(NotificationRecipient::UserId)
                    .col(NotificationRecipient::Id)
                    .to_owned(),
            )
            .await?;

        // Unread badge and marking a Day as read.
        manager
            .create_index(
                Index::create()
                    .name("idx_notification_recipient_unread")
                    .table(NotificationRecipient::Table)
                    .col(NotificationRecipient::UserId)
                    .and_where(Expr::col(NotificationRecipient::ReadAt).is_null())
                    .to_owned(),
            )
            .await?;

        // Email worker and the Admin "unsent email" count.
        manager
            .create_index(
                Index::create()
                    .name("idx_notification_recipient_email_unsent")
                    .table(NotificationRecipient::Table)
                    .col(NotificationRecipient::UserId)
                    .and_where(Expr::col(NotificationRecipient::EmailStatus).eq("unsent"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(NotificationRecipient::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Notification::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Notification {
    Table,
    Id,
    Kind,
    JournalEntryId,
    CreatedAt,
    FlushedAt,
}

#[derive(DeriveIden)]
enum NotificationRecipient {
    Table,
    Id,
    NotificationId,
    UserId,
    ReadAt,
    EmailStatus,
}
