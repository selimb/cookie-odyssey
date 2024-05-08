use axum::{extract::State, response::IntoResponse};

use crate::server::AppState;

pub async fn journal_list(State(state): State<AppState>) -> impl IntoResponse {
    state.render("journal_list.html", &Default::default())
}
