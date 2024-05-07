use std::sync::Arc;

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use tera::Tera;

use crate::{config::AppConfig, routes};

#[derive(Clone)]
pub struct AppState {
    tera: Arc<Tera>,
}

impl AppState {
    pub fn render(
        &self,
        template_name: &str,
        context: &tera::Context,
    ) -> Result<Html<String>, TemplateError> {
        match self.tera.render(template_name, context) {
            Ok(s) => Ok(Html(s)),
            Err(err) => Err(TemplateError(err)),
        }
    }
}

pub struct TemplateError(tera::Error);

// Inspired by https://github.com/Altair-Bueno/axum-template/blob/main/src/engine/tera.rs
impl IntoResponse for TemplateError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()).into_response()
    }
}

pub fn mkapp(_conf: AppConfig) -> Result<Router, String> {
    let state = init_state()?;

    let router = Router::new()
        .route("/", get(routes::home))
        .with_state(state);
    Ok(router)
}

fn init_state() -> Result<AppState, String> {
    let tera = init_tera()?;
    let state = AppState {
        tera: Arc::new(tera),
    };
    Ok(state)
}

fn init_tera() -> Result<Tera, String> {
    Tera::new("templates/**/*.html").map_err(|err| format!("Failed to initialize tera: {err}"))
}
