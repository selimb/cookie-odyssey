use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use minijinja::context;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::{AppState, RouteResult, Templ};
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
    let ctx = context! { journal };
    let html = templ.render_ctx("journal_detail.html", ctx)?;
    Ok(html.into_response())
}
