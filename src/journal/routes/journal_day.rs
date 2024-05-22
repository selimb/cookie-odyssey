use axum::{extract::Path, response::IntoResponse as _};
use minijinja::context;
use serde::Deserialize;

use crate::{
    journal::{
        queries::query_journal_by_slug,
        routes::{JournalEntryNewPath, JournalEntryNewQuery},
    },
    AppState, Route, RouteResult, Templ,
};

#[derive(Debug, Deserialize)]
pub struct JournalDayGetPath {
    pub slug: String,
    pub date: chrono::NaiveDate,
}

pub async fn journal_day_get(
    state: AppState,
    templ: Templ,
    Path(JournalDayGetPath { slug, date }): Path<JournalDayGetPath>,
) -> RouteResult {
    let journal = query_journal_by_slug(slug.clone(), &state.db).await?;
    let journal = match journal {
        Ok(journal) => journal,
        Err(err) => {
            return Ok(err.render(&templ).into_response());
        }
    };

    let href_journal_entry_new = Route::JournalEntryNewGet(Some((
        &JournalEntryNewPath { slug },
        &JournalEntryNewQuery { date: Some(date) },
    )))
    .as_path();
    let ctx = context! {
        journal,
        date,
        href_journal_entry_new,
    };

    todo!()
}
