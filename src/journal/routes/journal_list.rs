use axum::{extract::State, response::IntoResponse};
use minijinja::context;
use sea_orm::{EntityTrait, QueryOrder};
use serde::Serialize;
use url::Url;

use crate::{utils::date_utils::date_from_sqlite, AppState, Route, RouteResult, Templ};
use entities::{prelude::*, *};

#[derive(Serialize, Debug)]
struct JournalListItem {
    id: i32,
    name: String,
    start_date: chrono::NaiveDateTime,
    end_date: Option<chrono::NaiveDateTime>,
    href: String,
    cover_url: Option<Url>,
}

pub async fn journal_list(State(state): State<AppState>, templ: Templ) -> RouteResult {
    let journals = Journal::find()
        .find_also_related(File)
        .order_by_desc(journal::Column::StartDate)
        .all(&state.db)
        .await?;
    let journals = journals
        .into_iter()
        .map(|(journal, _cover)| {
            let start_date = chrono::NaiveDateTime::new(
                date_from_sqlite(journal.start_date).unwrap(),
                Default::default(),
            );
            let end_date = journal.end_date.map(|d| {
                chrono::NaiveDateTime::new(date_from_sqlite(d).unwrap(), Default::default())
            });
            JournalListItem {
                id: journal.id,
                name: journal.name,
                start_date,
                end_date,
                cover_url: None,
                href: Route::JournalDetailGet {
                    slug: Some(&journal.slug),
                }
                .as_path()
                .into_owned(),
            }
        })
        .collect::<Vec<_>>();
    let ctx = context! { journals, href_new => Route::JournalNewGet.as_path() };
    let html = templ.render_ctx("journal_list.html", ctx)?;
    Ok(html.into_response())
}
