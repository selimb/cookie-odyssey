use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use minijinja::context;

use crate::{journal::queries::query_journal_by_slug, AppState, Route, RouteResult, Templ};

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
