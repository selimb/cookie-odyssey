use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};
use thiserror::Error;
use tracing::error;

// Inspired by https://users.rust-lang.org/t/need-help-with-askama-axum-error-handling/108791/7
#[derive(Error, Debug)]
pub enum RouteError {
    #[error("template error")]
    TemplateError(#[from] tera::Error),
    #[error("db error")]
    DbError(#[from] sea_orm::DbErr),
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
