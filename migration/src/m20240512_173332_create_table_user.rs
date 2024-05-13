use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(User::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(User::Email).string().unique_key().not_null())
                    .col(ColumnDef::new(User::Password).string().not_null())
                    .col(ColumnDef::new(User::FirstName).string().not_null())
                    .col(ColumnDef::new(User::LastName).string().not_null())
                    .col(
                        ColumnDef::new(User::Approved)
                            .boolean()
                            .not_null()
                            .default(Value::Bool(Some(false))),
                    )
                    .col(
                        ColumnDef::new(User::Admin)
                            .boolean()
                            .not_null()
                            .default(Value::Bool(Some(false))),
                    )
                    .col(ColumnDef::new(User::LastLogin).integer().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Email,
    Password,
    FirstName,
    LastName,
    Approved,
    // Would use a Role enum, but SQLite doesn't have enums, and the sea-orm
    // docs on enums is confusing.
    Admin,
    LastLogin,
}
