use axum::extract::State;
use sea_orm::{EntityTrait, QueryOrder};
use tera::Context;

use crate::{route_utils::HtmlResult, server::AppState};
use entities::{prelude::*, *};

// FIXME auth
pub async fn journal_list(State(state): State<AppState>) -> HtmlResult {
    let journals = Journal::find()
        .find_also_related(File)
        .order_by_desc(journal::Column::StartDate)
        .all(&state.db)
        .await?;
    let mut context = Context::new();
    context.insert("journals", &journals);
    let resp = state.render("journal_list.html", &context)?;
    Ok(resp)
}
