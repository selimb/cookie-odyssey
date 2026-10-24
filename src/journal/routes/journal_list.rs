use axum::{extract::State, response::IntoResponse};
use minijinja::context;
use sea_orm::{EntityTrait, QueryOrder};
use serde::Serialize;

use crate::{AppState, Route, RouteResult, Templ};
use entities::{prelude::*, *};

#[derive(Serialize, Debug)]
struct JournalListItem {
    id: i32,
    name: String,
    start_date: chrono::NaiveDateTime,
    end_date: Option<chrono::NaiveDateTime>,
    href: String,
    cover_url: Option<String>,
}

pub async fn page_journal_list_get(State(state): State<AppState>, templ: Templ) -> RouteResult {
    let journals = Journal::find()
        .find_also_related(File)
        .order_by_desc(journal::Column::StartDate)
        .all(&state.db)
        .await?;
    let mut items = Vec::with_capacity(journals.len());
    for (journal, cover) in journals {
        let cover_url = match cover {
            None => None,
            Some(cover) => Some(state.storage.sign_url(cover.bucket, cover.key).await?),
        };
        // See [date-as-datetime].
        let start_date = chrono::NaiveDateTime::new(journal.start_date, Default::default());
        let end_date = journal
            .end_date
            .map(|d| chrono::NaiveDateTime::new(d, Default::default()));
        items.push(JournalListItem {
            id: journal.id,
            name: journal.name,
            start_date,
            end_date,
            cover_url,
            href: Route::JournalDetailGet {
                slug: Some(&journal.slug),
            }
            .as_path()
            .into_owned(),
        });
    }
    let ctx = context! { journals => items, href_new => Route::JournalNewGet.as_path() };
    let html = templ.render_ctx("journal_list.html", ctx)?;
    Ok(html.into_response())
}
