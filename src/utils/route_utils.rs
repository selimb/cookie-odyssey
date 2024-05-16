use app_config::AppEnv;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use minijinja::context;
use serde::Serialize;
use serde_json::json;
use thiserror::Error;
use tracing::error;

use crate::AppState;

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
            true => "Something went wrong".to_string(),
            false => self.to_string(),
        };
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

pub type RouteResult = Result<Response, RouteError>;

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
        let ctx = context! { error => &self.msg };
        let body = state
            .template_engine
            .get_template("common/form_error.html")?
            .render(ctx)?;

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

#[derive(Serialize)]
pub struct Toast {
    message: String,
    class: String,
}

impl Toast {
    pub fn error(err: impl std::error::Error) -> Self {
        error!("Unhandled error: {err:#?}");
        let message = if AppEnv::is_dev() {
            err.to_string()
        } else {
            "Something went wrong".to_string()
        };
        Self::danger(message)
    }

    pub fn danger(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: "is-danger".into(),
        }
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            class: "is-success".into(),
        }
    }

    pub fn into_headers(&self) -> [(String, String); 1] {
        // See [toast] for the handler.
        let trigger = json!({
            "app.toast": &self,
        })
        .to_string();

        [("HX-Trigger".to_string(), trigger)]
    }
}

impl IntoResponse for Toast {
    fn into_response(self) -> Response {
        let resp = (StatusCode::OK, [("HX-Reswap", "None")], self.into_headers());
        resp.into_response()
    }
}
