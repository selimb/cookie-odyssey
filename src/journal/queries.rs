use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder};

use entities::{prelude::*, *};
use serde::Serialize;

use crate::{storage::FileStore, NotFound};

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
    pub url: String,
}

#[derive(Serialize, Debug)]
pub struct JournalEntryFull {
    pub entry: journal_entry::Model,
    pub journal: journal::Model,
    pub media_list: Vec<MediaFull>,
}

pub async fn query_journal_entry_by_id(
    id: i32,
    db: &DatabaseConnection,
    storage: &FileStore,
) -> Result<Result<JournalEntryFull, NotFound>, anyhow::Error> {
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

    let media_db = JournalEntryMedia::find()
        .filter(journal_entry_media::Column::JournalEntryId.eq(entry.id))
        .find_also_related(File)
        .order_by_asc(journal_entry_media::Column::Order)
        .all(db)
        .await?;
    let mut media_list: Vec<MediaFull> = Vec::new();
    for (media, file) in media_db {
        let file = file.expect("Should be non-null");
        let url = storage.sign_url(file.bucket, file.key).await?;
        let m = MediaFull {
            id: media.id,
            order: media.order,
            file_id: media.file_id,
            url,
        };
        media_list.push(m);
    }

    Ok(Ok(JournalEntryFull {
        entry,
        journal,
        media_list,
    }))
}
