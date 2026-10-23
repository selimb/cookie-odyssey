use axum::{
    extract::Path,
    response::{IntoResponse as _, Redirect},
};
use entities::prelude::*;
use sea_orm::EntityTrait;

use crate::{
    journal::routes::JournalDayGetPath, AppState, AuthSession, NotFound, Route, RouteResult, Templ,
};

/// See [entry-link].
/// Redirects to the Entry's current page, which is responsible for marking
/// Notifications as read.
pub async fn page_entry_link_get(
    state: AppState,
    templ: Templ,
    session: AuthSession,
    Path(entry_id): Path<i32>,
) -> RouteResult {
    let q = JournalEntry::find_by_id(entry_id);
    let q = session.backend.filter_journal_entries(&session, q).await?;
    let entry = match q.one(&state.db).await? {
        Some(entry) => entry,
        None => {
            return Ok(NotFound::for_entity("entry")
                .render(&templ)?
                .into_response())
        }
    };
    let journal = Journal::find_by_id(entry.journal_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Entry {} has no Journal", entry.id))?;

    let href = entry_page_href(&journal.slug, entry.date, entry.id);
    Ok(Redirect::to(&href).into_response())
}

/// The current page of an Entry, scrolled to the Entry.
/// Shared by [entry-link] and by pages that list Entries, which build the link
/// at render time and so need no stable link.
pub fn entry_page_href(slug: &str, date: chrono::NaiveDate, entry_id: i32) -> String {
    let path = Route::JournalDayGet(Some(&JournalDayGetPath {
        slug: slug.to_string(),
        date,
    }))
    .as_path();
    format!("{path}#entry-{entry_id}")
}
