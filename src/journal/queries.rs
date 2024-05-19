use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder};

use entities::{prelude::*, *};
use serde::Serialize;

use crate::NotFound;

pub async fn query_journal_by_slug(
    slug: String,
    db: &DatabaseConnection,
) -> Result<Result<journal::Model, NotFound>, DbErr> {
    let journal = Journal::find()
        .filter(journal::Column::Slug.eq(slug))
        .one(db)
        .await?;
    match journal {
        Some(journal) => Ok(Ok(journal)),
        None => Ok(Err(NotFound::for_entity("journal"))),
    }
}

#[derive(Serialize, Debug)]
pub struct MediaFull {
    pub id: i32,
    pub order: i32,
    pub file_id: i32,
    pub file_url: String,
}

#[derive(Serialize, Debug)]
pub struct JournalEntryFull {
    pub entry: journal_entry::Model,
    pub journal: journal::Model,
    pub media: Vec<MediaFull>,
}

pub async fn query_journal_entry_by_id(
    id: i32,
    db: &DatabaseConnection,
) -> Result<Result<JournalEntryFull, NotFound>, DbErr> {
    let entry = JournalEntry::find_by_id(id)
        .find_also_related(Journal)
        .one(db)
        .await?;
    let (entry, journal) = match entry {
        Some((entry, Some(journal))) => (entry, journal),
        _ => {
            return Ok(Err(NotFound::for_entity("entry")));
        }
    };

    let media = JournalEntryMedia::find()
        .filter(journal_entry_media::Column::JournalEntryId.eq(entry.id))
        .find_also_related(File)
        .order_by_asc(journal_entry_media::Column::Order)
        .all(db)
        .await?;
    let media = media
        .into_iter()
        .map(|(media, file)| -> MediaFull {
            MediaFull {
                id: media.id,
                order: media.order,
                file_id: media.file_id,
                file_url: file.expect("Should never be null").url,
            }
        })
        .collect();

    Ok(Ok(JournalEntryFull {
        entry,
        journal,
        media,
    }))
}
