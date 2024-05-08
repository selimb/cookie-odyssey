use crate::{journal::routes as journal, server::AppState};
use axum::{routing::get, Router};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(journal::journal_list))
        .with_state(state)
}
