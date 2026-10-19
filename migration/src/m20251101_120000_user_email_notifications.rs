use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240512_173332_create_table_user::User;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(User::Table)
                    .add_column(boolean(UserEmailNotifications::EmailNotifications).default(true))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(User::Table)
                    .drop_column(UserEmailNotifications::EmailNotifications)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum UserEmailNotifications {
    EmailNotifications,
}
