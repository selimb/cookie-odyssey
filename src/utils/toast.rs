use app_config::AppEnv;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::json;
use tracing::error;

// [toast]
static TOAST_EVT: &str = "app.toast";

#[derive(Serialize)]
pub struct Toast {
    message: String,
    variant: String,
    error: bool,
    auto_close: bool,
}

impl Toast {
    pub fn error(err: impl std::error::Error) -> Self {
        let message = if AppEnv::is_dev() {
            err.to_string()
        } else {
            "Something went wrong".to_string()
        };
        error!("Unhandled error: {err:#?}\n{message:#?}");
        Self {
            message,
            variant: "error".into(),
            auto_close: false,
            error: true,
        }
    }

    pub fn danger(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            variant: "error".into(),
            auto_close: false,
            error: false,
        }
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            variant: "success".into(),
            auto_close: true,
            error: false,
        }
    }

    pub fn into_headers(&self) -> [(String, String); 1] {
        let trigger = json!({
            TOAST_EVT: &self,
        })
        .to_string();

        [("HX-Trigger".to_string(), trigger)]
    }
}

impl IntoResponse for Toast {
    fn into_response(self) -> Response {
        let status = if self.error {
            StatusCode::INTERNAL_SERVER_ERROR
        } else {
            StatusCode::OK
        };
        let resp = (status, [("HX-Reswap", "none")], self.into_headers());
        resp.into_response()
    }
}
