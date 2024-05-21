use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use itertools::Itertools;
use minijinja::context;
use serde::Serialize;

use crate::{
    journal::queries::query_journal_by_slug,
    utils::date_utils::{date_from_sqlite, time_from_sqlite},
    AppState, Route, RouteResult, Templ,
};
use entities::{prelude::*, *};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect};

pub async fn journal_detail_get(
    state: State<AppState>,
    templ: Templ,
    Path(slug): Path<String>,
) -> RouteResult {
    let journal = query_journal_by_slug(slug, &state.db).await?;
    let journal = match journal {
        Ok(journal) => journal,
        Err(err) => {
            return Ok(err.render(&templ).into_response());
        }
    };

    let entries_by_day = query_entries_by_day(journal.id, &state.db).await?;

    // XXX Don't show draft to non-admins
    let href_journal_entry_new = Route::JournalEntryNewGet {
        slug: Some(&journal.slug),
    }
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
}

async fn query_entries_by_day(
    journal_id: i32,
    db: &DatabaseConnection,
) -> Result<Vec<(chrono::NaiveDate, Vec<EntrySlim>)>, DbErr> {
    let entries = JournalEntry::find()
        .filter(journal_entry::Column::JournalId.eq(journal_id))
        .columns([
            journal_entry::Column::Id,
            journal_entry::Column::Title,
            journal_entry::Column::Date,
            journal_entry::Column::Time,
        ])
        .all(db)
        .await?;

    let entries = entries
        .into_iter()
        .map(|e| {
            println!("address {}", e.address);
            let date = date_from_sqlite(e.date).unwrap();
            let time = time_from_sqlite(e.time).unwrap();
            let datetime = chrono::NaiveDateTime::new(date, time);
            EntrySlim {
                id: e.id,
                title: e.title,
                date,
                datetime,
            }
        })
        .sorted_by_key(|e| e.datetime);
    let mut entries_by_day = Vec::new();
    for (date, chunk) in &entries.into_iter().chunk_by(|e| e.date) {
        entries_by_day.push((date, chunk.collect_vec()))
    }

    Ok(entries_by_day)
}
