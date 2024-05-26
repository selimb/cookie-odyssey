use axum::{extract::Path, response::IntoResponse as _};
use minijinja::context;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::{
    journal::{
        queries::{query_journal_by_slug, query_media_for_journal_entry, MediaFull},
        routes::{JournalEntryNewPath, JournalEntryNewQuery},
    },
    storage::FileStore,
    utils::date_utils::{date_from_sqlite, date_to_sqlite, time_from_sqlite},
    AppState, AuthSession, Route, RouteResult, Templ,
};
use entities::{prelude::*, *};

#[derive(Debug, Deserialize)]
pub struct JournalDayGetPath {
    pub slug: String,
    pub date: chrono::NaiveDate,
}

pub async fn journal_day_get(
    state: AppState,
    templ: Templ,
    session: AuthSession,
    Path(JournalDayGetPath { slug, date }): Path<JournalDayGetPath>,
) -> RouteResult {
    let journal = query_journal_by_slug(slug.clone(), &state.db).await?;
    let journal = match journal {
        Ok(journal) => journal,
        Err(err) => {
            return Ok(err.render(&templ).into_response());
        }
    };

    let entries =
        query_entries_for_day(&journal, &date, &state.db, &session, &state.storage).await?;

    let datetime = chrono::NaiveDateTime::new(date, Default::default());
    let href_journal_detail = Route::JournalDetailGet { slug: Some(&slug) }.as_path();
    let href_journal_entry_new = Route::JournalEntryNewGet(Some((
        &JournalEntryNewPath { slug },
        &JournalEntryNewQuery { date: Some(date) },
    )))
    .as_path();
    let ctx = context! {
        journal,
        datetime,
        entries,
        href_journal_detail,
        href_journal_entry_new,
    };

    let html = templ.render_ctx("journal_day.html", ctx)?;
    Ok(html.into_response())
}

#[derive(Serialize, Debug)]
struct Entry {
    datetime: chrono::NaiveDateTime,
    time: chrono::NaiveTime,
    title: String,
    address: String,
    text: String,
    draft: bool,
    href_edit: String,
    media: Vec<MediaFull>,
}

async fn query_entries_for_day(
    journal: &journal::Model,
    date: &chrono::NaiveDate,
    db: &DatabaseConnection,
    auth: &AuthSession,
    storage: &FileStore,
) -> Result<Vec<Entry>, anyhow::Error> {
    let mut q = JournalEntry::find()
        .filter(journal_entry::Column::JournalId.eq(journal.id))
        .filter(journal_entry::Column::Date.eq(date_to_sqlite(*date)));
    q = auth.backend.filter_journal_entries(auth, q).await?;
    let entries_db = q.all(db).await?;

    // TODO: Avoid N+1 query, but in this case `N` should not be greater than 5, so meh.
    let mut entries: Vec<Entry> = Vec::new();
    for e in entries_db {
        let date = date_from_sqlite(e.date).unwrap();
        let time = time_from_sqlite(e.time).unwrap();
        let datetime = chrono::NaiveDateTime::new(date, time);

        let href_edit = Route::JournalEntryEditGet {
            entry_id: Some(e.id),
        }
        .as_path()
        .into_owned();

        let media = query_media_for_journal_entry(e.id, db, storage).await?;

        let entry = Entry {
            title: e.title,
            address: e.address,
            text: e.text,
            draft: e.draft,
            datetime,
            time,
            href_edit,
            media,
        };
        entries.push(entry);
    }
    entries.sort_by_key(|e| e.datetime);

    Ok(entries)
}
