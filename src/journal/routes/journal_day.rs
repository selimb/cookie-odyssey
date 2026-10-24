use axum::{extract::Path, response::IntoResponse as _};
use minijinja::context;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use crate::{
    auth::utils::unwrap_user_from_session,
    comment::{queries::CommentScope, routes::CommentList},
    journal::{
        queries::{
            day_number, query_entry_views, query_journal_by_slug, query_neighbours, CardKey,
        },
        routes::{JournalEntryNewPath, JournalEntryNewQuery},
    },
    notification::queries::{mark_read, ReadScope},
    AppState, AuthSession, Route, RouteResult, Templ,
};
use entities::{prelude::*, *};

#[derive(Debug, Deserialize)]
pub struct JournalDayGetPath {
    pub slug: String,
    pub date: chrono::NaiveDate,
}

pub async fn page_journal_day_get(
    state: AppState,
    mut templ: Templ,
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

    // Viewing a Day marks its Notifications as read, regardless of how the User
    // got here.
    let user_id = unwrap_user_from_session(&session).0.id;
    mark_read(
        &state.db,
        user_id,
        ReadScope::Day {
            journal_id: journal.id,
            date,
        },
    )
    .await?;
    templ.refresh_badges(&state.db).await?;

    // In a Scrapbook, the Entries of a Getaway are shown with the Getaway.
    // In a Journal, no Entry has a Getaway, see [journal-no-getaway].
    let entries_q = JournalEntry::find()
        .filter(journal_entry::Column::JournalId.eq(journal.id))
        .filter(journal_entry::Column::Date.eq(date))
        .filter(journal_entry::Column::GetawayId.is_null());
    let entries = query_entry_views(entries_q, &state.db, &session, &state.storage).await?;
    let comments = CommentList {
        scope: CommentScope {
            journal_id: journal.id,
            date: Some(date),
            getaway_id: None,
        },
        partial: false,
    }
    .query_and_render(&state.db, &templ, &session)
    .await?;

    let (nav_prev, nav_next) =
        query_neighbours(&journal, &CardKey::day(date), &state.db, &session).await?;

    // See [date-as-datetime].
    let datetime = chrono::NaiveDateTime::new(date, Default::default());
    let day_number = day_number(&journal, date);
    let href_journal_detail = Route::JournalDetailGet { slug: Some(&slug) }.as_path();
    let href_journal_entry_new = Route::JournalEntryNewGet(Some((
        &JournalEntryNewPath { slug: slug.clone() },
        &JournalEntryNewQuery {
            date: Some(date),
            getaway_id: None,
        },
    )))
    .as_path();

    let ctx = context! {
        journal,
        datetime,
        day_number,
        entries,
        comments_fragment => comments.0,
        href_journal_detail,
        href_journal_entry_new,
        nav_prev,
        nav_next,
    };

    let html = templ.render_ctx("journal_day.html", ctx)?;
    Ok(html.into_response())
}
