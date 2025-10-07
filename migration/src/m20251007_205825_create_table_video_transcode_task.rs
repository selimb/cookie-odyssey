use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240508_221939_create_table_file::File;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(VideoTranscodeTask::Table)
                    .if_not_exists()
                    .col(pk_auto(VideoTranscodeTask::Id))
                    .col(timestamp(VideoTranscodeTask::CreatedAt))
                    .col(timestamp_null(VideoTranscodeTask::UpdatedAt))
                    .col(string(VideoTranscodeTask::Status).default("pending").check(
                        Expr::col(VideoTranscodeTask::Status).is_in([
                            "pending",
                            "completed",
                            "failed",
                        ]),
                    ))
                    .col(string(VideoTranscodeTask::Detail))
                    .col(integer(VideoTranscodeTask::FileId))
                    .foreign_key(
                        ForeignKey::create()
                            .from(VideoTranscodeTask::Table, VideoTranscodeTask::FileId)
                            .to(File::Table, File::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(VideoTranscodeTask::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum VideoTranscodeTask {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    Status,
    Detail,
    FileId,
}
