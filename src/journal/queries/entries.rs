use sea_orm::{DatabaseConnection, EntityTrait};

use entities::{prelude::*, *};

use crate::{storage::FileStore, NotFound};

use super::media::{query_media_for_journal_entry, MediaFull};

#[derive(Debug)]
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
    let media_list = query_media_for_journal_entry(entry.id, db, storage).await?;

    Ok(Ok(JournalEntryFull {
        entry,
        journal,
        media_list,
    }))
}
