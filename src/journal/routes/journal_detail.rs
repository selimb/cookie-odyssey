use anyhow::Context as _;
use axum::{
    extract::{Path, State},
    response::IntoResponse as _,
};
use itertools::Itertools;
use minijinja::context;
use serde::Serialize;

use crate::{
    journal::queries::query_journal_by_slug,
    utils::date_utils::{date_from_sqlite, time_from_sqlite},
    AppState, AuthSession, Route, RouteResult, Templ,
};
use entities::{prelude::*, *};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};

use super::{JournalDayGetPath, JournalEntryNewPath, JournalEntryNewQuery};

pub async fn journal_detail_get(
    state: State<AppState>,
    templ: Templ,
    session: AuthSession,
    Path(slug): Path<String>,
) -> RouteResult {
    let journal = query_journal_by_slug(slug.clone(), &state.db).await?;
    let journal = match journal {
        Ok(journal) => journal,
        Err(err) => {
            return Ok(err.render(&templ).into_response());
        }
    };

    let entries_by_day =
        query_entries_by_day(journal.id, &journal.slug, &state.db, &session).await?;

    let href_journal_entry_new = Route::JournalEntryNewGet(Some((
        &JournalEntryNewPath { slug },
        &JournalEntryNewQuery::default(),
    )))
    .as_path();

    let ctx = context! { journal, entries_by_day, href_journal_entry_new };
    let html = templ.render_ctx("journal_detail.html", ctx)?;
    Ok(html.into_response())
}

#[derive(Serialize, Debug)]
struct EntrySlim {
    id: i32,
    title: String,
    date: chrono::NaiveDate,
    datetime: chrono::NaiveDateTime,
    draft: bool,
}

async fn query_entries_by_day(
    journal_id: i32,
    journal_slug: impl AsRef<str>,
    db: &DatabaseConnection,
    auth: &AuthSession,
) -> Result<Vec<(chrono::NaiveDate, String, Vec<EntrySlim>)>, anyhow::Error> {
    let mut q = JournalEntry::find()
        .columns([
            journal_entry::Column::Id,
            journal_entry::Column::Title,
            journal_entry::Column::Date,
            journal_entry::Column::Time,
            journal_entry::Column::Draft,
        ])
        .filter(journal_entry::Column::JournalId.eq(journal_id));
    q = auth.backend.filter_journal_entries(auth, q).await?;
    let entries = q.all(db).await.context("DB query failed")?;

    let entries = entries
        .into_iter()
        .map(|e| {
            let date = date_from_sqlite(e.date).unwrap();
            let time = time_from_sqlite(e.time).unwrap();
            let datetime = chrono::NaiveDateTime::new(date, time);
            EntrySlim {
                id: e.id,
                title: e.title,
                date,
                datetime,
                draft: e.draft,
            }
        })
        .sorted_by_key(|e| e.datetime);
    let mut entries_by_day = Vec::new();
    for (date, chunk) in &entries.into_iter().chunk_by(|e| e.date) {
        let href = Route::JournalDayGet(Some(&JournalDayGetPath {
            slug: journal_slug.as_ref().to_string(),
            date,
        }))
        .as_path()
        .to_string();

        entries_by_day.push((date, href, chunk.collect_vec()))
    }

    Ok(entries_by_day)
}
