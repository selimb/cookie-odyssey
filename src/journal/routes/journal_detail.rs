use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use minijinja::context;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::{AppState, NotFound, Route, RouteResult, Templ};
use entities::{prelude::*, *};

pub async fn journal_detail_get(
    state: State<AppState>,
    templ: Templ,
    Path(slug): Path<String>,
) -> RouteResult {
    let journal = Journal::find()
        .filter(journal::Column::Slug.eq(slug))
        .one(&state.db)
        .await?;
    let journal = match journal {
        Some(journal) => journal,
        None => {
            let resp = NotFound::for_entity("journal").render(&templ)?;
            return Ok(resp.into_response());
        }
    };

    let href_journal_entry_new = Route::JournalEntryNewGet {
        slug: Some(&journal.slug),
    }
    .as_path();

    let ctx = context! { journal, href_journal_entry_new };
    let html = templ.render_ctx("journal_detail.html", ctx)?;
    Ok(html.into_response())
}

async fn group_by_day() {
    todo!()
}
