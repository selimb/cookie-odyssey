use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};
use minijinja::context;

use crate::{AppState, RouteError};

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
