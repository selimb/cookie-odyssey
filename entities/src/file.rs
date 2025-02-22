//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize)]
#[sea_orm(table_name = "file")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub bucket: String,
    pub key: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::journal::Entity")]
    Journal,
    #[sea_orm(has_many = "super::journal_entry_media::Entity")]
    JournalEntryMedia,
}

impl Related<super::journal::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Journal.def()
    }
}

impl Related<super::journal_entry_media::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalEntryMedia.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
