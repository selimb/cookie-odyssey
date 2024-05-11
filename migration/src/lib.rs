pub use sea_orm_migration::prelude::*;

mod m20240508_221939_create_table_file;
mod m20240508_223223_create_table_journal;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240508_221939_create_table_file::Migration),
            Box::new(m20240508_223223_create_table_journal::Migration),
        ]
    }
}
