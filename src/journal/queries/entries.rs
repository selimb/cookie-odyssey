use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder, Select};

use entities::{prelude::*, *};
use serde::Serialize;

use super::media::{query_media_for_journal_entry, MediaFull};
use crate::{storage::FileStore, AuthSession, NotFound, Route};

/// An Entry as rendered on a Day or Getaway page.
#[derive(Serialize, Debug)]
pub struct EntryView {
    pub id: i32,
    pub datetime: chrono::NaiveDateTime,
    /// Whether the date differs from the previous Entry's.
    pub new_date: bool,
    pub title: String,
    pub address: String,
    pub text: String,
    pub draft: bool,
    pub href_edit: String,
    pub media: Vec<MediaFull>,
}

/// Lists the Entries selected by `q` that the User can see, in trip order.
pub async fn query_entry_views(
    q: Select<JournalEntry>,
    db: &DatabaseConnection,
    auth: &AuthSession,
    storage: &FileStore,
) -> Result<Vec<EntryView>, anyhow::Error> {
    let q = auth.backend.filter_journal_entries(auth, q).await?;
    let entries_db = q
        .order_by_asc(journal_entry::Column::Date)
        .order_by_asc(journal_entry::Column::Time)
        .order_by_asc(journal_entry::Column::Id)
        .all(db)
        .await?;

    // TODO: Avoid N+1 query, but in this case `N` should not be greater than 5, so meh.
    let mut entries: Vec<EntryView> = Vec::with_capacity(entries_db.len());
    for e in entries_db {
        let new_date = entries
            .last()
            .is_none_or(|prev| prev.datetime.date() != e.date);
        let href_edit = Route::JournalEntryEditGet {
            entry_id: Some(e.id),
        }
        .as_path()
        .into_owned();
        let media = query_media_for_journal_entry(e.id, db, storage).await?;
        entries.push(EntryView {
            id: e.id,
            datetime: chrono::NaiveDateTime::new(e.date, e.time),
            new_date,
            title: e.title,
            address: e.address,
            text: e.text,
            draft: e.draft,
            href_edit,
            media,
        });
    }
    Ok(entries)
}

#[derive(Debug)]
pub struct JournalEntryFull {
    pub entry: journal_entry::Model,
    pub journal: journal::Model,
    /// The name of the Entry's Getaway.
    pub getaway_name: Option<String>,
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
    let getaway_name = match entry.getaway_id {
        None => None,
        Some(getaway_id) => JournalGetaway::find_by_id(getaway_id)
            .one(db)
            .await?
            .map(|g| g.name),
    };
    let media_list = query_media_for_journal_entry(entry.id, db, storage).await?;

    Ok(Ok(JournalEntryFull {
        entry,
        journal,
        getaway_name,
        media_list,
    }))
}
