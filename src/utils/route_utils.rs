use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};
use thiserror::Error;
use tracing::error;

// Inspired by https://users.rust-lang.org/t/need-help-with-askama-axum-error-handling/108791/7
#[derive(Error, Debug)]
pub enum RouteError {
    #[error("template error: {0:?}")]
    TemplateError(#[from] tera::Error),
    #[error("db error: {0:?}")]
    DbError(#[from] sea_orm::DbErr),
    #[error("axum error: {0:?}")]
    AxumError(#[from] axum::http::Error),
    #[error("something went wrong: {0:?}")]
    Anyhow(#[from] anyhow::Error),
}

impl IntoResponse for RouteError {
    fn into_response(self) -> axum::response::Response {
        error!("Unhandled error: {self:#?}");
        // FIXME test
        // FIXME improve
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

pub type HtmlResult = Result<Html<String>, RouteError>;
