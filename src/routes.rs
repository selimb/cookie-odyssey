use axum::{extract::State, response::IntoResponse};

use crate::server::AppState;

pub async fn home(State(state): State<AppState>) -> impl IntoResponse {
    state.render("index.html", &Default::default())
}
