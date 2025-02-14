use axum::{extract::Path, response::IntoResponse as _};
use minijinja::context;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::{
    comment::routes::CommentList,
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
    let comments = CommentList {
        journal_id: journal.id,
        date: Some(date),
        partial: false,
    }
    .query_and_render(&state.db, &templ, &session)
    .await?;

    let (day_prev, day_next) = query_surrounding_days(&journal, &date, &state.db, &session).await?;

    let datetime = chrono::NaiveDateTime::new(date, Default::default());
    let href_journal_detail = Route::JournalDetailGet { slug: Some(&slug) }.as_path();
    let href_journal_entry_new = Route::JournalEntryNewGet(Some((
        &JournalEntryNewPath { slug: slug.clone() },
        &JournalEntryNewQuery { date: Some(date) },
    )))
    .as_path();
    let href_journal_day_prev = day_prev.map(|day| {
        Route::JournalDayGet(Some(&JournalDayGetPath {
            slug: slug.clone(),
            date: day,
        }))
        .as_path()
    });
    let href_journal_day_next = day_next.map(|day| {
        Route::JournalDayGet(Some(&JournalDayGetPath {
            slug: slug.clone(),
            date: day,
        }))
        .as_path()
    });

    let ctx = context! {
        journal,
        datetime,
        entries,
        comments_fragment => comments.0,
        href_journal_detail,
        href_journal_entry_new,
        href_journal_day_prev,
        href_journal_day_next,
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

async fn query_surrounding_days(
    journal: &journal::Model,
    date: &chrono::NaiveDate,
    db: &DatabaseConnection,
    auth: &AuthSession,
) -> Result<(Option<chrono::NaiveDate>, Option<chrono::NaiveDate>), anyhow::Error> {
    // TODO: Only select date.
    let mut q_base = JournalEntry::find().filter(journal_entry::Column::JournalId.eq(journal.id));
    q_base = auth.backend.filter_journal_entries(auth, q_base).await?;

    let q_prev = q_base
        .clone()
        .filter(journal_entry::Column::Date.lt(date_to_sqlite(*date)))
        .order_by_desc(journal_entry::Column::Date);
    let q_next = q_base
        .filter(journal_entry::Column::Date.gt(date_to_sqlite(*date)))
        .order_by_asc(journal_entry::Column::Date);

    let entry_prev = q_prev.one(db).await?;
    let entry_next = q_next.one(db).await?;

    let day_prev = entry_prev.map(|entry| date_from_sqlite(entry.date).unwrap());
    let day_next = entry_next.map(|entry| date_from_sqlite(entry.date).unwrap());

    Ok((day_prev, day_next))
}
