use app_config::AppEnv;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};
use thiserror::Error;
use tracing::error;

use crate::AppState;

// Inspired by https://users.rust-lang.org/t/need-help-with-askama-axum-error-handling/108791/7
#[derive(Error, Debug)]
pub enum RouteError {
    #[error("template error: {0:?}")]
    TemplateError(#[from] tera::Error),
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
            true => "Something went wrong".to_string(),
            false => self.to_string(),
        };
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

pub type HtmlResult = Result<Html<String>, RouteError>;

pub struct FormError {
    msg: String,
    status: StatusCode,
}

impl FormError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            status: StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    pub fn render(&self, state: &AppState) -> Result<impl IntoResponse, RouteError> {
        let mut context = tera::Context::new();
        context.insert("error", &self.msg);
        let body = state.tera.render("common/form_error.html", &context)?;
        Ok((
            self.status,
            [
                ("HX-Reswap", "outerHTML"),
                // Matches [form-errors-id]
                ("HX-Retarget", "find #form_error"),
            ],
            Html(body),
        ))
    }
}

impl From<axum::extract::rejection::FormRejection> for FormError {
    fn from(value: axum::extract::rejection::FormRejection) -> Self {
        FormError::new(&value.body_text())
    }
}

pub struct PermissionDenied {
    msg: String,
}

impl PermissionDenied {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }

    pub fn render(&self, state: &AppState) -> Result<impl IntoResponse, RouteError> {
        let mut context = tera::Context::new();
        context.insert("msg", &self.msg);
        let body = state.tera.render("oops.html", &context)?;
        Ok(Html(body))
    }
}
