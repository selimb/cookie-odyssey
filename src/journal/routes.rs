use axum::{
    extract::{Request, State},
    http::Method,
    Form,
};
use sea_orm::{EntityTrait, QueryOrder};
use serde::Deserialize;
use tera::Context;

use crate::{server::AppState, utils::route_utils::HtmlResult};
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

#[derive(Deserialize, Debug)]
pub struct JournalNew {
    pub name: String,
}

pub async fn journal_new(
    State(state): State<AppState>,
    method: Method,
    Form(form): Form<JournalNew>,
) -> HtmlResult {
    println!("journal_new method: {}", method);
    let mut context = Context::new();
    let resp = state.render("journal_new.html", &context)?;
    Ok(resp)
}
