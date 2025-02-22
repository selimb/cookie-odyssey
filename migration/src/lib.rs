pub use sea_orm_migration::prelude::*;

mod m20240508_221939_create_table_file;
mod m20240508_223223_create_table_journal;
mod m20240512_173332_create_table_user;
mod m20240518_025335_create_table_journal_entry;
mod m20240518_135530_create_table_journal_entry_media;
mod m20240526_203520_create_table_journal_comment;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240508_221939_create_table_file::Migration),
            Box::new(m20240508_223223_create_table_journal::Migration),
            Box::new(m20240512_173332_create_table_user::Migration),
            Box::new(m20240518_025335_create_table_journal_entry::Migration),
            Box::new(m20240518_135530_create_table_journal_entry_media::Migration),
            Box::new(m20240526_203520_create_table_journal_comment::Migration),
        ]
    }
}
