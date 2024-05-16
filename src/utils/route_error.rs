use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;
use tracing::error;

use app_config::AppEnv;

// Inspired by https://users.rust-lang.org/t/need-help-with-askama-axum-error-handling/108791/7
#[derive(Error, Debug)]
pub enum RouteError {
    #[error("template error: {0:?}")]
    TemplateError(#[from] minijinja::Error),
    #[error("db error: {0:?}")]
    DbError(#[from] sea_orm::DbErr),
    #[error("axum error: {0:?}")]
    AxumError(#[from] axum::http::Error),
    #[error("anyhow error: {0:?}")]
    Anyhow(#[from] anyhow::Error),
    #[error("other: {0:?}")]
    Other(String),
}

impl IntoResponse for RouteError {
    fn into_response(self) -> axum::response::Response {
        error!("Unhandled error: {self:#?}");
        let body = match AppEnv::is_dev() {
            false => self.to_string(),
            true => "Something went wrong".to_string(),
        };
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

pub type RouteResult = Result<Response, RouteError>;
