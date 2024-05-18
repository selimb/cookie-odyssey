use axum::{extract::State, response::IntoResponse};
use minijinja::context;
use sea_orm::{EntityTrait, QueryOrder};
use serde::Serialize;
use url::Url;

use crate::{AppState, Route, RouteResult, Templ};
use entities::{prelude::*, *};

#[derive(Serialize, Debug)]
struct JournalListItem {
    id: i32,
    name: String,
    start_date: String,
    end_date: Option<String>,
    href: String,
    cover_url: Option<Url>,
}

// FIXME auth
pub async fn journal_list(State(state): State<AppState>, templ: Templ) -> RouteResult {
    let journals = Journal::find()
        .find_also_related(File)
        .order_by_desc(journal::Column::StartDate)
        .all(&state.db)
        .await?;
    let journals = journals
        .into_iter()
        .map(|(journal, _cover)| JournalListItem {
            id: journal.id,
            name: journal.name,
            start_date: journal.start_date,
            end_date: journal.end_date,
            cover_url: None,
            href: Route::JournalDetailGet {
                slug: Some(&journal.slug),
            }
            .as_path()
            .into_owned(),
        })
        .collect::<Vec<_>>();
    let ctx = context! { journals, href_new => Route::JournalNewGet.as_path() };
    let html = templ.render_ctx("journal_list.html", ctx)?;
    Ok(html.into_response())
}
