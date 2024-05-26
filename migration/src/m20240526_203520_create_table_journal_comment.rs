use sea_orm_migration::prelude::*;

use super::m20240508_223223_create_table_journal::Journal;
use super::m20240512_173332_create_table_user::User;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(JournalComment::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(JournalComment::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(JournalComment::JournalId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(JournalComment::Table, JournalComment::JournalId)
                            .to(Journal::Table, Journal::Id),
                    )
                    .col(ColumnDef::new(JournalComment::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(JournalComment::Table, JournalComment::UserId)
                            .to(User::Table, User::Id),
                    )
                    .col(
                        ColumnDef::new(JournalComment::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(JournalComment::Date).date().null())
                    .col(ColumnDef::new(JournalComment::Text).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(JournalComment::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum JournalComment {
    Table,
    Id,
    JournalId,
    UserId,
    CreatedAt,
    Date,
    Text,
}
