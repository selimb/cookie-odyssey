use axum::{
    extract::{Path, State},
    response::IntoResponse as _,
};
use minijinja::context;

use crate::{
    comment::{queries::CommentScope, routes::CommentList},
    journal::queries::{query_cards, query_journal_by_slug, CardView},
    AppState, AuthSession, Route, RouteResult, Templ,
};

use super::{JournalEntryNewPath, JournalEntryNewQuery};

pub async fn page_journal_detail_get(
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

    // [cards-newest-first] Newest first.
    let cards: Vec<CardView> = query_cards(&journal, &state.db, &session)
        .await?
        .iter()
        .rev()
        .map(|card| CardView::new(card, &journal))
        .collect();
    let comments = CommentList {
        scope: CommentScope {
            journal_id: journal.id,
            date: None,
            getaway_id: None,
        },
        partial: false,
    }
    .query_and_render(&state.db, &templ, &session)
    .await?;

    let href_journal_entry_new = Route::JournalEntryNewGet(Some((
        &JournalEntryNewPath { slug: slug.clone() },
        &JournalEntryNewQuery::default(),
    )))
    .as_path();

    let href_journal_edit = Route::JournalEditGet { slug: Some(&slug) }.as_path();

    let ctx = context! {
        journal,
        href_journal_edit,
        cards,
        href_journal_entry_new,
        comments_fragment => comments.0,
    };
    let html = templ.render_ctx("journal_detail.html", ctx)?;
    Ok(html.into_response())
}
