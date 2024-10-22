//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize)]
#[sea_orm(table_name = "journal_entry_media")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub journal_entry_id: i32,
    pub order: i32,
    pub caption: String,
    pub file_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::file::Entity",
        from = "Column::FileId",
        to = "super::file::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    File,
    #[sea_orm(
        belongs_to = "super::journal_entry::Entity",
        from = "Column::JournalEntryId",
        to = "super::journal_entry::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    JournalEntry,
}

impl Related<super::file::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::File.def()
    }
}

impl Related<super::journal_entry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalEntry.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
