use axum::response::IntoResponse;

use crate::{RouteResult, Templ};

pub async fn demo_thumbnail_get(templ: Templ) -> RouteResult {
    let html = templ.render("demo/thumbnail-demo.html")?;

    Ok(html.into_response())
}
