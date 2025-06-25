pub use sea_orm_migration::prelude::*;

mod m20240508_221939_create_table_file;
mod m20240508_223223_create_table_journal;
mod m20240512_173332_create_table_user;
mod m20240518_025335_create_table_journal_entry;
mod m20240518_135530_create_table_journal_entry_media;
mod m20240526_203520_create_table_journal_comment;
mod m20250520_001650_media_type;
mod m20250622_181713_journal_entry_media_width_height_thumbnail;

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
            Box::new(m20250520_001650_media_type::Migration),
            Box::new(m20250622_181713_journal_entry_media_width_height_thumbnail::Migration),
        ]
    }
}
